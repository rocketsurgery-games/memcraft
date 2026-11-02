//! Buttons pop back out after a while.
//!
//! Pressing a button (see [`crate::interact`]) sets its pressed bit; this
//! module remembers the press and clears the bit again once the button's
//! delay has run out: 20 game ticks (one second) for stone, 30 for wood. There
//! is no redstone, so nothing else happens while it is down.
//!
//! [`PendingReleases`] is the pure bookkeeping, driven by elapsed time so it
//! can be tested without an app; [`release_buttons`] feeds it the frame time
//! and puts the buttons that are due back up.

use bevy::prelude::*;

use crate::hotbar::Block;
use crate::world::{self, WorldMap};
use crate::physics::Solids;

/// Bit 3 of a lever's or button's data: on / pressed.
pub const POWERED: u8 = 8;

/// Game ticks per second.
const TICKS_PER_SECOND: f32 = 20.0;

pub struct ButtonsPlugin;

impl Plugin for ButtonsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingReleases>().add_systems(Update, release_buttons);
    }
}

/// How long a button stays pressed, in game ticks, or `None` for blocks that
/// aren't buttons: stone (77) 20, wooden (143) 30.
pub fn press_ticks(id: u8) -> Option<u32> {
    match id {
        77 => Some(20),
        143 => Some(30),
        _ => None,
    }
}

/// The button blocks that are down, with the game tick each one pops up at.
#[derive(Resource, Default, Debug)]
pub struct PendingReleases {
    /// Game ticks elapsed since the app started, with the fraction of a tick
    /// that hasn't been counted yet.
    ticks: u64,
    fraction: f32,
    due: Vec<(IVec3, u64)>,
}

impl PendingReleases {
    /// Notes that the button at `pos` was pressed just now and pops up after
    /// `ticks` game ticks.
    pub fn press(&mut self, pos: IVec3, ticks: u32) {
        let due = self.ticks + ticks as u64;
        // Pressing again can't happen (a pressed button swallows the click),
        // but a stale entry must not release the new press early.
        self.due.retain(|&(p, _)| p != pos);
        self.due.push((pos, due));
    }

    /// Lets `seconds` pass and returns the positions whose time is up, in the
    /// order they were pressed.
    pub fn advance(&mut self, seconds: f32) -> Vec<IVec3> {
        self.fraction += seconds * TICKS_PER_SECOND;
        let whole = self.fraction.floor();
        self.fraction -= whole;
        self.ticks += whole as u64;
        let now = self.ticks;
        let (mut up, down): (Vec<_>, Vec<_>) = std::mem::take(&mut self.due).into_iter().partition(|&(_, due)| due <= now);
        self.due = down;
        up.sort_by_key(|&(_, due)| due);
        up.into_iter().map(|(p, _)| p).collect()
    }
}

/// The block a pressed button turns into when it pops up; `None` when the
/// block isn't a pressed button (it was broken or replaced meanwhile).
pub fn released((id, data): Block) -> Option<Block> {
    (press_ticks(id).is_some() && data & POWERED != 0).then_some((id, data & !POWERED))
}

/// Pops up the buttons whose time has run out.
fn release_buttons(time: Res<Time>, mut pending: ResMut<PendingReleases>, mut map: ResMut<WorldMap>, mut solids: ResMut<Solids>) {
    for pos in pending.advance(time.delta_secs()) {
        if let Some((id, data)) = released(map.block(pos.x, pos.y, pos.z)) {
            world::set_block(&mut map, &mut solids, pos.x, pos.y, pos.z, id, data);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: IVec3 = IVec3::new(3, 64, -5);

    #[test]
    fn stone_and_wooden_buttons_have_their_own_delays() {
        assert_eq!(press_ticks(77), Some(20));
        assert_eq!(press_ticks(143), Some(30));
        assert!((0..=255u8).filter(|&id| press_ticks(id).is_some()).eq([77, 143]));
    }

    #[test]
    fn a_press_pops_up_after_exactly_its_ticks() {
        let mut r = PendingReleases::default();
        r.press(P, 20);
        // 19 ticks (0.95 s) in 60 Hz frames: still down.
        for _ in 0..57 {
            assert!(r.advance(1.0 / 60.0).is_empty());
        }
        // One second in all: up, once.
        let mut up = vec![];
        for _ in 0..3 {
            up.extend(r.advance(1.0 / 60.0));
        }
        assert_eq!(up, [P]);
        assert!(r.advance(10.0).is_empty(), "released only once");
    }

    #[test]
    fn a_long_frame_releases_everything_that_is_due() {
        let mut r = PendingReleases::default();
        r.press(IVec3::X, 30);
        r.advance(0.25);
        r.press(IVec3::Y, 20);
        r.press(IVec3::Z, 20);
        // One long frame: tick 25, when both stone buttons are due but the wooden one isn't.
        assert_eq!(r.advance(1.0), [IVec3::Y, IVec3::Z]);
        assert_eq!(r.advance(0.25), [IVec3::X]);
    }

    #[test]
    fn a_wooden_button_outlasts_a_stone_one_pressed_together() {
        let mut r = PendingReleases::default();
        r.press(IVec3::X, 30);
        r.press(IVec3::Y, 20);
        assert_eq!(r.advance(1.0), [IVec3::Y]);
        assert_eq!(r.advance(0.5), [IVec3::X]);
    }

    #[test]
    fn only_a_pressed_button_pops_up() {
        assert_eq!(released((77, 8 | 3)), Some((77, 3)));
        assert_eq!(released((143, 8)), Some((143, 0)));
        assert_eq!(released((77, 3)), None, "already up");
        assert_eq!(released((0, 0)), None, "broken meanwhile");
        assert_eq!(released((69, 8)), None, "a lever stays on");
    }
}
