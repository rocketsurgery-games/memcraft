---
id: mem-6ad1
title: 'Levers and buttons: flip and press'
status: shaving
type: feature
priority: 3
created: '2026-11-01T03:38:52Z'
updated: '2026-11-01T23:57:28Z'
parent: mem-f4c5
depends_on:
- mem-6934
---

Right-clicking a lever flips its data bit (on/off); a button sets its pressed bit and pops back after ~1s (stone 20 ticks, wood 30). No redstone propagation (out of scope, see simulation yak). Needs lever/button shapes in blocks::shape if they still draw as cubes. Interaction takes precedence over placing when the target is interactable (same rule as mem-1457). Verify: unit test for the data-bit flip; headless drive showing the lever changes on click.

---
▸ 2026-11-01T23:40:27Z [lever-1]
kind: move
from: hairy
to: shaving
thread: ksQQOn_lPggNQrKkBKNvXOzaN5LOAD1VZJIBxBD-0Otq811F0oo4eGE3HRbr
---

moved: hairy -> shaving

---
▸ 2026-11-01T23:57:28Z [lever-1]

Checkpoint b9d91c7 (local, not pushed).

- Lever (69) and button (77 stone, 143 wood) shapes already exist in blocks::model, so no shape work.
- interact.rs: lever flips bit 8; button sets bit 8 and swallows clicks while down.
- New buttons.rs: PendingReleases (tick bookkeeping driven by elapsed seconds, 20 ticks/s) + release_buttons system; targeting::edit_blocks schedules a release on a press.
- Unit tests pass (cargo test -p memcraft-viewer: 80).

Heads-up for the herd: HEAD a48f18b has the same tree as 1569374 (menu), so the paintings/item-frame changes of 7bb9ab7 are only uncommitted in my worktree. I committed only my own files/hunks.

Next: clippy native+wasm, web build, Playwright drive.
