//! Mobs, minecarts and dropped items, drawn once at startup as boxes.
//!
//! They are entity records of the world's columns (Bedrock NBT). The type
//! is the low byte of `id` (newer saves set flag bits above it, as for
//! paintings, see `entities`). A mob is a coloured box of its standard
//! Minecraft hitbox size, with a smaller box for the head; a dropped item is
//! a small cube showing the item's block or icon. Types without an entry in
//! [`KINDS`] (projectiles, slimes, which have no fixed size, ...) are not
//! drawn.
//!
//! Like paintings they are static: no AI, no animation, and they do not
//! collide, cannot be targeted or broken and are not saved back.

use std::collections::BTreeMap;

use bevy::prelude::*;
use mcpe_world::{Tag, World as PeWorld};

use crate::blocks::{self, atlas::tile as t, Shape, Tile};
use crate::item_icons;
use crate::mesh::{tile_rect, MeshData, FACES};
use crate::world::{to_bevy, BlockMaterials, WorldMap};

/// The entity type of a dropped item.
const ITEM: i64 = 64;
/// A dropped item is a cube of this edge, centred on its `Pos`.
const ITEM_EDGE: f32 = 0.25;
/// Babies are drawn at this fraction of the adult size.
const BABY: f32 = 0.5;

/// Where a mob's head box goes, with its edge in blocks.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Head {
    None,
    /// On top of the body, which is that much shorter (people, creepers).
    Top(f32),
    /// Facing forward at the top of the body, half of it sticking out in
    /// front (animals).
    Front(f32),
}

/// How a type of entity is drawn.
#[derive(Debug, PartialEq)]
struct Kind {
    /// The low byte of the entity `id`.
    id: u8,
    name: &'static str,
    /// The hitbox: width (and depth) and height, in blocks.
    size: [f32; 2],
    head: Head,
    body: [f32; 3],
    head_color: [f32; 3],
}

/// An sRGB colour from `0xrrggbb`.
const fn hex(c: u32) -> [f32; 3] {
    [(c >> 16 & 0xff) as f32 / 255.0, (c >> 8 & 0xff) as f32 / 255.0, (c & 0xff) as f32 / 255.0]
}

const fn kind(id: u8, name: &'static str, size: [f32; 2], head: Head, body: u32, head_color: u32) -> Kind {
    Kind { id, name, size, head, body: hex(body), head_color: hex(head_color) }
}

use Head::{Front, Top};

