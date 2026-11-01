//! A world saved by `mcpe_world::WorldWriter` converts like the world it was
//! saved from.

use mcpe_world::{Column, World, WorldWriter};

const SAMPLE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World");

#[test]
fn converts_a_saved_world_like_the_edited_one() {
    let mut world = World::open_dir(SAMPLE).unwrap();
    let mut writer = WorldWriter::open_dir(SAMPLE).unwrap();
    let positions: Vec<_> = world.columns.keys().copied().step_by(11).collect();
    for pos in &positions {
        let col = world.columns.get_mut(pos).unwrap();
        col.ids[Column::index(7, 100, 7)] = 57;
        col.ids[Column::index(7, 2, 7)] = 0;
        col.recompute_heights();
        writer.put_column(col).unwrap();
    }
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("mcpe2java_saved_world");
    let _ = std::fs::remove_dir_all(&dir);
    writer.finish().unwrap().write_dir(&dir).unwrap();

    let reloaded = World::open_dir(&dir).unwrap();
    let want = mcpe2java::convert(&world);
    let got = mcpe2java::convert(&reloaded);
    assert_eq!(got.report.chunks, 256);
    assert_eq!(got.report.chunks, want.report.chunks);
    assert_eq!(got.report.block_entities, want.report.block_entities);
    assert_eq!(got.report.entities, want.report.entities);
    assert_eq!(got.files.keys().collect::<Vec<_>>(), want.files.keys().collect::<Vec<_>>());
    for (name, bytes) in &want.files {
        if name.starts_with("region/") {
            assert!(bytes == &got.files[name], "{name} differs");
        }
    }
}
