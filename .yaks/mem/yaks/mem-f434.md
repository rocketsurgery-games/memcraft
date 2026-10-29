---
id: mem-f434
title: Host memcraft-viewer web build on GitHub Pages
status: hairy
type: task
priority: 3
created: '2026-10-29T02:36:14Z'
updated: '2026-10-29T12:08:48Z'
---

Publish the browser build of `memcraft-viewer` (world embedded in the wasm) on GitHub Pages for rocketsurgery-games/memcraft. Joel approved public hosting ("lock it down later if we have to") and chose our own build over mcraft.fun's `?map=` route.

## Work

- Add a Pages workflow: build with `crates/memcraft-viewer/web/build.sh` (needs the `wasm32-unknown-unknown` target and the `wasm-bindgen-cli` version from `Cargo.lock`), then upload `crates/memcraft-viewer/web/dist`.
- The repo is currently private. Pages needs it public, or an org plan that allows Pages on private repos. Check before wiring the workflow.
- Size: 26 MB wasm, 7.6 MB gzipped (Pages serves gzip). Consider `wasm-opt -Oz` in CI, since `build.sh` already runs it when present.
- Verify by loading the Pages URL in a browser and flying around.
