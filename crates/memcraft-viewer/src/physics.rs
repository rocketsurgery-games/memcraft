//! Walking physics, the way Minecraft does it: an axis-aligned box swept
//! through the voxel grid one axis at a time, with gravity, jumping and
//! automatic stepping over low ledges.
//!
//! The terrain is queried directly (see [`Solids`]); there are no colliders
//! to build or keep in sync with the meshes.

use std::collections::HashMap;

use bevy::prelude::*;
use mcpe_world::{ChunkPos, World as PeWorld, HEIGHT};

use crate::blocks::{self, Box16, Boxes, Near, Shape, FULL_BLOCK};

/// Player box: 0.6 wide and 1.8 tall, with the eyes 1.62 above the feet.
pub const HALF_WIDTH: f32 = 0.3;
pub const HEIGHT_BLOCKS: f32 = 1.8;
pub const EYE_HEIGHT: f32 = 1.62;

/// Blocks per second squared.
const GRAVITY: f32 = 32.0;
/// Fastest fall, in blocks per second.
const TERMINAL_VELOCITY: f32 = 78.0;
/// Launch speed of a jump: peaks about 1.27 blocks up, enough for a full block.
const JUMP_SPEED: f32 = 9.0;
/// Ledges up to this high are walked up without jumping (Minecraft's 0.6).
const STEP_HEIGHT: f32 = 0.6;
/// Penetration smaller than this doesn't count as touching, so resting on a
/// surface isn't mistaken for being stuck in it by float rounding.
const EPS: f32 = 1e-3;
/// Frames longer than this are simulated in several steps.
const MAX_STEP: f32 = 1.0 / 60.0;
/// A single long frame (a stall, a hidden tab) is cut to this much time.
const MAX_FRAME: f32 = 0.1;

const COLUMN_BLOCKS: usize = 16 * 16 * HEIGHT;

/// A block as `id << 8 | data`.
type Block = u16;

fn pack(id: u8, data: u8) -> Block {
    Block::from(id) << 8 | Block::from(data)
}

/// What can be stood on or run into: a compact copy of the world's blocks,
/// so the viewer doesn't have to keep the whole `mcpe_world::World` alive.
/// The solid parts of a block (see [`collision`]) are worked out when asked,
/// so a fence is right whatever stands next to it, with no bookkeeping when
/// a block changes.
///
/// Columns that aren't loaded count as solid, and so does everything below
/// the world, so the player can't walk off the edge into the void.
#[derive(Resource, Default)]
pub struct Solids {
    columns: HashMap<ChunkPos, Box<[Block]>>,
}

/// The boxes of a block that stop the player, as `blocks::look` draws it:
/// cubes, slabs and glass are solid, flowers and air are not, and neither is
/// liquid (there is no swimming yet, so you sink through it). Box-shaped
/// blocks say for themselves (open doors and gates can be walked through,
/// fences are 1.5 tall).
fn collision(id: u8, data: u8, near: Near) -> Boxes {
    match blocks::look(id, data).shape {
        _ if matches!(id, 8..=11) => Boxes::default(),
        Shape::Air | Shape::Cross => Boxes::default(),
        Shape::Cube | Shape::Translucent => Boxes::of(&[FULL_BLOCK]),
        Shape::Partial(h) => Boxes::of(&[[0, 0, 0, 16, (h * 16.0).round() as u8, 16]]),
        Shape::Boxes => blocks::model(id, data, near).collision,
    }
}

impl Solids {
    /// A copy of every block of `world`.
    pub fn from_world(world: &PeWorld) -> Self {
        let mut solids = Self::default();
        for (pos, col) in &world.columns {
            let column = solids.column_mut(*pos);
            // Both lay their blocks out the same way (see `Self::index`).
            for (to, (&id, &data)) in column.iter_mut().zip(col.ids.iter().zip(&col.data)) {
                *to = pack(id, data);
            }
        }
        solids
    }

    fn index(x: usize, y: usize, z: usize) -> usize {
        (x * 16 + z) * HEIGHT + y
    }

