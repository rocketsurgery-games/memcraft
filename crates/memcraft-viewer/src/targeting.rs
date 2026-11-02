//! Aiming at blocks, breaking them and placing new ones, while the pointer
//! is locked.
//!
//! A ray from the camera (see [`cast`]) finds the block being looked at; it
//! is outlined and marked by a crosshair. [`BREAK_BUTTON`] removes it,
//! [`PLACE_BUTTON`] uses the block if it reacts to a click (see
//! [`interact`]) and otherwise puts the hotbar's selected block against the
//! face that was hit (see [`can_place`]). The buttons live in `controls`.
//!
//! Every block is treated as a full cube here, whatever its shape.

use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow};

use crate::blocks::{self, Shape};
use crate::buttons::{press_ticks, PendingReleases, POWERED};
use crate::controls::{grab_pointer, is_locked, walk, Controller, Mode, BREAK_BUTTON, PLACE_BUTTON};
use crate::hotbar::{Block, Hotbar};
use crate::interact::interact;
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
                // Before the click that locks the pointer is handled, so that click only locks.
                edit_blocks.before(grab_pointer),
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

/// Clicks: the left button breaks the targeted block, the right one uses it
/// (a door opens, a lever flips, a button goes down, a sign's editor opens) or, if it does nothing, places a block against it.
#[allow(clippy::too_many_arguments)] // a Bevy system: each parameter is a resource or message it uses
fn edit_blocks(
    mouse: Res<ButtonInput<MouseButton>>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    target: Res<Target>,
    hotbar: Res<Hotbar>,
    mut map: ResMut<WorldMap>,
    mut solids: ResMut<Solids>,
    mut pending: ResMut<PendingReleases>,
    camera: Single<(&Transform, &Controller)>,
    mut sign_clicks: MessageWriter<EditSign>,
) {
    let Some(hit) = target.0.filter(|_| is_locked(&cursor)) else {
        return;
    };
    if mouse.just_pressed(BREAK_BUTTON) {
        let b = hit.block;
        let old = map.block(b.x, b.y, b.z);
        if world::set_block(&mut map, &mut solids, b.x, b.y, b.z, 0, 0) {
            info!("broke {old:?} at {b}");
        }
    } else if mouse.just_pressed(PLACE_BUTTON) {
        let b = hit.block;
        let target = map.block(b.x, b.y, b.z);
        // A sign opens its text editor (see `sign_editor`) rather than changing a block.
        if signs::is_sign(target) {
            sign_clicks.write(EditSign(b));
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
        let (cell, Some(block)) = (hit.front(), hotbar.selected_block()) else {
            return;
        };
        if can_place(map.block(cell.x, cell.y, cell.z), block, cell, player_box(controller.mode, transform.translation))
            && world::set_block(&mut map, &mut solids, cell.x, cell.y, cell.z, block.0, block.1)
        {
            info!("placed {block:?} at {cell}");
        }
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

    const STONE: Block = (1, 0);
    const AIR: Block = (0, 0);

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
}
