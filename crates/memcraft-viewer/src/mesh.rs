//! Builds vertex buffers for one chunk column. Independent of Bevy so it can
//! be tested and reused.

use mcpe_world::{World, HEIGHT};

use crate::blocks::{atlas, look, model, Box16, Layer, Look, Shape, Tile};

/// Interleaved-by-field vertex data, ready for a GPU mesh.
#[derive(Default)]
pub struct MeshData {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub colors: Vec<[f32; 4]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Adds a quad. `corners` run bottom-left, bottom-right, top-right,
    /// top-left as seen from the front; `uv` is the `[u0, v0, u1, v1]` rectangle
    /// of the atlas mapped onto them.
    pub fn quad(&mut self, corners: [[f32; 3]; 4], normal: [f32; 3], color: [f32; 4], uv: [f32; 4]) {
        let base = self.positions.len() as u32;
        let [u0, v0, u1, v1] = uv;
        self.positions.extend(corners);
        self.normals.extend([normal; 4]);
        self.colors.extend([color; 4]);
        self.uvs.extend([[u0, v1], [u1, v1], [u1, v0], [u0, v0]]);
        self.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// One column's geometry, split by render pass (see [`Layer`]).
#[derive(Default)]
pub struct ColumnMesh {
    pub opaque: MeshData,
    pub cutout: MeshData,
    pub translucent: MeshData,
}

impl ColumnMesh {
    fn layer(&mut self, layer: Layer) -> &mut MeshData {
        match layer {
            Layer::Opaque => &mut self.opaque,
            Layer::Cutout => &mut self.cutout,
            Layer::Blend => &mut self.translucent,
        }
    }
}

/// The atlas rectangle `[u0, v0, u1, v1]` of the part `[u0, v0, u1, v1]`
/// (each 0..1, v downwards) of a tile. Inset by a sliver of a texel so
/// nearest sampling never reads the neighbouring tile.
pub fn tile_rect(tile: Tile, [u0, v0, u1, v1]: [f32; 4]) -> [f32; 4] {
    const INSET: f32 = 0.01;
    let (w, h) = (atlas::WIDTH as f32, atlas::HEIGHT as f32);
    let px = atlas::TILE_PX as f32;
    let (x, y) = ((tile as u32 % atlas::COLS) as f32 * px, (tile as u32 / atlas::COLS) as f32 * px);
    [(x + px * u0 + INSET) / w, (y + px * v0 + INSET) / h, (x + px * u1 - INSET) / w, (y + px * v1 - INSET) / h]
}

/// The atlas rectangle of the bottom `height` (0..1) of a tile.
fn tile_uv(tile: Tile, height: f32) -> [f32; 4] {
    tile_rect(tile, [0.0, 1.0 - height, 1.0, 1.0])
}

/// Where a point of face `face` (an index into [`FACES`]) of a block, in
/// sixteenths of the block, falls on a tile pasted on the face with its top
/// up (sides) or north (top and bottom): `(u, v)` in sixteenths of the tile.
fn face_uv(face: usize, p: [u8; 3]) -> (u8, u8) {
    match face {
        0 => (16 - p[2], 16 - p[1]),
        1 => (p[2], 16 - p[1]),
        2 => (p[0], p[2]),
        3 => (p[0], 16 - p[2]),
        4 => (p[0], 16 - p[1]),
        _ => (16 - p[0], 16 - p[1]),
    }
}

/// Whether face `face` of box `i` of `boxes` lies flat against a face of
/// another box that looks the other way and covers it, so nobody can see it.
fn covered_by_sibling(boxes: &[Box16], i: usize, face: usize) -> bool {
    // Axis of the face, and whether it is the box's far side on that axis.
    let (a, far) = (face / 2, face.is_multiple_of(2));
    let (u, v) = [(1, 2), (0, 2), (0, 1)][a];
    let b = boxes[i];
    let plane = if far { b[a + 3] } else { b[a] };
    boxes.iter().enumerate().any(|(j, o)| {
        // `o`'s opposite face must be on the same plane, and span ours.
        let theirs = if far { o[a] } else { o[a + 3] };
        j != i && theirs == plane && o[u] <= b[u] && o[u + 3] >= b[u + 3] && o[v] <= b[v] && o[v + 3] >= b[v + 3]
    })
}

/// Six face directions: offset and the face's corners on a unit cube
/// (bottom-left, bottom-right, top-right, top-left seen from outside), with a
/// fixed light factor per direction so shapes read clearly. The order matches
/// `Look::tiles`.
const FACES: [([i32; 3], [[f32; 3]; 4], f32); 6] = [
    ([1, 0, 0], [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]], 0.8),
    ([-1, 0, 0], [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]], 0.8),
    ([0, 1, 0], [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]], 1.0),
    ([0, -1, 0], [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]], 0.5),
    ([0, 0, 1], [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]], 0.65),
    ([0, 0, -1], [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]], 0.65),
];

fn shade(look: &Look, k: f32, sky: f32) -> [f32; 4] {
    let [r, g, b] = look.color.map(|v| v * k * sky);
    [r, g, b, look.alpha]
}

