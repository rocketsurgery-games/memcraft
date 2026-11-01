---
id: mem-8d69
title: 'Block catalog: names and placeable variants'
status: shaving
type: task
priority: 3
created: '2026-11-01T01:16:13Z'
updated: '2026-11-01T01:18:15Z'
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

---
▸ 2026-11-01T01:17:35Z [herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving

---
▸ 2026-11-01T01:17:35Z [herd]

Claim: worker `cat-1` (Delta subthread), base `1277a16`.

- Scope: a new `crates/memcraft-viewer/src/catalog.rs` plus its `mod` line in `main.rs`. Read `blocks::look`; don't change it.
- Out of scope: hotbar, picker, any UI.
- Evidence: unit tests (every entry draws, unique names), category counts in the summary note.
- Judge: yakherd at landing.

---
▸ 2026-11-01T01:18:15Z [cat-1]

start: Sun Nov  1 01:18:12 UTC 2026; yaks path: /home/user/.local/share/yaks/farms/memcraft/.yaks/mem/yaks/mem-8d69.md
