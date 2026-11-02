---
id: mem-e242
title: Render mobs and dropped items as simple boxes
status: shaving
type: feature
priority: 4
created: '2026-11-01T03:38:52Z'
updated: '2026-11-02T08:38:09Z'
parent: mem-f4c5
---

Static, no AI. Draw mobs (and optionally dropped items) from entity records as coloured boxes sized per mob type, or skip types we cannot size. Follow-up to the paintings/item-frames yak.

---
▸ 2026-11-02T08:32:20Z [mobs-1]
kind: move
from: hairy
to: shaving
thread: ksQQOn_lPggNQrKkBKNvXOzaN5LOAD7G0JIBxBD-0Otq811F0oo4eGE3HRbr
---

moved: hairy -> shaving

---
▸ 2026-11-02T08:38:09Z [mobs-1]

mobs-1 started: new module crates/memcraft-viewer/src/mobs.rs (mobs/minecarts as hitbox boxes + head box, dropped items as cubes), sample world census from entity ids (low byte). Building with CARGO_TARGET_DIR=/home/user/memcraft/target (shared, disk is tight).
