//! Fly through an old Pocket Edition world.
//!
//! Runs natively (`cargo run -p memcraft-viewer --release [-- <world dir>]`) or
//! in the browser (see `web/build.sh`). Natively, an optional argument names a
//! world directory to open instead of the embedded sample world; Save (in the
//! menu, or Ctrl+S) then writes the edits back to it (see `save`).
//!
//! Controls: click to capture the mouse (Esc releases
//! it and opens the menu), WASD to move, Q/E down/up, Shift to go fast,
//! scroll to change speed,
//! F to switch between flying and walking (Space jumps), 1-9 to choose a block,
//! left click to break the block in the crosshair, right click to use a door,
//! lever or button, or else to place one.

mod blocks;
mod buttons;
mod catalog;
mod controls;
mod debug_view;
#[cfg(any(target_arch = "wasm32", test))]
mod download;
mod entities;
mod font;
mod hotbar;
mod interact;
mod item_icons;
mod menu;
mod mesh;
mod physics;
mod save;
mod sign_editor;
mod signs;
mod targeting;
mod toast;
mod world;

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::image::ImageSampler;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::prelude::*;
use controls::{Controller, ControlsPlugin};
use mcpe_world::World as PeWorld;
use physics::Solids;
use std::path::{Path, PathBuf};
use world::{BlockMaterials, WorldMap};

mod world_files {
    include!(concat!(env!("OUT_DIR"), "/world_files.rs"));
}

/// The texture atlas as a GPU image, for the world's materials and the hotbar icons.
#[derive(Resource)]
struct AtlasImage(Handle<Image>);

const SKY: Color = Color::srgb(0.53, 0.72, 0.98);

fn main() {
    let dir = world_dir_arg();
    let world = load_world(dir.as_deref()).unwrap_or_else(|e| {
        eprintln!("error: opening {}: {e}", dir.as_deref().unwrap_or(Path::new("the embedded world")).display());
        std::process::exit(1);
    });
    let map = WorldMap::new(world);
    let mut app = App::new();
    app.insert_resource(Solids::from_world(map.world())).insert_resource(map);
    app.add_plugins(save::SavePlugin { dir });
    app
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
        .add_plugins((ControlsPlugin, targeting::TargetingPlugin, buttons::ButtonsPlugin, menu::MenuPlugin, signs::SignsPlugin, sign_editor::SignEditorPlugin, toast::ToastPlugin))
        .insert_resource(ClearColor(SKY))
        .add_plugins(hotbar::HotbarPlugin)
        .add_systems(Startup, setup)
        // After `Startup`, so the world and materials `setup` inserts are there.
        .add_systems(PostStartup, entities::spawn)
        .add_systems(Update, world::remesh)
        .run();
}

/// The world directory named on the command line, if any. The browser has
/// no command line.
#[cfg(not(target_arch = "wasm32"))]
fn world_dir_arg() -> Option<PathBuf> {
    let mut args = std::env::args_os().skip(1);
    let dir = args.next().map(PathBuf::from);
    if args.next().is_some() || dir.as_deref().is_some_and(|d| d.starts_with("-")) {
        eprintln!("usage: memcraft-viewer [world dir]\n(the directory holding level.dat and db/; the embedded world if omitted)");
        std::process::exit(2);
    }
    dir
}

#[cfg(target_arch = "wasm32")]
fn world_dir_arg() -> Option<PathBuf> {
    None
}

/// Loads the world in `dir`, or the embedded one.
fn load_world(dir: Option<&Path>) -> mcpe_world::Result<PeWorld> {
    match dir {
        Some(dir) => PeWorld::open_dir(dir),
        None => PeWorld::from_files(world_files::LEVEL_DAT, embedded_db_files()),
    }
}

/// The files of the embedded world's `db/`.
fn embedded_db_files() -> impl Iterator<Item = (String, Vec<u8>)> {
    world_files::DB_FILES.iter().map(|(n, b)| (n.to_string(), b.to_vec()))
}

/// A writer over the embedded world, for saving it somewhere other than a
/// directory (the browser's download).
#[cfg(any(target_arch = "wasm32", test))]
fn embedded_writer() -> mcpe_world::Result<mcpe_world::WorldWriter> {
    mcpe_world::WorldWriter::from_files(world_files::LEVEL_DAT.to_vec(), embedded_db_files())
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
    map: Res<WorldMap>,
) {
    let world = map.world();

    // Lighting is baked into vertex colours (which multiply the texture), so
    // the materials are unlit. They differ only in how texture alpha is used.
    let atlas = images.add(atlas_image());
    commands.insert_resource(AtlasImage(atlas.clone()));
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

    let view = debug_view::requested().unwrap_or_else(|| start_view(world));
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
        Text::new(world.name().unwrap_or("MemCraft")),
        TextFont { font_size: FontSize::Px(14.0), ..default() },
        Node { position_type: PositionType::Absolute, top: px(8), left: px(10), ..default() },
    ));
    // The columns are meshed by `world::remesh` on the first frame.
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
    let (yaw, pitch) = match rot[..] {
        [yaw, pitch] => (yaw.to_radians(), pitch.to_radians()),
        _ => (0.0, 0.3),
    };
    // Minecraft yaw 0 faces +Z and turns toward -X; positive pitch looks down.
    let dir = Vec3::new(-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos());
    // Step back and up from the exact eye position, so the first frame shows
    // the surroundings rather than whatever the player was standing against.
    let flat = Vec3::new(dir.x, 0.0, dir.z).normalize_or_zero();
    let camera = eye - flat * 8.0 + Vec3::Y * 4.0;
    Transform::from_translation(camera).looking_at(eye + flat * 4.0, Vec3::Y)
}
