//! Saving: what the menu's Save button (and Ctrl+S, which works with the
//! mouse captured) does, natively and in the browser.
//!
//! Both send [`SaveRequested`]; [`save_on_request`] answers it by saving the
//! edited columns (see `WorldMap::edited_columns`; they are re-encoded over
//! the world's own files, see [`edited_world`], and lighting is not
//! recomputed, so edited columns keep their old light) and reports the
//! result as a [`Toast`], which is also logged.
//!
//! - **Native:** [`save_world`] writes them back into the world directory the
//!   viewer was started with (the button is greyed out unless a directory
//!   was given, see [`SaveAvailable`]; the embedded world has none). The
//!   previous `db/` is copied to `db.backup-<UTC time>` beside it before
//!   anything is replaced; backups are never deleted.
//! - **Browser:** the page has no place to write to, so the button says
//!   "Download world (.zip)" and offers the whole world, edits included, as a
//!   zip (see `download`).

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::prelude::*;
use mcpe_world::{Column, SavedWorld, WorldWriter};

use crate::menu::{SaveAvailable, SaveRequested};
use crate::toast::Toast;
use crate::world::WorldMap;

/// The world directory the viewer was started with and saves to; `None` for
/// the embedded world, and always in the browser.
#[derive(Resource)]
struct WorldDir(Option<PathBuf>);

pub struct SavePlugin {
    /// The directory the world was loaded from; `None` for the embedded world.
    pub dir: Option<PathBuf>,
}

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        // The menu plugin only makes sure these exist; set them either way.
        app.add_message::<SaveRequested>().add_message::<Toast>();
        #[cfg(not(target_arch = "wasm32"))]
        app.insert_resource(SaveAvailable(self.dir.is_some()));
        #[cfg(target_arch = "wasm32")]
        {
            debug_assert!(self.dir.is_none(), "the browser has no world directory");
            app.insert_resource(SaveAvailable(true)).insert_resource(crate::menu::SaveLabel("Download world (.zip)"));
        }
        app.insert_resource(WorldDir(self.dir.clone()));
        app.add_systems(Update, (save_shortcut, save_on_request).chain());
    }
}

/// Ctrl+S asks for a save, like the menu button.
fn save_shortcut(keys: Res<ButtonInput<KeyCode>>, mut saves: MessageWriter<SaveRequested>) {
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    if ctrl && keys.just_pressed(KeyCode::KeyS) {
        saves.write(SaveRequested);
    }
}

fn save_on_request(mut requests: MessageReader<SaveRequested>, dir: Res<WorldDir>, mut map: ResMut<WorldMap>, mut toasts: MessageWriter<Toast>) {
    if requests.read().count() == 0 {
        return;
    }
    let (text, ok) = match save_now(&mut map, dir.0.as_deref()) {
        Ok(text) => {
            info!("{text}");
            (text, true)
        }
        Err(text) => {
            error!("{text}");
            (text, false)
        }
    };
    toasts.write(Toast { text, ok });
}

/// Saves the world the way this platform does; what to tell the player
/// either way.
#[cfg(not(target_arch = "wasm32"))]
fn save_now(map: &mut WorldMap, dir: Option<&Path>) -> Result<String, String> {
    let Some(dir) = dir else {
        return Err("Not saved: this is the embedded world; start with a world directory to save edits".into());
    };
    match save_world(map, dir) {
        Ok(None) => Ok("Nothing to save: no edits since the last save".into()),
        Ok(Some(saved)) => Ok(format!(
            "Saved {} to {}; the previous db/ is kept in {}",
            columns(saved.columns),
            dir.display(),
            saved.backup.display()
        )),
        Err(e) => Err(format!("Saving to {} failed: {e}", dir.display())),
    }
}

/// The browser offers a zip instead. The edits stay marked as unsaved: every
/// download is the embedded world plus all of them.
#[cfg(target_arch = "wasm32")]
fn save_now(map: &mut WorldMap, _dir: Option<&std::path::Path>) -> Result<String, String> {
    let failed = |e: &dyn std::fmt::Display| format!("Download failed: {e}");
    let zip = crate::download::embedded_world_zip(map).map_err(|e| failed(&*e))?;
    crate::download::start_download(&zip.file_name, &zip.bytes).map_err(|e| failed(&e))?;
    let edits = if zip.columns == 0 { "no edits".to_string() } else { format!("{} edited", columns(zip.columns)) };
    Ok(format!("Downloading {} ({edits})", zip.file_name))
}

