//! Fly through an old Pocket Edition world.
//!
//! Runs natively (`cargo run -p memcraft-viewer --release`) or in the browser
//! (see `web/build.sh`). Controls: click to capture the mouse (Esc releases
//! it), WASD to move, Q/E down/up, Shift to go fast, scroll to change speed,
//! F to switch between flying and walking (Space jumps).

mod blocks;
mod catalog;
mod controls;
mod hotbar;
mod mesh;
mod physics;
mod world;

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::ImageSampler;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::prelude::*;
use controls::{Controller, ControlsPlugin};
use mcpe_world::World as PeWorld;
use physics::Solids;
use world::{BlockMaterials, WorldMap};

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
        .init_resource::<hotbar::Hotbar>()
        .add_systems(Startup, setup)
        .add_systems(Update, world::remesh)
        .run();
}

fn load_world() -> PeWorld {
    let files = world_files::DB_FILES.iter().map(|(n, b)| (n.to_string(), b.to_vec()));
    PeWorld::from_files(world_files::LEVEL_DAT, files).expect("embedded world should load")
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let world = load_world();
    commands.insert_resource(Solids::from_world(&world));

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
    commands.insert_resource(BlockMaterials {
        opaque: material(AlphaMode::Opaque),
        cutout: material(AlphaMode::Mask(0.5)),
        translucent: material(AlphaMode::Blend),
    });

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
    // The columns are meshed by `world::remesh` on the first frame.
    commands.insert_resource(WorldMap::new(world));
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
