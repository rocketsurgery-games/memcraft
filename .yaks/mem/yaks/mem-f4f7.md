---
id: mem-f4f7
title: Hotbar UI
status: shaving
type: task
priority: 3
created: '2026-11-01T01:16:13Z'
updated: '2026-11-01T01:59:31Z'
parent: mem-4fcd
depends_on:
- mem-56d9
---

A Minecraft-style hotbar: 9 slots at the bottom centre with block icons, a highlight on the selected slot, the selected block's name shown briefly on change.

## Scope

- Renders the shared `Hotbar` resource; number keys 1-9 select. Scroll wheel too, if the input question on the parent yak goes that way.
- Icons from the atlas tile of each block (catalog, or `blocks::look` until the catalog lands).
- Sensible default slots (stone, dirt, grass, planks, log, glass, cobblestone, brick, torch).

## Evidence

Screenshot via drive.js with a non-default slot selected; works in the browser and native builds.

---
▸ 2026-11-01T01:59:30Z [herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving

---
▸ 2026-11-01T01:59:31Z [herd]

Claim: worker `hot-1` (Delta subthread), base `8fc3164`.

- Scope: `hotbar.rs` (keep the `Hotbar` type's public shape: `slots`, `selected`, `selected_block`) and its plugin registration in `main.rs`. Icons via `catalog` (`Entry::icon`, names) and the atlas.
- Number keys 1-9 only. Scroll is pending Joel's answer on `mem-4fcd`: don't touch `controls.rs`.
- Out of scope: targeting/placing (`tgt-1`), the picker.
- Evidence: drive.js screenshot with a non-default slot selected (keys via `KEYS`), unit test for selection.
- Judge: yakherd at landing.
