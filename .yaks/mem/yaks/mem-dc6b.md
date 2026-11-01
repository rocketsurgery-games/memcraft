---
id: mem-dc6b
title: 'Native: load a world dir from the command line, save in place with backup'
status: shaving
type: task
priority: 3
created: '2026-11-01T16:25:13Z'
updated: '2026-11-01T16:25:34Z'
parent: mem-f5cf
depends_on:
- mem-a86b
---

Per Joel (mem-f5cf answer 4): the native viewer takes an optional world directory argument (embedded sample world stays the default) and can save edits back in place via mcpe-world's WorldWriter, keeping a backup copy of the previous db/ first. Save trigger can be a key until the menu's Save hook (mem-d6f2) exists; mem-5219 then wires the menu. Track which columns changed in WorldMap. Evidence: edit, save, restart with the dir, edit is visible; backup dir exists; mcpe2java converts the saved world.

---
▸ 2026-11-01T16:25:34Z [delta:herd]
kind: claim
---

Claim (delta:herd) for worker `delta:native-1`.

Scope:

- Native only (`cfg(not(target_arch = "wasm32"))`): optional world-dir CLI argument; without it the embedded sample world loads as today. Find how the world is loaded now (`build.rs` embeds it; `world.rs`/`main.rs` read it) and keep one code path for "a set of world files", whichever source.
- `WorldMap` records which columns were edited (`set_block` marks them). A `save_world` function converts edited columns back to `mcpe_world::Column`s and writes via `WorldWriter` (API in `crates/mcpe-world/src/writer.rs`, landed in mem-a86b), first copying the existing `db/` to a timestamped backup sibling dir.
- Trigger: a temporary key (e.g. Ctrl+S) logging "saved N columns to <dir>"; expose `save_world` as a plain function/event so the game menu (mem-d6f2, worker menu-1, in progress) and mem-5219 can call it. Saving the embedded world: log that there is no directory to save to.
- Recompute heights for edited columns (`Column::recompute_heights`). Lighting stays stale (known follow-up).
- Out of scope: browser download, server, menu UI.

Judge: delta:herd re-runs the evidence on main.

Evidence:

- A test: load a copy of the sample world dir, edit blocks through `WorldMap`, save, reload with mcpe-world, edit present; backup dir exists with the old db.
- `cargo test --workspace`, clippy native and wasm, `sh crates/memcraft-viewer/web/build.sh` (wasm must still build).
- Native run if feasible headless is not required; `mcpe2java` converting the saved test world is.

---
▸ 2026-11-01T16:25:34Z [delta:herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving
