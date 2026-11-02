---
id: mem-b3b2
title: Placement orientation for directional blocks
status: shaving
type: task
priority: 4
created: '2026-11-01T01:16:13Z'
updated: '2026-11-02T02:59:19Z'
parent: mem-4fcd
depends_on:
- mem-6934
---

Placed blocks take their orientation from the face clicked and the player's facing, as in Minecraft: log axis, stairs facing and upside-down, top/bottom slabs, torches on walls, furnaces/chests/pumpkins facing the player, two-block doors and beds, ladders and signs on walls.

Data values follow PE 0.16 (see `mcpe2java/src/blocks.rs` for where PE differs from Java).

---
▸ 2026-11-02T02:53:46Z [orient-1]
kind: move
from: hairy
to: shaving
thread: ksQQOn_lPggNQrKkBKNvXOzaN5LOAD4CsJIBxBD-0Otq811F0oo4eGE3HRbr
---

moved: hairy -> shaving

---
▸ 2026-11-02T02:59:19Z [orient-1]

Checkpoint: new src/orient.rs (pure rule, per-family tests), targeting::placement/hit_height, top slabs drawn (blocks.rs), purpur slab 205 now a slab, Sign catalog entry. Tests still compiling. Not yet run: cargo test/clippy/web build, drive ORIENT stage.
