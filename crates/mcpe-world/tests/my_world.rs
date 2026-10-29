//! Decodes the checked-in sample world, `worlds/My World` (PE ~0.16, mid-2016).
//! It holds both terrain layouts, so it exercises both decoders.

use mcpe_world::{TerrainFormat, World};

fn world() -> World {
    World::open_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World")).unwrap()
}

#[test]
fn loads_level_dat() {
    let w = world();
    assert_eq!(w.name(), Some("My World"));
    assert_eq!(w.spawn(), (128, 64, 128));
}

#[test]
fn decodes_every_chunk_of_the_limited_world() {
    let w = world();
    // Old "limited" worlds are 16x16 chunks (256x256 blocks).
    assert_eq!(w.columns.len(), 256);
    let legacy = w.columns.values().filter(|c| c.format == TerrainFormat::LegacyTerrain).count();
    assert_eq!(legacy, 54);
    assert_eq!(w.columns.len() - legacy, 202);
    for c in w.columns.values() {
        assert!((0..16).contains(&c.pos.x) && (0..16).contains(&c.pos.z));
        assert!(c.biomes.is_some(), "{:?} has no biomes", c.pos);
    }
}

#[test]
fn bedrock_floor_and_surface() {
    let w = world();
    let mut floor = 0;
    let mut total = 0;
    for c in w.columns.values() {
        for x in 0..16 {
            for z in 0..16 {
                total += 1;
                // The floor is bedrock (id 7), except where builds replaced it
                // in creative mode (e.g. a gold block at y=0).
                floor += (c.block(x, 0, z).0 == 7) as usize;
                assert!((1..128).any(|y| c.block(x, y, z).0 != 0), "{:?} ({x},{z}) is empty", c.pos);
            }
        }
    }
    assert!(floor * 100 > total * 99, "only {floor}/{total} floor blocks are bedrock");
}

#[test]
fn decodes_players_and_block_entities() {
    let w = world();
    assert!(w.players.contains_key("~local_player"));
    let chests = w
        .columns
        .values()
        .flat_map(|c| &c.block_entities)
        .filter(|be| be.get("id").and_then(|t| t.as_str()) == Some("Chest"))
        .count();
    assert!(chests > 0);
}
