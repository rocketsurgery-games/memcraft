//! LevelDB key layout.
//!
//! Chunk records are keyed `x: i32le, z: i32le, [dimension: i32le], tag: u8,
//! [sub-chunk y: u8]`; the dimension is omitted for the overworld. Everything
//! else is a plain ASCII name (`~local_player`, `portals`, ...).

/// Chunk coordinates (blocks / 16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Overworld,
    Nether,
    End,
    Other(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkTag {
    /// `0x2d`: heightmap + biomes for sub-chunk-era columns.
    Data2D,
    /// `0x2f`: one 16-block-tall section.
    SubChunk(u8),
    /// `0x30`: whole 128-block column (PE 0.9 - 0.16).
    LegacyTerrain,
    /// `0x31`: concatenated block-entity NBT.
    BlockEntity,
    /// `0x32`: concatenated entity NBT.
    Entity,
    /// Anything else (pending ticks, finalization state, version, ...).
    Other(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    Chunk { pos: ChunkPos, dim: Dimension, tag: ChunkTag },
    Named(String),
    Unknown,
}

fn i32_at(k: &[u8], at: usize) -> i32 {
    i32::from_le_bytes(k[at..at + 4].try_into().unwrap())
}

/// Builds an overworld chunk key: `x`, `z`, tag, and the sub-chunk `y` for
/// [`ChunkTag::SubChunk`]. The inverse of [`parse`] for overworld chunk keys.
pub fn chunk_key(pos: ChunkPos, tag: ChunkTag) -> Vec<u8> {
    let mut k = pos.x.to_le_bytes().to_vec();
    k.extend(pos.z.to_le_bytes());
    match tag {
        ChunkTag::Data2D => k.push(0x2d),
        ChunkTag::SubChunk(y) => k.extend([0x2f, y]),
        ChunkTag::LegacyTerrain => k.push(0x30),
        ChunkTag::BlockEntity => k.push(0x31),
        ChunkTag::Entity => k.push(0x32),
        ChunkTag::Other(t) => k.push(t),
    }
    k
}

pub fn parse(k: &[u8]) -> Key {
    // Named keys are printable ASCII. Check first: some names (e.g.
    // "BiomeData", "dimension0") happen to have chunk-key lengths.
    if !k.is_empty() && k.iter().all(|c| c.is_ascii_graphic()) {
        return Key::Named(String::from_utf8_lossy(k).into_owned());
    }
    let (dim, rest) = match k.len() {
        9 | 10 => (Dimension::Overworld, 8),
        13 | 14 => (
            match i32_at(k, 8) {
                0 => Dimension::Overworld,
                1 => Dimension::Nether,
                2 => Dimension::End,
                d => Dimension::Other(d),
            },
            12,
        ),
        _ => return Key::Unknown,
    };
    let has_y = k.len() == rest + 2;
    let tag = match (k[rest], has_y) {
        (0x2f, true) => ChunkTag::SubChunk(k[rest + 1]),
        (_, true) => return Key::Unknown,
        (0x2d, _) => ChunkTag::Data2D,
        (0x30, _) => ChunkTag::LegacyTerrain,
        (0x31, _) => ChunkTag::BlockEntity,
        (0x32, _) => ChunkTag::Entity,
        (t, _) => ChunkTag::Other(t),
    };
    Key::Chunk { pos: ChunkPos { x: i32_at(k, 0), z: i32_at(k, 4) }, dim, tag }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(x: i32, z: i32, tail: &[u8]) -> Vec<u8> {
        let mut k = x.to_le_bytes().to_vec();
        k.extend(z.to_le_bytes());
        k.extend(tail);
        k
    }

    #[test]
    fn parses_chunk_keys() {
        assert_eq!(
            parse(&key(-1, 7, &[0x2f, 3])),
            Key::Chunk { pos: ChunkPos { x: -1, z: 7 }, dim: Dimension::Overworld, tag: ChunkTag::SubChunk(3) }
        );
        let mut nether = key(2, 3, &[]);
        nether.extend(1i32.to_le_bytes());
        nether.push(0x30);
        assert_eq!(
            parse(&nether),
            Key::Chunk { pos: ChunkPos { x: 2, z: 3 }, dim: Dimension::Nether, tag: ChunkTag::LegacyTerrain }
        );
    }

    #[test]
    fn chunk_key_inverts_parse() {
        let pos = ChunkPos { x: -5, z: 300 };
        for tag in [
            ChunkTag::Data2D,
            ChunkTag::SubChunk(6),
            ChunkTag::LegacyTerrain,
            ChunkTag::BlockEntity,
            ChunkTag::Entity,
        ] {
            assert_eq!(parse(&chunk_key(pos, tag)), Key::Chunk { pos, dim: Dimension::Overworld, tag });
        }
    }

    #[test]
    fn ascii_names_win_over_chunk_lengths() {
        assert_eq!(parse(b"BiomeData"), Key::Named("BiomeData".into()));
        assert_eq!(parse(b"dimension0"), Key::Named("dimension0".into()));
    }
}
