//! Aiming at blocks, breaking them and placing new ones, while the pointer
//! is locked.
//!
//! A ray from the camera (see [`cast`]) finds the block being looked at; it
//! is outlined and marked by a crosshair. [`BREAK_BUTTON`] removes it,
//! [`PLACE_BUTTON`] uses the block if it reacts to a click (see
//! [`interact`]) and otherwise puts the hotbar's selected block against the
//! face that was hit (see [`can_place`]), and [`PICK_BUTTON`] copies the block
//! into the selected hotbar slot. The buttons live in `controls`.
//!
//! Every block is treated as a full cube here, whatever its shape. What a
//! placed block turns out as, and the cells it takes, is `orient`'s rule.

use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::blocks::{self, Shape};
use crate::catalog;
use crate::container_view::OpenContainer;
use crate::containers;
use crate::buttons::{press_ticks, PendingReleases, POWERED};
use crate::controls::{is_locked, walk, Controller, Mode, WorldMouse, BREAK_BUTTON, PICK_BUTTON, PLACE_BUTTON};
use crate::hotbar::{Block, Hotbar};
use crate::interact::interact;
use crate::orient::orient;
use crate::physics::{Solids, EYE_HEIGHT, HALF_WIDTH, HEIGHT_BLOCKS};
use crate::sign_editor::EditSign;
use crate::signs;
use crate::world::{self, WorldMap};

/// How far the player reaches, in blocks.
const WALK_REACH: f32 = 6.0;
const FLY_REACH: f32 = 12.0;
/// Half the side of the box a flying camera keeps clear of placed blocks.
const CAMERA_HALF: f32 = 0.1;
/// Boxes closer than this don't overlap (as in `physics`, so a player resting
/// on a block isn't taken to be inside it).
const EPS: f32 = 1e-3;
/// The outline is a little bigger than the block so it isn't hidden by its faces,
/// and drawn with bars this thick (in blocks).
const OUTLINE_SCALE: f32 = 1.004;
const OUTLINE_WIDTH: f32 = 0.02;

pub struct TargetingPlugin;

impl Plugin for TargetingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Target>().add_systems(Startup, spawn_feedback).add_systems(
            Update,
            (
                // The click that locks the pointer never gets here, in any order: see `WorldMouse`.
                (edit_blocks, pick_block),
                // After the camera has moved, so the outline is where the view is.
                (update_target, show_feedback).chain().after(walk),
            ),
        );
    }
}

/// A block the ray hit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Hit {
    pub block: IVec3,
    /// Unit vector out of the face that was hit, pointing at the viewer's side.
    pub normal: IVec3,
}

impl Hit {
    /// The cell in front of the face that was hit.
    fn front(&self) -> IVec3 {
        self.block + self.normal
    }
}

/// What the player is looking at, as of the last frame.
#[derive(Resource, Default)]
struct Target(Option<Hit>);

/// Marks the crosshair, shown while the pointer is locked.
#[derive(Component)]
struct Crosshair;

/// Marks the box drawn around the targeted block.
#[derive(Component)]
struct Outline;

/// Whether a ray stops at this block: anything with a shape, except liquids.
fn targetable((id, data): Block) -> bool {
    !matches!(id, 8..=11) && blocks::look(id, data).shape != Shape::Air
}

/// Blocks that placing replaces instead of building against: air, liquids,
/// and the plants and layers that Minecraft lets you build over.
fn replaceable((id, _): Block) -> bool {
    matches!(id, 0 | 8..=11 | 31 | 51 | 78 | 106)
}

/// Walks a ray from `origin` along `dir` through the block grid (Amanatides
/// and Woo) and returns the first block for which `hit` is true, if it is
/// within `reach` blocks. A block the origin is inside is skipped, so the
/// hit always has a face to report.
fn cast(origin: Vec3, dir: Vec3, reach: f32, hit: impl Fn(IVec3) -> bool) -> Option<Hit> {
    let dir = dir.try_normalize()?;
    let mut cell = origin.floor().as_ivec3();
    let step = IVec3::new(dir.x.signum() as i32, dir.y.signum() as i32, dir.z.signum() as i32);
    // Distance along the ray to the next cell boundary on each axis, and
    // between two boundaries.
    let delta = (1.0 / dir).abs();
    let first = |o: f32, c: i32, d: f32| match d {
        d if d > 0.0 => (c as f32 + 1.0 - o) / d,
        d if d < 0.0 => (o - c as f32) / -d,
        _ => f32::INFINITY,
    };
    let mut next = Vec3::new(first(origin.x, cell.x, dir.x), first(origin.y, cell.y, dir.y), first(origin.z, cell.z, dir.z));
    loop {
        let axis = if next.x <= next.y && next.x <= next.z {
            0
        } else if next.y <= next.z {
            1
        } else {
            2
        };
        if next[axis] > reach {
            return None;
        }
        cell[axis] += step[axis];
        next[axis] += delta[axis];
        if hit(cell) {
            let mut normal = IVec3::ZERO;
            normal[axis] = -step[axis];
            return Some(Hit { block: cell, normal });
        }
    }
}

