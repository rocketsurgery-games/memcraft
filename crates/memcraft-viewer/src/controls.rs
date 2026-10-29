//! Camera controls: click to lock the pointer and look around, Esc to
//! release it; WASD/QE to fly, Shift to go fast, scroll to change speed.
//!
//! This replaces Bevy's `FreeCamera`, which only looks around while the
//! right button is held and keeps its grab state private.

use std::f32::consts::FRAC_PI_2;

use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};

pub struct ControlsPlugin;

impl Plugin for ControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (grab_pointer, look, fly).chain());
    }
}

/// The camera the player steers.
#[derive(Component)]
pub struct Controller {
    /// Radians of turn per pixel of mouse motion.
    pub sensitivity: f32,
    /// Blocks per second, before the scroll multiplier.
    pub speed: f32,
    pub fast_speed: f32,
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
        Self { sensitivity: 0.2 / 180.0, speed, fast_speed, scroll_factor: 0.05, speed_multiplier: 1.0, yaw, pitch }
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

fn is_locked(cursor: &CursorOptions) -> bool {
    cursor.grab_mode != CursorGrabMode::None
}

fn set_locked(cursor: &mut CursorOptions, locked: bool) {
    cursor.grab_mode = if locked { CursorGrabMode::Locked } else { CursorGrabMode::None };
    cursor.visible = !locked;
}

/// A left click locks the pointer, and it stays locked until Esc or until
/// the window loses focus. In the browser, Esc is handled by the browser
/// itself (the page never sees it), so the lock is re-read from the page.
fn grab_pointer(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut window: Single<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    #[cfg(target_arch = "wasm32")] mut browser_had_lock: Local<bool>,
) {
    let (window, cursor) = &mut *window;
    let locked = is_locked(cursor);
    if !locked && window.focused && mouse.just_pressed(MouseButton::Left) {
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

/// Free flight: W/S along the view direction (pitch included), A/D sideways,
/// Q/E straight down and up.
fn fly(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    mut camera: Single<(&mut Transform, &mut Controller)>,
) {
    let (transform, controller) = &mut *camera;
    let lines = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR,
    };
    controller.speed_multiplier = (controller.speed_multiplier * (controller.scroll_factor * lines).exp()).clamp(0.05, 50.0);

    let axis = |pos: KeyCode, neg: KeyCode| keys.pressed(pos) as i32 as f32 - keys.pressed(neg) as i32 as f32;
    let (forward, right, up) =
        (axis(KeyCode::KeyW, KeyCode::KeyS), axis(KeyCode::KeyD, KeyCode::KeyA), axis(KeyCode::KeyE, KeyCode::KeyQ));
    let dir = forward * *transform.forward() + right * *transform.right() + up * Vec3::Y;
    if dir == Vec3::ZERO {
        return;
    }
    let fast = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    let speed = if fast { controller.fast_speed } else { controller.speed } * controller.speed_multiplier;
    transform.translation += dir.normalize() * speed * time.delta_secs();
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
