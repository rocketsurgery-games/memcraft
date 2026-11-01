//! Edits columns of the checked-in sample world, saves it with `WorldWriter`
//! and reads it back.

use std::path::{Path, PathBuf};

use mcpe_world::{Column, TerrainFormat, World, WorldWriter};
use nbt::{compound, Tag};

const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World");

/// A fresh, empty directory under cargo's per-target scratch space.
fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap() {
        let e = e.unwrap();
        let dst = to.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            copy_dir(&e.path(), &dst);
        } else {
            std::fs::copy(e.path(), dst).unwrap();
        }
    }
}

/// Edits a few columns: one of each layout, plus entity changes. Returns the
/// edited positions.
fn edit(world: &mut World) -> Vec<mcpe_world::ChunkPos> {
    let pick = |world: &World, f: TerrainFormat, skip: usize| {
        *world.columns.iter().filter(|(_, c)| c.format == f).nth(skip).unwrap().0
    };
    let legacy = pick(world, TerrainFormat::LegacyTerrain, 3);
    let sub = pick(world, TerrainFormat::SubChunkV0, 5);
    let sub_ents = pick(world, TerrainFormat::SubChunkV0, 9);

    let c = world.columns.get_mut(&legacy).unwrap();
    for (x, y, z, id, data) in [(3, 64, 3, 57, 0), (4, 100, 4, 35, 14), (5, 127, 5, 1, 0), (6, 1, 6, 0, 0)] {
        let i = Column::index(x, y, z);
        c.ids[i] = id;
        c.data[i] = data;
    }
    c.recompute_heights();

    // Building up into sections that were not stored, and mining a whole
    // vertical strip.
    let c = world.columns.get_mut(&sub).unwrap();
    for (x, y, z, id, data) in [(0, 127, 0, 41, 0), (1, 90, 1, 5, 3), (2, 20, 2, 0, 0)] {
        let i = Column::index(x, y, z);
        c.ids[i] = id;
        c.data[i] = data;
    }
    for y in 0..128 {
        c.ids[Column::index(9, y, 9)] = 0;
    }
    c.recompute_heights();

    let c = world.columns.get_mut(&sub_ents).unwrap();
    c.block_entities.push(compound([("id", Tag::String("Sign".into())), ("x", Tag::Int(1)), ("y", Tag::Int(70)), ("z", Tag::Int(2))]));
    c.entities.clear();
    vec![legacy, sub, sub_ents]
}

fn save(world: &World, dir: &Path, positions: &[mcpe_world::ChunkPos]) {
    let mut writer = WorldWriter::open_dir(SAMPLE).unwrap();
    for pos in positions {
        writer.put_column(&world.columns[pos]).unwrap();
    }
    writer.finish().unwrap().write_dir(dir).unwrap();
}

#[test]
fn edited_world_reads_back_as_edited() {
    let original = World::open_dir(SAMPLE).unwrap();
    let mut edited = World::open_dir(SAMPLE).unwrap();
    let positions = edit(&mut edited);

    let dir = scratch("edited_world");
    save(&edited, &dir, &positions);
    let reloaded = World::open_dir(&dir).unwrap();

    assert_eq!(reloaded.columns.len(), 256);
    for (pos, col) in &reloaded.columns {
        let want = if positions.contains(pos) { &edited.columns[pos] } else { &original.columns[pos] };
        assert!(col == want, "column {pos:?} differs after save");
    }
    // The edits themselves arrived.
    let legacy = &reloaded.columns[&positions[0]];
    assert_eq!(legacy.block(3, 64, 3), (57, 0));
    assert_eq!(legacy.block(4, 100, 4), (35, 14));
    let sub = &reloaded.columns[&positions[1]];
    assert_eq!(sub.block(0, 127, 0), (41, 0));
    assert_eq!(sub.block(1, 90, 1), (5, 3));
    assert_eq!(sub.block(9, 50, 9), (0, 0));
    let ents = &reloaded.columns[&positions[2]];
    assert!(ents.entities.is_empty());
    assert!(ents.block_entities.iter().any(|b| b.get("id").and_then(Tag::as_str) == Some("Sign") && b.get("y").and_then(Tag::as_i64) == Some(70)));

    // Everything outside the chunk records is as it was.
    assert_eq!(reloaded.level, original.level);
    assert_eq!(reloaded.players.len(), original.players.len());
    assert_eq!(std::fs::read(dir.join("level.dat")).unwrap(), std::fs::read(Path::new(SAMPLE).join("level.dat")).unwrap());
}

#[test]
fn saving_in_place_replaces_db_and_keeps_other_files() {
    let dir = scratch("in_place");
    copy_dir(Path::new(SAMPLE), &dir);
    let mut world = World::open_dir(&dir).unwrap();
    let positions = edit(&mut world);

    for round in 0..2 {
        let mut writer = WorldWriter::open_dir(&dir).unwrap();
        for pos in &positions {
            world.columns.get_mut(pos).unwrap().ids[Column::index(8, 110 + round, 8)] = 20;
            writer.put_column(&world.columns[pos]).unwrap();
        }
        writer.finish().unwrap().write_dir(&dir).unwrap();

        let reloaded = World::open_dir(&dir).unwrap();
        for pos in &positions {
            assert_eq!(reloaded.columns[pos].block(8, 110 + round, 8), (20, 0));
        }
    }
    assert!(dir.join("levelname.txt").exists());
    assert!(dir.join("players").exists());
    assert!(!dir.join("db.new").exists() && !dir.join("db.old").exists());
}

#[test]
fn a_column_without_light_cannot_be_written_as_legacy_terrain() {
    let mut world = World::open_dir(SAMPLE).unwrap();
    let col = world.columns.values_mut().find(|c| c.format == TerrainFormat::LegacyTerrain).unwrap();
    col.sky_light = None;
    let mut writer = WorldWriter::open_dir(SAMPLE).unwrap();
    assert!(writer.put_column(col).is_err());
}
