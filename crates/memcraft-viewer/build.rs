//! Embeds the sample world in the binary, so the browser build needs no
//! file loading. `MEMCRAFT_WORLD` overrides which world directory is used.

use std::fmt::Write;
use std::path::PathBuf;

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let world = std::env::var("MEMCRAFT_WORLD")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest.join("../../worlds/My World"));
    let world = world.canonicalize().unwrap_or_else(|e| panic!("world {}: {e}", world.display()));
    println!("cargo:rerun-if-env-changed=MEMCRAFT_WORLD");
    println!("cargo:rerun-if-changed={}", world.display());

    let mut db: Vec<_> = std::fs::read_dir(world.join("db"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .collect();
    db.sort();

    let mut out = String::new();
    writeln!(out, "pub const LEVEL_DAT: &[u8] = include_bytes!({:?});", world.join("level.dat")).unwrap();
    writeln!(out, "pub const DB_FILES: &[(&str, &[u8])] = &[").unwrap();
    for p in db {
        let name = p.file_name().unwrap().to_str().unwrap();
        writeln!(out, "    ({name:?}, include_bytes!({p:?})),").unwrap();
    }
    writeln!(out, "];").unwrap();
    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("world_files.rs");
    std::fs::write(dest, out).unwrap();
}
