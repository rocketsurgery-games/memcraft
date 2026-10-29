//! Java Anvil chunk and region encoding (pre-1.13 numeric-id format).

use std::io::Write;

use mcpe_world::{ChunkPos, Column, HEIGHT};
use nbt::{compound, Endian, Tag};

use crate::blocks;
use crate::light::Lit;

/// PE biome ids match Java's numbering; PE stores one per column.
fn biomes(col: &Column) -> Vec<i8> {
    let b = col.biomes.unwrap_or([1; 256]); // plains
    b.iter().map(|&v| v as i8).collect()
}

fn pack_nibbles(src: impl Iterator<Item = u8>) -> Vec<i8> {
    let mut out = vec![0i8; 2048];
    for (i, v) in src.enumerate() {
        out[i >> 1] |= ((v & 0x0f) << ((i & 1) * 4)) as i8;
    }
    out
}

/// Builds the NBT of one Java chunk from a PE column and its computed light.
pub fn chunk(col: &Column, lit: &Lit, tile_entities: Vec<Tag>, data_version: i32) -> Tag {
    // Java section arrays are indexed y*256 + z*16 + x; PE columns are x, z, y.
    let src = |x: usize, y: usize, z: usize| mcpe_world::Column::index(x, y, z);
    let order = |sy: usize| {
        (0..4096).map(move |i| {
            let (y, z, x) = (sy * 16 + i / 256, (i / 16) % 16, i % 16);
            (x, y, z)
        })
    };

    let mut sections = Vec::new();
    for sy in 0..HEIGHT / 16 {
        let mut ids = Vec::with_capacity(4096);
        let mut data = Vec::with_capacity(4096);
        for (x, y, z) in order(sy) {
            let (id, d) = blocks::to_java(col.ids[src(x, y, z)], col.data[src(x, y, z)]);
            ids.push(id as i8);
            data.push(d);
        }
        if ids.iter().all(|&b| b == 0) {
            continue;
        }
        sections.push(compound([
            ("Y", Tag::Byte(sy as i8)),
            ("Blocks", Tag::ByteArray(ids)),
            ("Data", Tag::ByteArray(pack_nibbles(data.into_iter()))),
            ("SkyLight", Tag::ByteArray(pack_nibbles(order(sy).map(|(x, y, z)| lit.sky[src(x, y, z)])))),
            ("BlockLight", Tag::ByteArray(pack_nibbles(order(sy).map(|(x, y, z)| lit.block[src(x, y, z)])))),
        ]));
    }

    let level = compound([
        ("xPos", Tag::Int(col.pos.x)),
        ("zPos", Tag::Int(col.pos.z)),
        ("LastUpdate", Tag::Long(0)),
        ("InhabitedTime", Tag::Long(0)),
        ("TerrainPopulated", Tag::Byte(1)),
        ("LightPopulated", Tag::Byte(1)),
        ("V", Tag::Byte(1)),
        ("Biomes", Tag::ByteArray(biomes(col))),
        ("HeightMap", Tag::IntArray(lit.height.to_vec())),
        ("Sections", Tag::List(10, sections)),
        ("Entities", Tag::List(10, vec![])),
        ("TileEntities", Tag::List(10, tile_entities)),
    ]);
    compound([("DataVersion", Tag::Int(data_version)), ("Level", level)])
}

/// One 32x32-chunk region file.
#[derive(Default)]
pub struct Region {
    chunks: Vec<(usize, Vec<u8>)>,
}

impl Region {
    pub fn set(&mut self, pos: ChunkPos, chunk: Tag) {
        let slot = (pos.x.rem_euclid(32) + pos.z.rem_euclid(32) * 32) as usize;
        let mut raw = Vec::new();
        nbt::write(&mut raw, "", &chunk, Endian::Big);
        let mut z = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        z.write_all(&raw).unwrap();
        self.chunks.push((slot, z.finish().unwrap()));
    }

    /// Encodes the `.mca` file: an 8 KiB header (sector offsets, timestamps)
    /// then each chunk padded to 4 KiB sectors as `len: u32be, 2 (zlib), data`.
    pub fn encode(mut self) -> Vec<u8> {
        self.chunks.sort_by_key(|c| c.0);
        let mut out = vec![0u8; 8192];
        for (slot, data) in self.chunks {
            let offset = out.len() / 4096;
            let len = data.len() + 1;
            out.extend((len as u32).to_be_bytes());
            out.push(2);
            out.extend(data);
            out.resize(out.len().next_multiple_of(4096), 0);
            let sectors = out.len() / 4096 - offset;
            assert!(sectors < 256, "chunk too large for a region sector run");
            let entry = ((offset as u32) << 8) | sectors as u32;
            out[slot * 4..slot * 4 + 4].copy_from_slice(&entry.to_be_bytes());
        }
        out
    }
}
