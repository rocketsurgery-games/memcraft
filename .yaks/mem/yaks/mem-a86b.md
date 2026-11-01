---
id: mem-a86b
title: 'World writer: save edited columns as PE world records'
status: shaving
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-01T15:41:38Z'
parent: mem-f5cf
---

`mcpe-world` can only read. Saving needs the reverse: encode a column (blocks, data, light, heights, biomes, block entities, entities) back into the records PE 0.16 reads, and write them into the LevelDB world (rusty-leveldb can write), producing a world dir or a zip.

Format depends on the open question on the parent yak.

## Evidence

Round-trip test on the sample world: load, change blocks in a few columns, save, reload, compare; mcpe2java converts the saved world.

---
▸ 2026-11-01T15:32:22Z [delta:herd]
kind: claim
---

Claim (delta:herd) for worker `delta:writer-1`. Format decided by Joel on mem-f5cf: write back into the PE 0.16 LevelDB world.

Scope:

- In `mcpe-world`: encode a chunk (the inverse of the reader: terrain/blocks+data+light+heights+biomes record, block entities, entities as little-endian NBT) and write records into a LevelDB world dir with rusty-leveldb (already a dependency). API shape: open a world dir for writing (or copy-then-write), `put_chunk`, flush; keep `level.dat` untouched apart from what PE needs. Reading must stay exactly as it is.
- Writing must work natively; on wasm it is enough that it compiles if rusty-leveldb's in-memory env makes it possible (the Pages download fallback comes later; record a `Decision:` on what you found).
- An NBT writer in `crates/nbt` if missing (add tests).
- Out of scope: viewer wiring and save UI (mem-5219), server, zip download.

Judge: delta:herd re-runs the evidence on main.

Evidence:

- Round-trip test on the sample world: load, change blocks in a few columns, save to a temp dir, reload, compare all fields; unchanged columns byte-identical where feasible.
- `mcpe2java` converts the saved world (run its CLI or test on the temp dir) without errors.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`.

---
▸ 2026-11-01T15:32:23Z [delta:herd]
kind: move
from: hairy
to: shaving
---

moved: hairy -> shaving

---
▸ 2026-11-01T15:34:00Z [delta:writer-1]

Start: 2026-11-01T15:33:28Z (`date -u`), base 48ed523, checkout /home/user/memcraft-4.

`yaks path mem-a86b`: /home/user/.local/share/yaks/farms/memcraft/.yaks/mem/yaks/mem-a86b.md

Read claim note and parent mem-f5cf (Joel: write back into the PE LevelDB world is fine). `crates/nbt` already has `write`/`compound`; the writer need not add one (will confirm and add tests only if gaps).

---
▸ 2026-11-01T15:41:38Z [delta:writer-1]

Progress (2026-11-01 ~16:15Z): implemented in `crates/mcpe-world`: `WorldWriter` (open_dir/from_files, put_column, finish -> `SavedWorld`, write_dir), `encode.rs` (column -> records), `db.rs` shared `MemDb`, `key::chunk_key`; `Column` gained `heights`, `grass_colors`, `PartialEq`, `recompute_heights()`. Tests: unit (re-encode all 256 sample columns == stored records; heights recompute == stored for all 256; untouched records identical after saving edited ones), integration `tests/save_world.rs`, and `crates/mcpe2java/tests/saved_world.rs`. `cargo test -p mcpe-world -p nbt -p mcpe2java` green. Remaining: clippy, wasm check, CLI on saved dir, workspace test.
