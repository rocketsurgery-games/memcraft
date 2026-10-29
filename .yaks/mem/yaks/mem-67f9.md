---
id: mem-67f9
title: 'mcpe2java: convert item frames, entities, piston heads'
status: shorn
type: task
priority: 4
created: '2026-10-29T02:36:14Z'
updated: '2026-10-29T18:49:00Z'
---

Currently dropped: 9 item frames (PE block 199 + ItemFrame block entity → Java ItemFrame entity with Facing), mobs/items (157 of type 83 = paintings? check), PistonArm, Cauldron. Low priority; the world reads fine without them.

---
▸ 2026-10-29T18:49:00Z [delta:Web Deploy v0]

Done by yakherd. Sample world before: skipped ItemFrame 9, PistonArm 9, Cauldron 1, NetherReactor 381; no entities written. After: 265 entities (9 item frames with items + 256 PE entities minus 1 dropped item with no Java id), only NetherReactor skipped (core already becomes iron block). Item frames: PE data 0..3 = E/W/S/N (checked: data 2 frames have their wall at z-1) -> Java Facing. Paintings: PE TileX/Y/Z is the wall block; Java's = PE + facing normal (checked vs Pos). Entities: mobs/animals, items, paintings, minecarts, boats -> Java 1.12 ids (type = low byte of PE id). Extended pistons (PistonArm State 2): base data |8, head data facing|8 if sticky. Cauldron fill 0..6 -> 0..3. Enchantment ids renumbered PE->Java (was copied verbatim before, a latent bug for chests too); horse armor 417-419 added (leather 416 has no 1.12 item -> frame empty). Evidence: 3 new sample-world tests (frames+items+ench, entities+painting tile, piston bits) + cauldron unit test; cargo test --workspace pass; clippy -D warnings native + wasm clean. Not verified: loading in a real Java 1.12 client.

---
▸ 2026-10-29T18:49:00Z [delta:Web Deploy v0]

moved: hairy -> shorn