/// Meshes the column at chunk `(cx, cz)` in world-space block coordinates.
///
/// `light` returns a 0..1 brightness for the air block a face looks into;
/// it lets caves and interiors read darker than open ground.
pub fn mesh_column(world: &World, cx: i32, cz: i32, light: &dyn Fn(i32, i32, i32) -> f32) -> ColumnMesh {
    let mut mesh = ColumnMesh::default();
    let Some(col) = world.columns.get(&mcpe_world::ChunkPos { x: cx, z: cz }) else {
        return mesh;
    };

    let neighbour = |x: i32, y: i32, z: i32| -> (u8, Look) {
        let (id, data) = world.block(x, y, z);
        (id, look(id, data))
    };

    for lx in 0..16 {
        for lz in 0..16 {
            for y in 0..HEIGHT {
                let (id, data) = col.block(lx, y, lz);
                let l = look(id, data);
                if l.shape == Shape::Air {
                    continue;
                }
                let (x, y, z) = (cx * 16 + lx as i32, y as i32, cz * 16 + lz as i32);
                let origin = [x as f32, y as f32, z as f32];
                match l.shape {
                    Shape::Air => {}
                    Shape::Cube | Shape::Translucent => {
                        let out = mesh.layer(l.layer);
                        for (i, (d, corners, k)) in FACES.into_iter().enumerate() {
                            let (nx, ny, nz) = (x + d[0], y + d[1], z + d[2]);
                            let (nid, n) = neighbour(nx, ny, nz);
                            // Hidden behind an opaque cube (not leaves, which have holes),
                            // or inside a body of the same translucent block.
                            if (n.shape == Shape::Cube && n.layer == Layer::Opaque) || (l.shape == Shape::Translucent && same_fluid(id, nid)) {
                                continue;
                            }
                            // Below the world floor nobody can look.
                            if ny < 0 {
                                continue;
                            }
                            let c = corners.map(|p| [origin[0] + p[0], origin[1] + p[1], origin[2] + p[2]]);
                            let n = [d[0] as f32, d[1] as f32, d[2] as f32];
                            out.quad(c, n, shade(&l, k, light(nx, ny, nz)), tile_uv(l.tiles[i], 1.0));
                        }
                    }
                    Shape::Partial(h) => {
                        let sky = light(x, y, z);
                        let out = mesh.layer(l.layer);
                        for (i, (d, corners, k)) in FACES.into_iter().enumerate() {
                            // Only the bottom face can be hidden by the block below.
                            if d == [0, -1, 0] && neighbour(x, y - 1, z).1.shape == Shape::Cube {
                                continue;
                            }
                            let c = corners.map(|p| [origin[0] + p[0], origin[1] + p[1] * h, origin[2] + p[2]]);
                            let n = [d[0] as f32, d[1] as f32, d[2] as f32];
                            // Sides show the lower part of their tile.
                            let uv = tile_uv(l.tiles[i], if d[1] == 0 { h } else { 1.0 });
                            out.quad(c, n, shade(&l, k, sky), uv);
                        }
                    }
                    Shape::Boxes => {
                        let sky = light(x, y, z);
                        let boxes = model(id, data, &|dx, dy, dz| world.block(x + dx, y + dy, z + dz)).visual;
                        let out = mesh.layer(l.layer);
                        for (bi, b) in boxes.iter().enumerate() {
                            for (i, (d, corners, k)) in FACES.into_iter().enumerate() {
                                // A face on the block's edge is hidden by an opaque cube there.
                                let on_edge = if d.iter().sum::<i32>() > 0 { b[i / 2 + 3] == 16 } else { b[i / 2] == 0 };
                                if on_edge {
                                    let (nx, ny, nz) = (x + d[0], y + d[1], z + d[2]);
                                    let n = neighbour(nx, ny, nz).1;
                                    if (n.shape == Shape::Cube && n.layer == Layer::Opaque) || ny < 0 {
                                        continue;
                                    }
                                }
                                if covered_by_sibling(&boxes, bi, i) {
                                    continue;
                                }
                                // The box's own corner: its min or max on each axis.
                                let at = |p: [f32; 3]| [0, 1, 2].map(|a| if p[a] == 0.0 { b[a] } else { b[a + 3] });
                                let c = corners.map(|p| at(p).map(|v| f32::from(v) / 16.0));
                                let c = c.map(|p| [origin[0] + p[0], origin[1] + p[1], origin[2] + p[2]]);
                                // Texture the face with the part of the tile it covers.
                                let ((u0, v1), (u1, v0)) = (face_uv(i, at(corners[0])), face_uv(i, at(corners[2])));
                                let uv = [u0, v0, u1, v1].map(|v| f32::from(v) / 16.0);
                                let n = [d[0] as f32, d[1] as f32, d[2] as f32];
                                out.quad(c, n, shade(&l, k, sky), tile_rect(l.tiles[i], uv));
                            }
                        }
                    }
                    Shape::Cross => {
                        let sky = light(x, y, z);
                        let color = shade(&l, 0.9, sky);
                        let uv = tile_uv(l.tiles[0], 1.0);
                        let [ox, oy, oz] = origin;
                        let (a, b) = (0.15, 0.85);
                        // Two diagonal quads, each drawn from both sides.
                        for (p, q, n) in [
                            ([ox + a, oz + a], [ox + b, oz + b], [0.7, 0.0, -0.7]),
                            ([ox + a, oz + b], [ox + b, oz + a], [0.7, 0.0, 0.7]),
                        ] {
                            let quad = [[p[0], oy, p[1]], [q[0], oy, q[1]], [q[0], oy + 1.0, q[1]], [p[0], oy + 1.0, p[1]]];
                            mesh.cutout.quad(quad, n, color, uv);
                            let [c0, c1, c2, c3] = quad;
                            mesh.cutout.quad([c1, c0, c3, c2], n.map(|v: f32| -v), color, uv);
                        }
                    }
                }
            }
        }
    }
    mesh
}

