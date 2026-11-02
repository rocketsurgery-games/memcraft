---
id: mem-5d8c
title: 'Containers: open and view contents'
status: shaving
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-02T03:09:20Z'
parent: mem-f4c5
depends_on:
- mem-12d5
- mem-6934
---

Chests (including double chests), furnaces, dispensers, droppers, hoppers, brewing stands: right click opens a view of the block entity's `Items`.

Scope depends on the open question on the parent yak: view-only, or moving items between the container and the hotbar (which today holds blocks only, so items need icons and names first).

---
▸ 2026-11-01T03:39:06Z [herd]

Scope per Joel (mem-f4c5 answer 2): view-only first — open a chest/furnace/etc. UI showing its slots from the block entity. Item moves split out to mem-834e.

---
▸ 2026-11-02T02:53:40Z [chest-1]
kind: move
from: hairy
to: shaving
thread: ksQQOn_lPggNQrKkBKNvXOzaN5LOAD4BcZIBxBD-0Otq811F0oo4eGE3HRbr
---

moved: hairy -> shaving

---
▸ 2026-11-02T02:53:40Z [chest-1]

Started on chest-1, base 0c5f2b8. Plan: ContainerView message from targeting, container_view.rs (panel), CHEST drive stage.

---
▸ 2026-11-02T03:09:20Z [chest-1]
kind: checkpoint
commit: 5f8c67c96d45799f35a4fb2b59f85220781c5cd8
---

Checkpoint: containers.rs (slot reading, double-chest pairing, tests), container_view.rs (panel, Esc/E/Close), targeting hook, drive CHEST stage drafted. Unit tests pass (cargo test -p memcraft-viewer). Sample world has no double chest; chest at 93,77,110 has 11 stacks. Next: clippy native+wasm, web build, drive screenshot.
