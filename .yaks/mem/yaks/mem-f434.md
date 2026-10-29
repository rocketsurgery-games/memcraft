---
id: mem-f434
title: Host memcraft-viewer web build on GitHub Pages
status: hairy
type: task
priority: 3
created: '2026-10-29T02:36:14Z'
updated: '2026-10-29T16:55:47Z'
needs: human
---

Publish the browser build of `memcraft-viewer` (world embedded in the wasm) on GitHub Pages for rocketsurgery-games/memcraft. Joel approved public hosting ("lock it down later if we have to") and chose our own build over mcraft.fun's `?map=` route.

## Work

- Add a Pages workflow: build with `crates/memcraft-viewer/web/build.sh` (needs the `wasm32-unknown-unknown` target and the `wasm-bindgen-cli` version from `Cargo.lock`), then upload `crates/memcraft-viewer/web/dist`.
- The repo is currently private. Pages needs it public, or an org plan that allows Pages on private repos. Check before wiring the workflow.
- Size: 26 MB wasm, 7.6 MB gzipped (Pages serves gzip). Consider `wasm-opt -Oz` in CI, since `build.sh` already runs it when present.
- Verify by loading the Pages URL in a browser and flying around.

---
▸ 2026-10-29T16:55:47Z [delta:Set up remote yak workflow]

asked: needs human

`rocketsurgery-games/memcraft` is **private** (checked via `gh api`: `private: true`, `has_pages: false`, owner is an Organization). GitHub Pages on a private repo needs a paid org plan (Team/Enterprise), and even then the site itself is public unless it's Enterprise with access control.

Options:

1. Make the repo public, then enable Pages (GitHub Actions source).
2. Keep the repo private, if the org plan allows Pages on private repos (I can't see the plan with my token).
3. Publish the built `dist/` to a separate public repo (e.g. `rocketsurgery-games/memcraft-web`) from a workflow here. That keeps the source private and needs a deploy token secret.

Lean: whichever of 1/2 you prefer; I'll write the workflow (`.github/workflows/pages.yml`, build with `web/build.sh` + `wasm-opt -Oz`) so it works for both, and not flip any repo settings myself. Which one?
