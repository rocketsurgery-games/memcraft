---
id: mem-a1f7
title: World server for hosted saves
status: shaving
type: task
priority: 4
created: '2026-11-01T01:16:34Z'
updated: '2026-11-02T04:37:48Z'
parent: mem-f5cf
depends_on:
- mem-a86b
---

A small server that serves the web build and one world, and accepts saves from the viewer, so a hosted instance can persist edits. The Pages deployment stays read-only ("ephemeral").

Shape depends on the open question on the parent yak.

---
▸ 2026-11-02T04:35:37Z [server-1]
kind: move
from: hairy
to: shaving
thread: ksQQOn_lPggNQrKkBKNvXOzaN5LOAD5zeJIBxBD-0Otq811F0oo4eGE3HRbr
---

moved: hairy -> shaving

---
▸ 2026-11-02T04:37:48Z [server-1]

server-1 on mem-a1f7, base af1c65a.

Decision: new crate crates/memcraft-server (axum + tower-http ServeDir), one world dir, no auth. Endpoints: GET /api/world (JSON probe: name), GET /api/world/files (level.dat + db/* packed), PUT /api/world/columns (edited columns), static web dist as fallback.

Decision: save sends only edited columns, not the whole world. The server re-reads the dir and puts the columns over it (same as native save), so two viewers editing different columns don't clobber each other; a whole-world upload would be last-writer-wins on everything. Wire = the columns' own LevelDB records (what encode_column already makes), decoded by the existing decode path, so malformed input is rejected by the same code that reads worlds. Rejected: JSON/zip of blocks (new format to keep in sync).

Decision: shared save code (backup_db, utc_stamp, 'put columns over a dir with backup') moves from viewer save.rs into mcpe-world; native save and the server both call it. Packing helpers live in mcpe-world too (wire module).

Decision: the page probes and fetches the world in index.html JS before init() (async fetch inside a wasm-bindgen-futures task would be fatal: winit's control-flow exception thrown inside a task stalls the task queue); the viewer reads it from window and POSTs saves from Rust via web-sys fetch.
