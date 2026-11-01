//! Camera controls: click to lock the pointer and look around, Esc to
//! release it; WASD/QE to fly, Shift to go fast, scroll to change speed.
//! F switches between flying and walking (gravity, solid blocks, Space to
//! jump; see `physics`).
//!
//! This replaces Bevy's `FreeCamera`, which only looks around while the
//! right button is held and keeps its grab state private.

use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

use mcpe_world::HEIGHT;

use crate::physics::{Body, Solids, EYE_HEIGHT};

/// The mouse buttons, all in one place. Middle is kept for picking a block.
pub const LOCK_BUTTON: MouseButton = MouseButton::Left;
pub const BREAK_BUTTON: MouseButton = MouseButton::Left;
pub const PLACE_BUTTON: MouseButton = MouseButton::Right;

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Solids>()
            .add_systems(Update, (grab_pointer, look, toggle_mode, scroll_speed, fly, walk).chain());
    }
}

/// How the camera moves.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Mode {
    /// Through everything, in any direction.
    #[default]
    Fly,
    /// On the ground: gravity, solid blocks, jumping.
    Walk,
}

/// The camera the player steers.
#[derive(Component)]
pub struct Controller {
    /// Radians of turn per pixel of mouse motion.
    pub sensitivity: f32,
    /// Blocks per second, before the scroll multiplier.
    pub speed: f32,
    pub fast_speed: f32,
    /// Blocks per second on foot, and with Shift held (Minecraft's walk and sprint).
    pub walk_speed: f32,
    pub sprint_speed: f32,
    pub mode: Mode,
    body: Body,
    /// Each scroll line multiplies the speed by `e^scroll_factor`.
    pub scroll_factor: f32,
    speed_multiplier: f32,
    yaw: f32,
    pitch: f32,
}

impl Controller {
    /// A controller looking the way `transform` already faces.
    pub fn new(transform: &Transform, speed: f32, fast_speed: f32) -> Self {
        let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        Self { sensitivity: 0.2 / 180.0, speed, fast_speed, scroll_factor: 0.05, walk_speed: 4.3,
            sprint_speed: 5.6, mode: Mode::Fly, body: Body::default(), speed_multiplier: 1.0, yaw, pitch }
    }

    /// Turns by a mouse delta in pixels; pitch stops at straight up and down.
    fn turn(&mut self, delta: Vec2) {
        self.yaw -= delta.x * self.sensitivity;
        self.pitch = (self.pitch - delta.y * self.sensitivity).clamp(-FRAC_PI_2, FRAC_PI_2);
    }

    fn rotation(&self) -> Quat {
        Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, 0.0)
    }
}

pub fn is_locked(cursor: &CursorOptions) -> bool {
    cursor.grab_mode != CursorGrabMode::None
}

fn set_locked(cursor: &mut CursorOptions, locked: bool) {
    cursor.grab_mode = if locked { CursorGrabMode::Locked } else { CursorGrabMode::None };
    cursor.visible = !locked;
}

/// A click of [`LOCK_BUTTON`] locks the pointer, and it stays locked until
/// Esc or until the window loses focus. In the browser, Esc is handled by the
/// browser itself (the page never sees it), so the lock is re-read from the
/// page. The click that locks must not also act on a block: `targeting`
/// handles clicks before this system, while the pointer is still free.
pub fn grab_pointer(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    #[cfg(target_arch = "wasm32")] mut browser_had_lock: Local<bool>,
) {
    let (window, cursor) = &mut *window;
    let locked = is_locked(cursor);
    if !locked && window.focused && mouse.just_pressed(LOCK_BUTTON) {
        set_locked(cursor, true);
    } else if locked && (keys.just_pressed(KeyCode::Escape) || !window.focused) {
        set_locked(cursor, false);
    }

    // The browser grants the lock a moment after it is requested, so only a
    // lock it granted and then dropped counts as a release.
    #[cfg(target_arch = "wasm32")]
    {
        let has_lock = web_sys::window()
            .and_then(|w| w.document())
            .is_some_and(|d| d.pointer_lock_element().is_some());
        if *browser_had_lock && !has_lock && is_locked(cursor) {
            set_locked(cursor, false);
        }
        *browser_had_lock = has_lock;
    }
}

fn look(
    motion: Res<AccumulatedMouseMotion>,
    cursor: Single<&CursorOptions, With<PrimaryWindow>>,
    mut camera: Single<(&mut Transform, &mut Controller)>,
) {
    if !is_locked(&cursor) || motion.delta == Vec2::ZERO {
        return;
    }
    let (transform, controller) = &mut *camera;
    controller.turn(motion.delta);
    transform.rotation = controller.rotation();
}

