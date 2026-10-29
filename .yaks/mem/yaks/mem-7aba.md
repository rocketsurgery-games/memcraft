---
id: mem-7aba
title: Add open-source textures
status: shaving
type: task
priority: 3
created: '2026-10-29T11:59:27Z'
updated: '2026-10-29T17:19:38Z'
---

And make sure they get rendered. Let's find something as close as we can to the originals, while still being freely distributable.

---
▸ 2026-10-29T17:19:38Z [delta:Set up remote yak workflow]

moved: hairy -> shaving

---
▸ 2026-10-29T17:19:38Z [delta:Set up remote yak workflow]

Claim (yakherd: this thread). Owner: worker `tex-1`.

- Scope: texture atlas + UVs in `crates/memcraft-viewer/src/mesh.rs`, `blocks.rs` (per-face texture choice), material setup in `main.rs`, asset files under `crates/memcraft-viewer/assets/` with a LICENSE/attribution file, embedded at build time like the world.
- Out of scope: `controls.rs` and camera/movement (owned by `mem-f13a`).
- Evidence: license text of the chosen pack recorded; headless browser screenshots before/after from the same viewpoint; tests still pass; wasm size before/after.
- The pack choice is a design fork: worker records candidates and asks, but prototypes with its lean meanwhile.
