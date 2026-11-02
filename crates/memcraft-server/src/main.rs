//! `memcraft-server --world <dir> [--web <dir>] [--addr <host:port>]`; see the
//! library docs for what it serves.

use std::path::PathBuf;
use std::process::exit;

const USAGE: &str = "usage: memcraft-server --world <world dir> [--web <web build dir>] [--addr <host:port>]
  --world  the directory holding level.dat and db/ (edits are saved into it)
  --web    the viewer's browser build (default: crates/memcraft-viewer/web/dist; see
           crates/memcraft-viewer/web/build.sh)
  --addr   where to listen (default 127.0.0.1:8080; 0.0.0.0:8080 serves the LAN)";

fn die(message: &str) -> ! {
    eprintln!("{message}");
    exit(2)
}

#[tokio::main]
async fn main() {
    let (mut world, mut web, mut addr) = (None, PathBuf::from("crates/memcraft-viewer/web/dist"), "127.0.0.1:8080".to_string());
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().unwrap_or_else(|| die(&format!("{arg} needs a value\n{USAGE}")));
        match arg.as_str() {
            "--world" => world = Some(PathBuf::from(value())),
            "--web" => web = PathBuf::from(value()),
            "--addr" => addr = value(),
            _ => die(USAGE),
        }
    }
    let Some(world) = world else { die(USAGE) };
    // Fail now, not on the first request, if this is not a world.
    let opened = mcpe_world::World::open_dir(&world).unwrap_or_else(|e| die(&format!("error: opening {}: {e}", world.display())));
    let web = web.join("index.html").is_file().then_some(web);
    if web.is_none() {
        eprintln!("warning: no web build found (--web); only the /api endpoints are served");
    }
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| die(&format!("error: listening on {addr}: {e}")));
    println!(
        "serving {:?} ({} columns) from {} on http://{}/",
        opened.name().unwrap_or(""),
        opened.columns.len(),
        world.display(),
        listener.local_addr().unwrap()
    );
    axum::serve(listener, memcraft_server::app(world, web.as_deref())).await.unwrap();
}