    #[allow(dead_code, reason = "used by block targeting and placing, next")]
    /// Follows a block edit. Positions outside the loaded world are ignored
    /// (they stay solid).
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u8, data: u8) {
        let pos = ChunkPos { x: x.div_euclid(16), z: z.div_euclid(16) };
        if let Some(c) = self.columns.get_mut(&pos).filter(|_| (0..HEIGHT as i32).contains(&y)) {
            c[Self::index(x.rem_euclid(16) as usize, y as usize, z.rem_euclid(16) as usize)] = pack(id, data);
        }
    }

    fn column_mut(&mut self, pos: ChunkPos) -> &mut [Block] {
        self.columns.entry(pos).or_insert_with(|| vec![0; COLUMN_BLOCKS].into())
    }

    /// Makes block `(x, y, z)` a solid box from the floor up to `height`
    /// (a multiple of an eighth, 0 to 1) of its own, loading its (otherwise
    /// empty) column if needed. It is a layer of snow.
    #[cfg(test)]
    pub fn set(&mut self, x: i32, y: i32, z: i32, height: f32) {
        let eighths = (height * 8.0).round() as u8;
        assert!(eighths as f32 == height * 8.0 && eighths <= 8, "not a snow layer height: {height}");
        let pos = ChunkPos { x: x.div_euclid(16), z: z.div_euclid(16) };
        let i = Self::index(x.rem_euclid(16) as usize, y as usize, z.rem_euclid(16) as usize);
        self.column_mut(pos)[i] = if eighths == 0 { 0 } else { pack(78, eighths - 1) };
    }

    /// Loads an empty column, as the world would for open air.
    #[cfg(test)]
    pub fn load_column(&mut self, x: i32, z: i32) {
        self.column_mut(ChunkPos { x, z });
    }

    /// The block at `(x, y, z)`; `None` if it is solid for want of knowing
    /// better (below the world, in an unloaded column). Above the world is air.
    fn block(&self, x: i32, y: i32, z: i32) -> Option<(u8, u8)> {
        if y >= HEIGHT as i32 {
            return Some((0, 0));
        }
        if y < 0 {
            return None;
        }
        let pos = ChunkPos { x: x.div_euclid(16), z: z.div_euclid(16) };
        let c = self.columns.get(&pos)?;
        let b = c[Self::index(x.rem_euclid(16) as usize, y as usize, z.rem_euclid(16) as usize)];
        Some(((b >> 8) as u8, b as u8))
    }

    /// The solid boxes of block `(x, y, z)`, in sixteenths of the block.
    fn boxes(&self, x: i32, y: i32, z: i32) -> Boxes {
        match self.block(x, y, z) {
            None => Boxes::of(&[FULL_BLOCK]),
            Some((id, data)) => collision(id, data, &|dx, dy, dz| self.block(x + dx, y + dy, z + dz).unwrap_or((0, 0))),
        }
    }

    /// How high the solid part of block `(x, y, z)` reaches, 0.0 to 1.5.
    #[cfg(test)]
    fn height(&self, x: i32, y: i32, z: i32) -> f32 {
        self.boxes(x, y, z).iter().map(|b| f32::from(b[4]) / 16.0).fold(0.0, f32::max)
    }

    /// The blocks that may have solid parts within `[lo, hi]`: those it
    /// overlaps, and the row below them, since a fence reaches up into the
    /// block above it.
    fn cells(lo: Vec3, hi: Vec3) -> impl Iterator<Item = IVec3> {
        let (lo, hi) = (lo.floor().as_ivec3(), hi.floor().as_ivec3());
        (lo.x..=hi.x).flat_map(move |x| {
            (lo.y - 1..=hi.y).flat_map(move |y| (lo.z..=hi.z).map(move |z| IVec3::new(x, y, z)))
        })
    }

    /// The solid boxes of a block, in world space as `(min, max)`.
    fn cell_boxes(&self, cell: IVec3) -> impl Iterator<Item = (Vec3, Vec3)> {
        let sixteenth = |b: Box16, i: usize| f32::from(b[i]) / 16.0;
        self.boxes(cell.x, cell.y, cell.z).into_iter().map(move |b| {
            let origin = cell.as_vec3();
            let (lo, hi) = (Vec3::new(sixteenth(b, 0), sixteenth(b, 1), sixteenth(b, 2)), Vec3::new(sixteenth(b, 3), sixteenth(b, 4), sixteenth(b, 5)));
            (origin + lo, origin + hi)
        })
    }

    /// Whether a player with their feet at `feet` fits there.
    pub fn is_free(&self, feet: Vec3) -> bool {
        let b = Aabb::at(feet);
        let (lo, hi) = (b.min + EPS, b.max - EPS);
        !Self::cells(lo, hi)
            .flat_map(|cell| self.cell_boxes(cell))
            .any(|(bmin, bmax)| (0..3).all(|i| bmax[i] > lo[i] && bmin[i] < hi[i]))
    }

    /// How far `b` can move along `axis` (a unit vector) by `delta` before
    /// it hits something; `delta` itself if the way is clear.
    fn clip(&self, b: Aabb, axis: Vec3, delta: f32) -> f32 {
        let a = if axis.x != 0.0 { 0 } else if axis.y != 0.0 { 1 } else { 2 };
        // Cells to look at: those the box overlaps on the two other axes,
        // and everything it sweeps through on `axis`.
        let (mut lo, mut hi) = (b.min + EPS, b.max - EPS);
        lo[a] = b.min[a].min(b.min[a] + delta);
        hi[a] = b.max[a].max(b.max[a] + delta);
        let mut d = delta;
        for (bmin, bmax) in Self::cells(lo, hi).flat_map(|cell| self.cell_boxes(cell)) {
            // Skip boxes that don't overlap the player's on the other axes.
            if (0..3).any(|i| i != a && (bmax[i] <= b.min[i] + EPS || bmin[i] >= b.max[i] - EPS)) {
                continue;
            }
            if d > 0.0 && bmin[a] >= b.max[a] - EPS {
                d = d.min((bmin[a] - b.max[a]).max(0.0));
            } else if d < 0.0 && bmax[a] <= b.min[a] + EPS {
                d = d.max((bmax[a] - b.min[a]).min(0.0));
            }
        }
        d
    }
}