/// Water (still and flowing) and lava merge into one body; other
/// translucent blocks merge only with themselves.
fn same_fluid(a: u8, b: u8) -> bool {
    let norm = |id| match id {
        8 | 9 => 9,
        10 | 11 => 11,
        id => id,
    };
    norm(a) == norm(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A block, `(id, data)`, at local `(x, y, z)`.
    type Placed = ((usize, usize, usize), (u8, u8));

    /// The sample world with some blocks set in the open air above column
    /// `(8, 8)`, meshed.
    fn mesh_with(blocks: &[Placed]) -> ColumnMesh {
        let mut w = mcpe_world::World::open_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World")).unwrap();
        let col = w.columns.get_mut(&mcpe_world::ChunkPos { x: 8, z: 8 }).unwrap();
        for &((x, y, z), (id, data)) in blocks {
            assert_eq!(col.block(x, y, z), (0, 0), "({x}, {y}, {z}) should be open air");
            let i = mcpe_world::Column::index(x, y, z);
            (col.ids[i], col.data[i]) = (id, data);
        }
        mesh_column(&w, 8, 8, &|_, _, _| 1.0)
    }

    fn quads(m: &MeshData) -> usize {
        m.indices.len() / 6
    }

    #[test]
    fn fence_boxes_are_meshed_without_their_hidden_faces() {
        let none = quads(&mesh_with(&[]).opaque);
        // A lone post is a box of six faces.
        assert_eq!(quads(&mesh_with(&[((5, 120, 5), (85, 0))]).opaque), none + 6);
        // Two fences side by side: each has a post and two rails whose
        // face against the post is hidden by it.
        let pair = mesh_with(&[((5, 120, 5), (85, 0)), ((6, 120, 5), (85, 0))]);
        assert_eq!(quads(&pair.opaque), none + 2 * (6 + 2 * 5));
    }

    #[test]
    fn box_faces_hide_against_opaque_cubes_only() {
        let none = quads(&mesh_with(&[]).opaque);
        // A closed door (a plate on the west edge) next to stone to the west:
        // the lower half loses its west face to the stone. The stone keeps its
        // face towards the door, which is not an opaque cube.
        let door = |stone: u8| mesh_with(&[((5, 120, 5), (64, 0)), ((5, 121, 5), (64, 8)), ((4, 120, 5), (stone, 0))]);
        assert_eq!(quads(&door(1).opaque), none + 5 + 6 + 6);
        // Glass is not an opaque cube: the plate shows all its faces.
        let with_glass = door(20);
        assert_eq!(quads(&with_glass.opaque), none + 6 + 6);
        assert_eq!(quads(&with_glass.translucent), 6 + quads(&mesh_with(&[]).translucent));
    }

    #[test]
    fn box_textures_show_the_part_of_the_tile_they_cover() {
        // The top of a 3/16 thick plate against the west edge: the strip
        // `x` in 0..3, running the tile's whole depth.
        let top = [0, 16, 16, 3, 16, 0];
        let (u0, v1) = face_uv(2, [top[0], top[1], top[2]]);
        let (u1, v0) = face_uv(2, [top[3], top[4], top[5]]);
        assert_eq!([u0, v0, u1, v1], [0, 0, 3, 16]);
        // Its east face is the whole tile, right way up.
        let (u0, v1) = face_uv(0, [3, 0, 16]);
        let (u1, v0) = face_uv(0, [3, 16, 0]);
        assert_eq!([u0, v0, u1, v1], [0, 0, 16, 16]);
    }

    #[test]
    fn sample_world_meshes_without_hidden_faces_blowing_up() {
        let w = mcpe_world::World::open_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../worlds/My World")).unwrap();
        let m = mesh_column(&w, 8, 8, &|_, _, _| 1.0);
        let quads = m.opaque.indices.len() / 6;
        // A solid 16x16x128 column has ~33k block faces; culling should leave a small fraction.
        assert!(quads > 256, "too few faces: {quads}");
        assert!(quads < 20_000, "face culling looks broken: {quads}");
        assert_eq!(m.opaque.positions.len(), m.opaque.colors.len());
        assert_eq!(m.opaque.positions.len(), m.opaque.uvs.len());
    }
}
