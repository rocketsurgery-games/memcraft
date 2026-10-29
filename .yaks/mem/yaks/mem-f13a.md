---
id: mem-f13a
title: Add walk mode
status: shaving
type: feature
priority: 3
created: '2026-10-29T12:01:49Z'
updated: '2026-10-29T18:29:25Z'
---

This is a decent sized chunk of work, and the first crack at collision detection. So let's take the time to build it carefully, using existing bevy crates where helpful. Consider carefully whether it's worth doing "real" physics, or hacking something together the way the original MC did.

If one of the bevy physics crates can be used largely OOTB, without being obscenely expensive, perhaps that would be better than playing "physics whack-a-mole" with a more hand-crafted solution.

---
▸ 2026-10-29T17:19:38Z [delta:Set up remote yak workflow]

moved: hairy -> shaving

---
▸ 2026-10-29T17:19:38Z [delta:Set up remote yak workflow]

Claim (yakherd: this thread). Owner: worker `walk-1`.

- Scope: `crates/memcraft-viewer/src/controls.rs` (extend the controller with a walk mode and a fly/walk toggle), a new collision/physics module in the viewer, `main.rs` only to register it, `Cargo.toml` if a physics crate is adopted, README controls line.
- Out of scope: `blocks.rs`, `mesh.rs` rendering (owned by `mem-7aba`), except reading `blocks::look` shapes for solidity.
- Evidence: unit tests for collision (falling onto ground, not walking through a wall, stepping/jumping up one block), headless browser run showing the player standing on terrain and walking, wasm size before/after.
- Judge: the yakherd at landing. The physics-crate-vs-hand-rolled choice is a `Decision:` note with numbers.

---
▸ 2026-10-29T17:22:17Z [walk-1]

Started: 2026-10-29T17:20Z (`date -u`, walk-1).
`yaks path mem-f13a`: /home/user/.local/share/yaks/farms/memcraft/.yaks/mem/yaks/mem-f13a.md

Plan: (1) measure physics-crate cost vs hand-rolled (baseline wasm size/build time first), (2) Decision note, (3) implement in new `physics.rs` + `controls.rs`, tests, browser run.

---
▸ 2026-10-29T17:54:52Z [walk-1]

Decision: hand-rolled Minecraft-style AABB-vs-voxel sweep, not a physics crate. Rejected: avian3d 0.7.0 (spiked), bevy_rapier3d 0.36.0 (not built; same shape of problem). Real numbers (17:3x-17:5x Z, this 8-core runner, wasm-release profile, same Bevy 0.19.1):

- Compatibility: crates.io deps show avian3d 0.7.0 -> bevy ^0.19.0 and bevy_rapier3d 0.36.0 -> bevy ^0.19.0 (+rapier3d 0.35); avian3d 0.8 / bevy_rapier3d 0.37 / bevy_tnua 0.33 need Bevy 0.20 (not buildable on our rustc 1.95-era pin). So both work with 0.19.1, avian with default-features=false features 3d,f32,parry-f32. I built a working spike (static `Collider::voxels` of the whole world + kinematic cuboid + `MoveAndSlide`) and it compiled first try.
- Wasm size: baseline 22,250,036 B (after wasm-opt -Oz); with avian3d + spike 25,793,246 B = +3.54 MB (+15.9%) for a feature that needs ~300 lines. Cargo.lock +579 lines (obvhs, parry3d, glamx, bevy_heavy, ...). Cold wasm-release build 7m35 -> 8m16 (+~40 s; bevy was rebuilt both times because avian turns on extra bevy features).
- Feeding it the terrain: world is 256 columns, 4,362,243 full cubes. Per-block colliders is a non-starter. Whole-world `Collider::voxels`: all 4.36M cubes build in 236 ms native release (surface-only: 232,668 voxels, 31 ms + 528 ms to collect them, which is slow because it queries 6 neighbours per block via World::block). wasm is typically 2-4x slower, so ~0.1-1 s of added startup, plus memory for the BVH/voxel grid on top of our meshes. A trimesh of the ~300k quads would be worse (BVH over 600k triangles).
- What it does NOT give: voxel colliders are full cubes only, so slabs/snow/carpet (`Shape::Partial`) would be solid cubes or non-solid; no "unloaded column = wall" (player walks off the edge and falls forever), and no auto step-up (MoveAndSlide slides only; step-up, ground detection and jump still hand-written, i.e. the whack-a-mole is only moved, not removed). Also a full solver pipeline (PhysicsPlugins, spatial-query BVH sync in FixedPostUpdate) runs for one kinematic body.
- Why hand-rolled wins: the world already IS a grid; solidity is one lookup `blocks::look(id,data).shape`. Sweeping a 0.6x1.8x0.6 box one axis at a time (Y, X, Z, MC order) touches ~10-30 cells per call, can't tunnel at any speed, handles Partial heights exactly, and makes chunk-boundary behaviour trivial (one lookup function, no seams between colliders). Cost: 0 added dependencies, ~1 byte/block (4 MB RAM for 256 columns... see physics.rs `Solids`) and a few hundred lines + unit tests.
- Would reconsider a crate if we later need dynamic bodies, mobs-vs-mobs, projectiles or non-axis-aligned shapes.

![2-walk-landed](mem-f13a/2-walk-landed.png)

![3-walk-moved](mem-f13a/3-walk-moved.png)