/// An axis-aligned box.
#[derive(Clone, Copy)]
struct Aabb {
    min: Vec3,
    max: Vec3,
}

impl Aabb {
    /// The player's box, feet centred on `feet`.
    fn at(feet: Vec3) -> Self {
        Self {
            min: feet - Vec3::new(HALF_WIDTH, 0.0, HALF_WIDTH),
            max: feet + Vec3::new(HALF_WIDTH, HEIGHT_BLOCKS, HALF_WIDTH),
        }
    }

    fn moved(self, by: Vec3) -> Self {
        Self { min: self.min + by, max: self.max + by }
    }
}

/// A walking player's motion state; the position lives in the `Transform`.
#[derive(Default, Clone, Copy)]
pub struct Body {
    pub velocity: Vec3,
    pub on_ground: bool,
}

impl Body {
    /// Moves `feet` through `dt` seconds of walking at `wish` (the horizontal
    /// velocity the player is steering for, in blocks per second), jumping
    /// if asked to and standing on something.
    pub fn step(&mut self, feet: &mut Vec3, solids: &Solids, wish: Vec2, jump: bool, dt: f32) {
        let mut left = dt.min(MAX_FRAME);
        while left > 0.0 {
            let dt = left.min(MAX_STEP);
            left -= dt;
            self.substep(feet, solids, wish, jump, dt);
        }
    }

