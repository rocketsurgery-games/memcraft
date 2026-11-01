---
id: mem-b3b2
title: Placement orientation for directional blocks
status: hairy
type: task
priority: 4
created: '2026-11-01T01:16:13Z'
updated: '2026-11-01T01:16:13Z'
parent: mem-4fcd
depends_on:
- mem-6934
---

Placed blocks take their orientation from the face clicked and the player's facing, as in Minecraft: log axis, stairs facing and upside-down, top/bottom slabs, torches on walls, furnaces/chests/pumpkins facing the player, two-block doors and beds, ladders and signs on walls.

Data values follow PE 0.16 (see `mcpe2java/src/blocks.rs` for where PE differs from Java).
