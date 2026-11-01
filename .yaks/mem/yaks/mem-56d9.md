---
id: mem-56d9
title: 'Editable world: runtime block store and per-column remeshing'
status: shaving
type: task
priority: 3
created: '2026-11-01T01:16:13Z'
updated: '2026-11-01T01:18:11Z'
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