    fn substep(&mut self, feet: &mut Vec3, solids: &Solids, wish: Vec2, jump: bool, dt: f32) {
        if jump && self.on_ground {
            self.velocity.y = JUMP_SPEED;
        }
        self.velocity.y = (self.velocity.y - GRAVITY * dt).max(-TERMINAL_VELOCITY);
        self.velocity.x = wish.x;
        self.velocity.z = wish.y;

        // Vertical first, so `on_ground` is current when deciding to step up.
        let want = self.velocity * dt;
        let dy = solids.clip(Aabb::at(*feet), Vec3::Y, want.y);
        feet.y += dy;
        self.on_ground = want.y < 0.0 && dy > want.y;
        if dy != want.y {
            self.velocity.y = 0.0;
        }

        let (dx, dz) = self.slide(*feet, solids, want.x, want.z);
        let flat = Vec2::new(dx, dz);
        let blocked = flat.distance_squared(Vec2::new(want.x, want.z)) > 1e-10;
        if blocked && self.on_ground {
            // Try the same move one step up, then come back down onto
            // whatever is there.
            let up = solids.clip(Aabb::at(*feet), Vec3::Y, STEP_HEIGHT);
            let raised = *feet + Vec3::Y * up;
            let (sx, sz) = self.slide(raised, solids, want.x, want.z);
            if Vec2::new(sx, sz).length_squared() > flat.length_squared() + 1e-10 {
                let landed = solids.clip(Aabb::at(raised + Vec3::new(sx, 0.0, sz)), Vec3::Y, -up);
                *feet = raised + Vec3::new(sx, landed, sz);
                return;
            }
        }
        feet.x += dx;
        feet.z += dz;
        if blocked {
            if dx != want.x {
                self.velocity.x = 0.0;
            }
            if dz != want.z {
                self.velocity.z = 0.0;
            }
        }
    }

