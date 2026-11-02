//! Embeds the sample world, the texture atlas and the painting picture in the
//! binary, so the browser build needs no file loading. `MEMCRAFT_WORLD` overrides which world
//! directory is used.

use std::fmt::Write;
use std::path::{Path, PathBuf};

fn main() {
    atlas();
    paintings();
    world();
}

/// Pixels per tile edge.
const TILE: usize = 16;
/// Tiles per atlas row.
const COLS: usize = 16;

// Biome tints, fixed. The textures are greyscale (and fairly dark) where
// Minecraft tints them, so these multipliers may exceed one; results clamp.
const GRASS: [f32; 3] = [0.77, 1.18, 0.49];
const FOLIAGE: [f32; 3] = [0.52, 0.95, 0.30];
const SPRUCE: [f32; 3] = [0.37, 0.67, 0.37];
const BIRCH: [f32; 3] = [0.67, 1.04, 0.44];

/// The tint baked into a texture, by file stem.
fn tint(name: &str) -> Option<[f32; 3]> {
    Some(match name {
        "grass_block_top" | "grass_block_side" | "grass" | "fern" | "tall_grass_bottom" | "tall_grass_top"
        | "large_fern_bottom" | "large_fern_top" => GRASS,
        "oak_leaves" | "jungle_leaves" | "acacia_leaves" | "dark_oak_leaves" | "vine" | "lily_pad" => FOLIAGE,
        "spruce_leaves" => SPRUCE,
        "birch_leaves" => BIRCH,
        _ => return None,
    })
}

/// Reads the first `TILE`x`TILE` frame of a PNG (animated textures are
/// vertical strips) as RGBA.
fn load(path: &Path) -> Vec<u8> {
    let mut decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.width as usize, TILE, "{}: textures must be {TILE} pixels wide", path.display());
    let rgba = buf[..info.buffer_size()]
        .chunks(info.color_type.samples())
        .flat_map(|p| match info.color_type {
            png::ColorType::Rgba => [p[0], p[1], p[2], p[3]],
            png::ColorType::Rgb => [p[0], p[1], p[2], 255],
            png::ColorType::GrayscaleAlpha => [p[0], p[0], p[0], p[1]],
            png::ColorType::Grayscale => [p[0], p[0], p[0], 255],
            png::ColorType::Indexed => unreachable!("EXPAND removes palettes"),
        })
        .collect::<Vec<u8>>();
    rgba[..TILE * TILE * 4].to_vec()
}

fn multiply(pixels: &mut [u8], tint: [f32; 3]) {
    for p in pixels.chunks_mut(4) {
        for (c, t) in p.iter_mut().zip(tint) {
            *c = (*c as f32 * t).round().min(255.0) as u8;
        }
    }
}

/// Packs `assets/textures/*.png` into one atlas: raw RGBA pixels plus Rust
/// constants naming each tile. Tile 0 is solid white, for flat-coloured blocks.
/// A texture `x.png` with a sibling `x_overlay.png` gets the overlay (tinted)
/// composited on top, as for the sides of grass blocks.
fn atlas() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let dir = manifest.join("assets/textures");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "png"))
        .map(|p| p.file_stem().unwrap().to_str().unwrap().to_string())
        .filter(|n| !n.ends_with("_overlay"))
        .collect();
    names.sort();

    let mut tiles = vec![vec![255u8; TILE * TILE * 4]];
    for name in &names {
        let mut px = load(&dir.join(format!("{name}.png")));
        let overlay = dir.join(format!("{name}_overlay.png"));
        if overlay.exists() {
            let mut over = load(&overlay);
            if let Some(t) = tint(name) {
                multiply(&mut over, t);
            }
            for (b, o) in px.chunks_mut(4).zip(over.chunks(4)) {
                let a = o[3] as u32;
                for i in 0..3 {
                    b[i] = ((o[i] as u32 * a + b[i] as u32 * (255 - a)) / 255) as u8;
                }
                b[3] = b[3].max(o[3]);
            }
        } else if let Some(t) = tint(name) {
            multiply(&mut px, t);
        }
        tiles.push(px);
    }

    let rows = tiles.len().div_ceil(COLS);
    let (width, height) = (COLS * TILE, rows * TILE);
    let mut rgba = vec![0u8; width * height * 4];
    for (i, tile) in tiles.iter().enumerate() {
        let (x0, y0) = (i % COLS * TILE, i / COLS * TILE);
        for y in 0..TILE {
            let dst = ((y0 + y) * width + x0) * 4;
            rgba[dst..dst + TILE * 4].copy_from_slice(&tile[y * TILE * 4..(y + 1) * TILE * 4]);
        }
    }

    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out.join("atlas.rgba"), rgba).unwrap();
    let mut src = String::new();
    writeln!(src, "pub const TILE_PX: u32 = {TILE};").unwrap();
    writeln!(src, "pub const COLS: u32 = {COLS};").unwrap();
    writeln!(src, "pub const WIDTH: u32 = {width};").unwrap();
    writeln!(src, "pub const HEIGHT: u32 = {height};").unwrap();
    writeln!(src, "/// Raw RGBA8 (sRGB) pixels, `WIDTH` x `HEIGHT`.").unwrap();
    writeln!(src, "pub static RGBA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/atlas.rgba\"));").unwrap();
    writeln!(src, "#[allow(dead_code)]\npub mod tile {{").unwrap();
    writeln!(src, "    pub const WHITE: u16 = 0;").unwrap();
    for (i, name) in names.iter().enumerate() {
        writeln!(src, "    pub const {}: u16 = {};", name.to_uppercase(), i + 1).unwrap();
    }
    writeln!(src, "}}").unwrap();
    std::fs::write(out.join("atlas.rs"), src).unwrap();
}

/// Embeds `assets/paintings/*.png` (the picture all painting motifs are cut
/// from) as raw RGBA, with its size.
fn paintings() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let path = manifest.join("assets/paintings/paintings_kristoffer_zetterstrand.png");
    println!("cargo:rerun-if-changed={}", path.display());
    let mut decoder = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba, "{}: expected RGBA", path.display());
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(out.join("paintings.rgba"), &buf[..info.buffer_size()]).unwrap();
    let mut src = String::new();
    writeln!(src, "pub const WIDTH: u32 = {};", info.width).unwrap();
    writeln!(src, "pub const HEIGHT: u32 = {};", info.height).unwrap();
    writeln!(src, "/// Raw RGBA8 (sRGB) pixels, `WIDTH` x `HEIGHT`.").unwrap();
    writeln!(src, "pub static RGBA: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/paintings.rgba\"));").unwrap();
    std::fs::write(out.join("paintings.rs"), src).unwrap();
}

fn world() {
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
