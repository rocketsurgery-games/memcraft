//! Saving: re-encoded columns written back into a world's LevelDB.

use std::path::Path;

use crate::db::MemDb;
use crate::encode::encode_column;
use crate::{read_world_dir, Column, Result};

/// A world open for writing.
///
/// Works on an in-memory copy of the world's files (like [`World`](crate::World)
/// does for reading), so nothing on disk changes until
/// [`SavedWorld::write_dir`], and the same code runs in the browser.
pub struct WorldWriter {
    db: MemDb,
    level_dat: Vec<u8>,
}

impl WorldWriter {
    /// Opens the world directory (the one containing `level.dat` and `db/`).
    pub fn open_dir(dir: impl AsRef<Path>) -> Result<WorldWriter> {
        let (level_dat, db_files) = read_world_dir(dir.as_ref())?;
        WorldWriter::from_files(level_dat, db_files)
    }

    /// Opens a world from `level.dat` bytes and the files of its `db/` folder.
    pub fn from_files(
        level_dat: Vec<u8>,
        db_files: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<WorldWriter> {
        Ok(WorldWriter { db: MemDb::open(db_files)?, level_dat })
    }

    /// Writes `col` over whatever the database holds for its position: its
    /// terrain records, block entities and entities. Only these records of
    /// this column change; everything else stays as it was.
    ///
    /// Heights are written as stored in [`Column::heights`]; call
    /// [`Column::recompute_heights`] after editing blocks. Light is written
    /// as stored too and is not recomputed.
    pub fn put_column(&mut self, col: &Column) -> Result<()> {
        for rec in encode_column(col)? {
            if rec.if_present && self.db.db.get(&rec.key).is_none() {
                continue;
            }
            match rec.value {
                Some(v) => self.db.db.put(&rec.key, &v)?,
                None => self.db.db.delete(&rec.key)?,
            }
        }
        Ok(())
    }

    /// Flushes the database and returns the world's files.
    pub fn finish(self) -> Result<SavedWorld> {
        Ok(SavedWorld { level_dat: self.level_dat, db_files: self.db.close()? })
    }
}

/// The files of a saved world.
pub struct SavedWorld {
    /// `level.dat`, byte for byte as it was opened.
    pub level_dat: Vec<u8>,
    /// The files of `db/` (`CURRENT`, `MANIFEST-*`, `*.ldb`, `*.log`, ...).
    pub db_files: Vec<(String, Vec<u8>)>,
}

impl SavedWorld {
    /// Writes the world into `dir`, creating it if needed; this may be the
    /// directory the world was opened from. `level.dat` and `db/` are
    /// replaced and other files in `dir` (`levelname.txt`, ...) are left alone.
    ///
    /// The new `db/` is written beside the old one and swapped in by
    /// renaming, so an interrupted save leaves a complete database.
    pub fn write_dir(&self, dir: impl AsRef<Path>) -> Result<()> {
        let dir = dir.as_ref();
        let (staged, old) = (dir.join("db.new"), dir.join("db.old"));
        for d in [&staged, &old] {
            if d.exists() {
                std::fs::remove_dir_all(d)?;
            }
        }
        std::fs::create_dir_all(&staged)?;
        for (name, bytes) in &self.db_files {
            std::fs::write(staged.join(name), bytes)?;
        }
        std::fs::write(dir.join("level.dat"), &self.level_dat)?;
        let live = dir.join("db");
        if live.exists() {
            std::fs::rename(&live, &old)?;
        }
        std::fs::rename(&staged, &live)?;
        if old.exists() {
            std::fs::remove_dir_all(&old)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use super::*;
    use crate::{db, World};

    fn sample_dir() -> PathBuf {
        PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World"))
    }

    /// Every record in the sample world's database, by key.
    fn records_of(files: Vec<(String, Vec<u8>)>) -> BTreeMap<Vec<u8>, Vec<u8>> {
        let mut out = BTreeMap::new();
        db::for_each_entry(files, |k, v| {
            out.insert(k, v);
        })
        .unwrap();
        out
    }

    #[test]
    fn encoding_a_decoded_column_reproduces_its_records() {
        let (_, files) = read_world_dir(&sample_dir()).unwrap();
        let stored = records_of(files);
        let world = World::open_dir(sample_dir()).unwrap();
        let (mut checked, mut mismatched) = (0, Vec::new());
        for col in world.columns.values() {
            for rec in encode_column(col).unwrap() {
                if rec.if_present && !stored.contains_key(&rec.key) {
                    continue;
                }
                checked += 1;
                if stored.get(&rec.key) != rec.value.as_ref() {
                    mismatched.push((col.pos, col.format, rec.key.last().copied()));
                }
            }
        }
        assert!(checked > 256 * 3, "{checked}");
        assert!(mismatched.is_empty(), "{} of {checked} records differ, e.g. {:?}", mismatched.len(), &mismatched[..mismatched.len().min(8)]);
    }

    #[test]
    fn recomputed_heights_match_the_stored_ones() {
        let world = World::open_dir(sample_dir()).unwrap();
        for col in world.columns.values() {
            let mut c = col.clone();
            c.recompute_heights();
            assert_eq!(c.heights, col.heights, "{:?}", col.pos);
        }
    }

    #[test]
    fn untouched_columns_keep_their_exact_records() {
        let (level, files) = read_world_dir(&sample_dir()).unwrap();
        let before = records_of(files.clone());
        let mut world = World::open_dir(sample_dir()).unwrap();
        let edited: Vec<_> = world.columns.keys().copied().step_by(37).collect();
        let mut writer = WorldWriter::from_files(level.clone(), files).unwrap();
        for pos in &edited {
            let col = world.columns.get_mut(pos).unwrap();
            col.ids[Column::index(1, 100, 1)] = 1;
            col.recompute_heights();
            writer.put_column(col).unwrap();
        }
        let saved = writer.finish().unwrap();
        assert_eq!(saved.level_dat, level);
        let after = records_of(saved.db_files);
        let touched = |k: &Vec<u8>| matches!(crate::key::parse(k), crate::key::Key::Chunk { pos, .. } if edited.contains(&pos));
        let before_rest: Vec<_> = before.iter().filter(|(k, _)| !touched(k)).collect();
        let after_rest: Vec<_> = after.iter().filter(|(k, _)| !touched(k)).collect();
        assert!(before_rest.len() > 1500, "{}", before_rest.len());
        assert_eq!(before_rest, after_rest);
        assert_ne!(before, after);
    }
}
