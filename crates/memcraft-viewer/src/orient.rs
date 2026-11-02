//! Which blocks a click puts down, and with what data: the rule that turns
//! the catalog's plain `(id, data)` into the orientation Minecraft would give
//! it for the face that was clicked and the way the player looks. See
//! [`orient`]; `targeting` does the clicking and the checks on the cells.
//!
//! Data values are Pocket Edition 0.16's, as `blocks` reads them back.
//!
//! Not oriented (placed as the catalog has them): pillars (hay, quartz,
//! purpur) that `blocks::look` draws upright only, pistons, repeaters, rails,
//! anvils and vines.

use std::f32::consts::TAU;

use bevy::prelude::IVec3;

use crate::blocks::{self, DIRS};
use crate::hotbar::Block;

/// A compass direction as an index into [`blocks::DIRS`]: 0 south (+z), 1
/// west (-x), 2 north (-z), 3 east (+x). Turning clockwise seen from above
/// adds one.
type Compass = usize;

const UP: IVec3 = IVec3::Y;

/// The horizontal direction the camera looks in, given its yaw (about +y, as
/// Bevy has it: zero looks along -z, a positive turn goes towards -x).
fn heading(yaw: f32) -> Compass {
    let (x, z) = (-yaw.sin(), -yaw.cos());
    match (x.abs() > z.abs(), x > 0.0, z > 0.0) {
        (true, true, _) => 3,
        (true, false, _) => 1,
        (false, _, true) => 0,
        (false, _, false) => 2,
    }
}

fn opposite(d: Compass) -> Compass {
    (d + 2) % 4
}

/// The direction of a face normal, if it is horizontal.
fn compass_of(normal: IVec3) -> Option<Compass> {
    DIRS.iter().position(|&(x, z)| normal == IVec3::new(x, 0, z))
}

/// Data values by [`Compass`] of the stairs' rise (east 0, west 1, south 2,
/// north 3), which trapdoors use for the way they face as well.
const BY_COMPASS: [u8; 4] = [2, 1, 3, 0];

/// Data by the [`Compass`] direction a thing faces, for everything that
/// stores its facing as north 2, south 3, west 4, east 5: the front of a
/// furnace, chest or dispenser, a ladder, wall sign or button, which face
/// away from the wall they are on.
const FACES: [u8; 4] = [3, 4, 2, 5];

/// A torch's or lever's data on the wall that a face with this [`Compass`]
/// normal belongs to (east 1, west 2, south 3, north 4).
const ON_WALL: [u8; 4] = [3, 2, 4, 1];

