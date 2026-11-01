---
id: mem-12d5
title: 'Block shapes: doors, trapdoors, gates, fences, panes, stairs, signs'
status: shaving
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-01T02:00:25Z'
parent: mem-f4c5
---

Prerequisite for interactions: today doors are drawn as full cubes, stairs and signs as half slabs, and physics treats them all as whole blocks, so an open door would look and feel exactly like a closed one.

## Scope

- A box-list shape in `blocks.rs`/`mesh.rs` (one or more axis-aligned boxes per block, from id and data): doors (open/closed, hinge, facing), trapdoors, fence gates, fences and walls, glass panes and iron bars (connecting to neighbours), stairs, standing and wall signs, ladders on their wall, levers/buttons.
- Physics (`physics.rs` `Solids`) collides with the same boxes, so open doors and gates are passable and fences are 1.5 high.
- PE 0.16 data values.

## Evidence

- Unit tests: box lists for a closed/open door and a stairs block; a walk test through an open door.
- drive.js screenshots of a door, fence and stairs in the sample world, before and after.

---
▸ 2026-11-01T01:59:30Z [herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving

---
▸ 2026-11-01T01:59:30Z [herd]

Claim: worker `shape-1` (Delta subthread), base `8fc3164`.

- Scope: `blocks.rs` (a box-list shape and the blocks that use it), `mesh.rs` (meshing boxes), `physics.rs` (`fill`/`Solids` collide with the same boxes; may change `Solids`' representation).
- Out of scope: `world.rs`, `controls.rs`, any input or UI, interactions (toggling doors is `mem-1457`).
- Evidence: unit tests named in the yak, existing physics tests still pass, drive.js screenshots of a door, fence and stairs before/after, quad count and wasm size before/after.
- Judge: yakherd at landing.

---
▸ 2026-11-01T02:00:25Z [shape-1]

Started 2026-11-01T02:00:21Z (`date -u`).

`yaks path mem-12d5`: `/home/user/.local/share/yaks/farms/memcraft/.yaks/mem/yaks/mem-12d5.md`. Worktree `/home/user/memcraft-3`, base `8fc3164`.
