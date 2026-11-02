//! Packing a world, or some of its columns, into one byte string, for
//! sending between a world server and a viewer.
//!
//! Both are a magic number followed by entries, each `name length: u32le,
//! value length: u32le, name, value`:
//!
//! - **A world** ([`pack_world_dir`], [`unpack_world`]): the entry `level.dat`
//!   and one entry `db/<file>` per file of the database.
//! - **Columns** ([`pack_columns`], [`unpack_columns`]): the database
//!   records the columns encode to (what [`WorldWriter::put_column`] would
//!   write; the name is the record's key), so unpacking is the code that
//!   reads a world and rejects whatever it would.
//!
//! [`WorldWriter::put_column`]: crate::WorldWriter::put_column

use std::collections::BTreeMap;
use std::path::Path;

use crate::encode::encode_column;
use crate::key::{self, Dimension};
use crate::{decode_column, read_world_dir, ChunkPos, Column, DbFiles, Error, Result};

const WORLD_MAGIC: &[u8; 4] = b"MCW1";
const COLUMNS_MAGIC: &[u8; 4] = b"MCC1";

fn pack(magic: &[u8; 4], entries: impl IntoIterator<Item = (impl AsRef<[u8]>, impl AsRef<[u8]>)>) -> Vec<u8> {
    let mut out = magic.to_vec();
    for (name, value) in entries {
        let (name, value) = (name.as_ref(), value.as_ref());
        out.extend((name.len() as u32).to_le_bytes());
        out.extend((value.len() as u32).to_le_bytes());
        out.extend(name);
        out.extend(value);
    }
    out
}

fn unpack(magic: &[u8; 4], bytes: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
    let bad = |why: &str| Error::Format(format!("packed data: {why}"));
    let mut rest = bytes.strip_prefix(magic).ok_or_else(|| bad("wrong magic number"))?;
    let mut out = Vec::new();
    while !rest.is_empty() {
        let (head, tail) = rest.split_at_checked(8).ok_or_else(|| bad("truncated entry header"))?;
        let len = |at: usize| u32::from_le_bytes(head[at..at + 4].try_into().unwrap()) as usize;
        let (name_len, value_len) = (len(0), len(4));
        let body_len = name_len.checked_add(value_len).filter(|n| *n <= tail.len()).ok_or_else(|| bad("truncated entry"))?;
        let (name, value) = tail[..body_len].split_at(name_len);
        out.push((name.to_vec(), value.to_vec()));
        rest = &tail[body_len..];
    }
    Ok(out)
}

/// The world directory `dir` (`level.dat` and `db/`) as one byte string.
pub fn pack_world_dir(dir: impl AsRef<Path>) -> Result<Vec<u8>> {
    let (level_dat, db_files) = read_world_dir(dir.as_ref())?;
    let db = db_files.into_iter().map(|(name, bytes)| (format!("db/{name}"), bytes));
    Ok(pack(WORLD_MAGIC, std::iter::once(("level.dat".to_string(), level_dat)).chain(db)))
}

/// The `level.dat` bytes and `db/` files of a [`pack_world_dir`] string, as
/// [`World::from_files`](crate::World::from_files) and
/// [`WorldWriter::from_files`](crate::WorldWriter::from_files) take them.
pub fn unpack_world(bytes: &[u8]) -> Result<(Vec<u8>, DbFiles)> {
    let (mut level_dat, mut db_files) = (None, Vec::new());
    for (name, value) in unpack(WORLD_MAGIC, bytes)? {
        match (std::str::from_utf8(&name), name.as_slice()) {
            (_, b"level.dat") => level_dat = Some(value),
            (Ok(file), _) if file.strip_prefix("db/").is_some_and(|f| !f.is_empty() && !f.contains('/')) => {
                db_files.push((file["db/".len()..].to_string(), value));
            }
            _ => return Err(Error::Format(format!("packed world: unexpected entry {:?}", String::from_utf8_lossy(&name)))),
        }
    }
    Ok((level_dat.ok_or_else(|| Error::Format("packed world: no level.dat".into()))?, db_files))
}

/// `columns`, as the records they are stored as.
pub fn pack_columns(columns: &[Column]) -> Result<Vec<u8>> {
    let mut records = Vec::new();
    for col in columns {
        // Deleted records (an emptied entity list) are not sent: the column
        // unpacks without them and encodes to the deletions again.
        records.extend(encode_column(col)?.into_iter().filter_map(|r| Some((r.key, r.value?))));
    }
    Ok(pack(COLUMNS_MAGIC, records))
}

