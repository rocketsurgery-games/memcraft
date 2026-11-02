//! The endpoints, driven through the router (no socket needed) against a
//! temporary copy of the sample world.

use std::path::{Path, PathBuf};

use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use mcpe_world::{Column, World};
use tower::ServiceExt;

const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World");

/// A copy of the sample world in a fresh directory outside the tree, removed on drop.
struct Copy(PathBuf);

impl Copy {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("memcraft-server-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("db")).unwrap();
        std::fs::copy(Path::new(SAMPLE).join("level.dat"), dir.join("level.dat")).unwrap();
        for e in std::fs::read_dir(Path::new(SAMPLE).join("db")).unwrap() {
            let e = e.unwrap();
            std::fs::copy(e.path(), dir.join("db").join(e.file_name())).unwrap();
        }
        Self(dir)
    }

    fn backups(&self) -> Vec<String> {
        let mut found: Vec<_> = std::fs::read_dir(&self.0)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("db.backup-"))
            .collect();
        found.sort();
        found
    }

    fn db_files(&self, dir: &str) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<_> = std::fs::read_dir(self.0.join(dir))
            .unwrap()
            .map(|e| e.unwrap())
            .map(|e| (e.file_name().to_string_lossy().into_owned(), std::fs::read(e.path()).unwrap()))
            .collect();
        files.sort();
        files
    }
}

impl Drop for Copy {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn call(app: &Router, method: Method, uri: &str, body: Vec<u8>) -> (StatusCode, Vec<u8>) {
    let request = Request::builder().method(method).uri(uri).body(Body::from(body)).unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    (response.status(), to_bytes(response.into_body(), usize::MAX).await.unwrap().to_vec())
}

fn json(bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(bytes).unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(bytes)))
}

/// The world as a viewer loads it from the server.
async fn load(app: &Router) -> World {
    let (status, bytes) = call(app, Method::GET, "/api/world/files", vec![]).await;
    assert_eq!(status, StatusCode::OK);
    let (level_dat, db_files) = mcpe_world::unpack_world(&bytes).unwrap();
    World::from_files(&level_dat, db_files).unwrap()
}

/// `world`'s column number `n` with a block set at `(1, 100, 1)`.
fn edited(world: &World, n: usize, id: u8) -> Column {
    let mut col = world.columns.values().nth(n).unwrap().clone();
    col.ids[Column::index(1, 100, 1)] = id;
    col.recompute_heights();
    col
}

#[tokio::test]
async fn the_probe_names_the_world() {
    let copy = Copy::new("probe");
    let app = memcraft_server::app(&copy.0, None);
    let (status, body) = call(&app, Method::GET, "/api/world", vec![]).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json(&body), serde_json::json!({ "name": "My World" }));
}

#[tokio::test]
async fn the_served_files_are_the_world_on_disk() {
    let copy = Copy::new("files");
    let app = memcraft_server::app(&copy.0, None);
    let served = load(&app).await;
    let on_disk = World::open_dir(&copy.0).unwrap();
    assert_eq!(served.columns.len(), on_disk.columns.len());
    assert!(served.columns.iter().all(|(pos, col)| *col == on_disk.columns[pos]));
}

