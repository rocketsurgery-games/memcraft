//! Reader for LevelDB-era Minecraft: Pocket Edition worlds.
//!
//! Pocket Edition 0.9 through 0.17 stored worlds in Mojang's LevelDB fork,
//! with chunk terrain in one of two layouts:
//!
//! - `LegacyTerrain` (key tag `0x30`, PE 0.9 - 0.16): one 16x16x128 column.
//! - `SubChunkPrefix` version 0 (key tag `0x2f`, PE 0.17 / Bedrock 1.0):
//!   16x16x16 sections. A world upgraded in place can hold both layouts at
//!   once, chunk by chunk.
//!
//! Both use pre-flattening numeric block ids plus a 4-bit data value. This
//! crate normalizes them into one [`Column`] representation. Later Bedrock
//! formats (paletted sub-chunks, 1.2+) are out of scope.
//!
//! The database is opened from an in-memory copy of its files, so the source
//! world is never modified and the same code can run where there is no
//! filesystem (WebAssembly).
//!
//! # Writing
//!
//! [`WorldWriter`] is the inverse: it opens the same in-memory copy for
//! writing, [`put_column`](WorldWriter::put_column)s re-encoded columns as the
//! records Pocket Edition reads (each column keeps the layout it was read in),
//! and [`finish`](WorldWriter::finish)es into the files of a world, which can be
//! written to a directory ([`SavedWorld::write_dir`]) or handed to a zip
//! writer. `level.dat` and every record that is not re-put are left as they
//! were.
//!
//! ```no_run
//! # fn main() -> mcpe_world::Result<()> {
//! use mcpe_world::{World, WorldWriter};
//!
//! let mut world = World::open_dir("worlds/My World")?;
//! let col = world.columns.values_mut().next().unwrap();
//! col.ids[mcpe_world::Column::index(0, 100, 0)] = 1; // a stone block
//! col.recompute_heights();
//!
//! let mut writer = WorldWriter::open_dir("worlds/My World")?;
//! writer.put_column(col)?;
//! writer.finish()?.write_dir("saved/My World")?;
//! # Ok(()) }
//! ```

mod db;
mod encode;
mod key;
mod writer;

pub use key::{ChunkPos, Dimension};
pub use writer::{SavedWorld, WorldWriter};

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use nbt::{Endian, Tag};

