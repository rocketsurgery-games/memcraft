//! A small server for hosted saves: it serves the web viewer's build and one
//! world directory, and lets the viewer save its edits into that directory.
//! No authentication: for local or LAN use.
//!
//! - `GET /api/world`: the probe the viewer makes at startup. `200` with
//!   `{"name": "<world name>"}` means a save endpoint is here.
//! - `GET /api/world/files`: the world as it is on disk, packed (see
//!   [`mcpe_world::pack_world_dir`]) for the viewer to load.
//! - `PUT /api/world/columns`: the edited columns, packed (see
//!   [`mcpe_world::pack_columns`]); they are put over the world on disk, after
//!   copying `db/` to `db.backup-<UTC time>` (see [`mcpe_world::save_columns`],
//!   the same as the native viewer's Save). `200` with
//!   `{"columns": n, "backup": "<directory name>"}`; `400` for a body that is
//!   not columns.
//! - Anything else is a file of the web build.
//!
//! The directory is read again for every request, so edits made to it by other
//! means show up on the next page load; saves are one at a time.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};
use tower_http::services::ServeDir;

/// The largest save accepted: far more than a world's worth of edited columns
/// (about 40 KB each).
const MAX_SAVE_BYTES: usize = 256 << 20;

#[derive(Clone)]
struct Hosted {
    world: Arc<PathBuf>,
    /// Held while saving: two saves must not interleave their read, backup
    /// and swap of `db/`.
    saving: Arc<Mutex<()>>,
}

/// The server's routes, for the world directory `world`; `web` is the viewer's
/// build (`crates/memcraft-viewer/web/dist`), served at `/` if given.
pub fn app(world: impl Into<PathBuf>, web: Option<&Path>) -> Router {
    let hosted = Hosted { world: Arc::new(world.into()), saving: Arc::default() };
    let api = Router::new()
        .route("/world", get(probe))
        .route("/world/files", get(files))
        .route("/world/columns", axum::routing::put(save))
        .layer(DefaultBodyLimit::max(MAX_SAVE_BYTES))
        .with_state(hosted);
    let router = Router::new().nest("/api", api);
    match web {
        Some(web) => router.fallback_service(ServeDir::new(web)),
        None => router,
    }
}

/// An error answered to the client, and logged.
struct Failure(StatusCode, String);

impl IntoResponse for Failure {
    fn into_response(self) -> Response {
        eprintln!("{}: {}", self.0, self.1);
        (self.0, self.1).into_response()
    }
}

fn internal(e: impl std::fmt::Display) -> Failure {
    Failure(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}

/// API answers are never cached: the world changes with every save.
const NO_STORE: (header::HeaderName, &str) = (header::CACHE_CONTROL, "no-store");

async fn probe(State(hosted): State<Hosted>) -> Result<impl IntoResponse, Failure> {
    let level_dat = std::fs::read(hosted.world.join("level.dat")).map_err(internal)?;
    let (_, level) = mcpe_world::parse_level_dat(&level_dat).map_err(internal)?;
    let name = level.get("LevelName").and_then(mcpe_world::Tag::as_str).unwrap_or("");
    Ok(([NO_STORE], Json(json!({ "name": name }))))
}

async fn files(State(hosted): State<Hosted>) -> Result<impl IntoResponse, Failure> {
    let packed = tokio::task::spawn_blocking(move || mcpe_world::pack_world_dir(&*hosted.world)).await.map_err(internal)?.map_err(internal)?;
    Ok(([NO_STORE, (header::CONTENT_TYPE, "application/octet-stream")], packed))
}

async fn save(State(hosted): State<Hosted>, body: Bytes) -> Result<impl IntoResponse, Failure> {
    let columns = mcpe_world::unpack_columns(&body).map_err(|e| Failure(StatusCode::BAD_REQUEST, e.to_string()))?;
    let saved = tokio::task::spawn_blocking(move || {
        let _one_at_a_time = hosted.saving.lock().unwrap_or_else(|e| e.into_inner());
        mcpe_world::save_columns(&*hosted.world, &columns)
    })
    .await
    .map_err(internal)?
    .map_err(internal)?;
    let answer: Value = match saved {
        Some(saved) => {
            let backup = saved.backup.file_name().map(|n| n.to_string_lossy().into_owned());
            eprintln!("saved {} column(s); previous db/ kept in {}", saved.columns, backup.as_deref().unwrap_or("?"));
            json!({ "columns": saved.columns, "backup": backup })
        }
        None => json!({ "columns": 0, "backup": null }),
    };
    Ok(([NO_STORE], Json(answer)))
}
