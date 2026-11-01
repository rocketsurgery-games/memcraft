---
id: mem-5d8c
title: 'Containers: open and view contents'
status: hairy
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-01T03:39:06Z'
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
