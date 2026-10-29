//! Converts the checked-in sample world and reads the result back.

use std::io::Read;

use nbt::{Endian, Tag};

fn converted() -> mcpe2java::JavaWorld {
    let w = mcpe_world::World::open_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World")).unwrap();
    mcpe2java::convert(&w)
}

/// Decodes every chunk of a region file.
fn region_chunks(mca: &[u8]) -> Vec<Tag> {
    let mut out = Vec::new();
    for slot in 0..1024 {
        let entry = u32::from_be_bytes(mca[slot * 4..slot * 4 + 4].try_into().unwrap());
        if entry == 0 {
            continue;
        }
        let off = (entry >> 8) as usize * 4096;
        let len = u32::from_be_bytes(mca[off..off + 4].try_into().unwrap()) as usize;
        assert_eq!(mca[off + 4], 2, "zlib compression");
        let mut raw = Vec::new();
        flate2::read::ZlibDecoder::new(&mca[off + 5..off + 4 + len]).read_to_end(&mut raw).unwrap();
        out.push(nbt::read(&raw, Endian::Big).unwrap().1);
    }
    out
}

#[test]
fn produces_one_region_with_every_chunk() {
    let java = converted();
    let names: Vec<_> = java.files.keys().cloned().collect();
    assert_eq!(names, ["level.dat", "region/r.0.0.mca"]);
    let chunks = region_chunks(&java.files["region/r.0.0.mca"]);
    assert_eq!(chunks.len(), 256);
}

#[test]
fn level_dat_is_gzipped_java_nbt() {
    let java = converted();
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(&java.files["level.dat"][..]).read_to_end(&mut raw).unwrap();
    let (_, root, _) = nbt::read(&raw, Endian::Big).unwrap();
    let data = root.get("Data").unwrap();
    assert_eq!(data.get("LevelName").unwrap().as_str(), Some("My World"));
    assert_eq!(data.get("Version").unwrap().get("Name").unwrap().as_str(), Some("1.12.2"));
    let pos = data.get("Player").unwrap().get("Pos").unwrap().as_list().unwrap();
    assert_eq!(pos.len(), 3);
}

#[test]
fn signs_keep_their_text() {
    let java = converted();
    let signs: Vec<String> = region_chunks(&java.files["region/r.0.0.mca"])
        .iter()
        .flat_map(|c| c.get("Level").unwrap().get("TileEntities").unwrap().as_list().unwrap().to_vec())
        .filter(|t| t.get("id").and_then(Tag::as_str) == Some("minecraft:sign"))
        .map(|t| t.get("Text3").unwrap().as_str().unwrap().to_string())
        .collect();
    assert_eq!(signs.len(), 24);
    assert!(signs.contains(&r#"{"text":"waterfall"}"#.to_string()));
}

#[test]
fn surface_is_sky_lit() {
    let java = converted();
    let chunks = region_chunks(&java.files["region/r.0.0.mca"]);
    // Air in the topmost written section of each chunk is open sky, so it
    // must be lit. (Solid blocks correctly carry 0; all-air sections above
    // are omitted and Java treats them as fully lit.)
    let (mut lit, mut dark) = (0, 0);
    for c in &chunks {
        let sections = c.get("Level").unwrap().get("Sections").unwrap().as_list().unwrap();
        let top = sections.last().unwrap();
        let (Tag::ByteArray(ids), Tag::ByteArray(sky)) = (top.get("Blocks").unwrap(), top.get("SkyLight").unwrap())
        else {
            panic!()
        };
        for (i, &id) in ids.iter().enumerate() {
            if id == 0 {
                let l = (sky[i / 2] as u8 >> ((i & 1) * 4)) & 0x0f;
                if l > 0 { lit += 1 } else { dark += 1 }
            }
        }
    }
    // A few air pockets are sealed inside builds.
    assert!(dark * 100 < lit, "{dark} dark vs {lit} lit air blocks");
}
