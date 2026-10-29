//! Builds vertex buffers for one chunk column. Independent of Bevy so it can
//! be tested and reused.

use mcpe_world::{World, HEIGHT};

use crate::blocks::{atlas, look, Layer, Look, Shape, Tile};

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
    fn quad(&mut self, corners: [[f32; 3]; 4], normal: [f32; 3], color: [f32; 4], uv: [f32; 4]) {
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

/// The atlas rectangle `[u0, v0, u1, v1]` of the bottom `height` (0..1) of a
/// tile. Inset by a sliver of a texel so nearest sampling never reads the
/// neighbouring tile.
fn tile_uv(tile: Tile, height: f32) -> [f32; 4] {
    const INSET: f32 = 0.01;
    let (w, h) = (atlas::WIDTH as f32, atlas::HEIGHT as f32);
    let px = atlas::TILE_PX as f32;
    let (x, y) = ((tile as u32 % atlas::COLS) as f32 * px, (tile as u32 / atlas::COLS) as f32 * px);
    let (top, bottom) = (y + px * (1.0 - height), y + px);
    [(x + INSET) / w, (top + INSET) / h, (x + px - INSET) / w, (bottom - INSET) / h]
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
