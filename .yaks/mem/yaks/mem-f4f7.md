---
id: mem-f4f7
title: Hotbar UI
status: hairy
type: task
priority: 3
created: '2026-11-01T01:16:13Z'
updated: '2026-11-01T01:16:13Z'
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