/// The box the player occupies, as `(min, max)`: the body in walk mode, a
/// small box around the camera while flying.
fn player_box(mode: Mode, camera: Vec3) -> (Vec3, Vec3) {
    match mode {
        Mode::Walk => {
            let feet = camera - Vec3::Y * EYE_HEIGHT;
            (feet - Vec3::new(HALF_WIDTH, 0.0, HALF_WIDTH), feet + Vec3::new(HALF_WIDTH, HEIGHT_BLOCKS, HALF_WIDTH))
        }
        Mode::Fly => (camera - Vec3::splat(CAMERA_HALF), camera + Vec3::splat(CAMERA_HALF)),
    }
}

/// Whether `block` may be put in `cell`, which holds `existing` now:
/// the cell must be free to build in, and the block must not trap the player
/// (blocks you can walk through, like flowers, may go anywhere).
fn can_place(existing: Block, block: Block, cell: IVec3, player: (Vec3, Vec3)) -> bool {
    if !replaceable(existing) {
        return false;
    }
    if matches!(blocks::look(block.0, block.1).shape, Shape::Air | Shape::Cross) {
        return true;
    }
    let (lo, hi) = (cell.as_vec3(), cell.as_vec3() + Vec3::ONE);
    let overlaps = player.0.cmplt(hi - EPS).all() && player.1.cmpgt(lo + EPS).all();
    !overlaps
}

/// How far up its block the ray meets the face `hit` (0..1): where on the
/// face the click landed, for blocks that go in the upper or lower half.
fn hit_height(origin: Vec3, dir: Vec3, hit: Hit) -> f32 {
    let axis = if hit.normal.x != 0 { 0 } else if hit.normal.y != 0 { 1 } else { 2 };
    let plane = hit.block[axis] + i32::from(hit.normal[axis] > 0);
    let y = origin.y + (plane as f32 - origin[axis]) / dir[axis] * dir.y;
    if y.is_finite() { (y - hit.block.y as f32).clamp(0.0, 1.0) } else { 0.0 }
}

/// The cells that placing `block` against `hit` fills, with what goes in each,
/// or none if it can't be placed: some cell is outside the loaded world
/// (`at` gives no block there) or not free (see [`can_place`]).
fn placement(
    block: Block,
    hit: Hit,
    height: f32,
    yaw: f32,
    player: (Vec3, Vec3),
    at: impl Fn(IVec3) -> Option<Block>,
) -> Vec<(IVec3, Block)> {
    let cells: Vec<_> = orient(block, hit.normal, height, yaw).into_iter().map(|(offset, b)| (hit.front() + offset, b)).collect();
    let free = cells.iter().all(|&(cell, b)| at(cell).is_some_and(|existing| can_place(existing, b, cell, player)));
    if free { cells } else { Vec::new() }
}

/// Clicks: the left button breaks the targeted block, the right one uses it
/// (a door opens, a lever flips, a button goes down, a sign's editor or a container's window opens) or, if it does nothing, places a block against it.
#[allow(clippy::too_many_arguments)] // a Bevy system: each parameter is a resource or message it uses
fn edit_blocks(
    mouse: Res<WorldMouse>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    target: Res<Target>,
    hotbar: Res<Hotbar>,
    mut map: ResMut<WorldMap>,
    mut solids: ResMut<Solids>,
    mut pending: ResMut<PendingReleases>,
    camera: Single<(&Transform, &Controller)>,
    mut sign_clicks: MessageWriter<EditSign>,
    mut container_clicks: MessageWriter<OpenContainer>,
) {
    let Some(hit) = target.0.filter(|_| is_locked(&cursor)) else {
        return;
    };
    if mouse.0.just_pressed(BREAK_BUTTON) {
        let b = hit.block;
        let old = map.block(b.x, b.y, b.z);
        if world::set_block(&mut map, &mut solids, b.x, b.y, b.z, 0, 0) {
            info!("broke {old:?} at {b}");
        }
    } else if mouse.0.just_pressed(PLACE_BUTTON) {
        let b = hit.block;
        let target = map.block(b.x, b.y, b.z);
        // A sign opens its text editor (see `sign_editor`) rather than changing a block.
        if signs::is_sign(target) {
            sign_clicks.write(EditSign(b));
            return;
        }
        // So does a container (see `container_view`).
        if containers::kind(target).is_some() {
            container_clicks.write(OpenContainer(b));
            return;
        }
        if let Some(edits) = interact(target, b, |p| map.block(p.x, p.y, p.z)) {
            for (p, (id, data)) in edits {
                if world::set_block(&mut map, &mut solids, p.x, p.y, p.z, id, data) {
                    info!("toggled {target:?} at {b}: {p} is now {:?}", (id, data));
                }
            }
            // A button that went down pops up again later.
            if let Some(ticks) = press_ticks(target.0).filter(|_| target.1 & POWERED == 0) {
                pending.press(b, ticks);
            }
            return;
        }
        let (transform, controller) = *camera;
        let Some(block) = hotbar.selected_block() else {
            return;
        };
        let height = hit_height(transform.translation, *transform.forward(), hit);
        let player = player_box(controller.mode, transform.translation);
        let cells = placement(block, hit, height, controller.yaw(), player, |c| map.holds(c).then(|| map.block(c.x, c.y, c.z)));
        for (cell, placed) in cells {
            if world::set_block(&mut map, &mut solids, cell.x, cell.y, cell.z, placed.0, placed.1) {
                info!("placed {placed:?} at {cell}");
            }
        }
    }
}

