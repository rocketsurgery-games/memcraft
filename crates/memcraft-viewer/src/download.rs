//! The browser's "save": the world, with the edits made since the page
//! loaded, as a `.zip` of one world folder (`level.dat`, `levelname.txt`,
//! `db/*`) that the player downloads and can unpack into a PE worlds folder.
//!
//! The viewer in the browser only has the embedded world's files, so the
//! zip is built from those every time: the edited columns are put over a
//! fresh [`WorldWriter`] (the same step as the native save, see
//! `save::edited_world`) and the edits are *not* marked saved, as the next
//! download must hold them again.
//!
//! Building the zip is plain code (tested natively); only handing it to the
//! browser, [`start_download`], needs the wasm target.

use std::io::{Cursor, Write};

use mcpe_world::SavedWorld;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use crate::save::edited_world;
use crate::world::WorldMap;
use crate::{embedded_writer, world_files};

/// A world folder zipped for download.
pub struct WorldZip {
    /// The download's file name, `<world name>.zip`.
    pub file_name: String,
    pub bytes: Vec<u8>,
    /// How many edited columns it holds.
    pub columns: usize,
}

/// The embedded world with `map`'s edits, zipped.
pub fn embedded_world_zip(map: &WorldMap) -> Result<WorldZip, Box<dyn std::error::Error>> {
    let columns = map.edited_columns();
    let saved = edited_world(embedded_writer()?, &columns)?;
    let folder = folder_name(map.world().name().unwrap_or(""));
    let bytes = zip_world(&folder, &saved, world_files::EXTRA_FILES)?;
    Ok(WorldZip { file_name: format!("{folder}.zip"), bytes, columns: columns.len() })
}

/// Zips `saved` and the `extra` top-level files into one folder, `folder/`.
fn zip_world(folder: &str, saved: &SavedWorld, extra: &[(&str, &[u8])]) -> zip::result::ZipResult<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut add = |path: String, bytes: &[u8]| -> zip::result::ZipResult<()> {
        zip.start_file(path, options)?;
        zip.write_all(bytes)?;
        Ok(())
    };
    add(format!("{folder}/level.dat"), &saved.level_dat)?;
    for (name, bytes) in extra {
        add(format!("{folder}/{name}"), bytes)?;
    }
    for (name, bytes) in &saved.db_files {
        add(format!("{folder}/db/{name}"), bytes)?;
    }
    Ok(zip.finish()?.into_inner())
}

/// A world name made safe as a folder and file name: letters, digits, space,
/// `-`, `_` and `.` stay, anything else becomes `_`.
fn folder_name(name: &str) -> String {
    let clean: String = name.chars().map(|c| if c.is_alphanumeric() || " -_.".contains(c) { c } else { '_' }).collect();
    let clean = clean.trim_matches([' ', '.']);
    if clean.is_empty() { "world".into() } else { clean.into() }
}

