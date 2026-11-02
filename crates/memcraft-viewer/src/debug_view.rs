//! Starting the camera somewhere chosen from outside, so a scripted run (the
//! headless browser drive, see `web/drive.js`) can look straight at what it
//! tests instead of flying there.
//!
//! The request is `cam=x,y,z,yaw,pitch` in the page's query string (browser)
//! or in the `MEMCRAFT_CAM` environment variable (native): the eye position,
//! and the facing in degrees as the world's own player records have it (yaw 0
//! looks toward +Z and turns toward -X, positive pitch looks down).

use bevy::prelude::*;

/// A camera at `eye`, facing `yaw` and `pitch` (degrees; see the module docs).
pub fn view(eye: Vec3, yaw: f32, pitch: f32) -> Transform {
    let (yaw, pitch) = (yaw.to_radians(), pitch.to_radians());
    let dir = Vec3::new(-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos());
    Transform::from_translation(eye).looking_to(dir, Vec3::Y)
}

/// Reads `x,y,z,yaw,pitch` as the camera it asks for.
fn parse(text: &str) -> Option<Transform> {
    let v: Vec<f32> = text.split(',').map(|s| s.trim().parse().ok()).collect::<Option<_>>()?;
    match v[..] {
        [x, y, z, yaw, pitch] => Some(view(Vec3::new(x, y, z), yaw, pitch)),
        _ => None,
    }
}

/// The camera the page or the environment asked for, if any.
pub fn requested() -> Option<Transform> {
    #[cfg(target_arch = "wasm32")]
    let text = web_sys::window()?
        .location()
        .search()
        .ok()?
        .trim_start_matches('?')
        .split('&')
        .find_map(|kv| kv.strip_prefix("cam=").map(|s| s.replace("%2C", ",")));
    #[cfg(not(target_arch = "wasm32"))]
    let text = std::env::var("MEMCRAFT_CAM").ok();
    parse(&text?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_names_the_eye_and_the_facing() {
        let t = parse("174.5, 69, 201.5, 0, 30").unwrap();
        assert_eq!(t.translation, Vec3::new(174.5, 69.0, 201.5));
        // Yaw 0 faces +Z, and a positive pitch looks down.
        let f = *t.forward();
        assert!(f.z > 0.8 && f.y < -0.4 && f.x.abs() < 1e-5, "{f:?}");
        // Yaw 90 turns toward -X.
        assert!(parse("0,0,0,90,0").unwrap().forward().x < -0.99);
    }

    #[test]
    fn anything_else_is_not_a_request() {
        for bad in ["", "1,2,3", "1,2,3,4,5,6", "a,b,c,d,e"] {
            assert!(parse(bad).is_none(), "{bad:?}");
        }
    }
}