/// Middle click: puts the targeted block in the selected hotbar slot. A block
/// the catalog does not list as such (a door's upper half, a stair facing
/// north) stands for its catalog entry, see [`catalog::nearest`]; one it has
/// no entry for at all (a lit furnace) is not picked.
fn pick_block(
    mouse: Res<WorldMouse>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    target: Res<Target>,
    map: Res<WorldMap>,
    mut hotbar: ResMut<Hotbar>,
) {
    let Some(hit) = target.0.filter(|_| is_locked(&cursor) && mouse.0.just_pressed(PICK_BUTTON)) else {
        return;
    };
    let (id, data) = map.block(hit.block.x, hit.block.y, hit.block.z);
    if let Some(entry) = catalog::nearest(id, data) {
        hotbar.set_selected_block((entry.id, entry.data));
        info!("picked {} ({id}:{data}) from {}", entry.name, hit.block);
    }
}

/// Finds the block in the camera's sights.
fn update_target(map: Res<WorldMap>, camera: Single<(&Transform, &Controller)>, mut target: ResMut<Target>) {
    let (transform, controller) = *camera;
    let reach = match controller.mode {
        Mode::Walk => WALK_REACH,
        Mode::Fly => FLY_REACH,
    };
    target.0 = cast(transform.translation, *transform.forward(), reach, |c| targetable(map.block(c.x, c.y, c.z)));
}

