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
fn produces_one_region_with_every_chunk_plus_empty_neighbours() {
    let java = converted();
    let regions: Vec<_> = java.files.keys().filter(|k| k.starts_with("region/")).collect();
    assert_eq!(regions.len(), 9);
    assert!(java.files.contains_key("level.dat"));
    let chunks = region_chunks(&java.files["region/r.0.0.mca"]);
    assert_eq!(chunks.len(), 256);
    let empty = &java.files["region/r.-1.0.mca"];
    assert_eq!(empty.len(), 8192);
    assert!(region_chunks(empty).is_empty());
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
    // Spawn is the player's saved spot (eye y 66.99 -> feet 65).
    let spawn: Vec<_> = ["SpawnX", "SpawnY", "SpawnZ"].map(|k| data.get(k).unwrap().as_i64().unwrap()).to_vec();
    assert_eq!(spawn, [44, 65, 232]);
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

fn all_entities(java: &mcpe2java::JavaWorld) -> Vec<Tag> {
    region_chunks(&java.files["region/r.0.0.mca"])
        .iter()
        .flat_map(|c| c.get("Level").unwrap().get("Entities").unwrap().as_list().unwrap().to_vec())
        .collect()
}

fn int(t: &Tag, k: &str) -> i64 {
    t.get(k).unwrap().as_i64().unwrap()
}

/// Java (id, data) of a world block, read back from the region file.
fn java_block(java: &mcpe2java::JavaWorld, x: i32, y: i32, z: i32) -> (u8, u8) {
    let chunk = region_chunks(&java.files["region/r.0.0.mca"])
        .into_iter()
        .find(|c| {
            let l = c.get("Level").unwrap();
            int(l, "xPos") == (x >> 4) as i64 && int(l, "zPos") == (z >> 4) as i64
        })
        .unwrap();
    let sections = chunk.get("Level").unwrap().get("Sections").unwrap().as_list().unwrap().to_vec();
    let section = sections.iter().find(|s| int(s, "Y") == (y >> 4) as i64).unwrap();
    let i = ((y & 15) * 256 + (z & 15) * 16 + (x & 15)) as usize;
    let (Tag::ByteArray(ids), Tag::ByteArray(data)) = (section.get("Blocks").unwrap(), section.get("Data").unwrap()) else {
        panic!()
    };
    (ids[i] as u8, (data[i / 2] as u8 >> ((i & 1) * 4)) & 15)
}

#[test]
fn item_frames_become_entities_holding_their_items() {
    let java = converted();
    let frames: Vec<Tag> =
        all_entities(&java).into_iter().filter(|e| e.get("id").unwrap().as_str() == Some("minecraft:item_frame")).collect();
    assert_eq!(frames.len(), 9);
    // All hang on the south face of an iron wall at z = 236.
    for f in &frames {
        assert_eq!((int(f, "Facing"), int(f, "TileZ")), (0, 237));
    }
    let item = |x: i64, y: i64| {
        let f = frames.iter().find(|f| int(f, "TileX") == x && int(f, "TileY") == y).unwrap();
        f.get("Item").cloned()
    };
    let armor = item(116, 71).unwrap();
    assert_eq!(armor.get("id").unwrap().as_str(), Some("minecraft:diamond_horse_armor"));
    assert_eq!(armor.get("tag").unwrap().get("display").unwrap().get("Name").unwrap().as_str(), Some("Special diamond horse armor"));
    // PE enchantment 5 (thorns) is Java 7.
    let boots = item(114, 71).unwrap();
    assert_eq!(boots.get("id").unwrap().as_str(), Some("minecraft:iron_boots"));
    let ench = boots.get("tag").unwrap().get("ench").unwrap().as_list().unwrap()[0].clone();
    assert_eq!(int(&ench, "id"), 7);
    // Leather horse armour has no Java 1.12 item: the frame stays, empty.
    assert!(item(115, 70).is_none());
}

#[test]
fn entities_are_converted() {
    let java = converted();
    let entities = all_entities(&java);
    let count = |id: &str| entities.iter().filter(|e| e.get("id").unwrap().as_str() == Some(id)).count();
    assert!(count("minecraft:sheep") > 0 && count("minecraft:chicken") > 0 && count("minecraft:minecart") > 0);
    assert_eq!(java.report.skipped_entities.values().sum::<usize>(), 1);
    // A painting's PE tile is the wall block (x = 9); Java's is the block in front.
    let wasteland = entities.iter().find(|e| e.get("Motive").and_then(Tag::as_str) == Some("Wasteland")).unwrap();
    assert_eq!([int(wasteland, "TileX"), int(wasteland, "TileY"), int(wasteland, "TileZ")], [10, 68, 106]);
    assert_eq!(int(wasteland, "Facing"), 3);
    for e in &entities {
        assert_eq!(e.get("Pos").unwrap().as_list().unwrap().len(), 3);
    }
}

#[test]
fn extended_pistons_keep_their_heads() {
    let java = converted();
    // A sticky piston facing up (29:1), extended: base gets bit 8, sticky head 34:9.
    assert_eq!(java_block(&java, 161, 68, 211), (29, 1 | 8));
    assert_eq!(java_block(&java, 161, 69, 211), (34, 1 | 8));
    // A retracted one is untouched.
    assert_eq!(java_block(&java, 173, 66, 205), (29, 1));
    assert!(java.report.skipped_block_entities.keys().all(|k| k == "NetherReactor"));
}
