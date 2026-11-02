//! Sign text: the four lines of a `Sign` block entity, drawn on the face of
//! the sign board.
//!
//! Standing signs (block 63) and wall signs (68) are boxes drawn by the
//! mesher (see `blocks::model`); their text is a separate quad a hair in
//! front of the board, textured with the lines rendered by `font`. Each sign
//! with text is one entity, rebuilt when its block or its block entity is
//! edited (see [`WorldMap::take_edited_cells`]). Editing the text is the job
//! of `sign_editor`.
//!
//! Bedrock keeps the lines in `Text1` to `Text4`; newer saves keep one `Text`
//! with the lines apart by newlines. Both are read, and a write goes to the
//! form the entity already has.

use std::collections::HashSet;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use mcpe_world::{Tag, World as PeWorld};

use crate::entities::{Facing, Wall};
use crate::font;
use crate::hotbar::Block;
use crate::mesh::MeshData;
use crate::world::{to_bevy, WorldMap};

/// How far the text floats in front of the board, so it wins the depth test.
const LAYER: f32 = 0.003;

pub struct SignsPlugin;

impl Plugin for SignsPlugin {
    fn build(&self, app: &mut App) {
        // After `Startup`, so the world `setup` inserts is there.
        app.add_systems(PostStartup, spawn).add_systems(Update, sync);
    }
}

/// Marks the text entity of the sign at a cell.
#[derive(Component)]
struct SignText(IVec3);

/// Whether `block` is a sign: standing (63) or wall (68).
pub fn is_sign((id, _): Block) -> bool {
    matches!(id, 63 | 68)
}

/// Where the text goes on a sign: the way it faces, the vertical span of the
/// board (in blocks from the middle of the cell) and how far the front of the
/// board is out from it. Matches the boxes of `blocks::standing_sign` and
/// `blocks::wall_sign`. `None` for other blocks.
fn board((id, data): Block) -> Option<(Facing, [f32; 2], f32)> {
    match id {
        // Quarter turns of the board; the post is 8 px tall, the board 8 px
        // high and 2 px thick, centred.
        63 => Some(((((data as usize & 15) + 2) / 4 % 4) as Facing, [0.0, 0.5], 1.0 / 16.0)),
        // The board is against the wall behind it, 5 to 13 px up.
        68 => Some(([2, 0, 1, 3][((data as usize).max(2) - 2) % 4], [-3.0 / 16.0, 5.0 / 16.0], -0.5 + 2.0 / 16.0)),
        _ => None,
    }
}

/// The four lines of a `Sign` block entity (empty ones if it has none).
pub fn lines(be: &Tag) -> [String; 4] {
    let text = |k: &str| be.get(k).and_then(Tag::as_str).map(str::to_owned);
    if be.get("Text").is_some() {
        let all = text("Text").unwrap_or_default();
        let mut parts = all.split('\n').map(str::to_owned);
        return std::array::from_fn(|_| parts.next().unwrap_or_default());
    }
    ["Text1", "Text2", "Text3", "Text4"].map(|k| text(k).unwrap_or_default())
}

/// The block entity of the sign at `cell` with `lines` as its text: `old`
/// with the text replaced, or a new one.
pub fn with_lines(old: Option<&Tag>, cell: IVec3, lines: &[String; 4]) -> Tag {
    let mut entries = match old {
        Some(Tag::Compound(entries)) => entries.clone(),
        _ => vec![
            ("id".to_owned(), Tag::String("Sign".into())),
            ("x".to_owned(), Tag::Int(cell.x)),
            ("y".to_owned(), Tag::Int(cell.y)),
            ("z".to_owned(), Tag::Int(cell.z)),
        ],
    };
    let mut set = |key: &str, text: String| match entries.iter_mut().find(|(k, _)| k == key) {
        Some((_, tag)) => *tag = Tag::String(text),
        None => entries.push((key.to_owned(), Tag::String(text))),
    };
    if old.is_some_and(|be| be.get("Text").is_some()) {
        set("Text", lines.join("\n"));
    } else {
        for (n, line) in lines.iter().enumerate() {
            set(&format!("Text{}", n + 1), line.clone());
        }
    }
    Tag::Compound(entries)
}

/// The quad of the text of the sign `block`, as a mesh.
fn text_mesh(block: Block, cell: IVec3) -> Option<MeshData> {
    let (facing, [v0, v1], w) = board(block)?;
    let wall = Wall::new(cell.as_vec3().to_array().map(|v| v + 0.5), facing);
    let mut mesh = MeshData::default();
    let w = w + LAYER;
    wall.quad(&mut mesh, ([-0.5, v0, w], [0.5, v1, w]), (2, true), [1.0; 4], [0.0, 0.0, 1.0, 1.0]);
    Some(mesh)
}

