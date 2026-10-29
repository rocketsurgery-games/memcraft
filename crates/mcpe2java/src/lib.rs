//! Converts a LevelDB-era Pocket Edition world into a Java Edition 1.12.2
//! Anvil world.
//!
//! Java 1.12.2 is the last version that still used numeric block ids, which
//! Pocket Edition 0.16 shares almost exactly, so the conversion is close to
//! lossless for terrain. The output is a set of files (`level.dat` and
//! `region/r.X.Z.mca`) that [`write_dir`] or [`write_zip`] can save.

mod anvil;
pub mod blocks;
mod entities;
mod light;

use std::collections::{BTreeMap, HashMap};
use std::io::Write;
use std::path::Path;

use mcpe_world::{ChunkPos, World};
use nbt::{compound, Endian, Tag};

/// Java data version of 1.12.2.
const DATA_VERSION: i32 = 1343;

/// What the conversion produced and dropped.
#[derive(Debug, Default)]
pub struct Report {
    pub chunks: usize,
    pub block_entities: usize,
    /// Block entity ids we did not convert, with counts.
    pub skipped_block_entities: BTreeMap<String, usize>,
}

/// A converted world: relative path -> file contents.
pub struct JavaWorld {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: Report,
}

pub fn convert(world: &World) -> JavaWorld {
    let mut report = Report::default();

    let ids: HashMap<ChunkPos, Vec<u8>> = world
        .columns
        .iter()
        .map(|(pos, col)| (*pos, col.ids.iter().zip(&col.data).map(|(&i, &d)| blocks::to_java(i, d).0).collect()))
        .collect();
    let lit = light::light(&ids);

    let mut regions: BTreeMap<(i32, i32), anvil::Region> = BTreeMap::new();
    for (pos, col) in &world.columns {
        let mut block_entities = Vec::new();
        for be in &col.block_entities {
            match entities::block_entity(be) {
                Some(t) => {
                    block_entities.push(t);
                    report.block_entities += 1;
                }
                None => {
                    let id = be.get("id").and_then(Tag::as_str).unwrap_or("?").to_string();
                    *report.skipped_block_entities.entry(id).or_default() += 1;
                }
            }
        }
        let chunk = anvil::chunk(col, &lit[pos], block_entities, DATA_VERSION);
        regions.entry((pos.x.div_euclid(32), pos.z.div_euclid(32))).or_default().set(*pos, chunk);
        report.chunks += 1;
    }

    // Surround the world with empty region files. Readers on a read-only
    // filesystem (e.g. the browser client opening a zip) fail on a missing
    // region file, but treat an empty one as "no chunk here".
    let occupied: Vec<(i32, i32)> = regions.keys().copied().collect();
    for (rx, rz) in occupied {
        for (dx, dz) in [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)] {
            regions.entry((rx + dx, rz + dz)).or_default();
        }
    }

    let mut files = BTreeMap::new();
    for ((rx, rz), region) in regions {
        files.insert(format!("region/r.{rx}.{rz}.mca"), region.encode());
    }
    files.insert("level.dat".into(), level_dat(world));
    JavaWorld { files, report }
}

/// Builds Java `level.dat` (gzipped big-endian NBT) from the PE level and player.
fn level_dat(world: &World) -> Vec<u8> {
    let level = &world.level;
    let int = |k: &str| level.get(k).and_then(Tag::as_i64).unwrap_or(0) as i32;
    let long = |k: &str| level.get(k).and_then(Tag::as_i64).unwrap_or(0);
    let game_type = int("GameType");
    let (sx, sy, sz) = world.spawn();

    let mut data = vec![
        ("version", Tag::Int(19133)),
        ("DataVersion", Tag::Int(DATA_VERSION)),
        (
            "Version",
            compound([
                ("Id", Tag::Int(DATA_VERSION)),
                ("Name", Tag::String("1.12.2".into())),
                ("Snapshot", Tag::Byte(0)),
            ]),
        ),
        ("initialized", Tag::Byte(1)),
        ("LevelName", Tag::String(world.name().unwrap_or("Converted world").into())),
        // A void generator: nothing is generated past the old world edge.
        ("generatorName", Tag::String("flat".into())),
        ("generatorVersion", Tag::Int(0)),
        ("generatorOptions", Tag::String("3;minecraft:air;1;".into())),
        ("RandomSeed", Tag::Long(long("RandomSeed"))),
        ("MapFeatures", Tag::Byte(0)),
        ("GameType", Tag::Int(game_type)),
        ("hardcore", Tag::Byte(0)),
        ("allowCommands", Tag::Byte(1)),
        ("Difficulty", Tag::Byte(int("Difficulty") as i8)),
        ("SpawnX", Tag::Int(sx)),
        ("SpawnY", Tag::Int(sy)),
        ("SpawnZ", Tag::Int(sz)),
        ("Time", Tag::Long(long("currentTick"))),
        ("DayTime", Tag::Long(long("Time"))),
        ("LastPlayed", Tag::Long(long("LastPlayed") * 1000)),
        ("raining", Tag::Byte(0)),
        ("thundering", Tag::Byte(0)),
        // Old limited worlds end at a hard edge; keep players inside it.
        ("BorderCenterX", Tag::Double(128.0)),
        ("BorderCenterZ", Tag::Double(128.0)),
        ("BorderSize", Tag::Double(256.0)),
        (
            "GameRules",
            compound([
                ("doMobSpawning", Tag::String("false".into())),
                ("doFireTick", Tag::String("false".into())),
                ("doDaylightCycle", Tag::String("true".into())),
            ]),
        ),
    ];
    if let Some(p) = world.players.get("~local_player") {
        data.push(("Player", entities::player(p, game_type)));
    }
    let root = compound([("Data", compound(data))]);
    let mut raw = Vec::new();
    nbt::write(&mut raw, "", &root, Endian::Big);
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&raw).unwrap();
    gz.finish().unwrap()
}

impl JavaWorld {
    /// Writes the world under `dir` (which becomes the world folder).
    pub fn write_dir(&self, dir: &Path) -> std::io::Result<()> {
        for (rel, bytes) in &self.files {
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, bytes)?;
        }
        Ok(())
    }

    /// Writes the world as a zip with `level.dat` at its root.
    pub fn write_zip(&self, out: impl Write + std::io::Seek) -> zip::result::ZipResult<()> {
        let mut z = zip::ZipWriter::new(out);
        // Region files are already zlib-compressed per chunk.
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (rel, bytes) in &self.files {
            z.start_file(rel.as_str(), opts)?;
            z.write_all(bytes)?;
        }
        z.finish()?;
        Ok(())
    }
}
