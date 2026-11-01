---
id: mem-56d9
title: 'Editable world: runtime block store and per-column remeshing'
status: shorn
type: task
priority: 3
created: '2026-11-01T01:16:13Z'
updated: '2026-11-01T01:58:51Z'
parent: mem-4fcd
---

Foundation for every editing, interaction and save feature. Today `setup` in `crates/memcraft-viewer/src/main.rs` meshes the embedded world once and drops it; only physics' `Solids` copy survives.

## Scope

- Keep the world at runtime as a Bevy resource with `get`/`set_block(x, y, z, id, data)` (and block entities, for signs and containers later).
- Tag each column's mesh entities with their `ChunkPos`; a dirty set remeshes changed columns, plus the neighbour column when the change is on a column edge.
- Recompute that column's sky heights and update `Solids` in place.
- A shared `Hotbar` resource type (9 slots of `(id, data)`, selected index) so the targeting and hotbar workers build against one type.

## Evidence

- Unit test: `set_block` then remesh changes the column's quads and `Solids`.
- Native and wasm builds; drive.js still meshes 332,961 quads at startup.
- Remesh time for one column logged (should be a few ms).

Done by the yakherd as a prep commit, since every other yak in this family builds on its types.

---
▸ 2026-11-01T01:18:11Z [herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving

---
▸ 2026-11-01T01:18:11Z [herd]

Claim: yakherd (`herd`), prep commit in the yakherd checkout. Scope: `main.rs` setup/meshing, a new `world.rs` resource, `physics.rs` `Solids` setter, `mesh.rs` only if its signature must change. Evidence: unit test, native + wasm build, drive.js quad count.

---
▸ 2026-11-01T01:58:51Z [herd]

Landed as `4e25883` (pushed to origin/main).

- New [world.rs](../crates/memcraft-viewer/src/world.rs): `WorldMap` resource (owns the `PeWorld`, sky heights, a dirty set), `set_block`, `remesh` system, `BlockMaterials`, `ColumnPart(ChunkPos)` on mesh entities; `world::set_block` also updates `Solids`.
- Startup marks every column dirty, so the first frame meshes through the same `remesh` path.
- `physics::Solids::set_block` and a shared `fill` rule.
- New [hotbar.rs](../crates/memcraft-viewer/src/hotbar.rs): the shared `Hotbar` resource type (9 slots, default blocks, `selected_block`).
- The editing API carries `allow(dead_code)` with a reason until the targeting yak (`mem-6934`) uses it; that worker removes them.

Evidence:

- Unit test `editing_a_block_remeshes_its_column_and_the_neighbour_across_an_edge`: place/remove changes geometry and `Solids`, restores both; edge edits dirty the neighbour; out-of-world edits are refused.
- `cargo test --workspace` pass; clippy `-D warnings` native and wasm clean.
- Web build + drive.js: `meshed 256 columns, 332961 quads in 1498.3 ms` (same quad count as before; ~6 ms per column in headless SwiftShader); pointer lock lock/release/relock correct.
- A first browser run caught `std::time::Instant` panicking on wasm; fixed with `bevy::platform::time::Instant`.

---
▸ 2026-11-01T01:58:51Z [herd]
kind: move
from: shaving
to: shorn
---

moved: shaving -> shorn