    /// How far a box at `feet` gets moving by `dx` then `dz`.
    fn slide(&self, feet: Vec3, solids: &Solids, dx: f32, dz: f32) -> (f32, f32) {
        let b = Aabb::at(feet);
        let dx = solids.clip(b, Vec3::X, dx);
        let dz = solids.clip(b.moved(Vec3::X * dx), Vec3::Z, dz);
        (dx, dz)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-block-thick floor (top surface at y = 1) over the columns
    /// `x0..=x1` by `z0..=z1`, all loaded.
    fn floor(x0: i32, x1: i32, z0: i32, z1: i32) -> Solids {
        let mut s = Solids::default();
        for cx in x0.div_euclid(16)..=x1.div_euclid(16) {
            for cz in z0.div_euclid(16)..=z1.div_euclid(16) {
                s.load_column(cx, cz);
            }
        }
        for x in x0..=x1 {
            for z in z0..=z1 {
                s.set(x, 0, z, 1.0);
            }
        }
        s
    }

    /// Runs `secs` of 60 fps frames.
    fn run(body: &mut Body, feet: &mut Vec3, solids: &Solids, wish: Vec2, jump: bool, secs: f32) {
        for _ in 0..(secs * 60.0) as usize {
            body.step(feet, solids, wish, jump, 1.0 / 60.0);
        }
    }

    const WALK: Vec2 = Vec2::new(4.3, 0.0);

    #[test]
    fn falls_and_lands_on_the_ground() {
        let solids = floor(-8, 8, -8, 8);
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 10.0, 0.5));
        run(&mut body, &mut feet, &solids, Vec2::ZERO, false, 0.3);
        assert!(feet.y < 10.0 && feet.y > 1.0 && !body.on_ground, "still falling: {feet}");
        run(&mut body, &mut feet, &solids, Vec2::ZERO, false, 2.0);
        assert!((feet.y - 1.0).abs() < 1e-3, "rests on the floor top: {feet}");
        assert!(body.on_ground && body.velocity.y == 0.0);
    }

    #[test]
    fn long_frames_and_terminal_velocity_do_not_tunnel() {
        let solids = floor(-8, 8, -8, 8);
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 120.0, 0.5));
        for _ in 0..200 {
            body.step(&mut feet, &solids, Vec2::ZERO, false, 0.1);
        }
        assert!((feet.y - 1.0).abs() < 1e-3, "{feet}");
        // A stall of several seconds is capped, not integrated in one go.
        let mut feet = Vec3::new(0.5, 100.0, 0.5);
        body.step(&mut feet, &solids, Vec2::ZERO, false, 30.0);
        assert!(feet.y > 90.0, "{feet}");
    }

    #[test]
    fn standing_still_stays_put() {
        let solids = floor(-8, 8, -8, 8);
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        run(&mut body, &mut feet, &solids, Vec2::ZERO, false, 2.0);
        assert_eq!(feet, Vec3::new(0.5, 1.0, 0.5));
        assert!(body.on_ground);
    }

    #[test]
    fn walks_on_flat_ground_at_walking_speed() {
        let solids = floor(-8, 20, -8, 8);
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        run(&mut body, &mut feet, &solids, WALK, false, 2.0);
        assert!((feet.x - (0.5 + 2.0 * 4.3)).abs() < 0.05, "{feet}");
        assert!((feet.y - 1.0).abs() < 1e-3 && body.on_ground);
    }

    #[test]
    fn cannot_walk_through_a_wall() {
        let mut solids = floor(-8, 20, -8, 8);
        for y in 1..4 {
            for z in -8..=8 {
                solids.set(5, y, z, 1.0);
            }
        }
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        run(&mut body, &mut feet, &solids, WALK, true, 3.0);
        assert!(feet.x + HALF_WIDTH <= 5.0 + EPS, "inside the wall: {feet}");
        assert!(feet.x + HALF_WIDTH > 4.9, "should be pressed against it: {feet}");
        // Walking into it diagonally slides along it instead of sticking.
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        run(&mut body, &mut feet, &solids, Vec2::new(3.0, 3.0), false, 2.0);
        assert!(feet.x + HALF_WIDTH <= 5.0 + EPS && feet.z > 4.0, "{feet}");
    }

    #[test]
    fn a_fast_walker_cannot_tunnel_through_a_thin_wall() {
        let mut solids = floor(-8, 20, -8, 8);
        solids.set(5, 1, 0, 1.0);
        solids.set(5, 2, 0, 1.0);
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.0));
        for _ in 0..30 {
            body.step(&mut feet, &solids, Vec2::new(500.0, 0.0), false, 0.05);
        }
        assert!(feet.x + HALF_WIDTH <= 5.0 + EPS, "{feet}");
    }

    #[test]
    fn jumps_up_one_block_but_cannot_walk_up_it() {
        let mut solids = floor(-8, 20, -8, 8);
        for x in 5..=12 {
            for z in -8..=8 {
                solids.set(x, 1, z, 1.0);
            }
        }
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        run(&mut body, &mut feet, &solids, WALK, false, 3.0);
        assert!(feet.x < 5.0 && feet.y == 1.0, "a full block is too high to step: {feet}");

        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        run(&mut body, &mut feet, &solids, WALK, true, 2.0);
        run(&mut body, &mut feet, &solids, WALK, false, 0.5);
        assert!(feet.x > 6.0 && (feet.y - 2.0).abs() < 1e-3, "should be on top of the block: {feet}");
    }

    #[test]
    fn a_jump_reaches_one_block_but_not_two() {
        let mut solids = floor(-8, 20, -8, 8);
        for z in -8..=8 {
            solids.set(5, 1, z, 1.0);
            solids.set(5, 2, z, 1.0);
        }
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        run(&mut body, &mut feet, &solids, WALK, true, 3.0);
        run(&mut body, &mut feet, &solids, WALK, false, 0.5);
        assert!(feet.x < 5.0 && feet.y == 1.0, "two blocks is a wall: {feet}");

        // A jump in place peaks between 1 and 2 blocks up.
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        let mut peak = feet.y;
        for _ in 0..90 {
            body.step(&mut feet, &solids, Vec2::ZERO, true, 1.0 / 60.0);
            peak = peak.max(feet.y);
        }
        assert!(peak - 1.0 > 1.1 && peak - 1.0 < 1.5, "peak {peak}");
    }

    #[test]
    fn steps_up_a_slab_without_jumping() {
        let mut solids = floor(-8, 20, -8, 8);
        for x in 5..=8 {
            for z in -8..=8 {
                solids.set(x, 1, z, 0.5);
            }
        }
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        run(&mut body, &mut feet, &solids, WALK, false, 2.0);
        assert!(feet.x > 6.0 && (feet.y - 1.5).abs() < 1e-3, "{feet}");
        // ... and walks off the far side and down again.
        let mut solids = floor(-8, 20, -8, 8);
        solids.set(5, 1, 0, 0.5);
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.0));
        run(&mut body, &mut feet, &solids, WALK, false, 3.0);
        assert!(feet.x > 6.0 && (feet.y - 1.0).abs() < 1e-3, "{feet}");
    }

    /// A stone wall three high along `x = 5` with a one wide doorway at
    /// `z = 0`, over a floor.
    fn wall_with_doorway() -> Solids {
        let mut solids = floor(-8, 20, -8, 8);
        for y in 1..4 {
            for z in -8..=8 {
                solids.set(5, y, z, 1.0);
            }
        }
        for y in 1..4 {
            solids.set(5, y, 0, 0.0);
        }
        solids
    }

    /// Walks east from the doorway's west side at `z = 0.5` for three seconds.
    fn walk_through_the_doorway(solids: &Solids) -> Vec3 {
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        run(&mut body, &mut feet, solids, WALK, false, 3.0);
        feet
    }

    #[test]
    fn an_open_door_can_be_walked_through_and_a_closed_one_cannot() {
        // Facing east: closed, its plate is the west 3/16 of the doorway.
        let mut solids = wall_with_doorway();
        solids.set_block(5, 1, 0, 64, 0);
        solids.set_block(5, 2, 0, 64, 8);
        let feet = walk_through_the_doorway(&solids);
        assert!(feet.x + HALF_WIDTH <= 5.0 + EPS, "walked through a closed door: {feet}");
        assert!(feet.x + HALF_WIDTH > 4.9, "should be pressed against it: {feet}");
        // Open, with its hinge on the left, it stands against the north side.
        solids.set_block(5, 1, 0, 64, 4);
        let feet = walk_through_the_doorway(&solids);
        assert!(feet.x > 8.0 && (feet.y - 1.0).abs() < 1e-3, "stuck in an open doorway: {feet}");
        // Open with its hinge on the right, it stands on the south side instead.
        solids.set_block(5, 2, 0, 64, 9);
        let feet = walk_through_the_doorway(&solids);
        assert!(feet.x > 8.0, "stuck in an open doorway: {feet}");
    }

    #[test]
    fn open_gates_signs_and_levers_are_passable() {
        for (id, data) in [(107, 4), (107, 5), (63, 0), (68, 2), (69, 5), (77, 3)] {
            let mut solids = wall_with_doorway();
            solids.set_block(5, 1, 0, id, data);
            let feet = walk_through_the_doorway(&solids);
            assert!(feet.x > 8.0, "{id}:{data} stopped the player at {feet}");
        }
        // ... but a closed gate is shut.
        let mut solids = wall_with_doorway();
        solids.set_block(5, 1, 0, 107, 1);
        let feet = walk_through_the_doorway(&solids);
        assert!(feet.x + HALF_WIDTH <= 5.0 + 6.0 / 16.0 + EPS && feet.x > 4.5, "{feet}");
    }

    #[test]
    fn a_fence_is_a_barrier_that_cannot_be_jumped() {
        let mut solids = floor(-8, 20, -8, 8);
        for z in -8..=8 {
            solids.set_block(5, 1, z, 85, 0);
        }
        for jump in [false, true] {
            let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
            run(&mut body, &mut feet, &solids, WALK, jump, 3.0);
            // The post's face is 6/16 into the block, and a jump does not
            // reach the fence's top at 1.5.
            assert!(feet.x + HALF_WIDTH <= 5.0 + 6.0 / 16.0 + EPS, "over or through the fence: {feet}");
            assert!(feet.x + HALF_WIDTH > 5.0, "should be at the fence: {feet}");
            assert!(feet.y < 2.5 && feet.y >= 1.0, "{feet}");
        }
        // Its collision box is 1.5 high, standing in the block above it too.
        assert_eq!(solids.height(5, 1, 0), 1.5);
        assert!(!solids.is_free(Vec3::new(5.5, 2.0, 0.5)), "the top half block of a fence is solid");
        assert!(solids.is_free(Vec3::new(5.5, 2.5, 0.5)));
    }

    #[test]
    fn stairs_are_walked_up_without_jumping() {
        let mut solids = floor(-8, 20, -8, 8);
        for z in -8..=8 {
            // Rising east, then the full block they lead to.
            solids.set_block(5, 1, z, 53, 0);
            for x in 6..=9 {
                solids.set_block(x, 1, z, 1, 0);
            }
        }
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        run(&mut body, &mut feet, &solids, WALK, false, 2.0);
        assert!(feet.x > 6.5 && (feet.y - 2.0).abs() < 1e-3, "{feet}");
        // Upside down, they are a slab in the top half of the block with the
        // step under it: one can stand on top, but not in the block.
        let mut solids = floor(-8, 20, -8, 8);
        solids.set_block(5, 1, 0, 53, 4);
        assert!(!solids.is_free(Vec3::new(5.5, 1.0, 0.5)));
        assert!(solids.is_free(Vec3::new(5.5, 2.0, 0.5)));
    }

    #[test]
    fn hits_its_head_on_a_ceiling() {
        let mut solids = floor(-8, 8, -8, 8);
        for x in -8..=8 {
            for z in -8..=8 {
                solids.set(x, 3, z, 1.0); // underside at y = 3
            }
        }
        let (mut body, mut feet) = (Body::default(), Vec3::new(0.5, 1.0, 0.5));
        let mut peak = feet.y;
        for _ in 0..90 {
            body.step(&mut feet, &solids, Vec2::ZERO, true, 1.0 / 60.0);
            peak = peak.max(feet.y);
        }
        assert!(peak + HEIGHT_BLOCKS <= 3.0 + EPS, "head went through the ceiling: {peak}");
        assert!(peak > 1.1, "the jump should still rise: {peak}");
    }

    #[test]
    fn crosses_chunk_boundaries_without_falling_through() {
        // Floor over a 2x2 block of columns, with the player crossing both
        // the x = 16 and z = 16 seams, and then the x = 0 and z = 0 ones.
        let solids = floor(-20, 40, -20, 40);
        for (from, wish) in [
            (Vec3::new(10.0, 1.0, 10.0), Vec2::new(4.3, 4.3)),
            (Vec3::new(20.0, 1.0, 20.0), Vec2::new(-4.3, -4.3)),
            (Vec3::new(15.7, 1.0, 5.0), Vec2::new(0.5, 0.0)),
            (Vec3::new(5.0, 1.0, -0.3), Vec2::new(0.0, 0.5)),
        ] {
            let (mut body, mut feet) = (Body::default(), from);
            for _ in 0..240 {
                body.step(&mut feet, &solids, wish, false, 1.0 / 60.0);
                assert_eq!(feet.y, 1.0, "dropped while crossing a seam at {feet}");
            }
        }
        // Standing exactly astride a seam, with the floor only in the column
        // on one side: still held up by it.
        let mut solids = Solids::default();
        solids.load_column(0, 0);
        solids.load_column(1, 0);
        solids.set(15, 0, 0, 1.0);
        let (mut body, mut feet) = (Body::default(), Vec3::new(16.1, 5.0, 0.5));
        run(&mut body, &mut feet, &solids, Vec2::ZERO, false, 2.0);
        assert!((feet.y - 1.0).abs() < 1e-3, "{feet}");
    }

    #[test]
    fn the_world_edge_and_bottom_are_walls() {
        // Walking off a loaded column toward an unloaded one stops at its edge.
        let solids = floor(0, 15, 0, 15);
        let (mut body, mut feet) = (Body::default(), Vec3::new(8.0, 1.0, 8.0));
        run(&mut body, &mut feet, &solids, WALK, true, 5.0);
        assert!(feet.x + HALF_WIDTH <= 16.0 + EPS && feet.y >= 1.0, "{feet}");
        // With no floor at all, you stop at the bottom of the world.
        let mut empty = Solids::default();
        empty.load_column(0, 0);
        let (mut body, mut feet) = (Body::default(), Vec3::new(8.0, 50.0, 8.0));
        run(&mut body, &mut feet, &empty, Vec2::ZERO, false, 5.0);
        assert!(feet.y.abs() < 1e-3, "{feet}");
    }

    #[test]
    fn is_free_sees_solids_and_the_world_edge() {
        let mut solids = floor(0, 15, 0, 15);
        assert!(solids.is_free(Vec3::new(8.0, 1.0, 8.0)));
        assert!(!solids.is_free(Vec3::new(8.0, 0.5, 8.0)), "inside the floor");
        solids.set(8, 2, 8, 0.5);
        assert!(!solids.is_free(Vec3::new(8.0, 1.0, 8.0)), "slab in the way");
        assert!(solids.is_free(Vec3::new(8.0, 2.5, 8.0)), "standing on the slab");
        assert!(!solids.is_free(Vec3::new(15.9, 1.0, 8.0)), "box pokes into the unloaded column");
    }

    /// How high the solid part of block `y` reaches at the point `(x, z)`.
    fn ground(solids: &Solids, x: f32, y: i32, z: f32) -> f32 {
        let (bx, bz) = (x.floor(), z.floor());
        let (lx, lz) = (((x - bx) * 16.0) as u8, ((z - bz) * 16.0) as u8);
        let boxes = solids.boxes(bx as i32, y, bz as i32);
        boxes.iter().filter(|b| b[0] <= lx && lx < b[3] && b[2] <= lz && lz < b[5]).map(|b| f32::from(b[4]) / 16.0).fold(0.0, f32::max)
    }

    /// The real world: drop the player into many columns, crossing chunk
    /// seams, from the top of the world; each lands on the highest solid
    /// block there and nowhere deeper.
    #[test]
    fn lands_on_the_real_terrain_everywhere() {
        let files = crate::world_files::DB_FILES.iter().map(|(n, b)| (n.to_string(), b.to_vec()));
        let world = PeWorld::from_files(crate::world_files::LEVEL_DAT, files).expect("embedded world should load");
        let solids = Solids::from_world(&world);
        let (x0, z0) = (world.columns.keys().map(|p| p.x).min().unwrap() * 16, world.columns.keys().map(|p| p.z).min().unwrap() * 16);
        let mut checked = 0;
        // Centres of blocks, plus points 0.01 either side of each seam.
        for i in 0..100 {
            for j in 0..100 {
                let (x, z) = (x0 as f32 + 2.5 + i as f32 * 2.0 + (i % 3) as f32 * 0.4, z0 as f32 + 2.5 + j as f32 * 2.0);
                let top = (0..HEIGHT as i32).rev().find(|&y| ground(&solids, x, y, z) > 0.0);
                let Some(top) = top else { continue };
                let (mut body, mut feet) = (Body::default(), Vec3::new(x, HEIGHT as f32 - 0.5, z));
                if !solids.is_free(feet) {
                    continue;
                }
                run(&mut body, &mut feet, &solids, Vec2::ZERO, false, 6.0);
                let expect = top as f32 + ground(&solids, x, top, z);
                // The box is 0.6 wide, so a taller neighbour column can hold it up.
                assert!(feet.y >= expect - 1e-3, "fell through to {feet} at ({x}, {z}), terrain top {expect}");
                assert!(solids.is_free(feet), "ended up inside terrain at {feet}");
                checked += 1;
            }
        }
        assert!(checked > 1000, "only {checked} drops");
    }
}