/// The entity types we draw, with their standard hitbox sizes.
const KINDS: [Kind; 32] = [
    kind(10, "chicken", [0.4, 0.7], Front(0.25), 0xf2f2f2, 0xe8d8b8),
    kind(11, "cow", [0.9, 1.4], Front(0.5), 0x4a3728, 0x6b5a4a),
    kind(12, "pig", [0.9, 0.9], Front(0.5), 0xf0a8a0, 0xe89890),
    kind(13, "sheep", [0.9, 1.3], Front(0.5), 0xe8e8e8, 0xb8a890),
    kind(14, "wolf", [0.6, 0.85], Front(0.4), 0xc8c8c8, 0xd8d8d8),
    kind(15, "villager", [0.6, 1.8], Top(0.5), 0x6b4a2a, 0xc89878),
    kind(16, "mooshroom", [0.9, 1.4], Front(0.5), 0xa81818, 0xc8c8c8),
    kind(17, "squid", [0.95, 0.95], Head::None, 0x1c2c48, 0x1c2c48),
    kind(18, "rabbit", [0.4, 0.5], Front(0.25), 0x8a6a48, 0x9a7a58),
    kind(19, "bat", [0.5, 0.9], Head::None, 0x3a2a20, 0x3a2a20),
    kind(20, "iron golem", [1.4, 2.7], Top(0.6), 0xc8c8c0, 0xb8b8b0),
    kind(21, "snow golem", [0.7, 1.9], Top(0.5), 0xf8f8f8, 0xe8a050),
    kind(22, "ocelot", [0.6, 0.8], Front(0.35), 0xe8c060, 0xd8b050),
    kind(23, "horse", [1.4, 1.6], Front(0.5), 0x8a5a30, 0x7a4a28),
    kind(24, "donkey", [1.4, 1.6], Front(0.5), 0x7a6a5a, 0x6a5a4a),
    kind(25, "mule", [1.4, 1.6], Front(0.5), 0x5a4030, 0x4a3020),
    kind(26, "skeleton horse", [1.4, 1.6], Front(0.5), 0xd8d8d0, 0xc8c8c0),
    kind(27, "zombie horse", [1.4, 1.6], Front(0.5), 0x6a8a5a, 0x5a7a4a),
    kind(32, "zombie", [0.6, 1.95], Top(0.5), 0x2a4a8a, 0x5a8a4a),
    kind(33, "creeper", [0.6, 1.7], Top(0.5), 0x4aa83a, 0x58b848),
    kind(34, "skeleton", [0.6, 1.99], Top(0.5), 0xc8c8c0, 0xe0e0d8),
    kind(35, "spider", [1.4, 0.9], Head::None, 0x3a3030, 0x4a3030),
    kind(36, "zombie pigman", [0.6, 1.95], Top(0.5), 0xc87878, 0xd89898),
    kind(38, "enderman", [0.6, 2.9], Top(0.5), 0x181018, 0x181018),
    kind(39, "silverfish", [0.4, 0.3], Head::None, 0x808890, 0x808890),
    kind(40, "cave spider", [0.7, 0.5], Head::None, 0x1a4a48, 0x2a5a58),
    kind(41, "ghast", [4.0, 4.0], Head::None, 0xf0f0f0, 0xf0f0f0),
    kind(43, "blaze", [0.6, 1.8], Top(0.5), 0xe8b020, 0xf0c840),
    kind(44, "zombie villager", [0.6, 1.95], Top(0.5), 0x5a4a2a, 0x5a8a4a),
    kind(45, "witch", [0.6, 1.95], Top(0.5), 0x4a2a6a, 0xc89878),
    kind(84, "minecart", [0.98, 0.7], Head::None, 0x6a6a72, 0x6a6a72),
    kind(90, "boat", [1.5, 0.6], Head::None, 0x8a6a3a, 0x8a6a3a),
];

/// The mob or vehicle of an entity record, ready to be drawn.
#[derive(Debug, Clone, PartialEq)]
struct Mob {
    kind: &'static Kind,
    /// The feet, in world coordinates.
    pos: [f32; 3],
    /// Degrees; 0 faces +z and turns toward -x, as for the camera.
    yaw: f32,
    baby: bool,
}

/// A dropped item, ready to be drawn.
#[derive(Debug, Clone, PartialEq)]
struct Drop {
    /// The centre of the cube.
    pos: [f32; 3],
    yaw: f32,
    /// Atlas tile and tint per face, in the mesher's face order.
    faces: [(Tile, [f32; 3]); 6],
}

/// The entity type: the low byte of `id`.
fn kind_of(e: &Tag) -> Option<i64> {
    Some(e.get("id")?.as_i64()? & 0xff)
}

fn floats(e: &Tag, key: &str) -> Vec<f32> {
    e.get(key).and_then(Tag::as_list).map(|l| l.iter().filter_map(Tag::as_f64).map(|v| v as f32).collect()).unwrap_or_default()
}

/// `Pos` and the yaw of `Rotation`.
fn placement(e: &Tag) -> Option<([f32; 3], f32)> {
    let pos = floats(e, "Pos").try_into().ok()?;
    Some((pos, floats(e, "Rotation").first().copied().unwrap_or(0.0)))
}

/// Every mob or vehicle of a type we can draw, in the world's entity records.
fn mobs(world: &PeWorld) -> Vec<Mob> {
    world
        .columns
        .values()
        .flat_map(|col| &col.entities)
        .filter_map(|e| {
            let id = kind_of(e)?;
            let kind = KINDS.iter().find(|k| i64::from(k.id) == id)?;
            let (pos, yaw) = placement(e)?;
            let baby = e.get("IsBaby").and_then(Tag::as_i64).is_some_and(|b| b != 0);
            Some(Mob { kind, pos, yaw, baby })
        })
        .collect()
}