/// Height of an old Pocket Edition world, in blocks.
pub const HEIGHT: usize = 128;
/// Blocks in one 16x16xHEIGHT column.
pub const COLUMN_BLOCKS: usize = 16 * 16 * HEIGHT;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Db(String),
    Nbt(nbt::Error),
    Format(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Db(e) => write!(f, "leveldb: {e}"),
            Error::Nbt(e) => write!(f, "{e}"),
            Error::Format(e) => write!(f, "world format: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<nbt::Error> for Error {
    fn from(e: nbt::Error) -> Self {
        Error::Nbt(e)
    }
}

impl From<rusty_leveldb::Status> for Error {
    fn from(e: rusty_leveldb::Status) -> Self {
        Error::Db(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Which on-disk layout a column was decoded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TerrainFormat {
    LegacyTerrain,
    SubChunkV0,
}

/// One 16x16x128 chunk column, normalized from either terrain layout.
///
/// Block arrays are indexed by [`Column::index`] (y fastest, then z, then x,
/// the same order Pocket Edition used on disk).
#[derive(Clone, PartialEq)]
pub struct Column {
    pub pos: ChunkPos,
    pub format: TerrainFormat,
    /// Numeric block id per block.
    pub ids: Vec<u8>,
    /// Block data value (0..=15) per block, unpacked from nibbles.
    pub data: Vec<u8>,
    /// Sky light (0..=15) per block, when the source stored it.
    pub sky_light: Option<Vec<u8>>,
    /// Block light (0..=15) per block, when the source stored it.
    pub block_light: Option<Vec<u8>>,
    /// Biome id per (x, z), indexed `z * 16 + x`.
    pub biomes: Option<[u8; 256]>,
    /// Grass tint per (x, z), indexed `z * 16 + x`, as `0xRRGGBB`. Only
    /// `LegacyTerrain` records store it, next to the biome id.
    pub grass_colors: Option<[u32; 256]>,
    /// Height per (x, z), indexed `z * 16 + x`: the y above the highest
    /// non-air block. Kept as stored; [`Column::recompute_heights`] refreshes
    /// it after editing blocks.
    pub heights: Option<[u8; 256]>,
    /// Block entities (chests, signs, ...), Bedrock NBT.
    pub block_entities: Vec<Tag>,
    /// Entities (mobs, items, ...), Bedrock NBT.
    pub entities: Vec<Tag>,
}

impl Column {
    /// Index into the per-block arrays for local coordinates.
    pub fn index(x: usize, y: usize, z: usize) -> usize {
        debug_assert!(x < 16 && z < 16 && y < HEIGHT);
        (x * 16 + z) * HEIGHT + y
    }

    pub fn block(&self, x: usize, y: usize, z: usize) -> (u8, u8) {
        let i = Self::index(x, y, z);
        (self.ids[i], self.data[i])
    }

    /// Sets [`heights`](Column::heights) from the blocks: per (x, z), the y
    /// above the highest block that is not [`passes_sky_light`] (0 if none).
    pub fn recompute_heights(&mut self) {
        self.heights = Some(std::array::from_fn(|i| {
            let (x, z) = (i % 16, i / 16);
            let base = Column::index(x, 0, z);
            let top = self.ids[base..base + HEIGHT].iter().rposition(|&id| !passes_sky_light(id));
            top.map_or(0, |y| y as u8 + 1)
        }));
    }

    fn empty(pos: ChunkPos, format: TerrainFormat) -> Self {
        Column {
            pos,
            format,
            ids: vec![0; COLUMN_BLOCKS],
            data: vec![0; COLUMN_BLOCKS],
            sky_light: None,
            block_light: None,
            biomes: None,
            grass_colors: None,
            heights: None,
            block_entities: Vec::new(),
            entities: Vec::new(),
        }
    }
}

impl fmt::Debug for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Column")
            .field("pos", &self.pos)
            .field("format", &self.format)
            .field("block_entities", &self.block_entities.len())
            .field("entities", &self.entities.len())
            .finish_non_exhaustive()
    }
}

/// A loaded world.
pub struct World {
    /// `level.dat` contents (Bedrock NBT root compound).
    pub level: Tag,
    /// `level.dat` header storage version.
    pub level_dat_version: i32,
    /// Overworld columns; other dimensions are not yet decoded.
    pub columns: BTreeMap<ChunkPos, Column>,
    /// Player records by key (`~local_player`, `player_<id>`).
    pub players: BTreeMap<String, Tag>,
    /// Database keys that were not decoded, for diagnostics.
    pub unknown_keys: Vec<Vec<u8>>,
}

impl World {
    /// Loads a world directory (the one containing `level.dat` and `db/`).
    pub fn open_dir(dir: impl AsRef<Path>) -> Result<World> {
        let (level_dat, db_files) = read_world_dir(dir.as_ref())?;
        World::from_files(&level_dat, db_files)
    }

    /// Loads a world from `level.dat` bytes and the files of its `db/` folder.
    pub fn from_files(
        level_dat: &[u8],
        db_files: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<World> {
        let (level_dat_version, level) = parse_level_dat(level_dat)?;
        let mut world = World {
            level,
            level_dat_version,
            columns: BTreeMap::new(),
            players: BTreeMap::new(),
            unknown_keys: Vec::new(),
        };
        let mut records: BTreeMap<ChunkPos, Vec<(key::ChunkTag, Vec<u8>)>> = BTreeMap::new();
        db::for_each_entry(db_files, |k, v| {
            match key::parse(&k) {
                key::Key::Chunk { pos, dim: Dimension::Overworld, tag } => {
                    records.entry(pos).or_default().push((tag, v));
                }
                key::Key::Named(name) if name == "~local_player" || name.starts_with("player_") => {
                    // Some old player records are stubs (a 4-byte id), not NBT.
                    if let Ok((_, tag, _)) = nbt::read(&v, Endian::Little) {
                        world.players.insert(name, tag);
                    }
                }
                _ => world.unknown_keys.push(k),
            }
        })?;
        for (pos, recs) in records {
            if let Some(col) = decode_column(pos, recs)? {
                world.columns.insert(pos, col);
            }
        }
        Ok(world)
    }

    pub fn name(&self) -> Option<&str> {
        self.level.get("LevelName").and_then(Tag::as_str)
    }

    /// Spawn point from `level.dat`.
    pub fn spawn(&self) -> (i32, i32, i32) {
        let get = |k| self.level.get(k).and_then(Tag::as_i64).unwrap_or(0) as i32;
        (get("SpawnX"), get("SpawnY"), get("SpawnZ"))
    }

    /// The block at world coordinates, or air if not loaded.
    pub fn block(&self, x: i32, y: i32, z: i32) -> (u8, u8) {
        if !(0..HEIGHT as i32).contains(&y) {
            return (0, 0);
        }
        let pos = ChunkPos { x: x.div_euclid(16), z: z.div_euclid(16) };
        match self.columns.get(&pos) {
            Some(c) => c.block(x.rem_euclid(16) as usize, y as usize, z.rem_euclid(16) as usize),
            None => (0, 0),
        }
    }
}

/// Whether a block id is ignored by the heightmap: air and the non-full
/// blocks (glass, plants, rails, torches, stairs, slabs, fences, carpets, ...).
/// Leaves, water, ice, cobweb, farmland and cauldrons count as ground.
///
/// Every id of the sample world agrees with the heights Pocket Edition stored
/// there (a test checks all 256 columns); ids the sample lacks follow the same
/// rule (Java's light opacity of zero) but are unchecked.
fn passes_sky_light(id: u8) -> bool {
    matches!(
        id,
        0 | 6 | 20 | 26..=28 | 31 | 32 | 34 | 36..=40 | 44 | 50 | 51 | 53..=55 | 59 | 60 | 63..=72
            | 75..=78 | 83 | 85 | 90 | 92..=96 | 101 | 102 | 104..=109 | 111 | 113..=117 | 119 | 120 | 122
            | 126..=128 | 130..=136 | 139..=151 | 154 | 156 | 160 | 163..=167 | 171 | 175..=177
            | 182..=198 | 207
    )
}

/// The files of a world's `db/` folder, by name.
type DbFiles = Vec<(String, Vec<u8>)>;

/// Reads `level.dat` and the files of `db/` from a world directory.
fn read_world_dir(dir: &Path) -> Result<(Vec<u8>, DbFiles)> {
    let level_dat = std::fs::read(dir.join("level.dat"))?;
    let mut db_files = Vec::new();
    for entry in std::fs::read_dir(dir.join("db"))? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            db_files.push((name, std::fs::read(entry.path())?));
        }
    }
    Ok((level_dat, db_files))
}

/// Parses `level.dat`: an 8-byte header (storage version, payload length,
/// both little-endian i32) followed by a little-endian NBT compound.
pub fn parse_level_dat(bytes: &[u8]) -> Result<(i32, Tag)> {
    if bytes.len() < 8 {
        return Err(Error::Format("level.dat shorter than its header".into()));
    }
    let version = i32::from_le_bytes(bytes[0..4].try_into().unwrap());
    let (_, tag, _) = nbt::read(&bytes[8..], Endian::Little)?;
    Ok((version, tag))
}

// ids, 3 nibble arrays (data, sky light, block light), heightmap, biomes.
const LEGACY_TERRAIN_LEN: usize = COLUMN_BLOCKS * 5 / 2 + 256 + 1024;
const SUBCHUNK_V0_MIN_LEN: usize = 1 + 4096 + 2048;

fn unpack_nibbles(src: &[u8], dst: &mut [u8]) {
    for (i, d) in dst.iter_mut().enumerate() {
        let b = src[i >> 1];
        *d = if i & 1 == 0 { b & 0x0f } else { b >> 4 };
    }
}

fn decode_column(pos: ChunkPos, recs: Vec<(key::ChunkTag, Vec<u8>)>) -> Result<Option<Column>> {
    use key::ChunkTag as T;
    let mut col: Option<Column> = None;
    let mut subchunks = Vec::new();
    let mut data2d = None;
    let mut block_entities = Vec::new();
    let mut entities = Vec::new();

    for (tag, v) in recs {
        match tag {
            T::LegacyTerrain => {
                if v.len() != LEGACY_TERRAIN_LEN {
                    return Err(Error::Format(format!(
                        "LegacyTerrain at {pos:?} is {} bytes, expected {LEGACY_TERRAIN_LEN}",
                        v.len()
                    )));
                }
                col = Some(decode_legacy_terrain(pos, &v));
            }
            T::SubChunk(y) => subchunks.push((y, v)),
            T::Data2D => data2d = Some(v),
            T::BlockEntity => block_entities = nbt::read_all(&v, Endian::Little)?,
            T::Entity => entities = nbt::read_all(&v, Endian::Little)?,
            T::Other(_) => {}
        }
    }

    if col.is_none() && !subchunks.is_empty() {
        let mut c = Column::empty(pos, TerrainFormat::SubChunkV0);
        for (sy, v) in subchunks {
            decode_subchunk_v0(&mut c, pos, sy, &v)?;
        }
        if let Some(d) = data2d.filter(|d| d.len() >= 768) {
            // 256 little-endian i16 heights, then 256 biome ids.
            c.heights = Some(std::array::from_fn(|i| {
                let h = i16::from_le_bytes([d[i * 2], d[i * 2 + 1]]);
                h.clamp(0, u8::MAX as i16) as u8
            }));
            c.biomes = Some(d[512..768].try_into().unwrap());
        }
        col = Some(c);
    }

    Ok(col.map(|mut c| {
        c.block_entities = block_entities;
        c.entities = entities;
        c
    }))
}

fn decode_legacy_terrain(pos: ChunkPos, v: &[u8]) -> Column {
    let n = COLUMN_BLOCKS;
    let mut c = Column::empty(pos, TerrainFormat::LegacyTerrain);
    c.ids.copy_from_slice(&v[..n]);
    let nib = |i: usize| &v[n + i * n / 2..n + (i + 1) * n / 2];
    unpack_nibbles(nib(0), &mut c.data);
    let mut sky = vec![0; n];
    unpack_nibbles(nib(1), &mut sky);
    let mut block = vec![0; n];
    unpack_nibbles(nib(2), &mut block);
    c.sky_light = Some(sky);
    c.block_light = Some(block);
    c.heights = Some(v[n * 5 / 2..n * 5 / 2 + 256].try_into().unwrap());
    // After the 256-byte heightmap, one little-endian u32 per (x, z) column,
    // indexed z * 16 + x: biome id in the low byte, then the grass RGB tint.
    let biomes = &v[n * 5 / 2 + 256..];
    let biome_word = |i: usize| u32::from_le_bytes(biomes[i * 4..i * 4 + 4].try_into().unwrap());
    c.biomes = Some(std::array::from_fn(|i| biome_word(i) as u8));
    c.grass_colors = Some(std::array::from_fn(|i| biome_word(i) >> 8));
    c
}

fn decode_subchunk_v0(c: &mut Column, pos: ChunkPos, sy: u8, v: &[u8]) -> Result<()> {
    let bad = |why: String| Error::Format(format!("sub-chunk {sy} at {pos:?}: {why}"));
    if v.len() < SUBCHUNK_V0_MIN_LEN {
        return Err(bad(format!("{} bytes is too short", v.len())));
    }
    // Versions 0 and 2..=7 share this layout; 1 and 8+ are paletted (Bedrock 1.2+).
    if !matches!(v[0], 0 | 2..=7) {
        return Err(bad(format!("unsupported sub-chunk version {}", v[0])));
    }
    let base = sy as usize * 16;
    if base + 16 > HEIGHT {
        return Err(bad("above the 128-block world height".into()));
    }
    let ids = &v[1..4097];
    let mut data = [0u8; 4096];
    unpack_nibbles(&v[4097..4097 + 2048], &mut data);
    for x in 0..16 {
        for z in 0..16 {
            let src = (x * 16 + z) * 16;
            let dst = Column::index(x, base, z);
            c.ids[dst..dst + 16].copy_from_slice(&ids[src..src + 16]);
            c.data[dst..dst + 16].copy_from_slice(&data[src..src + 16]);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nibbles_low_first() {
        let mut out = [0u8; 4];
        unpack_nibbles(&[0x21, 0x43], &mut out);
        assert_eq!(out, [1, 2, 3, 4]);
    }

    #[test]
    fn subchunk_places_blocks_at_section_height() {
        let pos = ChunkPos { x: 0, z: 0 };
        let mut c = Column::empty(pos, TerrainFormat::SubChunkV0);
        let mut v = vec![0u8; SUBCHUNK_V0_MIN_LEN];
        // x=1, z=2, y=3 within the section.
        let i = (16 + 2) * 16 + 3;
        v[1 + i] = 42;
        v[4097 + i / 2] = 0x50; // odd index: high nibble
        decode_subchunk_v0(&mut c, pos, 2, &v).unwrap();
        assert_eq!(c.block(1, 32 + 3, 2), (42, 5));
    }
}
