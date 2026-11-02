//! Hosted saves (browser only): behind a world server (`memcraft-server`)
//! the viewer shows the server's world and Save sends the edits back to it.
//!
//! `web/index.html` probes the server before the viewer starts and, if one
//! answers, leaves the world's packed files in `window.memcraftServer.files`
//! ([`hosted_world`] reads them). Anywhere else (GitHub Pages, a plain static
//! server) nothing is there and the viewer keeps its embedded world and the
//! zip download, see `save` and `download`.
//!
//! Save then sends only the columns edited since the last save, as one `PUT`
//! (see [`mcpe_world::pack_columns`]); the server puts them over its copy and
//! keeps a backup. The edits are marked saved when the request is sent, so
//! edits made while it is in flight are kept for the next save, and marked
//! again if it fails. One save is in flight at a time: a second one could
//! arrive first and be overwritten by the older columns.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use mcpe_world::{ChunkPos, World as PeWorld};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::menu::SaveRequested;
use crate::toast::Toast;
use crate::world::WorldMap;

/// Relative to the page, so it works wherever the server is mounted.
const COLUMNS_URL: &str = "api/world/columns";

/// The world files the page fetched from the server, if it found one.
fn hosted_files() -> Option<Vec<u8>> {
    let window: JsValue = web_sys::window()?.into();
    let server = js_sys::Reflect::get(&window, &"memcraftServer".into()).ok().filter(|s| s.is_object())?;
    let files = js_sys::Reflect::get(&server, &"files".into()).ok()?.dyn_into::<js_sys::ArrayBuffer>().ok()?;
    Some(js_sys::Uint8Array::new(&files).to_vec())
}

/// Whether a world server is behind this page.
pub fn hosted() -> bool {
    hosted_files().is_some()
}

/// The server's world, if there is one.
pub fn hosted_world() -> Option<mcpe_world::Result<PeWorld>> {
    let files = hosted_files()?;
    Some(mcpe_world::unpack_world(&files).and_then(|(level_dat, db_files)| PeWorld::from_files(&level_dat, db_files)))
}

/// How a save request ended, handed from the fetch to [`save_to_server`].
struct Outcome {
    /// The columns the request carried.
    columns: Vec<ChunkPos>,
    result: Result<String, String>,
}

/// Where the one save that may be in flight is.
#[derive(Default)]
enum Save {
    #[default]
    Idle,
    Sending,
    /// Finished, not yet reported.
    Done(Outcome),
}

#[derive(Resource, Default)]
struct InFlight(Arc<Mutex<Save>>);

pub struct ServerSavePlugin;

impl Plugin for ServerSavePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InFlight>().add_systems(Update, (crate::save::save_shortcut, save_to_server).chain());
    }
}

/// Reports a finished save, and starts one if the player asked for it.
fn save_to_server(mut requests: MessageReader<SaveRequested>, in_flight: Res<InFlight>, mut map: ResMut<WorldMap>, mut toasts: MessageWriter<Toast>) {
    let mut toast = |result: Result<String, String>| {
        let ok = result.is_ok();
        let text = result.unwrap_or_else(|e| e);
        if ok { info!("{text}") } else { error!("{text}") }
        toasts.write(Toast { text, ok });
    };

    let mut state = in_flight.0.lock().unwrap();
    match std::mem::take(&mut *state) {
        Save::Done(done) => {
            if done.result.is_err() {
                map.mark_unsaved(done.columns);
            }
            toast(done.result);
        }
        other => *state = other,
    }
    if requests.read().count() == 0 {
        return;
    }
    if matches!(*state, Save::Sending) {
        return toast(Err("Still saving; try again in a moment".into()));
    }
    let columns = map.edited_columns();
    if columns.is_empty() {
        return toast(Ok("Nothing to save: no edits since the last save".into()));
    }
    let body = match mcpe_world::pack_columns(&columns) {
        Ok(body) => body,
        Err(e) => return toast(Err(format!("Saving to the server failed: {e}"))),
    };
    map.mark_saved();
    *state = Save::Sending;
    let positions: Vec<ChunkPos> = columns.iter().map(|c| c.pos).collect();
    let slot = in_flight.0.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let result = put(&body).await.map_err(|e| format!("Saving to the server failed: {e}"));
        *slot.lock().unwrap() = Save::Done(Outcome { columns: positions, result });
    });
}

/// `PUT`s the packed columns; what to tell the player.
async fn put(body: &[u8]) -> Result<String, String> {
    let fail = |e: JsValue| format!("{e:?}");
    let init = web_sys::RequestInit::new();
    init.set_method("PUT");
    init.set_body(&js_sys::Uint8Array::from(body));
    let window = web_sys::window().ok_or("no window")?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str_and_init(COLUMNS_URL, &init)).await.map_err(fail)?.unchecked_into();
    let text = JsFuture::from(response.text().map_err(fail)?).await.map_err(fail)?.as_string().unwrap_or_default();
    if !response.ok() {
        return Err(format!("the server answered {} {}", response.status(), text.trim()));
    }
    let answer = js_sys::JSON::parse(&text).map_err(fail)?;
    let get = |key: &str| js_sys::Reflect::get(&answer, &key.into()).ok();
    let columns = get("columns").and_then(|c| c.as_f64()).unwrap_or(0.0) as usize;
    let backup = get("backup").and_then(|b| b.as_string());
    Ok(match backup {
        Some(backup) => format!("Saved {} to the server; the previous db/ is kept in {backup}", crate::save::columns(columns)),
        None => "Nothing to save: no edits since the last save".into(),
    })
}