fn columns(n: usize) -> String {
    format!("{n} column{}", if n == 1 { "" } else { "s" })
}

/// The world of `writer` with `columns` put over it, ready to be written out.
/// Both the native save and the browser's download go through this.
pub fn edited_world(mut writer: WorldWriter, columns: &[Column]) -> mcpe_world::Result<SavedWorld> {
    for col in columns {
        writer.put_column(col)?;
    }
    writer.finish()
}

/// What a save wrote.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct Saved {
    pub columns: usize,
    /// The copy of the previous `db/`.
    pub backup: PathBuf,
}

/// Writes the columns edited in `map` into the world directory `dir`, after
/// copying its `db/` to a backup. `None` if there was nothing to save.
///
/// The world is re-read from disk and only the edited columns are put over
/// it, so whatever else is in the files stays as it was. On error nothing in
/// `dir` has changed or (if the final swap itself fails) the old `db/` is
/// still complete; the edits stay marked so a later save retries them.
#[cfg(not(target_arch = "wasm32"))]
pub fn save_world(map: &mut WorldMap, dir: &Path) -> Result<Option<Saved>, Box<dyn std::error::Error>> {
    let columns = map.edited_columns();
    if columns.is_empty() {
        return Ok(None);
    }
    let world = edited_world(WorldWriter::open_dir(dir)?, &columns)?;
    let backup = backup_db(dir)?;
    world.write_dir(dir)?;
    map.mark_saved();
    Ok(Some(Saved { columns: columns.len(), backup }))
}

