//! Fly through an old Pocket Edition world.
//!
//! Runs natively (`cargo run -p memcraft-viewer --release`) or in the browser
//! (see `web/build.sh`). Controls: click to capture the mouse (Esc releases
//! it), WASD to move, Q/E down/up, Shift to go fast, scroll to change speed,
//! F to switch between flying and walking (Space jumps).

mod blocks;
mod catalog;
mod controls;
mod mesh;
mod physics;

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::ImageSampler;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::prelude::*;
use controls::{Controller, ControlsPlugin};
use mcpe_world::{ChunkPos, World as PeWorld, HEIGHT};
use physics::Solids;

mod world_files {
    include!(concat!(env!("OUT_DIR"), "/world_files.rs"));
}

const SKY: Color = Color::srgb(0.53, 0.72, 0.98);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "MemCraft".into(),
                // In the browser, render into the page's canvas and fill its parent.
                canvas: Some("#memcraft".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(ControlsPlugin)
        .insert_resource(ClearColor(SKY))
        .add_systems(Startup, setup)
        .run();
}

fn load_world() -> PeWorld {
    let files = world_files::DB_FILES.iter().map(|(n, b)| (n.to_string(), b.to_vec()));
    PeWorld::from_files(world_files::LEVEL_DAT, files).expect("embedded world should load")
}

/// sRGB vertex colours to the linear space Bevy expects.
fn linear(c: [f32; 4]) -> [f32; 4] {
    let f = |v: f32| v.powf(2.2);
    [f(c[0]), f(c[1]), f(c[2]), c[3]]
}

fn to_bevy(m: mesh::MeshData) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, m.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, m.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, m.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, m.colors.into_iter().map(linear).collect::<Vec<_>>())
        .with_inserted_indices(Indices::U32(m.indices))
}

/// Highest opaque block per column, so faces under cover can be drawn darker.
fn sky_heights(world: &PeWorld) -> HashMap<ChunkPos, [i32; 256]> {
    world
        .columns
        .iter()
        .map(|(pos, col)| {
            let h = std::array::from_fn(|i| {
                let (x, z) = (i % 16, i / 16);
                (0..HEIGHT)
                    .rev()
                    .find(|&y| {
                        let (id, data) = col.block(x, y, z);
                        blocks::look(id, data).shape == blocks::Shape::Cube
                    })
                    .map_or(-1, |y| y as i32)
            });
            (*pos, h)
        })
        .collect()
}

/// The embedded texture atlas, sampled without filtering to keep the pixels sharp.
fn atlas_image() -> Image {
    let size = Extent3d { width: blocks::atlas::WIDTH, height: blocks::atlas::HEIGHT, depth_or_array_layers: 1 };
    let mut image = Image::new(
        size,
        TextureDimension::D2,
        blocks::atlas::RGBA.to_vec(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    image
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let world = load_world();
    commands.insert_resource(Solids::from_world(&world));
    let heights = sky_heights(&world);
    let light = |x: i32, y: i32, z: i32| -> f32 {
        let pos = ChunkPos { x: x.div_euclid(16), z: z.div_euclid(16) };
        match heights.get(&pos) {
            Some(h) if y <= h[(z.rem_euclid(16) * 16 + x.rem_euclid(16)) as usize] => 0.55,
            _ => 1.0,
        }
    };

    // Lighting is baked into vertex colours (which multiply the texture), so
    // the materials are unlit. They differ only in how texture alpha is used.
    let atlas = images.add(atlas_image());
    let mut material = |alpha_mode| {
        materials.add(StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: Some(atlas.clone()),
            unlit: true,
            alpha_mode,
            ..default()
        })
    };
    let opaque = material(AlphaMode::Opaque);
    let cutout = material(AlphaMode::Mask(0.5));
    let translucent = material(AlphaMode::Blend);

    let mut quads = 0;
    for pos in world.columns.keys() {
        let m = mesh::mesh_column(&world, pos.x, pos.z, &light);
        for (pass, material) in [(m.opaque, &opaque), (m.cutout, &cutout), (m.translucent, &translucent)] {
            quads += pass.indices.len() / 6;
            if !pass.is_empty() {
                commands.spawn((Mesh3d(meshes.add(to_bevy(pass))), MeshMaterial3d(material.clone())));
            }
        }
    }
    info!("meshed {} columns, {quads} quads", world.columns.len());

    let view = start_view(&world);
    commands.spawn((
        Camera3d::default(),
        // Colours are baked and final; tonemapping would only shift them
        // (and the default tonemapper needs LUTs we don't ship).
        Tonemapping::None,
        view,
        Controller::new(&view, 8.0, 30.0),
        DistanceFog { color: SKY, falloff: FogFalloff::Linear { start: 160.0, end: 420.0 }, ..default() },
    ));

    commands.spawn((
        Text::new(format!(
            "{}\nclick to look around (Esc releases) - WASD move - Q/E down/up - Shift fast - scroll: speed\nF: fly/walk (walk: Space jumps)",
            world.name().unwrap_or("MemCraft")
        )),
        TextFont { font_size: FontSize::Px(14.0), ..default() },
        Node { position_type: PositionType::Absolute, top: px(8), left: px(10), ..default() },
    ));
}

/// Starts at the local player's saved eye position and facing, or above spawn.
fn start_view(world: &PeWorld) -> Transform {
    let player = world.players.get("~local_player");
    let floats = |k: &str| -> Vec<f32> {
        player
            .and_then(|p| p.get(k))
            .and_then(|t| t.as_list())
            .map(|l| l.iter().filter_map(|t| t.as_f64()).map(|v| v as f32).collect())
            .unwrap_or_default()
    };
    let (pos, rot) = (floats("Pos"), floats("Rotation"));
    let eye = match pos[..] {
        [x, y, z] => Vec3::new(x, y, z),
        _ => {
            let (x, y, z) = world.spawn();
            Vec3::new(x as f32, y as f32 + 20.0, z as f32)
        }
    };
    // Minecraft yaw 0 faces +Z and turns toward -X; positive pitch looks down.
    let (yaw, pitch) = match rot[..] {
        [yaw, pitch] => (yaw.to_radians(), pitch.to_radians()),
        _ => (0.0, 0.3),
    };
    let dir = Vec3::new(-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos());
    // Step back and up from the exact eye position, so the first frame shows
    // the surroundings rather than whatever the player was standing against.
    let flat = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
    let camera = eye - flat * 8.0 + Vec3::Y * 4.0;
    Transform::from_translation(camera).looking_at(eye + flat * 4.0, Vec3::Y)
}
