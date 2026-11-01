---
id: mem-8d69
title: 'Block catalog: names and placeable variants'
status: hairy
type: task
priority: 3
created: '2026-11-01T01:16:13Z'
updated: '2026-11-01T01:16:13Z'
parent: mem-4fcd
---

A table of every block the viewer can place: `(id, data, name)` plus the atlas tile to show as its icon, in Minecraft's creative-inventory order where practical (building blocks, decoration, plants, redstone, ...).

Used by the hotbar (icons, names) and the block picker (search).

## Scope

- New module in `crates/memcraft-viewer/src/` (e.g. `catalog.rs`); may read `blocks::look` but must not change it.
- Variants by data value: wool colours, planks, logs, stone kinds, flowers, slabs, stairs, ...
- Skip technical blocks (piston heads, double slabs, lit furnace, portal, ...).

## Evidence

- Test: every entry's `look` is not `Air` and names are unique.
- A count by category in the summary note.