fn text_image(lines: &[String; 4]) -> Image {
    let size = Extent3d { width: font::WIDTH as u32, height: font::HEIGHT as u32, depth_or_array_layers: 1 };
    let mut image = Image::new(size, TextureDimension::D2, font::render(lines), TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::RENDER_WORLD);
    image.sampler = ImageSampler::nearest();
    image
}

/// Spawns the text entity of the sign at `cell`, if the block there is a sign
/// with something written on it.
fn spawn_text(commands: &mut Commands, map: &WorldMap, cell: IVec3, meshes: &mut Assets<Mesh>, materials: &mut Assets<StandardMaterial>, images: &mut Assets<Image>) -> bool {
    let block = map.block(cell.x, cell.y, cell.z);
    let Some(lines) = map.block_entity(cell).map(lines).filter(|l| l.iter().any(|l| !l.is_empty())) else {
        return false;
    };
    let Some(mesh) = text_mesh(block, cell) else {
        return false;
    };
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(images.add(text_image(&lines))),
        unlit: true,
        alpha_mode: AlphaMode::Mask(0.5),
        ..default()
    });
    commands.spawn((SignText(cell), Mesh3d(meshes.add(to_bevy(mesh))), MeshMaterial3d(material)));
    true
}

/// Every `Sign` block entity of the world.
fn sign_cells(world: &PeWorld) -> Vec<IVec3> {
    world
        .columns
        .values()
        .flat_map(|col| &col.block_entities)
        .filter(|be| be.get("id").and_then(Tag::as_str) == Some("Sign"))
        .filter_map(|be| Some(IVec3::new(be.get("x")?.as_i64()? as i32, be.get("y")?.as_i64()? as i32, be.get("z")?.as_i64()? as i32)))
        .collect()
}

/// Draws the text of every sign of the world.
fn spawn(mut commands: Commands, map: Res<WorldMap>, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>, mut images: ResMut<Assets<Image>>) {
    let cells = sign_cells(map.world());
    let drawn = cells.iter().filter(|&&cell| spawn_text(&mut commands, &map, cell, &mut meshes, &mut materials, &mut images)).count();
    info!("drew the text of {drawn} of {} signs", cells.len());
}

