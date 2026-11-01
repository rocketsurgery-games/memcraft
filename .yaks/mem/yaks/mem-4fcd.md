---
id: mem-4fcd
title: Basic object placement and removal
status: shaving
type: feature
priority: 3
created: '2026-10-31T23:19:11Z'
updated: '2026-11-01T02:00:57Z'
needs: human
---

We're not trying to reimplement minecraft game logic, just extend this viewer to allow for a simple creative-mode playground. To that end, we need the ability to remove and place blocks. Which also implies the ability to pick the current block. The most natural solution to this is to implement a very basic Minecraft-style toolbar, and a basic block/object search UI.

---
▸ 2026-11-01T01:17:10Z [herd]
kind: ask
needs: human
---

asked: needs human

Breakdown is in the child yaks; I'm doing the editable-world foundation (`mem-56d9`) myself as a prep commit. A few input and UI choices:

1. **Scroll wheel.** Minecraft uses it for the hotbar; today it changes fly speed. Lean: scroll = hotbar, speed moves to `+`/`-` (Ctrl+scroll zooms the page in browsers).
2. **Mouse buttons.** Lean: left = break, right = place, middle = pick-block, as in Minecraft; the click that locks the pointer does nothing else.
3. **UI toolkit.** Bevy UI (one code path for native and browser, but text entry is basic) or an HTML overlay (nicer search box, browser only). Lean: Bevy UI.
4. **Creative rules.** Instant break, no drops, unlimited blocks, any catalog block placeable, reach about 6 blocks walking and farther flying. Lean: yes.
5. **Orientation.** Logs, stairs, slabs, torches, doors placed by facing are a separate yak (`mem-b3b2`, p4), so first-pass placement uses the catalog's data value as-is. Lean: fine for a first pass.

---
▸ 2026-11-01T01:17:35Z [herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving

---
▸ 2026-11-01T01:18:11Z [herd]

Worker `cat-1` on `mem-8d69`: thread ksQQOn_lPggNQrKkBKNvXOzaN5LOADonT5IBxBArrcX3fBVIrK_YnmSnEPGN, default worker model, base `1277a16`.

---
▸ 2026-11-01T01:30:55Z [herd]

Worker `cat-1` died: its subthread hit the model spending limit ("Token-based spending limit reached"). It left an uncommitted 408-line `catalog.rs` draft in its worktree; the yakherd will review and finish it.

---
▸ 2026-11-01T01:59:31Z [herd]

Workers: `tgt-1` on `mem-6934`, `hot-1` on `mem-f4f7`; `shape-1` on `mem-12d5` (under `mem-f4c5`). All Delta subthreads, default worker model, base `8fc3164`.

---
▸ 2026-11-01T02:00:57Z [herd]

Worker threads (base `8fc3164`, default worker model):

- `shape-1` (`mem-12d5`): ksQQOn_lPggNQrKkBKNvXOzaN5LOADpPgpIBxBArrcX3fBVIrK_YnmSnEPGN
- `tgt-1` (`mem-6934`): ksQQOn_lPggNQrKkBKNvXOzaN5LOADpSnpIBxBArrcX3fBVIrK_YnmSnEPGN
- `hot-1` (`mem-f4f7`): ksQQOn_lPggNQrKkBKNvXOzaN5LOADpWzZIBxBArrcX3fBVIrK_YnmSnEPGN