/// Every dropped item whose block or icon we can show.
fn drops(world: &PeWorld) -> Vec<Drop> {
    world
        .columns
        .values()
        .flat_map(|col| &col.entities)
        .filter(|e| kind_of(e) == Some(ITEM))
        .filter_map(|e| {
            let (pos, yaw) = placement(e)?;
            Some(Drop { pos, yaw, faces: item_faces(e.get("Item")?)? })
        })
        .collect()
}

/// The tiles and tints that show an item stack on a cube: a block's own
/// faces, or an item's icon on every face. `None` for an empty stack or an
/// item without one.
fn item_faces(item: &Tag) -> Option<[(Tile, [f32; 3]); 6]> {
    let id = item.get("id")?.as_i64()?;
    let damage = item.get("Damage").and_then(Tag::as_i64).unwrap_or(0);
    if item.get("Count").and_then(Tag::as_i64).unwrap_or(1) <= 0 {
        return None;
    }
    match id {
        1..=255 => {
            let look = blocks::look(id as u8, damage as u8);
            (look.shape != Shape::Air).then(|| look.tiles.map(|tile| (tile, look.color)))
        }
        256.. => Some([(item_icons::icon(i16::try_from(id).ok()?, damage as i16)?, [1.0; 3]); 6]),
        _ => None,
    }
}

/// Adds the box `lo..hi` to `out`. The box is in a frame with its origin at
/// `origin` and turned by `turn` (the sine and cosine of the yaw) about the
/// vertical; `paint` gives the tile and the colour of each face.
fn cuboid(out: &mut MeshData, origin: [f32; 3], (sin, cos): (f32, f32), (lo, hi): ([f32; 3], [f32; 3]), paint: impl Fn(usize) -> (Tile, [f32; 3]), sky: f32) {
    // Yaw 0 faces +z; a turn of `yaw` takes +z to (-sin, cos).
    let turn = |[x, y, z]: [f32; 3]| [x * cos - z * sin, y, x * sin + z * cos];
    for (i, (d, corners, k)) in FACES.into_iter().enumerate() {
        let c = corners.map(|p| {
            let local = std::array::from_fn(|a| lo[a] + p[a] * (hi[a] - lo[a]));
            let world = turn(local);
            std::array::from_fn(|a| origin[a] + world[a])
        });
        let (tile, tint) = paint(i);
        let [r, g, b] = tint.map(|v| v * k * sky);
        out.quad(c, turn(d.map(|v| v as f32)), [r, g, b, 1.0], tile_rect(tile, [0.0, 0.0, 1.0, 1.0]));
    }
}

/// The boxes of a mob in its own frame (feet at the origin, facing +z), body
/// first: `(lo, hi, colour)`.
fn parts(mob: &Mob) -> Vec<([f32; 3], [f32; 3], [f32; 3])> {
    let Kind { size: [w, h], head, body, head_color, .. } = *mob.kind;
    let s = if mob.baby { BABY } else { 1.0 };
    let (w, h) = (w * s, h * s);
    let half = w / 2.0;
    let mut parts = Vec::new();
    match head {
        Head::None => parts.push(([-half, 0.0, -half], [half, h, half], body)),
        Head::Top(e) => {
            let e = e * s;
            parts.push(([-half, 0.0, -half], [half, h - e, half], body));
            parts.push(([-e / 2.0, h - e, -e / 2.0], [e / 2.0, h, e / 2.0], head_color));
        }
        Head::Front(e) => {
            let e = e * s;
            parts.push(([-half, 0.0, -half], [half, h * 0.75, half], body));
            parts.push(([-e / 2.0, h - e, half - e / 2.0], [e / 2.0, h, half + e / 2.0], head_color));
        }
    }
    parts
}

/// All the mobs and dropped items as one mesh textured from the atlas's
/// white tile and the block and icon tiles, lit by the sky above them.
fn mesh(mobs: &[Mob], drops: &[Drop], light: &dyn Fn(i32, i32, i32) -> f32) -> MeshData {
    let mut out = MeshData::default();
    let sky = |p: [f32; 3], lift: f32| light(p[0].floor() as i32, (p[1] + lift).floor() as i32, p[2].floor() as i32);
    for mob in mobs {
        let turn = mob.yaw.to_radians().sin_cos();
        let sky = sky(mob.pos, 0.5);
        for (lo, hi, color) in parts(mob) {
            cuboid(&mut out, mob.pos, turn, (lo, hi), |_| (t::WHITE, color), sky);
        }
    }
    for d in drops {
        let h = ITEM_EDGE / 2.0;
        cuboid(&mut out, d.pos, d.yaw.to_radians().sin_cos(), ([-h; 3], [h; 3]), |i| d.faces[i], sky(d.pos, 0.0));
    }
    out
}

