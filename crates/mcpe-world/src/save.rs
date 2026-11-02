//! Saving edited columns into a world directory, keeping a backup: what the
//! viewer's native Save and the world server both do.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{Column, Result, WorldWriter};

/// What a save wrote.
#[derive(Debug)]
pub struct Saved {
    pub columns: usize,
    /// The copy of the previous `db/`.
    pub backup: PathBuf,
}

/// Writes `columns` into the world directory `dir`, after copying its `db/`
/// to a backup. `None` if `columns` is empty (nothing is written).
///
/// The world is re-read from disk and only these columns are put over it, so
/// whatever else is in the files stays as it was (including columns changed
/// there since the caller loaded the world). On error nothing in `dir` has
/// changed or (if the final swap itself fails) the old `db/` is still
/// complete.
pub fn save_columns(dir: impl AsRef<Path>, columns: &[Column]) -> Result<Option<Saved>> {
    let dir = dir.as_ref();
    if columns.is_empty() {
        return Ok(None);
    }
    let mut writer = WorldWriter::open_dir(dir)?;
    writer.put_columns(columns)?;
    let world = writer.finish()?;
    let backup = backup_db(dir)?;
    world.write_dir(dir)?;
    Ok(Some(Saved { columns: columns.len(), backup }))
}

/// Copies `dir/db` to a new `dir/db.backup-<UTC time>` and returns it.
/// Backups are never deleted.
fn backup_db(dir: &Path) -> std::io::Result<PathBuf> {
    let stamp = utc_stamp(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()));
    // Two saves in one second must not share a backup.
    let backup = (1..)
        .map(|n| dir.join(if n == 1 { format!("db.backup-{stamp}") } else { format!("db.backup-{stamp}-{n}") }))
        .find(|p| !p.exists())
        .unwrap();
    std::fs::create_dir(&backup)?;
    for entry in std::fs::read_dir(dir.join("db"))? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), backup.join(entry.file_name()))?;
        }
    }
    Ok(backup)
}

/// `20261101-232210`: the UTC date and time of a Unix timestamp.
fn utc_stamp(secs: u64) -> String {
    let (days, rem) = ((secs / 86400) as i64, secs % 86400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}-{:02}{:02}{:02}", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_stamp_formats_unix_time() {
        assert_eq!(utc_stamp(0), "19700101-000000");
        assert_eq!(utc_stamp(951_827_696), "20000229-123456");
        assert_eq!(utc_stamp(1_793_575_330), "20261101-232210");
    }
}