/// F switches between flying and walking. Starting to walk inside terrain
/// (flying through a hill) lifts the player to the first free spot above;
/// if there is none, the player keeps flying.
fn toggle_mode(keys: Res<ButtonInput<KeyCode>>, solids: Res<Solids>, mut camera: Single<(&mut Transform, &mut Controller)>) {
    if !keys.just_pressed(KeyCode::KeyF) {
        return;
    }
    let (transform, controller) = &mut *camera;
    controller.body = Body::default();
    controller.mode = match controller.mode {
        Mode::Walk => Mode::Fly,
        Mode::Fly => {
            let feet = transform.translation - Vec3::Y * EYE_HEIGHT;
            match (0..4 * HEIGHT).map(|i| feet + Vec3::Y * (i as f32 * 0.25)).find(|&p| solids.is_free(p)) {
                Some(free) => {
                    transform.translation = free + Vec3::Y * EYE_HEIGHT;
                    Mode::Walk
                }
                None => Mode::Fly,
            }
        }
    };
    info!("{:?} mode at {:.1}", controller.mode, transform.translation);
}

/// The scroll wheel scales the speed, in either mode.
fn scroll_speed(scroll: Res<AccumulatedMouseScroll>, mut controller: Single<&mut Controller>) {
    let lines = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR,
    };
    controller.speed_multiplier = (controller.speed_multiplier * (controller.scroll_factor * lines).exp()).clamp(0.05, 50.0);
}

fn axis(keys: &ButtonInput<KeyCode>, pos: KeyCode, neg: KeyCode) -> f32 {
    keys.pressed(pos) as i32 as f32 - keys.pressed(neg) as i32 as f32
}

fn shift(keys: &ButtonInput<KeyCode>) -> bool {
    keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])
}

/// Free flight: W/S along the view direction (pitch included), A/D sideways,
/// Q/E straight down and up.
fn fly(time: Res<Time<Real>>, keys: Res<ButtonInput<KeyCode>>, mut camera: Single<(&mut Transform, &Controller)>) {
    let (transform, controller) = &mut *camera;
    if controller.mode != Mode::Fly {
        return;
    }
    let (forward, right, up) =
        (axis(&keys, KeyCode::KeyW, KeyCode::KeyS), axis(&keys, KeyCode::KeyD, KeyCode::KeyA), axis(&keys, KeyCode::KeyE, KeyCode::KeyQ));
    let dir = forward * *transform.forward() + right * *transform.right() + up * Vec3::Y;
    if dir == Vec3::ZERO {
        return;
    }
    let speed = if shift(&keys) { controller.fast_speed } else { controller.speed } * controller.speed_multiplier;
    transform.translation += dir.normalize() * speed * time.delta_secs();
}

/// Walking: W/S/A/D along the ground in the direction the player faces
/// (pitch ignored), Space to jump, Shift to sprint. The camera sits at the eyes.
pub fn walk(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    solids: Res<Solids>,
    mut camera: Single<(&mut Transform, &mut Controller)>,
) {
    let (transform, controller) = &mut *camera;
    if controller.mode != Mode::Walk {
        return;
    }
    let (forward, right) = (axis(&keys, KeyCode::KeyW, KeyCode::KeyS), axis(&keys, KeyCode::KeyD, KeyCode::KeyA));
    let (sin, cos) = controller.yaw.sin_cos();
    let wish = (forward * Vec2::new(-sin, -cos) + right * Vec2::new(cos, -sin)).normalize_or_zero();
    let speed = if shift(&keys) { controller.sprint_speed } else { controller.walk_speed } * controller.speed_multiplier;

    let mut feet = transform.translation - Vec3::Y * EYE_HEIGHT;
    controller.body.step(&mut feet, &solids, wish * speed, keys.pressed(KeyCode::Space), time.delta_secs());
    transform.translation = feet + Vec3::Y * EYE_HEIGHT;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_keeps_the_start_facing_and_clamps_pitch() {
        let start = Transform::from_xyz(0.0, 0.0, 0.0).looking_at(Vec3::new(1.0, -1.0, 2.0), Vec3::Y);
        let mut c = Controller::new(&start, 8.0, 30.0);
        assert!(c.rotation().angle_between(start.rotation) < 1e-4, "a new controller must not snap the view");

        c.turn(Vec2::new(0.0, -1e6));
        assert_eq!(c.pitch, FRAC_PI_2);
        c.turn(Vec2::new(0.0, 1e6));
        assert_eq!(c.pitch, -FRAC_PI_2);
    }
}