/// The columns of a [`pack_columns`] string, in position order. Anything
/// that is not the overworld record of a column is an error.
pub fn unpack_columns(bytes: &[u8]) -> Result<Vec<Column>> {
    let mut by_pos: BTreeMap<ChunkPos, Vec<(key::ChunkTag, Vec<u8>)>> = BTreeMap::new();
    for (k, v) in unpack(COLUMNS_MAGIC, bytes)? {
        match key::parse(&k) {
            key::Key::Chunk { pos, dim: Dimension::Overworld, tag } => by_pos.entry(pos).or_default().push((tag, v)),
            _ => return Err(Error::Format(format!("packed columns: {k:02x?} is not an overworld chunk record"))),
        }
    }
    by_pos
        .into_iter()
        .map(|(pos, recs)| decode_column(pos, recs)?.ok_or_else(|| Error::Format(format!("packed columns: no terrain for {pos:?}"))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::World;

    const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World");

    #[test]
    fn a_packed_world_dir_opens_as_the_same_world() {
        let packed = pack_world_dir(SAMPLE).unwrap();
        let (level_dat, db_files) = unpack_world(&packed).unwrap();
        let from_pack = World::from_files(&level_dat, db_files).unwrap();
        let from_dir = World::open_dir(SAMPLE).unwrap();
        assert_eq!(from_pack.columns.len(), from_dir.columns.len());
        assert!(from_pack.columns.iter().all(|(pos, col)| *col == from_dir.columns[pos]));
        assert_eq!(from_pack.name(), from_dir.name());
    }

    #[test]
    fn packed_columns_unpack_to_equal_columns() {
        let world = World::open_dir(SAMPLE).unwrap();
        let mut columns: Vec<Column> = world.columns.values().step_by(41).cloned().collect();
        // An entity list that is emptied and a block edit, as a save sends.
        columns[0].entities.clear();
        columns[1].ids[Column::index(1, 100, 1)] = 57;
        columns[1].recompute_heights();
        let back = unpack_columns(&pack_columns(&columns).unwrap()).unwrap();
        assert_eq!(back.len(), columns.len());
        assert!(back.iter().zip(&columns).all(|(a, b)| a == b));
    }

    #[test]
    fn nothing_packed_is_nothing_unpacked() {
        assert!(unpack_columns(&pack_columns(&[]).unwrap()).unwrap().is_empty());
    }

    #[test]
    fn malformed_input_is_an_error_not_a_panic() {
        let world = World::open_dir(SAMPLE).unwrap();
        let columns = pack_columns(&[world.columns.values().next().unwrap().clone()]).unwrap();
        assert!(unpack_columns(b"").is_err());
        assert!(unpack_columns(&pack_world_dir(SAMPLE).unwrap()).is_err(), "a world is not columns");
        assert!(unpack_world(&columns).is_err(), "columns are not a world");
        // Every truncation fails (or, at an entry boundary, is a smaller valid pack).
        for cut in [5, 11, 12, 100, columns.len() - 1] {
            assert!(unpack_columns(&columns[..cut]).is_err(), "cut at {cut}");
        }
        // A length that points past the end.
        let mut huge = COLUMNS_MAGIC.to_vec();
        huge.extend(u32::MAX.to_le_bytes());
        huge.extend(u32::MAX.to_le_bytes());
        assert!(unpack_columns(&huge).is_err());
        // Records that are not an overworld column's.
        let other = pack(COLUMNS_MAGIC, [(&b"~local_player"[..], &b"x"[..])]);
        assert!(unpack_columns(&other).is_err());
        let garbage = pack(COLUMNS_MAGIC, [(key::chunk_key(ChunkPos { x: 0, z: 0 }, key::ChunkTag::LegacyTerrain), vec![1, 2, 3])]);
        assert!(unpack_columns(&garbage).is_err());
        // A world needs its level.dat, and only its own entries.
        assert!(unpack_world(&pack(WORLD_MAGIC, [("db/CURRENT", "x")])).is_err());
        assert!(unpack_world(&pack(WORLD_MAGIC, [("level.dat", "x"), ("db/../evil", "x")])).is_err());
        assert!(unpack_world(&pack(WORLD_MAGIC, [("level.dat", "x"), ("elsewhere", "x")])).is_err());
    }
}
