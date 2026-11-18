---
id: mem-834e
title: 'Containers: take and place items'
status: shorn
type: feature
priority: 4
created: '2026-11-01T03:38:53Z'
updated: '2026-11-02T09:29:41Z'
parent: mem-f4c5
depends_on:
- mem-5d8c
---

Follow-up to view-only containers (mem-5d8c). Moving items between a container and the hotbar means the hotbar holds items as well as blocks (icons and names for ~200 items) and edits are written to the block entity.

---
▸ 2026-11-02T08:32:33Z [items-1]
kind: move
from: hairy
to: shaving
thread: ksQQOn_lPggNQrKkBKNvXOzaN5LOAD7ILpIBxBD-0Otq811F0oo4eGE3HRbr
---

moved: hairy -> shaving

---
▸ 2026-11-02T08:48:31Z [items-1]

Checkpoint: items.rs (Stack, icon, name, exchange/send), hotbar of stacks, container window clicks + set_stack write-back, item names. Next: run tests, drive stage, clippy, wasm build.

![chest-open](mem-834e/chest-open.png)

![chest-hover](mem-834e/chest-hover.png)

![chest-took](mem-834e/chest-took.png)

---
▸ 2026-11-02T09:16:36Z [items-1]

Decision: a hotbar slot is an `items::Stack` (id, damage, count) shared with containers; Hotbar::selected_block() is Some only for a block the viewer draws, so right click with any other item places nothing. Rejected: a Block|Item enum (two shapes for one thing, and the stack from a chest would need converting).

---
▸ 2026-11-02T09:16:37Z [items-1]

Decision: placing does not consume. Picker/pick-block put a stack of 1; counts only come from containers and only show when >1. Rejected: decrementing on place (this is a viewer, a creative-like hotbar; consuming would empty slots with no way to refill but a chest).

---
▸ 2026-11-02T09:16:37Z [items-1]

Decision: clicks. Container slot: swap with the selected hotbar slot (same item merges into the hotbar stack up to 64, rest stays). Shift-click: send to hotbar (same-item stacks with room first, then first empty slot). Hotbar slot click: send to the container the same way. 1-9 choose the selected slot inside the window (the pointer is free so hotbar's own key system is off). No splitting, no right-click, no hover+number swap. Non-stackable tools merge up to 64 too (spec: simple merge).

---
▸ 2026-11-02T09:16:37Z [items-1]

Decision: write-back goes through containers::set_stack: rewrites only the one slot's entry in Items (Count byte, Damage short, Slot byte, id short), keeps key order, other slots' entries (even ones the reader skips) and, for the same item, its other tags (name, enchants). A container block with no entity (trapped chest) gets a fresh one. Double chest slots carry their own cell+slot number. Known loss: a stack moved to the hotbar forgets its NBT tags.

---
▸ 2026-11-02T09:21:05Z [items-1]

Summary: hotbar slots are item stacks (new items.rs: Stack, icon, name, exchange/send; item_icons.rs now has ~150 PE names; entities::held_icon uses items::icon). The container window shows the hotbar, names the hovered item, and clicks move stacks (take/swap/merge, shift-click, put, 1-9 select); every change is written via containers::set_stack -> WorldMap::set_block_entity, so save/zip/server pick it up (test: edits_survive_the_world_writer). Evidence: cargo test --workspace ok (173 viewer tests), clippy native --workspace --all-targets and wasm clean, web/build.sh builds, drive CHEST stage (CAM=90.5,77.8,110.5,270,5 CHEST=1, NODE_PATH=~/tools/drive/node_modules node crates/memcraft-viewer/web/drive.js crates/memcraft-viewer/web/dist ~/shots1) takes the eggs into hotbar slot 0, 0 block edits; screenshots attached. Commit 2328c93. Gaps: no split/right-click; tools merge to 64; moved stacks lose NBT tags; items without an icon are invisible in the hotbar; no per-container slot rules (furnace output accepts items).

---
▸ 2026-11-02T09:29:41Z [herd-memcraft]

Landed as 76433fd (squash of items-1 2328c93 on top of fb63185 mobs; main.rs auto-merged). Verify: workspace tests, clippy native+wasm, web build OK; CHEST drive moved eggs into hotbar, 0 block edits.

---
▸ 2026-11-02T09:29:41Z [herd-memcraft]
kind: move
from: shaving
to: shorn
thread: ksQQOn_lPggNQrKkBKNvXOzaN5LOADYRe5IBxBCA0aLJyrhJPreUvVwLftHG
---

moved: shaving -> shorn
