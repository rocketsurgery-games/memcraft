---
id: mem-15e6
title: Render paintings and item frames
status: shaving
type: feature
priority: 3
created: '2026-11-01T03:38:52Z'
updated: '2026-11-01T15:44:07Z'
parent: mem-f4c5
---

Draw painting and item-frame entities from the world's entity records (mcpe2java decodes them). Paintings: motif -> size + texture region from the painting atlas; item frames: frame quad plus the item's icon. Static only. Verify: count of entities drawn on the bundled world, screenshot of at least one painting.

---
▸ 2026-11-01T03:40:06Z [herd]
kind: claim
---

Claim (herd) for worker `paint-1`.

Scope:

- Draw painting and item-frame entities from the sample world. `mcpe_world` chunks expose `entities` and `block_entities` (Bedrock NBT); `crates/mcpe2java/src/entities.rs` shows how paintings (entity id 83) and item frames (a block entity in PE 0.x, tile 199) are decoded — the viewer cannot depend on mcpe2java (no Cargo.toml edits), so reimplement the small bits needed or move shared helpers only if trivial; record a `Decision:`.
- Paintings: motif -> size and texture. Textures live in `crates/memcraft-viewer/assets/textures` (see `textures-sources.txt` and ATTRIBUTION for the licence). If no suitably licensed painting art is available, draw sized framed quads with a flat colour per motif and record the `Decision:`.
- Item frames: the frame quad plus the held item's icon if one is easy to get from the atlas; otherwise just the frame.
- Static only; no interaction. Entities do not collide.

Judge: herd re-runs the evidence on main.

Evidence:

- A log line or test with the count of paintings and item frames drawn on the bundled world.
- `cargo test --workspace`, clippy native and wasm, `sh crates/memcraft-viewer/web/build.sh`.
- Headless screenshot showing at least one painting (find its position from the entity data and point the camera there; drive.js accepts KEYS, or add a URL/debug spawn option only if it already exists).

---
▸ 2026-11-01T03:40:06Z [herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving

---
▸ 2026-11-01T15:21:48Z [herd]

Runner was wiped again (~11h ago, found Sun Nov  1 15:21:48 UTC 2026); workers door-1/paint-1/sign-1 show no progress since 03:40Z and their runner checkouts are gone. Treating as dead pending Joel's go-ahead to respawn.

---
▸ 2026-11-01T15:33:28Z [delta:paint-2]

Worker paint-2 start: Sun Nov  1 15:33:28 UTC 2026. yaks path: /home/user/.local/share/yaks/farms/memcraft/.yaks/mem/yaks/mem-15e6.md. Base main 48ed523.

---
▸ 2026-11-01T15:44:07Z [delta:paint-2]

Progress ($(date -u) approx): bundled world has 157 paintings (44 with plain id 83 plus 113 whose id has flag bits above the low byte, 4194387) and 9 item frames (block entity ItemFrame + block 199), holding armour boots and horse armour. Painting Pos is the slab centre, Direction 0..3 = +z,-x,-z,+x facing. Wrote src/entities.rs (decode, meshes, spawn system), build.rs embeds assets/paintings atlas, 28 PP textures added to assets/textures. Now compiling/testing.