/// Spawns the crosshair (two bars crossing at the screen centre) and the
/// outline (the twelve edges of a unit cube, as thin bars).
fn spawn_feedback(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
    let bar = |left: i32, top: i32, width: i32, height: i32| {
        (
            Node { position_type: PositionType::Absolute, left: px(left), top: px(top), width: px(width), height: px(height), ..default() },
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
        )
    };
    commands.spawn((
        Crosshair,
        Visibility::Hidden,
        Node {
            position_type: PositionType::Absolute,
            left: percent(50),
            top: percent(50),
            width: px(18),
            height: px(18),
            margin: UiRect { left: px(-9), top: px(-9), ..default() },
            ..default()
        },
        children![bar(0, 8, 18, 2), bar(8, 0, 2, 18)],
    ));

    // Thin bars along the twelve edges: unlike GPU lines they keep their
    // width, and render the same everywhere.
    let bar = meshes.add(Cuboid::new(1.0 + OUTLINE_WIDTH, OUTLINE_WIDTH, OUTLINE_WIDTH));
    let material = materials.add(StandardMaterial { base_color: Color::BLACK, unlit: true, ..default() });
    commands
        .spawn((Outline, Visibility::Hidden, Transform::default()))
        .with_children(|outline| {
            for (along, [a, b]) in [(Vec3::X, [Vec3::Y, Vec3::Z]), (Vec3::Y, [Vec3::X, Vec3::Z]), (Vec3::Z, [Vec3::X, Vec3::Y])] {
                for (sa, sb) in [(-0.5, -0.5), (-0.5, 0.5), (0.5, -0.5), (0.5, 0.5)] {
                    // The bar is built along x; turn it to lie along `along`.
                    let rotation = Quat::from_rotation_arc(Vec3::X, along);
                    outline.spawn((
                        Mesh3d(bar.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::from_translation(a * sa + b * sb).with_rotation(rotation),
                    ));
                }
            }
        });
}

/// Shows the crosshair while the pointer is locked, and the outline around
/// the targeted block.
fn show_feedback(
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    target: Res<Target>,
    mut crosshair: Single<&mut Visibility, (With<Crosshair>, Without<Outline>)>,
    mut outline: Single<(&mut Visibility, &mut Transform), With<Outline>>,
) {
    let locked = is_locked(&cursor);
    **crosshair = if locked { Visibility::Inherited } else { Visibility::Hidden };
    let (visibility, transform) = &mut *outline;
    match target.0.filter(|_| locked) {
        Some(hit) => {
            **visibility = Visibility::Inherited;
            **transform = Transform::from_translation(hit.block.as_vec3() + Vec3::splat(0.5)).with_scale(Vec3::splat(OUTLINE_SCALE));
        }
        None => **visibility = Visibility::Hidden,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::controls::{track_world_mouse, LOCK_BUTTON};

    /// Casts through a world that is empty except for the given blocks.
    fn cast_in(blocks: &[(i32, i32, i32)], origin: Vec3, dir: Vec3, reach: f32) -> Option<Hit> {
        let set: HashSet<IVec3> = blocks.iter().map(|&(x, y, z)| IVec3::new(x, y, z)).collect();
        cast(origin, dir, reach, |c| set.contains(&c))
    }

    fn hit(block: [i32; 3], normal: [i32; 3]) -> Option<Hit> {
        Some(Hit { block: IVec3::from(block), normal: IVec3::from(normal) })
    }

    #[test]
    fn rays_along_each_axis_report_the_face_they_meet() {
        let o = Vec3::new(0.5, 0.5, 0.5);
        for (dir, block, normal) in [
            ([1, 0, 0], [3, 0, 0], [-1, 0, 0]),
            ([-1, 0, 0], [-3, 0, 0], [1, 0, 0]),
            ([0, 1, 0], [0, 3, 0], [0, -1, 0]),
            ([0, -1, 0], [0, -3, 0], [0, 1, 0]),
            ([0, 0, 1], [0, 0, 3], [0, 0, -1]),
            ([0, 0, -1], [0, 0, -3], [0, 0, 1]),
        ] {
            let got = cast_in(&[(block[0], block[1], block[2])], o, IVec3::from(dir).as_vec3(), 6.0);
            assert_eq!(got, hit(block, normal), "along {dir:?}");
        }
    }

    #[test]
    fn diagonal_rays_hit_the_first_block_on_the_line() {
        // From the middle of cell (0,0,0) along (1,1,1), through (1,0,0) and (1,1,0) to (1,1,1).
        let got = cast_in(&[(1, 1, 1), (2, 2, 2)], Vec3::splat(0.5), Vec3::ONE, 6.0);
        assert_eq!(got.map(|h| h.block), Some(IVec3::ONE));

        // A shallow ray in the xz plane steps through (1,0,0) then (1,0,1) and reaches (2,0,1) by its -x face.
        let got = cast_in(&[(2, 0, 1)], Vec3::new(0.5, 0.5, 0.5), Vec3::new(2.0, 0.0, 1.0), 6.0);
        assert_eq!(got, hit([2, 0, 1], [-1, 0, 0]));

        // A block off the line is missed: the ray is at z = 1.25 where it enters x = 2.
        let got = cast_in(&[(2, 0, 2)], Vec3::new(0.5, 0.5, 0.5), Vec3::new(2.0, 0.0, 1.0), 6.0);
        assert_eq!(got, None);

        // Steeply down onto the top of a block.
        // (It reaches the top plane y = -2 at x = 2.375, z = 1.225.)
        let got = cast_in(&[(2, -3, 1)], Vec3::new(0.2, 0.9, 0.5), Vec3::new(3.0, -4.0, 1.0), 9.0);
        assert_eq!(got, hit([2, -3, 1], [0, 1, 0]));
    }

    #[test]
    fn rays_cross_chunk_seams_and_negative_coordinates() {
        // Across the chunk seam at x = 16, and the one at x = 0 going negative.
        let got = cast_in(&[(17, 70, 5)], Vec3::new(14.5, 70.5, 5.5), Vec3::X, 6.0);
        assert_eq!(got, hit([17, 70, 5], [-1, 0, 0]));
        // (It is at z = -2.3 where it enters x = -2.)
        let got = cast_in(&[(-3, 70, -3)], Vec3::new(1.2, 70.7, -1.5), Vec3::new(-1.0, 0.0, -0.25), 6.0);
        assert_eq!(got, hit([-3, 70, -3], [1, 0, 0]));
        // Starting in negative space, looking at positive.
        let got = cast_in(&[(0, 5, 0)], Vec3::new(-0.5, 5.5, 0.5), Vec3::X, 6.0);
        assert_eq!(got, hit([0, 5, 0], [-1, 0, 0]));
        // Exactly on a cell boundary, going negative: the cell behind is the first one entered.
        let got = cast_in(&[(-1, 5, 0)], Vec3::new(0.0, 5.5, 0.5), Vec3::NEG_X, 6.0);
        assert_eq!(got, hit([-1, 5, 0], [1, 0, 0]));
    }

    #[test]
    fn reach_is_measured_to_the_face_that_is_hit() {
        let o = Vec3::new(0.5, 0.5, 0.5);
        // The near face of x = 6 is 5.5 away; the one of x = 7 is 6.5.
        assert_eq!(cast_in(&[(6, 0, 0)], o, Vec3::X, 6.0), hit([6, 0, 0], [-1, 0, 0]));
        assert_eq!(cast_in(&[(7, 0, 0)], o, Vec3::X, 6.0), None);
        assert_eq!(cast_in(&[(7, 0, 0)], o, Vec3::X, 12.0), hit([7, 0, 0], [-1, 0, 0]));
        // A length that isn't 1 doesn't change the reach.
        assert_eq!(cast_in(&[(7, 0, 0)], o, Vec3::X * 40.0, 6.0), None);
        assert_eq!(cast_in(&[(0, 0, 0)], o, Vec3::ZERO, 6.0), None);
    }

    #[test]
    fn the_block_the_ray_starts_in_is_skipped() {
        let got = cast_in(&[(0, 0, 0), (2, 0, 0)], Vec3::new(0.5, 0.5, 0.5), Vec3::X, 6.0);
        assert_eq!(got, hit([2, 0, 0], [-1, 0, 0]));
    }

    #[test]
    fn rays_pass_through_air_and_liquids_but_stop_at_everything_else() {
        assert!(!targetable((0, 0)));
        assert!((8..=11).all(|id| !targetable((id, 0))));
        assert!([1, 2, 17, 20, 50, 44].iter().all(|&id| targetable((id, 0))), "stone, grass, log, glass, torch, slab");
    }

    #[test]
    fn rays_find_blocks_of_the_real_world_across_a_column_seam() {
        let mut map = WorldMap::new(crate::load_world(None).unwrap());
        // Clear a corridor through two columns (x = 47 and 48 are in different
        // ones), then close it with stone.
        for x in 44..=52 {
            assert!(map.set_block(x, 120, 40, 0, 0), "column of x = {x} is loaded");
        }
        assert!(map.set_block(48, 120, 40, 1, 0));
        let got = cast(Vec3::new(44.5, 120.5, 40.5), Vec3::X, 6.0, |c| targetable(map.block(c.x, c.y, c.z)));
        assert_eq!(got, hit([48, 120, 40], [-1, 0, 0]));
        let got = cast(Vec3::new(52.5, 120.5, 40.5), Vec3::NEG_X, 6.0, |c| targetable(map.block(c.x, c.y, c.z)));
        assert_eq!(got, hit([48, 120, 40], [1, 0, 0]));
    }

    /// A locked window, a world, a target and the hotbar, with `pick_block`.
    fn pick_app(target: Option<Hit>) -> App {
        let mut app = App::new();
        app.insert_resource(WorldMap::new(crate::load_world(None).unwrap()))
            .insert_resource(Target(target))
            .init_resource::<Hotbar>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<WorldMouse>()
            .add_systems(PreUpdate, track_world_mouse)
            .add_systems(Update, pick_block);
        let mut cursor = CursorOptions::default();
        crate::controls::set_locked(&mut cursor, true);
        app.world_mut().spawn((Window::default(), cursor, PrimaryWindow));
        app
    }

    fn middle_click(app: &mut App) {
        app.world_mut().resource_mut::<ButtonInput<MouseButton>>().press(PICK_BUTTON);
        app.update();
    }

    #[test]
    fn middle_click_puts_the_targeted_block_in_the_selected_slot() {
        let cell = IVec3::new(48, 120, 40);
        let hit = Some(Hit { block: cell, normal: IVec3::Y });
        let mut app = pick_app(hit);
        // A log lying on its side is picked as the plain catalog log.
        app.world_mut().resource_mut::<WorldMap>().set_block(cell.x, cell.y, cell.z, 17, 9);
        app.world_mut().resource_mut::<Hotbar>().selected = 3;
        middle_click(&mut app);
        let hotbar = app.world().resource::<Hotbar>();
        assert_eq!(hotbar.selected_block(), Some((17, 1)), "spruce log");
        assert_eq!(hotbar.slots[0], Some((1, 0)), "the other slots are as they were");
        assert_eq!(hotbar.selected, 3);
    }

    #[test]
    fn picking_needs_a_target_the_catalog_knows_and_a_locked_pointer() {
        let cell = IVec3::new(48, 120, 40);
        let hit = Some(Hit { block: cell, normal: IVec3::Y });
        let before = Hotbar::default();
        // Nothing in the sights.
        let mut app = pick_app(None);
        middle_click(&mut app);
        assert_eq!(*app.world().resource::<Hotbar>(), before);
        // A lit furnace has no entry.
        let mut app = pick_app(hit);
        app.world_mut().resource_mut::<WorldMap>().set_block(cell.x, cell.y, cell.z, 62, 3);
        middle_click(&mut app);
        assert_eq!(*app.world().resource::<Hotbar>(), before);
        // The pointer is free (a menu or the picker is up).
        let mut app = pick_app(hit);
        app.world_mut().resource_mut::<WorldMap>().set_block(cell.x, cell.y, cell.z, 17, 1);
        let mut cursors = app.world_mut().query::<&mut CursorOptions>();
        crate::controls::set_locked(&mut cursors.single_mut(app.world_mut()).unwrap(), false);
        middle_click(&mut app);
        assert_eq!(*app.world().resource::<Hotbar>(), before);
    }

    const STONE: Block = (1, 0);
    const AIR: Block = (0, 0);

    /// The stone block the crosshair is on, and the air cell above it, where a
    /// placed block would go.
    const AIMED: IVec3 = IVec3::new(48, 120, 40);
    const ABOVE: IVec3 = IVec3::new(48, 121, 40);

    /// What `edit_blocks`, `pick_block` and the pointer's systems make of a
    /// click in the real app, without rendering or a real input device: the
    /// pointer starts free (`locked` false) or locked, and `grab_pointer` runs
    /// `grab_first` or after the editing systems.
    fn click_app(locked: bool, grab_first: bool) -> App {
        use crate::controls::{grab_pointer, set_locked, Modal};
        use crate::buttons::PendingReleases;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(WorldMap::new(crate::load_world(None).unwrap()))
            .insert_resource(Target(Some(Hit { block: AIMED, normal: IVec3::Y })))
            .init_resource::<Hotbar>()
            .init_resource::<Solids>()
            .init_resource::<PendingReleases>()
            .init_resource::<Modal>()
            .init_resource::<WorldMouse>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_message::<EditSign>()
            .add_message::<OpenContainer>()
            .add_systems(PreUpdate, track_world_mouse);
        let editing = (edit_blocks, pick_block);
        if grab_first {
            app.add_systems(Update, (grab_pointer, editing).chain());
        } else {
            app.add_systems(Update, (editing, grab_pointer).chain());
        }
        let mut map = app.world_mut().resource_mut::<WorldMap>();
        assert!(map.set_block(AIMED.x, AIMED.y, AIMED.z, STONE.0, STONE.1));
        assert!(map.set_block(ABOVE.x, ABOVE.y, ABOVE.z, AIR.0, AIR.1));
        let mut cursor = CursorOptions::default();
        set_locked(&mut cursor, locked);
        app.world_mut().spawn((Window { focused: true, ..default() }, cursor, PrimaryWindow));
        let transform = Transform::from_translation(AIMED.as_vec3() + Vec3::new(0.5, 3.5, 0.5)).looking_to(Vec3::NEG_Y, Vec3::X);
        app.world_mut().spawn((Controller::new(&transform, 8.0, 30.0), transform));
        app
    }

    /// One frame as `bevy_input` delivers it: the buttons are pressed (or
    /// released) in `PreUpdate`, and cleared at the start of the next frame.
    fn frame(app: &mut App, press: &[MouseButton], release: &[MouseButton]) {
        let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
        mouse.clear();
        press.iter().for_each(|b| mouse.press(*b));
        release.iter().for_each(|b| mouse.release(*b));
        app.update();
    }

    fn locked(app: &mut App) -> bool {
        let mut cursors = app.world_mut().query::<&CursorOptions>();
        is_locked(cursors.single(app.world()).unwrap())
    }

    fn blocks(app: &App) -> (Block, Block) {
        let map = app.world().resource::<WorldMap>();
        (map.block(AIMED.x, AIMED.y, AIMED.z), map.block(ABOVE.x, ABOVE.y, ABOVE.z))
    }

    #[test]
    fn a_click_that_locks_the_pointer_edits_nothing_in_either_system_order() {
        for grab_first in [true, false] {
            // Left locks; the right and middle buttons go down in the same frame, and neither
            // places nor picks. Held on, the buttons still do nothing.
            let mut app = click_app(false, grab_first);
            let before = Hotbar::default();
            frame(&mut app, &[LOCK_BUTTON, PLACE_BUTTON, PICK_BUTTON], &[]);
            assert!(locked(&mut app), "the click locks (grab_first: {grab_first})");
            assert_eq!(blocks(&app), (STONE, AIR), "nothing is broken or placed (grab_first: {grab_first})");
            assert_eq!(*app.world().resource::<Hotbar>(), before);
            frame(&mut app, &[], &[]);
            assert_eq!(blocks(&app), (STONE, AIR), "still held (grab_first: {grab_first})");

            // The next click, with the pointer locked, does edit.
            frame(&mut app, &[], &[LOCK_BUTTON, PLACE_BUTTON, PICK_BUTTON]);
            frame(&mut app, &[BREAK_BUTTON], &[]);
            assert_eq!(blocks(&app).0, AIR, "a locked click breaks (grab_first: {grab_first})");
            frame(&mut app, &[PLACE_BUTTON], &[BREAK_BUTTON]);
            assert_eq!(blocks(&app).1, STONE, "and a locked right click places (grab_first: {grab_first})");
        }
    }

    #[test]
    fn a_press_that_began_free_stays_out_of_the_world_while_it_is_held() {
        for grab_first in [true, false] {
            // The right button goes down on a free pointer (it does not lock). Something else
            // (the Resume button) locks the pointer while the button is still down: the press
            // did not begin locked, so it is not a click on the world.
            let mut app = click_app(false, grab_first);
            frame(&mut app, &[PLACE_BUTTON], &[]);
            let mut cursors = app.world_mut().query::<&mut CursorOptions>();
            crate::controls::set_locked(&mut cursors.single_mut(app.world_mut()).unwrap(), true);
            frame(&mut app, &[], &[]);
            assert_eq!(blocks(&app), (STONE, AIR), "(grab_first: {grab_first})");
            assert!(!app.world().resource::<WorldMouse>().0.pressed(PLACE_BUTTON));
            // Released and pressed again, it is a click on the world.
            frame(&mut app, &[], &[PLACE_BUTTON]);
            frame(&mut app, &[PLACE_BUTTON], &[]);
            assert_eq!(blocks(&app).1, STONE, "(grab_first: {grab_first})");
        }
    }

    #[test]
    fn blocks_are_not_placed_inside_the_player() {
        // Standing with the feet in the middle of cell (10, 64, 10), eyes 1.62 up.
        let feet = Vec3::new(10.5, 64.0, 10.5);
        let player = player_box(Mode::Walk, feet + Vec3::Y * EYE_HEIGHT);
        // Refused at the feet and at the head.
        assert!(!can_place(AIR, STONE, IVec3::new(10, 64, 10), player));
        assert!(!can_place(AIR, STONE, IVec3::new(10, 65, 10), player));
        // The body is 0.6 wide, so a block next to it on the far side of 0.3 is fine...
        assert!(can_place(AIR, STONE, IVec3::new(11, 64, 10), player));
        assert!(can_place(AIR, STONE, IVec3::new(9, 65, 10), player));
        assert!(can_place(AIR, STONE, IVec3::new(10, 64, 11), player));
        // ...but not when the player stands on the seam and overlaps both cells.
        let on_seam = player_box(Mode::Walk, Vec3::new(11.0, 64.0 + EYE_HEIGHT, 10.5));
        assert!(!can_place(AIR, STONE, IVec3::new(11, 64, 10), on_seam));
        assert!(!can_place(AIR, STONE, IVec3::new(10, 64, 10), on_seam));
        // Above the head, below the feet (a pillar jump), and on a block's top surface.
        assert!(can_place(AIR, STONE, IVec3::new(10, 66, 10), player));
        assert!(can_place(AIR, STONE, IVec3::new(10, 63, 10), player));
        let jumping = player_box(Mode::Walk, Vec3::new(10.5, 65.3 + EYE_HEIGHT, 10.5));
        assert!(can_place(AIR, STONE, IVec3::new(10, 64, 10), jumping));
        // A plant doesn't trap anyone.
        assert!(can_place(AIR, (37, 0), IVec3::new(10, 64, 10), player));
    }

    #[test]
    fn a_flying_camera_only_keeps_its_own_small_box_clear() {
        let camera = Vec3::new(10.5, 70.5, 10.5);
        let player = player_box(Mode::Fly, camera);
        assert!(!can_place(AIR, STONE, IVec3::new(10, 70, 10), player));
        assert!(can_place(AIR, STONE, IVec3::new(10, 69, 10), player), "the body below doesn't count");
        assert!(can_place(AIR, STONE, IVec3::new(11, 70, 10), player));
    }

    #[test]
    fn only_replaceable_cells_take_a_block() {
        let far = (Vec3::splat(-100.0), Vec3::splat(-99.0));
        let cell = IVec3::new(0, 64, 0);
        assert!(can_place(AIR, STONE, cell, far));
        assert!(can_place((9, 3), STONE, cell, far), "water");
        assert!(can_place((31, 1), STONE, cell, far), "tall grass");
        assert!(!can_place(STONE, STONE, cell, far));
        assert!(!can_place((50, 5), STONE, cell, far), "torch");
    }

    const FAR: (Vec3, Vec3) = (Vec3::splat(-100.0), Vec3::splat(-99.0));

    /// The world is air everywhere except at the given cells.
    fn air_but(solid: &[IVec3]) -> impl Fn(IVec3) -> Option<Block> + '_ {
        move |c| Some(if solid.contains(&c) { STONE } else { AIR })
    }

    #[test]
    fn the_height_of_a_click_is_read_off_the_face_it_met() {
        // Looking along +x, level with a quarter of the way up block (3, 5, 0).
        let side = Hit { block: IVec3::new(3, 5, 0), normal: IVec3::NEG_X };
        assert_eq!(hit_height(Vec3::new(0.0, 5.25, 0.5), Vec3::X, side), 0.25);
        // Rising 1 for every 3 along: at the face, 3 along, it is a block higher.
        let h = hit_height(Vec3::new(0.0, 5.0, 0.5), Vec3::new(3.0, 1.0, 0.0), side);
        assert!((h - 1.0).abs() < 1e-5, "{h}");
        // A face towards -z, looking down: 2 along and 1 down from y = 65 is the block's foot.
        let north_face = Hit { block: IVec3::new(0, 64, 10), normal: IVec3::NEG_Z };
        let h = hit_height(Vec3::new(0.5, 65.0, 8.0), Vec3::new(0.0, -0.5, 1.0), north_face);
        assert!((h - 0.0).abs() < 1e-5, "{h}");
        // On the underside and the top, the height is 0 and 1.
        let top = Hit { block: IVec3::new(0, 64, 0), normal: IVec3::Y };
        assert_eq!(hit_height(Vec3::new(0.5, 70.0, 0.5), Vec3::NEG_Y, top), 1.0);
        let under = Hit { block: IVec3::new(0, 64, 0), normal: IVec3::NEG_Y };
        assert_eq!(hit_height(Vec3::new(0.5, 60.0, 0.5), Vec3::Y, under), 0.0);
    }

    #[test]
    fn a_door_takes_two_cells_and_both_must_be_free() {
        let hit = Hit { block: IVec3::new(0, 64, 0), normal: IVec3::Y };
        let cells = placement((64, 0), hit, 1.0, 0.0, FAR, air_but(&[]));
        assert_eq!(cells, vec![(IVec3::new(0, 65, 0), (64, 3)), (IVec3::new(0, 66, 0), (64, 8))]);
        // Stone where the upper half goes, or the world ends there: nothing at all.
        assert!(placement((64, 0), hit, 1.0, 0.0, FAR, air_but(&[IVec3::new(0, 66, 0)])).is_empty());
        let edge = |c: IVec3| (c.y < 66).then_some(AIR);
        assert!(placement((64, 0), hit, 1.0, 0.0, FAR, edge).is_empty());
        // The lower half is taken: also nothing.
        assert!(placement((64, 0), hit, 1.0, 0.0, FAR, air_but(&[IVec3::new(0, 65, 0)])).is_empty());
        // The player standing in the upper half's cell blocks it.
        let player = player_box(Mode::Walk, Vec3::new(0.5, 66.0 + EYE_HEIGHT, 0.5));
        assert!(placement((64, 0), hit, 1.0, 0.0, player, air_but(&[])).is_empty());
    }

    #[test]
    fn a_bed_takes_the_cell_ahead_for_its_head() {
        // Looking east (yaw -90 degrees), the head is the next cell to +x.
        let hit = Hit { block: IVec3::new(0, 64, 0), normal: IVec3::Y };
        let yaw = -std::f32::consts::FRAC_PI_2;
        let cells = placement((26, 0), hit, 1.0, yaw, FAR, air_but(&[]));
        assert_eq!(cells, vec![(IVec3::new(0, 65, 0), (26, 3)), (IVec3::new(1, 65, 0), (26, 3 | 8))]);
        assert!(placement((26, 0), hit, 1.0, yaw, FAR, air_but(&[IVec3::new(1, 65, 0)])).is_empty());
    }

    #[test]
    fn a_single_block_is_oriented_in_the_cell_in_front_of_the_face() {
        let hit = Hit { block: IVec3::new(0, 64, 0), normal: IVec3::X };
        let cells = placement((17, 2), hit, 0.5, 0.0, FAR, air_but(&[]));
        assert_eq!(cells, vec![(IVec3::new(1, 64, 0), (17, 2 | 4))]);
        assert!(placement((17, 2), hit, 0.5, 0.0, FAR, air_but(&[IVec3::new(1, 64, 0)])).is_empty());
        // Nothing to place against: a ladder on a floor.
        let floor = Hit { block: IVec3::new(0, 64, 0), normal: IVec3::Y };
        assert!(placement((65, 2), floor, 0.5, 0.0, FAR, air_but(&[])).is_empty());
    }

    /// The camera `web/drive.js` uses for its `CHEST` stage looks at a chest of
    /// the sample world that holds things.
    #[test]
    fn the_chest_drive_camera_aims_at_a_chest_with_items() {
        let map = WorldMap::new(crate::load_world(None).unwrap());
        let view = crate::debug_view::view(Vec3::new(90.5, 77.8, 110.5), 270.0, 5.0);
        let got = cast(view.translation, *view.forward(), WALK_REACH, |c| targetable(map.block(c.x, c.y, c.z)));
        let chest = got.expect("the camera sees something").block;
        assert_eq!(chest, IVec3::new(93, 77, 110));
        assert!(crate::containers::contents(&map, chest).unwrap().slots.iter().any(|s| s.stack.is_some()));
    }
}