#[tokio::test]
async fn a_save_round_trips_into_the_world_dir_with_a_backup() {
    let copy = Copy::new("save");
    let original = copy.db_files("db");
    let app = memcraft_server::app(&copy.0, None);

    let viewer = load(&app).await;
    let col = edited(&viewer, 10, 57);
    let pos = col.pos;
    let body = mcpe_world::pack_columns(&[col]).unwrap();
    let (status, answer) = call(&app, Method::PUT, "/api/world/columns", body).await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&answer));
    let answer = json(&answer);
    assert_eq!(answer["columns"], 1);

    // The world dir, opened fresh, shows the edit and nothing else changed.
    let after = World::open_dir(&copy.0).unwrap();
    assert_eq!(after.block(pos.x * 16 + 1, 100, pos.z * 16 + 1), (57, 0));
    assert_eq!(after.columns.len(), viewer.columns.len());
    assert!(after.columns.iter().all(|(p, col)| *p == pos || *col == viewer.columns[p]));
    assert_ne!(copy.db_files("db"), original);
    // The backup is the old db/, byte for byte, beside it.
    let backups = copy.backups();
    assert_eq!(backups, [answer["backup"].as_str().unwrap().to_string()]);
    assert_eq!(copy.db_files(&backups[0]), original);
    // And the server serves the saved world from now on.
    assert_eq!(load(&app).await.block(pos.x * 16 + 1, 100, pos.z * 16 + 1), (57, 0));
}

#[tokio::test]
async fn a_save_without_columns_changes_nothing() {
    let copy = Copy::new("empty");
    let original = copy.db_files("db");
    let app = memcraft_server::app(&copy.0, None);
    let (status, answer) = call(&app, Method::PUT, "/api/world/columns", mcpe_world::pack_columns(&[]).unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json(&answer)["columns"], 0);
    assert!(copy.backups().is_empty());
    assert_eq!(copy.db_files("db"), original);
}

#[tokio::test]
async fn a_bad_save_is_refused_and_changes_nothing() {
    let copy = Copy::new("bad");
    let original = copy.db_files("db");
    let app = memcraft_server::app(&copy.0, None);
    let whole_world = mcpe_world::pack_world_dir(&copy.0).unwrap();
    for body in [vec![], b"not columns".to_vec(), whole_world] {
        let (status, _) = call(&app, Method::PUT, "/api/world/columns", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    assert!(copy.backups().is_empty());
    assert_eq!(copy.db_files("db"), original);
}

#[tokio::test]
async fn saves_from_stale_viewers_keep_each_others_columns() {
    let copy = Copy::new("stale");
    let app = memcraft_server::app(&copy.0, None);
    // Two viewers load the same world and edit different columns.
    let (one, two) = (load(&app).await, load(&app).await);
    let (a, b) = (edited(&one, 3, 57), edited(&two, 200, 41));
    for col in [&a, &b] {
        let (status, _) = call(&app, Method::PUT, "/api/world/columns", mcpe_world::pack_columns(std::slice::from_ref(col)).unwrap()).await;
        assert_eq!(status, StatusCode::OK);
    }
    let after = World::open_dir(&copy.0).unwrap();
    assert_eq!(after.columns[&a.pos].block(1, 100, 1).0, 57);
    assert_eq!(after.columns[&b.pos].block(1, 100, 1).0, 41);
    assert_eq!(copy.backups().len(), 2);
}

#[tokio::test]
async fn the_web_build_is_served_beside_the_api() {
    let copy = Copy::new("web");
    let web = std::env::temp_dir().join(format!("memcraft-server-{}-web-dist", std::process::id()));
    std::fs::create_dir_all(&web).unwrap();
    std::fs::write(web.join("index.html"), "<title>MemCraft</title>").unwrap();
    std::fs::write(web.join("viewer.wasm"), b"\0asm").unwrap();
    let app = memcraft_server::app(&copy.0, Some(&web));

    let get = |uri: &'static str| call(&app, Method::GET, uri, vec![]);
    assert_eq!(get("/").await, (StatusCode::OK, b"<title>MemCraft</title>".to_vec()));
    assert_eq!(get("/viewer.wasm").await.0, StatusCode::OK);
    assert_eq!(get("/missing.js").await.0, StatusCode::NOT_FOUND);
    assert_eq!(get("/api/world").await.0, StatusCode::OK);
    // The wasm needs its own content type to be compiled while streaming.
    let request = Request::builder().uri("/viewer.wasm").body(Body::empty()).unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.headers()["content-type"], "application/wasm");
    let _ = std::fs::remove_dir_all(web);
}
