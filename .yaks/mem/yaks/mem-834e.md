---
id: mem-834e
title: 'Containers: take and place items'
status: shaving
type: feature
priority: 4
created: '2026-11-01T03:38:53Z'
updated: '2026-11-02T09:16:22Z'
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