/// What a click puts down, as `(offset, block)` pairs: the offsets are from the
/// cell in front of the face that was clicked, and there are two for a door,
/// a bed and a tall plant. Empty when the block has nowhere to go there (a
/// ladder or torch on a ceiling).
///
/// `normal` is the face's outward normal, `height` how far up that block the
/// click was (0..1), and `yaw` the camera's (see [`heading`]).
pub fn orient((id, data): Block, normal: IVec3, height: f32, yaw: f32) -> Vec<(IVec3, Block)> {
    let facing = heading(yaw);
    let one = |data| vec![(IVec3::ZERO, (id, data))];
    // Slabs, stairs and trapdoors go in the upper half when clicked on an
    // underside, or on the upper half of a side.
    let upper = normal.y < 0 || (normal.y == 0 && height >= 0.5);
    // The wall a click on a side face is on, for what hangs on walls.
    let wall = compass_of(normal);
    match id {
        // Logs lie along the axis of the face that was clicked.
        17 | 162 => one((data & 3) | if normal.x != 0 { 4 } else if normal.z != 0 { 8 } else { 0 }),
        44 | 158 | 182 | 205 => one((data & 7) | if upper { 8 } else { 0 }),
        _ if blocks::is_stairs(id) => one(BY_COMPASS[facing] | if upper { 4 } else { 0 }),
        // A trapdoor hangs on the wall it was put against; on a floor or
        // ceiling its hinge is on the side away from the player.
        96 | 167 => {
            let faces = wall.unwrap_or(opposite(facing));
            one(BY_COMPASS[faces] | if upper { 4 } else { 0 })
        }
        // Fence gates face the player, so they span the way the player's
        // sides point.
        _ if blocks::is_gate(id) => one(facing as u8),
        // The front of these faces the player.
        61 | 23 | 54 | 146 => one(FACES[opposite(facing)]),
        86 | 91 => one(opposite(facing) as u8),
        // Doors: the lower half faces the way the player looks, the upper
        // half has the left-hand hinge.
        _ if blocks::is_door(id) => {
            let lower = (facing as u8 + 1) % 4;
            vec![(IVec3::ZERO, (id, lower)), (UP, (id, 8))]
        }
        // A bed's foot is where the click puts it, its head ahead of it.
        26 => {
            let (dx, dz) = DIRS[facing];
            vec![(IVec3::ZERO, (id, facing as u8)), (IVec3::new(dx, 0, dz), (id, facing as u8 | 8))]
        }
        175 => vec![(IVec3::ZERO, (id, data & 7)), (UP, (id, (data & 7) | 8))],
        // Torches lean away from the wall, or stand on a floor.
        50 | 75 | 76 => match (normal.y, wall) {
            (1, _) => one(5),
            (_, Some(d)) => one(ON_WALL[d]),
            _ => vec![],
        },
        // A standing sign turns to face the player in sixteenths of a turn
        // (0 faces south); on a wall it is a wall sign.
        63 | 68 => match (normal.y, wall) {
            (1, _) => vec![(IVec3::ZERO, (63, (-yaw / TAU * 16.0).round().rem_euclid(16.0) as u8))],
            (_, Some(d)) => vec![(IVec3::ZERO, (68, FACES[d]))],
            _ => vec![],
        },
        65 => wall.map_or(vec![], |d| one(FACES[d])),
        // Buttons: ceiling 0, floor 1, wall 2..5.
        77 | 143 => match (normal.y, wall) {
            (-1, _) => one(0),
            (1, _) => one(1),
            _ => wall.map_or(vec![], |d| one(FACES[d])),
        },
        // Levers: ceiling 0 or 7 and floor 6 or 5 (the handle along the
        // player's axis), walls 1..4 by east, west, south, north.
        69 => {
            let along_x = facing % 2 == 1;
            match (normal.y, wall) {
                (-1, _) => one(if along_x { 0 } else { 7 }),
                (1, _) => one(if along_x { 6 } else { 5 }),
                (_, Some(d)) => one(ON_WALL[d]),
                _ => vec![],
            }
        }
        _ => one(data),
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::{FRAC_PI_2, PI};

    use bevy::prelude::*;

    use super::*;

    /// Camera yaws looking south, west, north and east.
    const LOOKING: [f32; 4] = [PI, FRAC_PI_2, 0.0, -FRAC_PI_2];
    const SOUTH: usize = 0;
    const WEST: usize = 1;
    const NORTH: usize = 2;
    const EAST: usize = 3;

    const TOP: IVec3 = IVec3::Y;
    const BOTTOM: IVec3 = IVec3::NEG_Y;
    /// The faces whose outward normal points south, west, north, east.
    const SIDE: [IVec3; 4] = [IVec3::Z, IVec3::NEG_X, IVec3::NEG_Z, IVec3::X];

    /// The one block a click puts down, as its data (or nothing).
    fn data(block: Block, normal: IVec3, height: f32, looking: usize) -> Option<(u8, u8)> {
        let placed = orient(block, normal, height, LOOKING[looking]);
        assert!(placed.len() <= 1, "{block:?} makes one block");
        placed.first().map(|&(offset, b)| {
            assert_eq!(offset, IVec3::ZERO);
            b
        })
    }

    fn data_of(block: Block, normal: IVec3, looking: usize) -> Option<u8> {
        data(block, normal, 0.5, looking).map(|b| b.1)
    }

    #[test]
    fn the_camera_yaw_gives_the_direction_it_looks_in() {
        for (looking, yaw) in LOOKING.into_iter().enumerate() {
            let forward = Quat::from_rotation_y(yaw) * Vec3::NEG_Z;
            assert_eq!(heading(yaw), looking, "forward {forward}");
            assert_eq!(forward.round().as_ivec3(), SIDE[looking], "looking {looking}");
            // A bit to either side still reads the same way.
            assert_eq!(heading(yaw + 0.3), looking);
            assert_eq!(heading(yaw - 0.3), looking);
        }
        // Pitch doesn't matter: only the yaw is read.
        let tilted = Quat::from_euler(EulerRot::YXZ, FRAC_PI_2, 1.0, 0.0) * Vec3::NEG_Z;
        assert!(tilted.x < 0.0, "a positive yaw turns towards -x");
    }

    #[test]
    fn logs_lie_along_the_axis_of_the_face_clicked() {
        assert_eq!(data_of((17, 1), TOP, NORTH), Some(1));
        assert_eq!(data_of((17, 1), BOTTOM, NORTH), Some(1));
        assert_eq!(data_of((17, 1), SIDE[EAST], NORTH), Some(1 | 4), "an east face: along x");
        assert_eq!(data_of((17, 1), SIDE[WEST], SOUTH), Some(1 | 4));
        assert_eq!(data_of((17, 2), SIDE[NORTH], EAST), Some(2 | 8), "a north face: along z");
        assert_eq!(data_of((162, 1), SIDE[SOUTH], EAST), Some(1 | 8));
        // The player's look does not matter.
        assert_eq!(data_of((17, 0), SIDE[EAST], EAST), data_of((17, 0), SIDE[EAST], SOUTH));
    }

    #[test]
    fn stairs_rise_the_way_the_player_looks() {
        // Rising east 0, west 1, south 2, north 3.
        for (looking, want) in [(EAST, 0), (WEST, 1), (SOUTH, 2), (NORTH, 3)] {
            for id in [53, 67, 134, 203] {
                assert_eq!(data_of((id, 0), TOP, looking), Some(want), "{id} looking {looking}");
            }
        }
    }

    #[test]
    fn stairs_go_upside_down_against_an_underside_or_an_upper_half() {
        assert_eq!(data((53, 0), BOTTOM, 0.0, EAST), Some((53, 4)));
        assert_eq!(data((53, 0), SIDE[NORTH], 0.25, EAST), Some((53, 0)));
        assert_eq!(data((53, 0), SIDE[NORTH], 0.75, EAST), Some((53, 4)));
        assert_eq!(data((53, 0), SIDE[NORTH], 0.5, NORTH), Some((53, 3 | 4)));
        assert_eq!(data((53, 0), TOP, 1.0, NORTH), Some((53, 3)), "the top of a block is not an upper half");
    }

    #[test]
    fn slabs_take_the_top_or_bottom_half_and_keep_their_material() {
        for id in [44, 158, 182, 205] {
            assert_eq!(data((id, 3), TOP, 1.0, NORTH), Some((id, 3)), "{id}");
            assert_eq!(data((id, 3), BOTTOM, 0.0, NORTH), Some((id, 3 | 8)));
            assert_eq!(data((id, 3), SIDE[EAST], 0.2, NORTH), Some((id, 3)));
            assert_eq!(data((id, 3), SIDE[EAST], 0.8, NORTH), Some((id, 3 | 8)));
        }
        // Not a slab: the upper-half rule doesn't touch full blocks.
        assert_eq!(data((1, 0), BOTTOM, 0.0, NORTH), Some((1, 0)));
    }

    #[test]
    fn trapdoors_hang_on_the_wall_they_are_put_against() {
        // East, west, south, north is 0..3, by the way they face.
        assert_eq!(data((96, 0), SIDE[EAST], 0.2, NORTH), Some((96, 0)));
        assert_eq!(data((96, 0), SIDE[WEST], 0.2, NORTH), Some((96, 1)));
        assert_eq!(data((96, 0), SIDE[SOUTH], 0.2, NORTH), Some((96, 2)));
        assert_eq!(data((167, 0), SIDE[NORTH], 0.8, NORTH), Some((167, 3 | 4)), "upper half");
        // On the floor it faces the player's back; on a ceiling it is upside down.
        assert_eq!(data((96, 0), TOP, 1.0, NORTH), Some((96, 2)));
        assert_eq!(data((96, 0), BOTTOM, 0.0, EAST), Some((96, 1 | 4)));
    }

    #[test]
    fn torches_lean_away_from_the_wall_and_stand_on_the_floor() {
        // East, west, south, north is 1..4, standing 5.
        for id in [50, 75, 76] {
            assert_eq!(data_of((id, 5), TOP, NORTH), Some(5));
            assert_eq!(data_of((id, 5), SIDE[EAST], NORTH), Some(1));
            assert_eq!(data_of((id, 5), SIDE[WEST], NORTH), Some(2));
            assert_eq!(data_of((id, 5), SIDE[SOUTH], NORTH), Some(3));
            assert_eq!(data_of((id, 5), SIDE[NORTH], NORTH), Some(4));
            assert_eq!(data_of((id, 5), BOTTOM, NORTH), None, "no torch hangs from a ceiling");
        }
    }

    #[test]
    fn furnaces_chests_and_dispensers_face_the_player() {
        // North 2, south 3, west 4, east 5: looking north, the front faces south.
        for id in [61, 23, 54, 146] {
            for (looking, want) in [(NORTH, 3), (SOUTH, 2), (EAST, 4), (WEST, 5)] {
                assert_eq!(data_of((id, 3), TOP, looking), Some(want), "{id} looking {looking}");
                assert_eq!(data_of((id, 3), SIDE[EAST], looking), Some(want), "the face clicked does not matter");
            }
        }
    }

    #[test]
    fn pumpkins_face_the_player() {
        // The front faces south 0, west 1, north 2, east 3.
        for id in [86, 91] {
            for (looking, want) in [(NORTH, 0), (EAST, 1), (SOUTH, 2), (WEST, 3)] {
                assert_eq!(data_of((id, 0), TOP, looking), Some(want), "{id} looking {looking}");
            }
        }
    }

    #[test]
    fn fence_gates_face_the_player() {
        // South 0, west 1, north 2, east 3.
        for (looking, want) in [(SOUTH, 0), (WEST, 1), (NORTH, 2), (EAST, 3)] {
            assert_eq!(data_of((107, 0), TOP, looking), Some(want));
            assert_eq!(data_of((187, 0), TOP, looking), Some(want));
        }
    }

    #[test]
    fn doors_are_two_blocks_facing_the_player() {
        // The lower half faces east 0, south 1, west 2, north 3 as the player looks.
        for (looking, want) in [(EAST, 0), (SOUTH, 1), (WEST, 2), (NORTH, 3)] {
            for id in [64, 71, 193, 197] {
                let placed = orient((id, 0), TOP, 0.0, LOOKING[looking]);
                assert_eq!(placed, vec![(IVec3::ZERO, (id, want)), (IVec3::Y, (id, 8))], "{id} looking {looking}");
            }
        }
    }

    #[test]
    fn beds_put_the_head_ahead_of_the_foot() {
        // The foot faces south 0, west 1, north 2, east 3; the head has bit 8.
        for (looking, (dx, dz)) in [(SOUTH, (0, 1)), (WEST, (-1, 0)), (NORTH, (0, -1)), (EAST, (1, 0))] {
            let placed = orient((26, 0), TOP, 0.0, LOOKING[looking]);
            let d = looking as u8;
            assert_eq!(placed, vec![(IVec3::ZERO, (26, d)), (IVec3::new(dx, 0, dz), (26, d | 8))]);
        }
    }

    #[test]
    fn tall_plants_get_their_upper_half() {
        let placed = orient((175, 4), TOP, 0.0, 0.0);
        assert_eq!(placed, vec![(IVec3::ZERO, (175, 4)), (IVec3::Y, (175, 4 | 8))]);
    }

    #[test]
    fn ladders_and_wall_signs_face_away_from_their_wall() {
        // North 2, south 3, west 4, east 5.
        for (face, want) in [(NORTH, 2), (SOUTH, 3), (WEST, 4), (EAST, 5)] {
            assert_eq!(data_of((65, 2), SIDE[face], SOUTH), Some(want), "ladder");
            assert_eq!(data((63, 0), SIDE[face], 0.5, SOUTH), Some((68, want)), "sign on a wall");
            assert_eq!(data((68, 0), SIDE[face], 0.5, SOUTH), Some((68, want)));
        }
        assert_eq!(data_of((65, 2), TOP, NORTH), None);
        assert_eq!(data_of((65, 2), BOTTOM, NORTH), None);
        assert_eq!(data((63, 0), BOTTOM, 0.0, NORTH), None);
    }

    #[test]
    fn standing_signs_turn_to_face_the_player_in_sixteenths() {
        // 0 faces south, 4 west, 8 north, 12 east; the player looks the other way.
        for (looking, want) in [(NORTH, 0), (EAST, 4), (SOUTH, 8), (WEST, 12)] {
            assert_eq!(data((63, 0), TOP, 0.0, looking), Some((63, want)), "looking {looking}");
        }
        // Halfway between looking north and looking west is a sign facing south-east.
        assert_eq!(orient((63, 0), TOP, 0.0, FRAC_PI_2 / 2.0)[0].1, (63, 14));
        let all: Vec<u8> = (0..64).map(|i| orient((63, 0), TOP, 0.0, i as f32 * 0.37 - 10.0)[0].1 .1).collect();
        assert!(all.iter().all(|&d| d < 16));
    }

    #[test]
    fn buttons_and_levers_stick_to_the_face_clicked() {
        for id in [77, 143] {
            assert_eq!(data_of((id, 0), BOTTOM, NORTH), Some(0));
            assert_eq!(data_of((id, 0), TOP, NORTH), Some(1));
            assert_eq!(data_of((id, 0), SIDE[NORTH], NORTH), Some(2));
            assert_eq!(data_of((id, 0), SIDE[EAST], NORTH), Some(5));
        }
        // Levers on walls: east 1, west 2, south 3, north 4.
        assert_eq!(data_of((69, 0), SIDE[EAST], NORTH), Some(1));
        assert_eq!(data_of((69, 0), SIDE[NORTH], NORTH), Some(4));
        // On a floor or ceiling the handle lies along the way the player looks.
        assert_eq!(data_of((69, 0), TOP, NORTH), Some(5));
        assert_eq!(data_of((69, 0), TOP, EAST), Some(6));
        assert_eq!(data_of((69, 0), BOTTOM, SOUTH), Some(7));
        assert_eq!(data_of((69, 0), BOTTOM, WEST), Some(0));
    }

    #[test]
    fn other_blocks_keep_the_data_they_have() {
        for block in [(1, 0), (5, 3), (35, 14), (20, 0), (85, 2), (45, 0)] {
            for normal in [TOP, BOTTOM, SIDE[WEST]] {
                assert_eq!(orient(block, normal, 0.7, 1.0), vec![(IVec3::ZERO, block)]);
            }
        }
    }

    #[test]
    fn every_orientation_draws_as_the_block_it_is() {
        // What is placed is something `blocks` knows how to draw, and
        // a block with a shape stays one.
        for e in crate::catalog::all() {
            for looking in [NORTH, EAST] {
                for normal in [TOP, BOTTOM, SIDE[WEST], SIDE[SOUTH]] {
                    for (_, (id, d)) in orient((e.id, e.data), normal, 0.75, LOOKING[looking]) {
                        assert!(d < 16, "{} {id}:{d}", e.name);
                        assert_eq!(
                            blocks::look(id, d).shape == blocks::Shape::Air,
                            blocks::look(e.id, e.data).shape == blocks::Shape::Air,
                            "{} {id}:{d}",
                            e.name
                        );
                    }
                }
            }
        }
    }
}
