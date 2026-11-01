---
id: mem-f4c5
title: Object interactions
status: shaving
type: feature
priority: 3
created: '2026-10-31T23:23:03Z'
updated: '2026-11-01T01:59:30Z'
needs: agent
---

Some starting points (I'm sure we need more than this):
- open/close doors
- read/write signs
- take/place objects
- ...

Let's start this yak by enumerating all the myriad other cases I'm not thinking of.

---
▸ 2026-11-01T01:17:10Z [herd]

Enumeration of interactions, grouped by what they need. Child yaks exist for the ones marked **(yak)**.

## Prerequisite

- **Real block shapes** (yak `mem-12d5`): doors, gates, trapdoors, fences, panes, stairs and signs draw as cubes or slabs today, and collide as whole blocks.

## Toggles (block data only, no game logic)

- **Doors, trapdoors, fence gates** (yak `mem-1457`), iron ones included since there is no redstone.
- Levers and buttons (lever flips; a button presses and pops back).
- Redstone lamps, repeaters (delay setting), comparators (mode), daylight sensor (invert).
- Note blocks (pitch), jukebox (insert/eject a disc).

## Text and contents (block entities)

- **Signs** (yak `mem-c910`): render text, edit text.
- **Containers** (yak `mem-5d8c`): chests (incl. double), furnaces, dispensers, droppers, hoppers, brewing stands.
- Flower pots: put in or take out a plant.
- Item frames: put in, rotate, take out an item (needs entity rendering first).
- Enchanting table, anvil, crafting table: open a UI. Probably out of scope (crafting is game logic).

## Small state changes

- Cake (eat a slice), crops (bone meal to grow), cauldron (fill/empty), composting-style things.
- Beds (sleep would need day/night; probably just look).
- TNT (ignite? would need explosions: out), fire (flint and steel), buckets placing water/lava (no flow simulation).

## Entities (the viewer draws none today)

- Render paintings, item frames, mobs and dropped items from the world's entity records (mcpe2java now decodes them).
- Interactions on them: ride minecarts/boats, rotate frames, name tags. Probably later.

## Simulation (out of scope for a viewer?)

- Redstone propagation, liquid flow, falling sand/gravel, plant growth, mob AI, day/night.

---
▸ 2026-11-01T01:17:10Z [herd]
kind: ask
needs: human
---

asked: needs human

Which of the enumerated interactions belong in the first pass?

1. **First set.** Lean: shapes, then doors/gates/trapdoors, levers/buttons, signs (render + edit) and containers. Everything else waits.
2. **Containers.** View-only first, or take/place items? Taking items means the hotbar must hold items as well as blocks (icons and names for ~200 items). Lean: view-only first, item moves as a follow-up.
3. **Entities.** Render paintings, item frames and mobs as a separate yak now (static, no AI)? Lean: yes, paintings and item frames first; mobs as simple boxes or skipped.
4. **Simulation** (redstone, liquids, falling blocks, growth): confirm out of scope. Lean: out.

---
▸ 2026-11-01T01:24:04Z [Joel Webber]
kind: answer
needs: agent
---

1. Sounds good. Let's file yaks for any we don't choose to do out of the gate.
2. View-only is fine to start.
3. :thumbs-up:
4. Yeah, out of scope. We can file a low-pri yak just to track the ideas.

---
▸ 2026-11-01T01:59:30Z [herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving
