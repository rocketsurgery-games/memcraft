//! Column -> records: the inverse of the decoders in `lib.rs`.
//!
//! A column is written in the layout it was read in
//! ([`TerrainFormat`](crate::TerrainFormat)), so a column that was not edited
//! encodes to the bytes it was decoded from.

use nbt::Endian;

use crate::key::{chunk_key, ChunkTag};
use crate::{Column, Error, Result, COLUMN_BLOCKS, HEIGHT, TerrainFormat};

/// One database write: `None` deletes the key.
pub struct Record {
    pub key: Vec<u8>,
    pub value: Option<Vec<u8>>,
    /// Write only if the database already has this key.
    pub if_present: bool,
}

/// Sections (16 blocks tall) per column.
const SECTIONS: usize = HEIGHT / 16;

/// Encodes `col` into every record it owns: terrain, block entities and
/// entities. An empty entity list comes back as a deletion.
pub fn encode_column(col: &Column) -> Result<Vec<Record>> {
    if col.ids.len() != COLUMN_BLOCKS || col.data.len() != COLUMN_BLOCKS {
        return Err(Error::Format(format!("column {:?}: block arrays must hold {COLUMN_BLOCKS} entries", col.pos)));
    }
    let mut out = Vec::new();
    match col.format {
        TerrainFormat::LegacyTerrain => out.push(Record {
            key: chunk_key(col.pos, ChunkTag::LegacyTerrain),
            value: Some(encode_legacy_terrain(col)?),
            if_present: false,
        }),
        TerrainFormat::SubChunkV0 => encode_subchunks(col, &mut out),
    }
    for (tag, tags) in [(ChunkTag::BlockEntity, &col.block_entities), (ChunkTag::Entity, &col.entities)] {
        let mut bytes = Vec::new();
        for t in tags {
            nbt::write(&mut bytes, "", t, Endian::Little);
        }
        out.push(Record {
            key: chunk_key(col.pos, tag),
            value: (!bytes.is_empty()).then_some(bytes),
            if_present: false,
        });
    }
    Ok(out)
}

fn pack_nibbles(src: &[u8], dst: &mut Vec<u8>) {
    dst.extend(src.chunks(2).map(|p| (p[0] & 0x0f) | (p[1] << 4)));
}

fn heights_of(col: &Column) -> [u8; 256] {
    col.heights.unwrap_or_else(|| {
        let mut c = col.clone();
        c.recompute_heights();
        c.heights.unwrap()
    })
}

fn encode_legacy_terrain(col: &Column) -> Result<Vec<u8>> {
    let light = |l: &Option<Vec<u8>>, what: &str| match l {
        Some(l) if l.len() == COLUMN_BLOCKS => Ok(l.clone()),
        _ => Err(Error::Format(format!("legacy column {:?} needs {what} for every block", col.pos))),
    };
    let sky = light(&col.sky_light, "sky light")?;
    let block = light(&col.block_light, "block light")?;

    let mut v = Vec::with_capacity(COLUMN_BLOCKS * 5 / 2 + 256 + 1024);
    v.extend_from_slice(&col.ids);
    pack_nibbles(&col.data, &mut v);
    pack_nibbles(&sky, &mut v);
    pack_nibbles(&block, &mut v);
    v.extend_from_slice(&heights_of(col));
    let biomes = col.biomes.unwrap_or([0; 256]);
    let colors = col.grass_colors.unwrap_or([0; 256]);
    for i in 0..256 {
        v.extend((biomes[i] as u32 | colors[i] << 8).to_le_bytes());
    }
    Ok(v)
}

fn encode_subchunks(col: &Column, out: &mut Vec<Record>) {
    // Sections up to the highest one holding a block are always written.
    // Above that, Pocket Edition sometimes keeps an all-air section (it
    // follows the column's history, not its blocks), so an existing one is
    // overwritten with air and a missing one stays missing.
    let top = (0..16 * 16)
        .map(|xz| {
            let base = xz * HEIGHT;
            col.ids[base..base + HEIGHT].iter().rposition(|&id| id != 0).map_or(0, |y| y / 16 + 1)
        })
        .max()
        .unwrap_or(0);
    for sy in 0..SECTIONS {
        let value = {
            let mut ids = Vec::with_capacity(4096);
            let mut data = Vec::with_capacity(4096);
            for x in 0..16 {
                for z in 0..16 {
                    let at = Column::index(x, sy * 16, z);
                    ids.extend_from_slice(&col.ids[at..at + 16]);
                    data.extend_from_slice(&col.data[at..at + 16]);
                }
            }
            let mut v = vec![0u8];
            v.extend(ids);
            pack_nibbles(&data, &mut v);
            Some(v)
        };
        out.push(Record { key: chunk_key(col.pos, ChunkTag::SubChunk(sy as u8)), value, if_present: sy >= top });
    }

    let mut v = Vec::with_capacity(768);
    for h in heights_of(col) {
        v.extend((h as i16).to_le_bytes());
    }
    v.extend(col.biomes.unwrap_or([0; 256]));
    out.push(Record { key: chunk_key(col.pos, ChunkTag::Data2D), value: Some(v), if_present: false });
}
