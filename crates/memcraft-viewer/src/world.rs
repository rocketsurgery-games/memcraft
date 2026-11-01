//! The world at runtime: the blocks, which can change, and the column meshes
//! drawn from them.
//!
//! [`WorldMap::set_block`] edits a block and marks its column (and the
//! neighbouring column, for a block on a column edge) for remeshing;
//! [`remesh`] rebuilds marked columns each frame. Startup marks every column,
//! so the first frame meshes the whole world through the same path.

use std::collections::{BTreeSet, HashMap};

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use mcpe_world::{ChunkPos, Column, World as PeWorld, HEIGHT};

use crate::blocks::{self, Shape};
use crate::mesh::{self, MeshData};
use crate::physics::Solids;

/// Faces in shadow (under an opaque block) are drawn this much darker.
const SHADOW: f32 = 0.55;

#[derive(Resource)]
pub struct WorldMap {
    world: PeWorld,
    /// Highest opaque cube per column position, indexed `z * 16 + x`
    /// (-1: none), so faces under cover can be drawn darker.
    heights: HashMap<ChunkPos, [i32; 256]>,
    /// Columns to remesh.
    dirty: BTreeSet<ChunkPos>,
}

impl WorldMap {
    /// Wraps `world`, with every column marked for meshing.
    pub fn new(world: PeWorld) -> Self {
        let heights = world.columns.iter().map(|(pos, col)| (*pos, std::array::from_fn(|i| column_height(col, i % 16, i / 16)))).collect();
        let dirty = world.columns.keys().copied().collect();
        Self { world, heights, dirty }
    }

    #[allow(dead_code, reason = "only the tests need it so far")]
    pub fn world(&self) -> &PeWorld {
        &self.world
    }

    /// The block at world position `(x, y, z)`; air outside the world.
    pub fn block(&self, x: i32, y: i32, z: i32) -> (u8, u8) {
        self.world.block(x, y, z)
    }

    /// Sets the block at `(x, y, z)` and marks what must be remeshed.
    /// Returns false (and changes nothing) outside the loaded world.
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, id: u8, data: u8) -> bool {
        let pos = ChunkPos { x: x.div_euclid(16), z: z.div_euclid(16) };
        let (lx, lz) = (x.rem_euclid(16) as usize, z.rem_euclid(16) as usize);
        let Some(col) = self.world.columns.get_mut(&pos).filter(|_| (0..HEIGHT as i32).contains(&y)) else {
            return false;
        };
        let i = Column::index(lx, y as usize, lz);
        col.ids[i] = id;
        col.data[i] = data & 0x0f;
        self.heights.get_mut(&pos).unwrap()[lz * 16 + lx] = column_height(col, lx, lz);

        self.dirty.insert(pos);
        // Faces on the far side of a column edge belong to the neighbour.
        for (on_edge, dx, dz) in [(lx == 0, -1, 0), (lx == 15, 1, 0), (lz == 0, 0, -1), (lz == 15, 0, 1)] {
            let n = ChunkPos { x: pos.x + dx, z: pos.z + dz };
            if on_edge && self.world.columns.contains_key(&n) {
                self.dirty.insert(n);
            }
        }
        true
    }

    /// Brightness (0..1) of the air block at `(x, y, z)`, which a face looks into.
    fn light(&self, x: i32, y: i32, z: i32) -> f32 {
        let pos = ChunkPos { x: x.div_euclid(16), z: z.div_euclid(16) };
        match self.heights.get(&pos) {
            Some(h) if y <= h[(z.rem_euclid(16) * 16 + x.rem_euclid(16)) as usize] => SHADOW,
            _ => 1.0,
        }
    }

    /// Meshes one column (see [`mesh::mesh_column`]) with this world's shading.
    pub fn mesh_column(&self, pos: ChunkPos) -> mesh::ColumnMesh {
        mesh::mesh_column(&self.world, pos.x, pos.z, &|x, y, z| self.light(x, y, z))
    }
}

/// Highest opaque cube in one column position, or -1.
fn column_height(col: &Column, x: usize, z: usize) -> i32 {
    (0..HEIGHT)
        .rev()
        .find(|&y| {
            let (id, data) = col.block(x, y, z);
            blocks::look(id, data).shape == Shape::Cube
        })
        .map_or(-1, |y| y as i32)
}

/// The three block materials, one per render pass (see `blocks::Layer`).
#[derive(Resource)]
pub struct BlockMaterials {
    pub opaque: Handle<StandardMaterial>,
    pub cutout: Handle<StandardMaterial>,
    pub translucent: Handle<StandardMaterial>,
}

