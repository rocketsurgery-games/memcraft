---
id: mem-dc6b
title: 'Native: load a world dir from the command line, save in place with backup'
status: shorn
type: task
priority: 3
created: '2026-11-01T16:25:13Z'
updated: '2026-11-02T00:11:17Z'
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

---
▸ 2026-11-01T16:25:50Z [delta:herd]

Worker not started yet: disk 12 GB free with three Bevy workers at 12-13 GB each; delta:native-1 starts when door-2 or paint-2 lands and its target is deleted.

---
▸ 2026-11-01T23:22:13Z [native-1]

native-1 start: Sun Nov  1 23:22:13 UTC 2026; checkout /home/user/memcraft-4; yak file /home/user/.local/share/yaks/farms/memcraft/.yaks/mem/yaks/mem-dc6b.md; base 1569374. Not shearing/pushing (per coordinator instruction).

---
▸ 2026-11-01T23:24:32Z [native-1]
kind: checkpoint
commit: cfa0c5a68b1d7ee0c46bb718f6450dce3224d2b0
---

Checkpoint: wip commit cfa0c5a (viewer: optional world dir arg, WorldMap edited tracking, save.rs with backup + Ctrl+S/menu SaveRequested). Building tests (CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 cargo test -p memcraft-viewer, log ~/native1-test.log).

---
▸ 2026-11-01T23:24:32Z [native-1]

Decision: wire native save to the menu's `SaveRequested` now (plus Ctrl+S sending the same message), rather than a bare key only.

- The yak says a key is fine "until the menu's Save hook exists"; mem-d6f2 landed it, so the key is no longer the only trigger.
- Native sets `SaveAvailable(true)` only when a world dir was given, so the button is greyed for the embedded world. mem-5219 keeps the browser half and any UI feedback (native only logs).
- Rejected: Ctrl+S only (leaves the menu button dead natively); a separate event type (would duplicate `SaveRequested`).

Decision: the backup is `<world dir>/db.backup-YYYYMMDD-HHMMSS[-n]` (UTC), inside the world dir, beside `db/`.

- A sibling of the world dir would show up as another world in PE's worlds folder.
- A save with no edited columns writes nothing and makes no backup. Backups are never pruned (follow-up).
- Rejected: backing up by renaming `db/` (a crash then leaves no live db; `write_dir` already swaps atomically).

---
▸ 2026-11-02T00:02:38Z [native-1]

Summary (native-1), commits cfa0c5a, 35204fd, a22c792 on main (base 1569374), not pushed, not shorn.

Shape of the change:

- `main.rs`: optional world-dir argument (a bad argument prints usage and exits 2, a world that fails to load exits 1); `load_world(Option<&Path>)` is the one loader (dir via `World::open_dir`, else the embedded files via `World::from_files`). `WorldMap` and `Solids` are now built in `main()`, and `setup` reads the world from `WorldMap`.
- `world.rs`: `WorldMap` records `edited` columns in `set_block` (the edited column only, not its neighbours); `edited_columns()` returns clones with `recompute_heights()`; `mark_saved()` clears them.
- `save.rs` (native only): `save_world(map, dir) -> Result<Option<Saved>>` re-opens the dir with `WorldWriter`, puts only the edited columns, copies `db/` to `db.backup-<UTC>[-n]`, then `write_dir`. No edits means no write and no backup. On error nothing changes and the edits stay pending. `SavePlugin` sets `SaveAvailable` only when a dir was given, handles `SaveRequested` (menu Save), and Ctrl+S sends the same message. The embedded world only logs why it cannot save.
- README: usage and Save behaviour; menu text no longer says "once saving exists".

Evidence (`CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0`, no native release builds):

- `cargo test --workspace`: all pass, viewer 73 tests including 7 new in `save::tests`: save in place and reload shows the edits with fresh heights, other columns equal, backup byte-identical to the old `db/`, other files untouched; no-edit save writes nothing, each save its own backup; failed save keeps edits; Ctrl+S and a menu request save; the embedded world does not.
- `cargo clippy --workspace --all-targets`: clean (after one `cloned_ref_to_slice_refs` fix).
- `cargo clippy -p memcraft-viewer --target wasm32-unknown-unknown`: clean.
- `sh crates/memcraft-viewer/web/build.sh`: builds, dist is 22M wasm + js + index.html (`~/native1-web.log`).
- `mcpe2java` on a saved test world (the save test's output kept in a temp dir, then removed): `converted "My World": 256 chunks, 89 block entities, 265 entities`, writing `level.dat` and `region`; identical counts to the untouched sample.

Not done / known gaps:

- No GUI run of the native window (no display): the dir argument and Save-in-the-window path are covered by tests, not seen.
- Lighting stays stale in edited columns; the saved world is untested in real PE.
- The saved `db/` has no `LOCK` file (the old one did; it stays in the backup). LevelDB recreates it.
- Backups are never pruned. Save feedback is log-only: UI toast and the browser half belong to mem-5219.
- `MEMCRAFT_WORLD` still picks the embedded world at build time, unchanged.

---
▸ 2026-11-02T00:11:17Z [herd-memcraft]

Landed 3c0db4d on main (native-1's a22c792; README conflict with paintings line resolved). Herd re-ran on main: cargo test --workspace (79 viewer tests), clippy native+wasm clean, web/build.sh OK. Gaps: native window not run (no display), lighting stale, untested in real PE, no LOCK file in saved db/, backups never pruned, log-only feedback.

---
▸ 2026-11-02T00:11:17Z [herd-memcraft]
kind: move
from: shaving
to: shorn
thread: ksQQOn_lPggNQrKkBKNvXOzaN5LOADYRe5IBxBCA0aLJyrhJPreUvVwLftHG
---

moved: shaving -> shorn