/// How many of each type there are, by name.
fn census(mobs: &[Mob]) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for m in mobs {
        *counts.entry(m.kind.name).or_insert(0) += 1;
    }
    counts
}

/// Spawns every mob and dropped item of the world.
pub fn spawn(mut commands: Commands, map: Res<WorldMap>, block_materials: Res<BlockMaterials>, mut meshes: ResMut<Assets<Mesh>>) {
    let mobs = mobs(map.world());
    let drops = drops(map.world());
    let light = |x, y, z| map.light(x, y, z);
    // Item icons have transparent corners, so everything uses the cutout pass.
    commands.spawn((Mesh3d(meshes.add(to_bevy(mesh(&mobs, &drops, &light)))), MeshMaterial3d(block_materials.cutout.clone())));
    info!("drew {} mobs {:?} and {} dropped items", mobs.len(), census(&mobs), drops.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PeWorld {
        PeWorld::open_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World")).unwrap()
    }

    fn kind_named(name: &str) -> &'static Kind {
        KINDS.iter().find(|k| k.name == name).unwrap()
    }

    fn mob(name: &str, yaw: f32, baby: bool) -> Mob {
        Mob { kind: kind_named(name), pos: [10.0, 64.0, 20.0], yaw, baby }
    }

    /// The corners of the mesh `mob` draws, as `(min, max)`.
    fn bounds(mob: Mob) -> (Vec3, Vec3) {
        let m = mesh(&[mob], &[], &|_, _, _| 1.0);
        let p = m.positions.iter().map(|&p| Vec3::from(p));
        (p.clone().fold(Vec3::MAX, Vec3::min), p.fold(Vec3::MIN, Vec3::max))
    }

    #[test]
    fn the_table_is_sane() {
        for (i, k) in KINDS.iter().enumerate() {
            assert!(KINDS[..i].iter().all(|o| o.id != k.id), "{} is listed twice", k.name);
            assert_ne!(i64::from(k.id), ITEM);
            assert!(k.size.iter().all(|&s| s > 0.0), "{} has no size", k.name);
            match k.head {
                Head::Top(e) | Head::Front(e) => assert!(e < k.size[1] && e >= 0.2, "{} has a bad head", k.name),
                Head::None => {}
            }
        }
    }

    #[test]
    fn a_mob_fills_its_hitbox() {
        // Upright: width by height, feet on the position.
        let (lo, hi) = bounds(mob("zombie", 0.0, false));
        assert!((lo - Vec3::new(9.7, 64.0, 19.7)).abs().max_element() < 1e-5, "{lo}");
        assert!((hi - Vec3::new(10.3, 65.95, 20.3)).abs().max_element() < 1e-5, "{hi}");
        // Without a head the box is the hitbox.
        let (lo, hi) = bounds(mob("spider", 0.0, false));
        assert!((hi - lo - Vec3::new(1.4, 0.9, 1.4)).abs().max_element() < 1e-5);
        // A baby is half the size.
        let (lo, hi) = bounds(mob("zombie", 0.0, true));
        assert!((hi - lo).y < 1.0 && (hi - lo).y > 0.9);
    }

    #[test]
    fn an_animals_head_points_the_way_it_faces() {
        // Yaw 0 faces +z; yaw 90 faces -x.
        let (_, hi) = bounds(mob("pig", 0.0, false));
        assert!((hi.z - (20.0 + 0.45 + 0.25)).abs() < 1e-5 && (hi.x - 10.45).abs() < 1e-5, "{hi}");
        let (lo, hi) = bounds(mob("pig", 90.0, false));
        assert!((lo.x - (10.0 - 0.45 - 0.25)).abs() < 1e-5 && (hi.z - 20.45).abs() < 1e-5, "{lo} {hi}");
        // Never taller than the hitbox.
        assert!((hi.y - 64.9).abs() < 1e-5);
    }

    #[test]
    fn faces_wind_outwards_after_a_turn() {
        let m = mesh(&[mob("cow", 130.0, false)], &[], &|_, _, _| 1.0);
        for q in 0..m.positions.len() / 4 {
            let c: Vec<Vec3> = (0..4).map(|i| Vec3::from(m.positions[q * 4 + i])).collect();
            let n = Vec3::from(m.normals[q * 4]);
            assert!((c[1] - c[0]).cross(c[2] - c[0]).dot(n) > 0.0, "quad {q} winds away from its normal");
        }
    }

    #[test]
    fn the_sample_world_has_its_mobs_and_dropped_items() {
        let world = sample();
        let mobs = mobs(&world);
        let counts = census(&mobs);
        println!("{} mobs {counts:?}", mobs.len());
        assert_eq!(
            counts,
            BTreeMap::from([
                ("chicken", 32),
                ("cow", 14),
                ("minecart", 13),
                ("ocelot", 12),
                ("pig", 11),
                ("sheep", 6),
                ("skeleton horse", 1),
                ("zombie horse", 1),
            ])
        );
        assert_eq!(drops(&world).len(), 10);
    }

    #[test]
    fn other_entities_are_not_mobs() {
        // Paintings (83) and dropped items (64) are drawn elsewhere or by `drops`.
        let world = sample();
        let all: usize = world.columns.values().map(|c| c.entities.len()).sum();
        let drawn = mobs(&world).len() + drops(&world).len();
        let paintings = world.columns.values().flat_map(|c| &c.entities).filter(|e| kind_of(e) == Some(83)).count();
        assert_eq!(all, drawn + paintings);
    }

    #[test]
    fn mobs_stand_in_the_open() {
        let world = sample();
        for m in mobs(&world) {
            let cell = m.pos.map(|v| v.floor() as i32);
            let (id, data) = world.block(cell[0], cell[1], cell[2]);
            assert_ne!(blocks::look(id, data).shape, Shape::Cube, "{m:?} is inside a block");
        }
    }

    /// Prints a camera (`?cam=` / `MEMCRAFT_CAM`, see `debug_view`) with the
    /// most mobs and dropped items in clear view, for a drive screenshot:
    /// `cargo test -p memcraft-viewer vantage -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn vantage() {
        let world = sample();
        let mut spots: Vec<Vec3> = mobs(&world).iter().map(|m| Vec3::from(m.pos) + Vec3::Y * 0.5).collect();
        spots.extend(drops(&world).iter().map(|d| Vec3::from(d.pos)));
        let open = |p: Vec3| {
            let c = p.floor().as_ivec3();
            let (id, data) = world.block(c.x, c.y, c.z);
            blocks::look(id, data).shape != Shape::Cube
        };
        let clear = |from: Vec3, to: Vec3| {
            let steps = ((to - from).length() * 8.0) as i32;
            (0..steps).all(|i| open(from.lerp(to, i as f32 / steps as f32)))
        };
        let mut best = (0, String::new());
        for &centre in &spots {
            let near: Vec<Vec3> = spots.iter().copied().filter(|s| s.distance(centre) < 5.0).collect();
            for radius in [4.0f32, 6.0, 8.0] {
                for k in 0..16 {
                    let a = k as f32 * std::f32::consts::TAU / 16.0;
                    for lift in [1.0, 2.0, 3.0] {
                        let eye = centre + Vec3::new(a.cos() * radius, lift, a.sin() * radius);
                        if !open(eye) {
                            continue;
                        }
                        let seen = near.iter().filter(|&&s| clear(eye, s)).count();
                        if seen > best.0 {
                            let d = near.iter().sum::<Vec3>() / near.len() as f32 - eye;
                            let yaw = (-d.x).atan2(d.z).to_degrees();
                            let pitch = (-d.y).atan2(d.x.hypot(d.z)).to_degrees();
                            best = (seen, format!("{:.1},{:.1},{:.1},{yaw:.0},{pitch:.0}", eye.x, eye.y, eye.z));
                        }
                    }
                }
            }
        }
        println!("CAM {} sees {}", best.1, best.0);
    }
}
