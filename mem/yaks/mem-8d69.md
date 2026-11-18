---
id: mem-8d69
title: 'Block catalog: names and placeable variants'
status: shorn
type: task
priority: 3
created: '2026-11-01T01:16:13Z'
updated: '2026-11-01T01:58:51Z'
parent: mem-4fcd
---

A table of every block the viewer can place: `(id, data, name)` plus the atlas tile to show as its icon, in Minecraft's creative-inventory order where practical (building blocks, decoration, plants, redstone, ...).

Used by the hotbar (icons, names) and the block picker (search).

## Scope

- New module in `crates/memcraft-viewer/src/` (e.g. `catalog.rs`); may read `blocks::look` but must not change it.
- Variants by data value: wool colours, planks, logs, stone kinds, flowers, slabs, stairs, ...
- Skip technical blocks (piston heads, double slabs, lit furnace, portal, ...).

## Evidence

- Test: every entry's `look` is not `Air` and names are unique.
- A count by category in the summary note.

---
▸ 2026-11-01T01:17:35Z [herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving

---
▸ 2026-11-01T01:17:35Z [herd]

Claim: worker `cat-1` (Delta subthread), base `1277a16`.

- Scope: a new `crates/memcraft-viewer/src/catalog.rs` plus its `mod` line in `main.rs`. Read `blocks::look`; don't change it.
- Out of scope: hotbar, picker, any UI.
- Evidence: unit tests (every entry draws, unique names), category counts in the summary note.
- Judge: yakherd at landing.

---
▸ 2026-11-01T01:18:15Z [cat-1]

start: Sun Nov  1 01:18:12 UTC 2026; yaks path: /home/user/.local/share/yaks/farms/memcraft/.yaks/mem/yaks/mem-8d69.md

---
▸ 2026-11-01T01:39:44Z [cat-1]

Decision: names/data follow what blocks::look draws, not Java names. Rejected: listing every data value look ignores (red sandstone data 1/2 -> one entry; slab 44:2 fake wood slab omitted, wood slabs are 158).

---
▸ 2026-11-01T01:39:44Z [cat-1]

Decision: skipped glass panes (102, 160) and dropper (125): look() draws panes as full glass cubes (duplicates of Glass/Stained Glass 241) and draws id 125 as planks (it treats 125 as the double wooden slab; mcpe2java says PE 125 is the dropper). blocks.rs not touched; follow-up for whoever owns look().

---
▸ 2026-11-01T01:39:44Z [cat-1]

Decision: orientation data is a fixed default (stairs/doors/gates 0, furnace/dispenser 3 so the front is the +z icon face, pistons data 1, torches 5). Rejected: one entry per facing (4x entries); placement facing is the hotbar/picker's concern.

---
▸ 2026-11-01T01:39:44Z [cat-1]

Decision: crops listed fully grown (wheat 59:7, carrots 141:7, potatoes 142:7, beetroot 244:7, nether wart 115:3) for a recognisable icon; double plants only as lower halves 175:0..5; stems, fire, cocoa, flower-less technical blocks omitted.

---
▸ 2026-11-01T01:39:44Z [cat-1]

Decision: icon() = look().tiles[4] (+z side), top (tiles[2]) for ids 2,60,92,110,116,198,243. Flat-coloured blocks (chests, bed, anvil, redstone dust...) yield WHITE; caller tints with look().color. Rejected: adding an icon field per entry.

---
▸ 2026-11-01T01:39:44Z [cat-1]

Decision: included Water (9), Lava (11), Bedrock, spawner, mushroom blocks, stonecutter/glowing obsidian/reactor core under Misc, since look() draws them; rejected leaving them out as not obtainable in survival. Catalog is a LazyLock<Vec<Entry>> built by a small Builder (static names, no allocation per name), not a literal 287-row table, so the dyed colours need no repetition.

---
▸ 2026-11-01T01:39:44Z [cat-1]

Evidence: cargo test -p memcraft-viewer: 24 passed, 0 failed (incl. catalog: every_entry_is_drawn, no_entry_uses_the_unknown_block_colour, names_are_unique, blocks_are_unique, data_fits_four_bits, categories_are_grouped_in_order, counts_by_category). cargo clippy -p memcraft-viewer --all-targets -- -D warnings: clean. cargo clippy -p memcraft-viewer --target wasm32-unknown-unknown -- -D warnings: clean. Counts by category: Building 122, Decoration 78, Plants 42, Redstone 35, Misc 10, total 287.

---
▸ 2026-11-01T01:39:44Z [cat-1]

Summary: new crates/memcraft-viewer/src/catalog.rs (Category, Entry{id,data,name,category}, Entry::look()/icon(), catalog::all()) plus 'mod catalog;' in main.rs with #![allow(dead_code)] inside catalog.rs. blocks.rs untouched.

---
▸ 2026-11-01T01:39:44Z [cat-1]
kind: move
from: shaving
to: shorn
---

moved: shaving -> shorn

---
▸ 2026-11-01T01:39:47Z [cat-1]
kind: checkpoint
branch: wip/cat-1
commit: 292326cb3f6770e798aa809ebe4514f4e84fc5e1
---

checkpoint

---
▸ 2026-11-01T01:56:40Z [herd]
kind: landed
commit: 918ad0c3f880df046982f3c25500cd9db5523416
source: 292326cb3f6770e798aa809ebe4514f4e84fc5e1
branch: main
---

landed: 918ad0c mem-8d69: block catalog with names, categories and icons

---
▸ 2026-11-01T01:58:51Z [herd]

Yakherd: landed as `918ad0c` on top of `4e25883`; re-ran `cargo test -p memcraft-viewer` (25 pass) and clippy `-D warnings` (clean). Follow-up for whoever owns `blocks.rs`: `look` draws PE 125 (dropper) as planks, and doors/gates of every wood as oak.