/// Marks a mesh entity as part of a column's geometry.
#[derive(Component)]
pub struct ColumnPart(pub ChunkPos);

/// sRGB vertex colours to the linear space Bevy expects.
fn linear(c: [f32; 4]) -> [f32; 4] {
    let f = |v: f32| v.powf(2.2);
    [f(c[0]), f(c[1]), f(c[2]), c[3]]
}

fn to_bevy(m: MeshData) -> Mesh {
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, m.positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, m.normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, m.uvs)
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, m.colors.into_iter().map(linear).collect::<Vec<_>>())
        .with_inserted_indices(Indices::U32(m.indices))
}

/// Rebuilds the meshes of every column marked dirty.
pub fn remesh(
    mut commands: Commands,
    mut map: ResMut<WorldMap>,
    materials: Res<BlockMaterials>,
    mut meshes: ResMut<Assets<Mesh>>,
    parts: Query<(Entity, &ColumnPart, &Mesh3d)>,
) {
    if map.dirty.is_empty() {
        return;
    }
    let start = bevy::platform::time::Instant::now();
    let dirty = std::mem::take(&mut map.dirty);
    for (entity, part, mesh) in &parts {
        if dirty.contains(&part.0) {
            meshes.remove(mesh.id());
            commands.entity(entity).despawn();
        }
    }
    let mut quads = 0;
    for &pos in &dirty {
        let m = map.mesh_column(pos);
        for (pass, material) in [(m.opaque, &materials.opaque), (m.cutout, &materials.cutout), (m.translucent, &materials.translucent)] {
            quads += pass.indices.len() / 6;
            if !pass.is_empty() {
                commands.spawn((ColumnPart(pos), Mesh3d(meshes.add(to_bevy(pass))), MeshMaterial3d(material.clone())));
            }
        }
    }
    info!("meshed {} columns, {quads} quads in {:.1} ms", dirty.len(), start.elapsed().as_secs_f64() * 1000.0);
}

/// Keeps physics in step with block edits.
pub fn set_block(map: &mut WorldMap, solids: &mut Solids, x: i32, y: i32, z: i32, id: u8, data: u8) -> bool {
    let changed = map.set_block(x, y, z, id, data);
    if changed {
        solids.set_block(x, y, z, id, data);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every vertex of a column's mesh, to compare geometry.
    fn vertices(m: &mesh::ColumnMesh) -> Vec<[f32; 3]> {
        [&m.opaque, &m.cutout, &m.translucent].iter().flat_map(|p| p.positions.iter().copied()).collect()
    }

    #[test]
    fn editing_a_block_remeshes_its_column_and_the_neighbour_across_an_edge() {
        let mut map = WorldMap::new(crate::load_world());
        let mut solids = Solids::from_world(map.world());
        map.dirty.clear();

        // Standing room on top of the highest cube somewhere inside chunk (2, 2).
        let pos = ChunkPos { x: 2, z: 2 };
        let (x, y, z) = (1..15)
            .flat_map(|lx| (1..15).map(move |lz| (lx, lz)))
            .map(|(lx, lz)| (32 + lx, map.heights[&pos][(lz * 16 + lx) as usize], 32 + lz))
            .find(|&(x, y, z)| y > 0 && solids.is_free(Vec3::new(x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5)))
            .unwrap();
        let before = vertices(&map.mesh_column(pos));
        let feet_above = Vec3::new(x as f32 + 0.5, y as f32 + 1.0, z as f32 + 0.5);

        // Place a block on top: the column changes, and only it is dirty.
        assert!(set_block(&mut map, &mut solids, x, y + 1, z, 1, 0));
        assert_eq!(map.dirty.iter().copied().collect::<Vec<_>>(), [pos]);
        assert_ne!(vertices(&map.mesh_column(pos)), before);
        assert!(!solids.is_free(feet_above));
        assert_eq!(map.heights[&pos][((z - 32) * 16 + x - 32) as usize], y + 1);

        // Remove it again: back to the original geometry.
        assert!(set_block(&mut map, &mut solids, x, y + 1, z, 0, 0));
        assert_eq!(vertices(&map.mesh_column(pos)), before);
        assert!(solids.is_free(feet_above));

        // An edge block also dirties the neighbour; outside the world nothing changes.
        map.dirty.clear();
        assert!(map.set_block(2 * 16, 100, 2 * 16 + 3, 1, 0));
        assert_eq!(map.dirty.iter().copied().collect::<Vec<_>>(), [ChunkPos { x: 1, z: 2 }, pos]);
        assert!(!map.set_block(-100, 70, 0, 1, 0));
        assert!(!map.set_block(x, HEIGHT as i32, z, 1, 0));
    }
}