/// Redraws the text of signs whose block or text was edited (a broken sign
/// loses its text with its block entity).
fn sync(
    mut commands: Commands,
    mut map: ResMut<WorldMap>,
    texts: Query<(Entity, &SignText)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let edited: HashSet<IVec3> = map.take_edited_cells().into_iter().collect();
    if edited.is_empty() {
        return;
    }
    for (entity, text) in &texts {
        if edited.contains(&text.0) {
            commands.entity(entity).despawn();
        }
    }
    for cell in edited {
        spawn_text(&mut commands, &map, cell, &mut meshes, &mut materials, &mut images);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::{self, Shape};

    fn sample() -> PeWorld {
        PeWorld::open_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World")).unwrap()
    }

    fn lines_of(text: [&str; 4]) -> [String; 4] {
        text.map(String::from)
    }

    #[test]
    fn the_four_lines_are_read_from_either_form() {
        let old = Tag::Compound(vec![
            ("id".into(), Tag::String("Sign".into())),
            ("Text1".into(), Tag::String("one".into())),
            ("Text3".into(), Tag::String("three".into())),
        ]);
        assert_eq!(lines(&old), lines_of(["one", "", "three", ""]));
        let new = Tag::Compound(vec![("Text".into(), Tag::String("a\nb\nc".into()))]);
        assert_eq!(lines(&new), lines_of(["a", "b", "c", ""]));
        assert_eq!(lines(&Tag::Compound(vec![])), lines_of(["", "", "", ""]));
    }

    #[test]
    fn writing_keeps_the_form_and_the_other_entries() {
        let cell = IVec3::new(1, 2, 3);
        let fresh = with_lines(None, cell, &lines_of(["a", "b", "", "d"]));
        assert_eq!(fresh.get("id").and_then(Tag::as_str), Some("Sign"));
        assert_eq!(fresh.get("y").and_then(Tag::as_i64), Some(2));
        assert_eq!(lines(&fresh), lines_of(["a", "b", "", "d"]));

        let edited = with_lines(Some(&fresh), cell, &lines_of(["x", "", "", ""]));
        assert_eq!(lines(&edited), lines_of(["x", "", "", ""]));
        let Tag::Compound(entries) = &edited else { panic!() };
        assert_eq!(entries.len(), 8, "no entry added or lost");

        let single = Tag::Compound(vec![("id".into(), Tag::String("Sign".into())), ("Text".into(), Tag::String("old".into()))]);
        let edited = with_lines(Some(&single), cell, &lines_of(["a", "b", "", ""]));
        assert_eq!(edited.get("Text").and_then(Tag::as_str), Some("a\nb\n\n"));
        assert!(edited.get("Text1").is_none());
        assert_eq!(lines(&edited), lines_of(["a", "b", "", ""]));
    }

    #[test]
    fn the_sample_world_has_signs_with_text() {
        let world = sample();
        let cells = sign_cells(&world);
        assert_eq!(cells.len(), 24, "mcpe2java's test finds 24 signs too");
        let map = WorldMap::new(world);
        for &cell in &cells {
            let block = map.block(cell.x, cell.y, cell.z);
            assert!(is_sign(block), "{cell} holds {block:?}");
            assert!(board(block).is_some());
            assert_eq!(blocks::look(block.0, block.1).shape, Shape::Boxes);
        }
        let written: Vec<[String; 4]> = cells.iter().map(|&c| lines(map.block_entity(c).unwrap())).filter(|l| l.iter().any(|l| !l.is_empty())).collect();
        // The sign of the screenshots: a standing sign facing +z.
        assert_eq!(map.block(14, 70, 45), (63, 0));
        assert_eq!(lines(map.block_entity(IVec3::new(14, 70, 45)).unwrap()), lines_of(["i love", "this", "waterfall", ""]));
        assert!(written.len() > 20, "most signs have text");
    }

    #[test]
    fn the_text_lies_on_the_front_of_the_board() {
        // A standing sign facing +z (data 0) and one facing -x (data 4), and
        // wall signs facing north (2) and east (5).
        for (block, front) in [((63, 0), Vec3::Z), ((63, 4), Vec3::NEG_X), ((68, 2), Vec3::NEG_Z), ((68, 5), Vec3::X)] {
            let cell = IVec3::new(10, 70, 10);
            let mesh = text_mesh(block, cell).unwrap();
            let boxes = blocks::model(block.0, block.1, &|_, _, _| (0, 0)).visual;
            // The board is the widest of the sign's boxes (a post is narrow).
            let board = boxes.iter().max_by_key(|b| (b[3] - b[0]) * (b[5] - b[2])).unwrap();
            let centre = cell.as_vec3() + Vec3::splat(0.5);
            let p = |i: usize| Vec3::from(mesh.positions[i]) - centre;
            let depth = p(0).dot(front);
            let (lo, hi) = (Vec3::new(board[0] as f32, board[1] as f32, board[2] as f32) / 16.0 - 0.5, Vec3::new(board[3] as f32, board[4] as f32, board[5] as f32) / 16.0 - 0.5);
            // On the board's front face, a hair out.
            let face = if front.x != 0.0 { if front.x > 0.0 { hi.x } else { -lo.x } } else if front.z > 0.0 { hi.z } else { -lo.z };
            assert!((depth - face - LAYER).abs() < 1e-4, "{block:?}: text at {depth}, front of the board at {face}");
            // And inside the board's extent up and across.
            for i in 0..4 {
                let q = p(i);
                assert!(q.y >= lo.y - 1e-4 && q.y <= hi.y + 1e-4, "{block:?}: {q}");
                let across = if front.x != 0.0 { q.z } else { q.x };
                assert!(across.abs() <= 0.5 + 1e-4);
            }
            assert_eq!(Vec3::from(mesh.normals[0]), front);
        }
    }

    #[test]
    fn breaking_a_sign_removes_its_text_and_the_edit_is_reported() {
        let mut map = WorldMap::new(crate::load_world(None).unwrap());
        let cell = sign_cells(map.world())[0];
        assert!(map.block_entity(cell).is_some());
        let block = map.block(cell.x, cell.y, cell.z);
        map.take_edited_cells();

        let tag = with_lines(map.block_entity(cell), cell, &lines_of(["hi", "", "", ""]));
        assert!(map.set_block_entity(cell, tag));
        assert_eq!(lines(map.block_entity(cell).unwrap())[0], "hi");
        assert_eq!(map.take_edited_cells(), [cell]);
        assert!(!map.set_block_entity(IVec3::new(-100_000, 70, 0), with_lines(None, IVec3::new(-100_000, 70, 0), &Default::default())));

        // Same block, other data: the text stays.
        assert!(map.set_block(cell.x, cell.y, cell.z, block.0, block.1 ^ 1));
        assert!(map.block_entity(cell).is_some());
        assert!(map.set_block(cell.x, cell.y, cell.z, 0, 0));
        assert!(map.block_entity(cell).is_none());
        assert_eq!(map.take_edited_cells(), [cell, cell]);
    }
}
