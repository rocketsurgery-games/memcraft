//! Sky and block light, computed from scratch.
//!
//! Pocket Edition sub-chunk records store no light, and Java clients and
//! servers that don't run a lighting engine (including the browser client we
//! target) render unlit chunks pitch black. So we light the whole world here
//! using Java's rules: a block passes on its light minus `max(1, opacity)`.

use std::collections::{HashMap, VecDeque};

use mcpe_world::{ChunkPos, HEIGHT};

use crate::blocks::{emission, opacity};

/// Per-block arrays use `mcpe_world::Column::index` order: y fastest, then z, then x.
fn index(x: usize, y: usize, z: usize) -> usize {
    (x * 16 + z) * HEIGHT + y
}

pub struct Lit {
    pub sky: Vec<u8>,
    pub block: Vec<u8>,
    /// Java heightmap: per (x, z) at `z * 16 + x`, the lowest y with full sky light above.
    pub height: [i32; 256],
}

/// Lights every column. `ids` holds translated Java block ids per column.
pub fn light(ids: &HashMap<ChunkPos, Vec<u8>>) -> HashMap<ChunkPos, Lit> {
    let positions: Vec<ChunkPos> = ids.keys().copied().collect();
    let slot: HashMap<ChunkPos, usize> = positions.iter().enumerate().map(|(i, p)| (*p, i)).collect();
    let cols: Vec<&[u8]> = positions.iter().map(|p| ids[p].as_slice()).collect();
    // Neighbour chunk slots: +x, -x, +z, -z.
    let neighbours: Vec<[Option<usize>; 4]> = positions
        .iter()
        .map(|p| {
            [(1, 0), (-1, 0), (0, 1), (0, -1)].map(|(dx, dz)| slot.get(&ChunkPos { x: p.x + dx, z: p.z + dz }).copied())
        })
        .collect();

    let mut sky: Vec<Vec<u8>> = vec![vec![0; 16 * 16 * HEIGHT]; cols.len()];
    let mut block: Vec<Vec<u8>> = vec![vec![0; 16 * 16 * HEIGHT]; cols.len()];
    let mut heights = vec![[0i32; 256]; cols.len()];

    let mut sky_queue = VecDeque::new();
    let mut block_queue = VecDeque::new();
    for (c, col) in cols.iter().enumerate() {
        for x in 0..16 {
            for z in 0..16 {
                // Straight down from the sky: full light until something absorbs it.
                let mut l = 15u8;
                let mut h = 0;
                for y in (0..HEIGHT).rev() {
                    let i = index(x, y, z);
                    let o = opacity(col[i]);
                    if o > 0 && h == 0 {
                        h = y as i32 + 1;
                    }
                    l = l.saturating_sub(o);
                    sky[c][i] = l;
                    if l > 1 {
                        sky_queue.push_back((c, x, y, z));
                    }
                    let e = emission(col[i]);
                    if e > 0 {
                        block[c][i] = e;
                        block_queue.push_back((c, x, y, z));
                    }
                }
                heights[c][z * 16 + x] = h;
            }
        }
    }
    spread(&cols, &neighbours, &mut sky, sky_queue);
    spread(&cols, &neighbours, &mut block, block_queue);

    positions
        .into_iter()
        .zip(sky.into_iter().zip(block).zip(heights))
        .map(|(p, ((sky, block), height))| (p, Lit { sky, block, height }))
        .collect()
}

/// Breadth-first flood fill from the queued blocks.
fn spread(
    cols: &[&[u8]],
    neighbours: &[[Option<usize>; 4]],
    light: &mut [Vec<u8>],
    mut queue: VecDeque<(usize, usize, usize, usize)>,
) {
    while let Some((c, x, y, z)) = queue.pop_front() {
        let l = light[c][index(x, y, z)];
        if l <= 1 {
            continue;
        }
        let mut visit = |c: usize, x: usize, y: usize, z: usize, queue: &mut VecDeque<_>| {
            let i = index(x, y, z);
            let next = l.saturating_sub(opacity(cols[c][i]).max(1));
            if next > light[c][i] {
                light[c][i] = next;
                queue.push_back((c, x, y, z));
            }
        };
        if y + 1 < HEIGHT {
            visit(c, x, y + 1, z, &mut queue);
        }
        if y > 0 {
            visit(c, x, y - 1, z, &mut queue);
        }
        match x {
            15 => {
                if let Some(n) = neighbours[c][0] {
                    visit(n, 0, y, z, &mut queue)
                }
            }
            _ => visit(c, x + 1, y, z, &mut queue),
        }
        match x {
            0 => {
                if let Some(n) = neighbours[c][1] {
                    visit(n, 15, y, z, &mut queue)
                }
            }
            _ => visit(c, x - 1, y, z, &mut queue),
        }
        match z {
            15 => {
                if let Some(n) = neighbours[c][2] {
                    visit(n, x, y, 0, &mut queue)
                }
            }
            _ => visit(c, x, y, z + 1, &mut queue),
        }
        match z {
            0 => {
                if let Some(n) = neighbours[c][3] {
                    visit(n, x, y, 15, &mut queue)
                }
            }
            _ => visit(c, x, y, z - 1, &mut queue),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn torch_in_a_sealed_box_crosses_chunk_borders() {
        // Two adjacent chunks, solid stone everywhere except a tunnel at y=10, z=8.
        let mut a = vec![1u8; 16 * 16 * HEIGHT];
        let mut b = vec![1u8; 16 * 16 * HEIGHT];
        for x in 0..16 {
            a[index(x, 10, 8)] = 0;
            b[index(x, 10, 8)] = 0;
        }
        a[index(14, 10, 8)] = 50; // torch, emits 14
        let ids = HashMap::from([(ChunkPos { x: 0, z: 0 }, a), (ChunkPos { x: 1, z: 0 }, b)]);
        let lit = light(&ids);
        let b = &lit[&ChunkPos { x: 1, z: 0 }];
        // Torch at x=14 -> 14; x=16 (next chunk's 0) is two steps away.
        assert_eq!(b.block[index(0, 10, 8)], 12);
        assert_eq!(b.sky[index(0, 10, 8)], 0, "sealed tunnel gets no sky light");
        assert_eq!(b.height[8 * 16], HEIGHT as i32);
    }

    #[test]
    fn open_sky_is_full_and_water_dims_it() {
        let mut col = vec![0u8; 16 * 16 * HEIGHT];
        for x in 0..16 {
            for z in 0..16 {
                col[index(x, 0, z)] = 7;
                for y in 1..5 {
                    col[index(x, y, z)] = 9; // water absorbs 3
                }
            }
        }
        let lit = light(&HashMap::from([(ChunkPos { x: 0, z: 0 }, col)]));
        let c = &lit[&ChunkPos { x: 0, z: 0 }];
        assert_eq!(c.sky[index(3, 10, 3)], 15);
        assert_eq!(c.sky[index(3, 4, 3)], 12);
        assert_eq!(c.sky[index(3, 1, 3)], 3);
        assert_eq!(c.height[3 * 16 + 3], 5);
    }
}
