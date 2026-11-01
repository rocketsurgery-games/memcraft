//! Walking physics, the way Minecraft does it: an axis-aligned box swept
//! through the voxel grid one axis at a time, with gravity, jumping and
//! automatic stepping over low ledges.
//!
//! The terrain is queried directly (see [`Solids`]); there are no colliders
//! to build or keep in sync with the meshes.

use std::collections::HashMap;

use bevy::prelude::*;
use mcpe_world::{ChunkPos, World as PeWorld, HEIGHT};

use crate::blocks::{self, Shape};

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

/// Sixteenths of a block that are solid from the bottom of the block up.
type Fill = u8;
const FULL: Fill = 16;

/// What can be stood on or run into: for every block, how much of it (from
/// the floor up) is solid.
///
/// This is a compact copy of the world, one byte per block, so the viewer
/// doesn't have to keep the whole `mcpe_world::World` alive. Columns that
/// aren't loaded count as solid, and so does everything below the world, so
/// the player can't walk off the edge into the void.
/// How much of a block is solid, as `blocks::look` draws it: cubes, slabs
/// and glass are solid, flowers and air are not, and neither is liquid
/// (there is no swimming yet, so you sink through it).
fn fill(id: u8, data: u8) -> Fill {
    match blocks::look(id, data).shape {
        _ if matches!(id, 8..=11) => 0,
        Shape::Air | Shape::Cross => 0,
        Shape::Cube | Shape::Translucent => FULL,
        Shape::Partial(h) => (h * f32::from(FULL)).round() as Fill,
    }
}

#[derive(Resource, Default)]
pub struct Solids {
    columns: HashMap<ChunkPos, Box<[Fill]>>,
}

impl Solids {
    /// Solidity of every block of `world` (see [`fill`]).
    pub fn from_world(world: &PeWorld) -> Self {
        let mut solids = Self::default();
        for (pos, col) in &world.columns {
            let column = solids.column_mut(*pos);
            for x in 0..16 {
                for z in 0..16 {
                    for y in 0..HEIGHT {
                        let (id, data) = col.block(x, y, z);
                        column[Self::index(x, y, z)] = fill(id, data);
                    }
                }
            }
        }
        solids
    }

    fn index(x: usize, y: usize, z: usize) -> usize {
        (x * 16 + z) * HEIGHT + y
    }

    /// Follows a block edit. Positions outside the loaded world are ignored
    /// (they stay solid).
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u8, data: u8) {
        let pos = ChunkPos { x: x.div_euclid(16), z: z.div_euclid(16) };
        if let Some(c) = self.columns.get_mut(&pos).filter(|_| (0..HEIGHT as i32).contains(&y)) {
            c[Self::index(x.rem_euclid(16) as usize, y as usize, z.rem_euclid(16) as usize)] = fill(id, data);
        }
    }

    fn column_mut(&mut self, pos: ChunkPos) -> &mut [Fill] {
        self.columns.entry(pos).or_insert_with(|| vec![0; COLUMN_BLOCKS].into())
    }

    /// Makes block `(x, y, z)` solid up to `height` (0 to 1) of its own,
    /// loading its (otherwise empty) column if needed.
    #[cfg(test)]
    pub fn set(&mut self, x: i32, y: i32, z: i32, height: f32) {
        let pos = ChunkPos { x: x.div_euclid(16), z: z.div_euclid(16) };
        let i = Self::index(x.rem_euclid(16) as usize, y as usize, z.rem_euclid(16) as usize);
        self.column_mut(pos)[i] = (height * f32::from(FULL)).round() as Fill;
    }

    /// Loads an empty column, as the world would for open air.
    #[cfg(test)]
    pub fn load_column(&mut self, x: i32, z: i32) {
        self.column_mut(ChunkPos { x, z });
    }

    /// How high the solid part of block `(x, y, z)` reaches, 0.0 to 1.0.
    fn height(&self, x: i32, y: i32, z: i32) -> f32 {
        if y < 0 {
            return 1.0;
        }
        if y >= HEIGHT as i32 {
            return 0.0;
        }
        let pos = ChunkPos { x: x.div_euclid(16), z: z.div_euclid(16) };
        match self.columns.get(&pos) {
            Some(c) => {
                let i = Self::index(x.rem_euclid(16) as usize, y as usize, z.rem_euclid(16) as usize);
                f32::from(c[i]) / f32::from(FULL)
            }
            None => 1.0,
        }
    }

    /// Whether a player with their feet at `feet` fits there.
    pub fn is_free(&self, feet: Vec3) -> bool {
        self.touching(Aabb::at(feet)).is_none()
    }

    /// Some solid block overlapping `b`, if any.
    fn touching(&self, b: Aabb) -> Option<IVec3> {
        let (lo, hi) = (b.min + EPS, b.max - EPS);
        for x in lo.x.floor() as i32..=hi.x.floor() as i32 {
            for y in lo.y.floor() as i32..=hi.y.floor() as i32 {
                for z in lo.z.floor() as i32..=hi.z.floor() as i32 {
                    if self.height(x, y, z) > 0.0 && y as f32 + self.height(x, y, z) > lo.y {
                        return Some(IVec3::new(x, y, z));
                    }
                }
            }
        }
        None
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
        for x in lo.x.floor() as i32..=hi.x.floor() as i32 {
            for y in lo.y.floor() as i32..=hi.y.floor() as i32 {
                for z in lo.z.floor() as i32..=hi.z.floor() as i32 {
                    let h = self.height(x, y, z);
                    if h == 0.0 {
                        continue;
                    }
                    let cell = Vec3::new(x as f32, y as f32, z as f32);
                    let (bmin, bmax) = (cell, cell + Vec3::new(1.0, h, 1.0));
                    // Skip blocks that don't overlap the box on the other axes
                    // (only slabs can miss on Y, as their cell is tested whole).
                    if (0..3).any(|i| i != a && (bmax[i] <= b.min[i] + EPS || bmin[i] >= b.max[i] - EPS)) {
                        continue;
                    }
                    if d > 0.0 && bmin[a] >= b.max[a] - EPS {
                        d = d.min((bmin[a] - b.max[a]).max(0.0));
                    } else if d < 0.0 && bmax[a] <= b.min[a] + EPS {
                        d = d.max((bmax[a] - b.min[a]).min(0.0));
                    }
                }
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
                let top = (0..HEIGHT as i32).rev().find(|&y| solids.height(x.floor() as i32, y, z.floor() as i32) > 0.0);
                let Some(top) = top else { continue };
                let (mut body, mut feet) = (Body::default(), Vec3::new(x, HEIGHT as f32 - 0.5, z));
                if !solids.is_free(feet) {
                    continue;
                }
                run(&mut body, &mut feet, &solids, Vec2::ZERO, false, 6.0);
                let expect = top as f32 + solids.height(x.floor() as i32, top, z.floor() as i32);
                // The box is 0.6 wide, so a taller neighbour column can hold it up.
                assert!(feet.y >= expect - 1e-3, "fell through to {feet} at ({x}, {z}), terrain top {expect}");
                assert!(solids.is_free(feet), "ended up inside terrain at {feet}");
                checked += 1;
            }
        }
        assert!(checked > 1000, "only {checked} drops");
    }
}