/// Hands `bytes` to the browser as a download named `file_name`: a Blob
/// behind a link that is clicked.
#[cfg(target_arch = "wasm32")]
pub fn start_download(file_name: &str, bytes: &[u8]) -> Result<(), String> {
    use wasm_bindgen::{closure::Closure, JsCast, JsValue};
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    let fail = |e: JsValue| format!("{e:?}");
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(bytes));
    let options = BlobPropertyBag::new();
    options.set_type("application/zip");
    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options).map_err(fail)?;
    let url = Url::create_object_url_with_blob(&blob).map_err(fail)?;

    let window = web_sys::window().ok_or("no window")?;
    let document = window.document().ok_or("no document")?;
    let link: HtmlAnchorElement = document.create_element("a").map_err(fail)?.unchecked_into();
    link.set_href(&url);
    link.set_download(file_name);
    link.style().set_property("display", "none").map_err(fail)?;
    document.body().ok_or("no body")?.append_child(&link).map_err(fail)?;
    link.click();
    link.remove();

    // The browser reads the Blob once the click has started the download;
    // free it a little later.
    let revoke = Closure::once_into_js(move || {
        let _ = Url::revoke_object_url(&url);
    });
    window.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 60_000).map_err(fail)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use mcpe_world::World;

    use super::*;

    /// The zip's entries, by path.
    fn unzip(bytes: Vec<u8>) -> Vec<(String, Vec<u8>)> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        (0..archive.len())
            .map(|i| {
                let mut file = archive.by_index(i).unwrap();
                let mut data = Vec::new();
                file.read_to_end(&mut data).unwrap();
                (file.name().unwrap().into_owned(), data)
            })
            .collect()
    }

    #[test]
    fn the_zip_is_one_world_folder_that_reopens_with_the_edits() {
        let mut map = WorldMap::new(crate::load_world(None).unwrap());
        let pos = *map.world().columns.keys().nth(20).unwrap();
        let (x, y, z) = (pos.x * 16 + 3, 120, pos.z * 16 + 4);
        assert_eq!(map.world().block(x, y, z), (0, 0));
        assert!(map.set_block(x, y, z, 57, 0));

        let zip = embedded_world_zip(&map).unwrap();
        assert_eq!((zip.file_name.as_str(), zip.columns), ("My World.zip", 1));
        let entries = unzip(zip.bytes);
        let file = |path: &str| entries.iter().find(|(n, _)| n == path).map(|(_, b)| b.as_slice());

        // One folder, holding what a PE world needs.
        assert!(entries.iter().all(|(n, _)| n.starts_with("My World/")), "{:?}", entries.iter().map(|e| &e.0).collect::<Vec<_>>());
        assert!(file("My World/level.dat").is_some());
        assert_eq!(file("My World/levelname.txt"), Some(&b"My World"[..]));
        let db: Vec<(String, Vec<u8>)> = entries
            .iter()
            .filter_map(|(n, b)| Some((n.strip_prefix("My World/db/")?.to_string(), b.clone())))
            .collect();
        assert!(db.iter().any(|(n, _)| n == "CURRENT"), "the database is there");

        // It opens as a world, with the edit and every other column.
        let reopened = World::from_files(file("My World/level.dat").unwrap(), db).unwrap();
        assert_eq!(reopened.block(x, y, z), (57, 0));
        assert_eq!(reopened.columns.len(), map.world().columns.len());
        let other = *map.world().columns.keys().next().unwrap();
        assert!(reopened.columns[&other] == map.world().columns[&other]);
    }

    #[test]
    fn each_download_holds_every_edit_so_far() {
        let mut map = WorldMap::new(crate::load_world(None).unwrap());
        let pos = *map.world().columns.keys().next().unwrap();
        assert!(map.set_block(pos.x * 16, 100, pos.z * 16, 1, 0));
        assert_eq!(embedded_world_zip(&map).unwrap().columns, 1);
        assert_eq!(embedded_world_zip(&map).unwrap().columns, 1, "a download does not mark the edits saved");
    }

    /// Checks what `web/drive.js` (with `SAVE=1`) downloaded, after it was
    /// unzipped: `DRIVE_WORLD` is the world folder in it and `DRIVE_EDIT` the
    /// `x,y,z` of the block the drive broke. The folder must open as a world
    /// that differs from the embedded one in that block, and only there.
    #[test]
    #[ignore = "needs the browser drive's download: DRIVE_WORLD=<unzipped folder> DRIVE_EDIT=x,y,z"]
    fn drives_download_has_its_edit() {
        let dir = std::env::var("DRIVE_WORLD").expect("DRIVE_WORLD");
        let edit = std::env::var("DRIVE_EDIT").expect("DRIVE_EDIT");
        let [x, y, z]: [i32; 3] = edit.split(',').map(|v| v.parse().unwrap()).collect::<Vec<_>>().try_into().unwrap();

        let saved = World::open_dir(&dir).unwrap();
        let original = crate::load_world(None).unwrap();
        assert_eq!(saved.name(), original.name());
        let (was, now) = (original.block(x, y, z), saved.block(x, y, z));
        println!("block at {x},{y},{z}: {was:?} in the embedded world, {now:?} in the download");
        assert_eq!(now, (0, 0));
        assert_ne!(was, now);
        assert_eq!(saved.columns.len(), original.columns.len());
        let changed: Vec<_> = original.columns.iter().filter(|(pos, col)| **col != saved.columns[pos]).map(|(pos, _)| *pos).collect();
        assert_eq!(changed.len(), 1, "{changed:?}");
    }

    #[test]
    fn world_names_become_safe_file_names() {
        assert_eq!(folder_name("My World"), "My World");
        assert_eq!(folder_name("a/b\\c:d"), "a_b_c_d");
        assert_eq!(folder_name(".."), "world");
        assert_eq!(folder_name(""), "world");
        assert_eq!(folder_name(" §Welt "), "_Welt");
    }
}