/// Copies `dir/db` to a new `dir/db.backup-<UTC time>` and returns it.
#[cfg(not(target_arch = "wasm32"))]
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
#[cfg(not(target_arch = "wasm32"))]
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
    use std::collections::BTreeMap;

    use mcpe_world::{ChunkPos, World};

    use super::*;

    const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World");

    /// A copy of the sample world in a fresh directory outside the tree,
    /// removed on drop.
    struct Copy(PathBuf);

    impl Copy {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("memcraft-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("db")).unwrap();
            for file in ["level.dat", "levelname.txt"] {
                std::fs::copy(Path::new(SAMPLE).join(file), dir.join(file)).unwrap();
            }
            for e in std::fs::read_dir(Path::new(SAMPLE).join("db")).unwrap() {
                let e = e.unwrap();
                std::fs::copy(e.path(), dir.join("db").join(e.file_name())).unwrap();
            }
            Self(dir)
        }
    }

    impl Drop for Copy {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// The files of a directory, by name.
    fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
        std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap())
            .filter(|e| e.file_type().unwrap().is_file())
            .map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap()))
            .collect()
    }

    fn backups(dir: &Path) -> Vec<PathBuf> {
        let mut found: Vec<_> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.file_name().unwrap().to_string_lossy().starts_with("db.backup-"))
            .collect();
        found.sort();
        found
    }

    /// Sets a block through the map, in the column at `(cx, cz)`.
    fn edit(map: &mut WorldMap, cx: i32, cz: i32, local: (i32, i32, i32), id: u8) {
        let (x, y, z) = (cx * 16 + local.0, local.1, cz * 16 + local.2);
        assert!(map.set_block(x, y, z, id, 0), "({x}, {y}, {z}) is not in the world");
    }

    #[test]
    fn utc_stamp_formats_unix_time() {
        assert_eq!(utc_stamp(0), "19700101-000000");
        assert_eq!(utc_stamp(951_827_696), "20000229-123456");
        assert_eq!(utc_stamp(1_793_575_330), "20261101-232210");
    }

    #[test]
    fn saving_writes_the_edits_in_place_and_backs_up_the_old_db() {
        let copy = Copy::new("save");
        let dir = &copy.0;
        let original = files(&dir.join("db"));
        let mut map = WorldMap::new(crate::load_world(Some(dir)).unwrap());
        let before = World::open_dir(dir).unwrap();
        let positions: Vec<ChunkPos> = before.columns.keys().copied().collect();
        let (a, b) = (positions[10], positions[100]);

        // Place a block high above the ground (height changes), remove another
        // from the ground.
        let ground = before.columns[&b].heights.unwrap()[5 * 16 + 5] as i32 - 1;
        assert_ne!(before.columns[&b].block(5, ground as usize, 5).0, 0);
        edit(&mut map, a.x, a.z, (3, 120, 4), 57);
        edit(&mut map, b.x, b.z, (5, ground, 5), 0);

        let saved = save_world(&mut map, dir).unwrap().expect("two columns were edited");
        assert_eq!(saved.columns, 2);

        // Reloading the directory shows the edits, with fresh heights...
        let after = World::open_dir(dir).unwrap();
        assert_eq!(after.block(a.x * 16 + 3, 120, a.z * 16 + 4), (57, 0));
        assert_eq!(after.block(b.x * 16 + 5, ground, b.z * 16 + 5), (0, 0));
        assert_eq!(after.columns[&a].heights.unwrap()[4 * 16 + 3], 121);
        assert!(after.columns[&b].heights.unwrap()[5 * 16 + 5] < ground as u8 + 1);
        // ...and the other columns are as before.
        assert_eq!(after.columns.len(), before.columns.len());
        for (pos, col) in &before.columns {
            if *pos != a && *pos != b {
                assert!(*col == after.columns[pos], "{pos:?} changed");
            }
        }

        // The backup is the old db/, byte for byte, and the live db/ differs.
        assert_eq!(backups(dir), std::slice::from_ref(&saved.backup));
        assert_eq!(saved.backup.parent().unwrap(), dir);
        assert_eq!(files(&saved.backup), original);
        assert_ne!(files(&dir.join("db")), original);
        // Other files in the world directory are left alone.
        assert_eq!(std::fs::read(dir.join("levelname.txt")).unwrap(), std::fs::read(Path::new(SAMPLE).join("levelname.txt")).unwrap());
        assert!(!dir.join("db.new").exists() && !dir.join("db.old").exists());
    }

    #[test]
    fn a_save_without_edits_writes_nothing_and_each_save_makes_its_own_backup() {
        let copy = Copy::new("again");
        let dir = &copy.0;
        let mut map = WorldMap::new(crate::load_world(Some(dir)).unwrap());
        let pos = *map.world().columns.keys().next().unwrap();
        let live = || files(&dir.join("db"));
        let original = live();

        assert!(save_world(&mut map, dir).unwrap().is_none());
        assert!(backups(dir).is_empty());
        assert_eq!(live(), original);

        edit(&mut map, pos.x, pos.z, (1, 100, 1), 1);
        let first = save_world(&mut map, dir).unwrap().unwrap();
        let after_first = live();
        // The edit was saved: saving again has nothing to do.
        assert!(save_world(&mut map, dir).unwrap().is_none());

        edit(&mut map, pos.x, pos.z, (2, 100, 2), 1);
        let second = save_world(&mut map, dir).unwrap().unwrap();
        assert_ne!(first.backup, second.backup);
        assert_eq!(files(&first.backup), original);
        assert_eq!(files(&second.backup), after_first);
        let reloaded = World::open_dir(dir).unwrap();
        assert_eq!(reloaded.block(pos.x * 16 + 1, 100, pos.z * 16 + 1), (1, 0));
        assert_eq!(reloaded.block(pos.x * 16 + 2, 100, pos.z * 16 + 2), (1, 0));
    }

    #[test]
    fn a_failed_save_changes_nothing_and_keeps_the_edits() {
        let copy = Copy::new("fail");
        let dir = &copy.0;
        let mut map = WorldMap::new(crate::load_world(Some(dir)).unwrap());
        let pos = *map.world().columns.keys().next().unwrap();
        edit(&mut map, pos.x, pos.z, (1, 100, 1), 1);

        std::fs::rename(dir.join("db"), dir.join("moved")).unwrap();
        assert!(save_world(&mut map, dir).is_err());
        assert!(backups(dir).is_empty());
        std::fs::rename(dir.join("moved"), dir.join("db")).unwrap();
        // The edit is still pending.
        assert_eq!(save_world(&mut map, dir).unwrap().unwrap().columns, 1);
    }

    fn app(dir: Option<PathBuf>) -> App {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<SaveAvailable>()
            .insert_resource(WorldMap::new(crate::load_world(dir.as_deref()).unwrap()))
            .add_plugins(SavePlugin { dir });
        app
    }

    #[test]
    fn ctrl_s_saves_a_world_directory_and_the_menu_can_save_only_then() {
        let copy = Copy::new("plugin");
        let mut app = app(Some(copy.0.clone()));
        assert!(app.world().resource::<SaveAvailable>().0);
        let pos = *app.world().resource::<WorldMap>().world().columns.keys().next().unwrap();
        edit(&mut app.world_mut().resource_mut::<WorldMap>(), pos.x, pos.z, (1, 100, 1), 1);

        // S alone does nothing (it is a movement key)...
        app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyS);
        app.update();
        assert!(backups(&copy.0).is_empty());
        // ...with Ctrl it saves.
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyS);
        keys.clear();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::KeyS);
        app.update();
        assert_eq!(backups(&copy.0).len(), 1);
        assert_eq!(World::open_dir(&copy.0).unwrap().block(pos.x * 16 + 1, 100, pos.z * 16 + 1), (1, 0));

        // A menu request saves too.
        edit(&mut app.world_mut().resource_mut::<WorldMap>(), pos.x, pos.z, (2, 100, 2), 1);
        app.world_mut().write_message(SaveRequested);
        app.update();
        assert_eq!(backups(&copy.0).len(), 2);
    }

    #[test]
    fn the_embedded_world_cannot_be_saved() {
        let mut app = app(None);
        assert!(!app.world().resource::<SaveAvailable>().0);
        let pos = *app.world().resource::<WorldMap>().world().columns.keys().next().unwrap();
        edit(&mut app.world_mut().resource_mut::<WorldMap>(), pos.x, pos.z, (1, 100, 1), 1);
        app.world_mut().write_message(SaveRequested);
        app.update(); // logs; nothing to write to, and nothing panics
        assert_eq!(app.world().resource::<WorldMap>().edited_columns().len(), 1);
    }

    /// Asks for a save and returns the toasts it sent.
    fn save_and_listen(app: &mut App) -> Vec<(bool, String)> {
        app.world_mut().resource_mut::<Messages<Toast>>().clear();
        app.world_mut().write_message(SaveRequested);
        app.update();
        app.world_mut().resource_mut::<Messages<Toast>>().drain().map(|t| (t.ok, t.text)).collect()
    }

    #[test]
    fn every_save_says_how_it_went() {
        let copy = Copy::new("toast");
        let mut app = app(Some(copy.0.clone()));
        let pos = *app.world().resource::<WorldMap>().world().columns.keys().next().unwrap();

        let toasts = save_and_listen(&mut app);
        assert_eq!(toasts.len(), 1);
        assert!(toasts[0].0 && toasts[0].1.starts_with("Nothing to save"), "{toasts:?}");

        edit(&mut app.world_mut().resource_mut::<WorldMap>(), pos.x, pos.z, (1, 100, 1), 1);
        std::fs::rename(copy.0.join("db"), copy.0.join("moved")).unwrap();
        let toasts = save_and_listen(&mut app);
        assert!(!toasts[0].0 && toasts[0].1.starts_with("Saving to ") && toasts[0].1.contains("failed"), "{toasts:?}");

        std::fs::rename(copy.0.join("moved"), copy.0.join("db")).unwrap();
        let toasts = save_and_listen(&mut app);
        assert!(toasts[0].0 && toasts[0].1.starts_with("Saved 1 column to "), "{toasts:?}");

        let mut embedded = self::app(None);
        let toasts = save_and_listen(&mut embedded);
        assert!(!toasts[0].0 && toasts[0].1.starts_with("Not saved"), "{toasts:?}");
    }

    #[test]
    fn column_heights_are_recomputed_only_in_the_saved_copy() {
        let mut map = WorldMap::new(crate::load_world(None).unwrap());
        let pos = *map.world().columns.keys().next().unwrap();
        edit(&mut map, pos.x, pos.z, (0, 127, 0), 1);
        let saved = &map.edited_columns()[0];
        assert_eq!(saved.heights.unwrap()[0], 128);
        assert_eq!(saved.block(0, 127, 0).0, 1);
    }
}
