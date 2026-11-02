//! What a right click does to a block other than placing against it.
//!
//! [`interact`] is the one place that knows which blocks react to a click
//! and what they change; `targeting` asks it first and only places a block
//! when it answers `None`. Doors, trapdoors and fence gates toggle their
//! open bit (iron ones too: there is no redstone to open them otherwise).
//! Levers, buttons and signs belong here too.

use bevy::prelude::*;

use crate::buttons::{press_ticks, POWERED};
use crate::hotbar::Block;

/// Doors: wooden (64, 193..=197) and iron (71).
fn is_door(id: u8) -> bool {
    matches!(id, 64 | 71 | 193..=197)
}

/// Trapdoors: wooden (96) and iron (167).
fn is_trapdoor(id: u8) -> bool {
    matches!(id, 96 | 167)
}

/// Fence gates: oak (107) and the other woods (183..=187).
fn is_gate(id: u8) -> bool {
    matches!(id, 107 | 183..=187)
}

/// Levers (69).
fn is_lever(id: u8) -> bool {
    id == 69
}

/// Bit 3 of a door's data marks its upper half.
const DOOR_UPPER: u8 = 8;

/// The edits a right click on `block` at `pos` makes, as `(position, new
/// block)` pairs, or `None` when the block does nothing and the click places
/// a block instead. An upper door half with no lower half under it still
/// answers, with no edits at all. `at` reads the world.
///
/// A door's open bit (4) lives in its lower half only (the upper half holds
/// the hinge, as `blocks` draws them), so a click on either half flips the
/// lower one. Trapdoors flip bit 8 and gates bit 4 of the block itself.
/// Levers flip their powered bit; a button sets it, and ignores the click
/// while it is down (`buttons` pops it back up).
pub fn interact(block: Block, pos: IVec3, at: impl Fn(IVec3) -> Block) -> Option<Vec<(IVec3, Block)>> {
    let (id, data) = block;
    if is_trapdoor(id) {
        return Some(vec![(pos, (id, data ^ 8))]);
    }
    if is_gate(id) {
        return Some(vec![(pos, (id, data ^ 4))]);
    }
    if is_lever(id) {
        return Some(vec![(pos, (id, data ^ POWERED))]);
    }
    if press_ticks(id).is_some() {
        return Some(if data & POWERED == 0 { vec![(pos, (id, data | POWERED))] } else { vec![] });
    }
    if !is_door(id) {
        return None;
    }
    let lower = if data & DOOR_UPPER != 0 { pos - IVec3::Y } else { pos };
    let (lower_id, lower_data) = at(lower);
    Some(if is_door(lower_id) && lower_data & DOOR_UPPER == 0 { vec![(lower, (lower_id, lower_data ^ 4))] } else { vec![] })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::blocks;
    use crate::physics::Solids;
    use crate::world::{self, WorldMap};

    /// Applies the edits of a click on `pos` to a small world, and says
    /// whether the block interacted.
    fn click(world: &mut HashMap<IVec3, Block>, pos: IVec3) -> bool {
        let at = |p: IVec3| world.get(&p).copied().unwrap_or((0, 0));
        let Some(edits) = interact(at(pos), pos, at) else {
            return false;
        };
        world.extend(edits);
        true
    }

    fn door_world(id: u8, lower: u8, upper: u8) -> HashMap<IVec3, Block> {
        HashMap::from([(IVec3::new(0, 64, 0), (id, lower)), (IVec3::new(0, 65, 0), (id, DOOR_UPPER | upper))])
    }

    #[test]
    fn clicking_either_half_of_a_door_flips_the_open_bit_of_the_lower_half() {
        for id in [64, 71, 193, 197] {
            for click_at in [64, 65] {
                let mut w = door_world(id, 2, 1);
                assert!(click(&mut w, IVec3::new(0, click_at, 0)));
                assert_eq!(w[&IVec3::new(0, 64, 0)], (id, 6), "door {id}, opened from y = {click_at}");
                assert_eq!(w[&IVec3::new(0, 65, 0)], (id, 9), "the hinge is untouched");
                assert!(click(&mut w, IVec3::new(0, click_at, 0)));
                assert_eq!(w, door_world(id, 2, 1), "closed again");
            }
        }
    }

    #[test]
    fn a_door_missing_its_lower_half_swallows_the_click_without_changing_anything() {
        let mut w = HashMap::from([(IVec3::new(0, 65, 0), (64, DOOR_UPPER))]);
        assert!(click(&mut w, IVec3::new(0, 65, 0)));
        assert_eq!(w[&IVec3::new(0, 65, 0)], (64, DOOR_UPPER));
        // A different block under an upper half isn't its lower half.
        let mut w = HashMap::from([(IVec3::new(0, 64, 0), (1, 0)), (IVec3::new(0, 65, 0), (64, DOOR_UPPER))]);
        assert!(click(&mut w, IVec3::new(0, 65, 0)));
        assert_eq!(w[&IVec3::new(0, 64, 0)], (1, 0));
        // A lower half with nothing above it still opens.
        let mut w = HashMap::from([(IVec3::new(0, 64, 0), (64, 2))]);
        assert!(click(&mut w, IVec3::new(0, 64, 0)));
        assert_eq!(w[&IVec3::new(0, 64, 0)], (64, 6));
    }

    #[test]
    fn trapdoors_and_gates_flip_their_own_open_bit() {
        for (id, data, open) in [(96, 2, 10), (96, 7, 15), (167, 1, 9), (107, 3, 7), (107, 6, 2), (183, 0, 4), (187, 1, 5)] {
            let p = IVec3::new(5, 70, 5);
            let mut w = HashMap::from([(p, (id, data))]);
            assert!(click(&mut w, p));
            assert_eq!(w[&p], (id, open), "{id}:{data}");
            assert!(click(&mut w, p));
            assert_eq!(w[&p], (id, data), "{id}:{data} closed again");
        }
    }

    #[test]
    fn a_lever_flips_its_powered_bit_whatever_it_is_mounted_on() {
        for data in 0..8 {
            let p = IVec3::new(2, 70, 2);
            let mut w = HashMap::from([(p, (69, data))]);
            assert!(click(&mut w, p));
            assert_eq!(w[&p], (69, data | 8), "lever {data} switched on");
            assert!(click(&mut w, p));
            assert_eq!(w[&p], (69, data), "lever {data} switched off");
        }
        let p = IVec3::ZERO;
        let mut w = HashMap::from([(p, (69, 5 | 8)), (p + IVec3::X, (69, 1))]);
        click(&mut w, p);
        assert_eq!(w[&(p + IVec3::X)], (69, 1), "a neighbouring lever is untouched");
    }

    #[test]
    fn a_button_is_pressed_by_a_click_and_ignores_clicks_while_down() {
        for (id, data) in [(77, 1), (77, 4), (143, 0), (143, 5)] {
            let p = IVec3::new(2, 70, 2);
            let mut w = HashMap::from([(p, (id, data))]);
            assert!(click(&mut w, p));
            assert_eq!(w[&p], (id, data | 8), "{id}:{data} pressed");
            // Down: the click is swallowed (it neither places nor extends the press).
            assert!(click(&mut w, p));
            assert_eq!(w[&p], (id, data | 8), "{id}:{data} still pressed");
        }
    }

    #[test]
    fn a_pressed_button_pops_back_up_after_its_delay() {
        use crate::buttons::{released, PendingReleases};
        for (id, ticks, seconds) in [(77u8, 20, 1.0f32), (143, 30, 1.5)] {
            let p = IVec3::new(2, 70, 2);
            let mut w = HashMap::from([(p, (id, 3))]);
            let mut pending = PendingReleases::default();
            // What `targeting` does around a click.
            click(&mut w, p);
            pending.press(p, press_ticks(id).unwrap());
            assert_eq!(press_ticks(id), Some(ticks));
            assert!(pending.advance(seconds - 0.1).is_empty(), "{id} is still down at {}s", seconds - 0.1);
            assert_eq!(w[&p], (id, 3 | 8));
            for q in pending.advance(0.1) {
                w.insert(q, released(w[&q]).unwrap());
            }
            assert_eq!(w[&p], (id, 3), "{id} is back up after {seconds}s");
        }
    }

    #[test]
    fn other_blocks_do_not_react_so_the_click_places() {
        for id in [0, 1, 17, 20, 50, 53, 65, 85, 101, 113, 139, 188] {
            let p = IVec3::ZERO;
            let mut w = HashMap::from([(p, (id, 0))]);
            assert!(!click(&mut w, p), "{id}");
            assert_eq!(w[&p], (id, 0));
        }
    }

    #[test]
    fn every_door_trapdoor_and_gate_the_world_models_reacts() {
        // The ids `blocks` shapes as doors, trapdoors and gates (see `blocks::model`).
        for id in 0..=255u8 {
            let near = |_, _, _| (0, 0);
            let shaped = |data| blocks::look(id, data).shape == blocks::Shape::Boxes && !blocks::model(id, data, &near).visual.is_empty();
            let reacts = interact((id, 0), IVec3::ZERO, |_| (id, 0)).is_some();
            let is_openable = matches!(id, 64 | 71 | 193..=197 | 96 | 167 | 107 | 183..=187);
            // Levers and buttons react too, but are not doors (see their own test).
            assert_eq!(reacts, is_openable || matches!(id, 69 | 77 | 143), "{id}");
            if is_openable {
                assert!(shaped(0), "{id} is drawn from boxes");
            }
        }
    }

    /// Clicks a door of the real world and checks what changes on screen and in
    /// the way: the mesh, and whether the player fits in the doorway.
    #[test]
    fn opening_a_door_of_the_world_remeshes_it_and_lets_the_player_through() {
        let mut map = WorldMap::new(crate::load_world(None).unwrap());
        let mut solids = Solids::from_world(map.world());
        let columns: Vec<_> = map.world().columns.values().collect();
        let door = columns
            .iter()
            .flat_map(|c| (0..16).flat_map(move |x| (0..16).flat_map(move |z| (1..120).map(move |y| (c, x, y, z)))))
            .map(|(c, x, y, z)| (IVec3::new(c.pos.x * 16 + x as i32, y as i32, c.pos.z * 16 + z as i32), c.block(x, y, z), c.block(x, y + 1, z)))
            .find(|&(_, (id, d), up)| id == 64 && d & 12 == 0 && up.0 == 64 && up.1 & 8 != 0)
            .expect("the world has a closed oak door")
            .0;
        // Clear the surroundings so only the door is in the way.
        for (dx, dy, dz) in (-2..=2).flat_map(|x| (-1..=3).flat_map(move |y| (-2..=2).map(move |z| (x, y, z)))) {
            let p = door + IVec3::new(dx, dy, dz);
            if (dx, dz) != (0, 0) || !(0..=1).contains(&dy) {
                world::set_block(&mut map, &mut solids, p.x, p.y, p.z, 0, 0);
            }
        }
        let at_of = |map: &WorldMap| {
            let m = map.block(door.x, door.y, door.z);
            (m, map.block(door.x, door.y + 1, door.z))
        };
        let (closed_lower, closed_upper) = at_of(&map);
        // The middle of the closed plate (see `blocks::door`) is where the player would stand.
        let near = |dx, dy, dz| map.block(door.x + dx, door.y + dy, door.z + dz);
        let plate = blocks::model(closed_lower.0, closed_lower.1, &near).collision.into_iter().next().unwrap();
        let feet = door.as_vec3() + Vec3::new((plate[0] + plate[3]) as f32 / 32.0, 0.0, (plate[2] + plate[5]) as f32 / 32.0);
        assert!(!solids.is_free(feet), "the closed door blocks the way");
        let before = map.mesh_column(bevy_pos(door));

        let at = |p: IVec3| map.block(p.x, p.y, p.z);
        let edits = interact(closed_upper, door + IVec3::Y, at).expect("a door interacts");
        for (p, (id, data)) in edits {
            assert!(world::set_block(&mut map, &mut solids, p.x, p.y, p.z, id, data));
        }
        assert_eq!(map.block(door.x, door.y, door.z).1, closed_lower.1 | 4);
        assert!(solids.is_free(feet), "the open door lets the player through");
        let open = map.mesh_column(bevy_pos(door));
        let (b, o) = (&before.opaque, &open.opaque);
        assert_ne!(b.positions, o.positions, "the door is drawn differently");
        assert_eq!(b.positions.len(), o.positions.len(), "still a plate on each half");
    }

    fn bevy_pos(p: IVec3) -> mcpe_world::ChunkPos {
        mcpe_world::ChunkPos { x: p.x.div_euclid(16), z: p.z.div_euclid(16) }
    }
}
