---
id: mem-a86b
title: 'World writer: save edited columns as PE world records'
status: shorn
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-01T16:25:13Z'
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

---
▸ 2026-11-01T16:12:11Z [delta:writer-1]

Decision: records are written back in each column's own layout (`LegacyTerrain` 0x30 stays 0x30; v0 sub-chunks stay 0x2f + 0x2d). Rejected: converting everything to LegacyTerrain (needs light we do not have for the 202 sub-chunk columns) or everything to sub-chunks (PE 0.16 reads 0x30). Needed new `Column` fields to make re-encoding exact: `heights` (u8 per x,z), `grass_colors` (RGB half of the legacy biome word); decoders fill them, nothing else about reading changed.

Decision: `WorldWriter` works on an in-memory copy of the world's files and `finish()` returns `SavedWorld { level_dat, db_files }`; `write_dir` stages `db.new`, renames `db` -> `db.old`, `db.new` -> `db`, then removes `db.old`; `level.dat` is written byte-identical. Rejected: writing LevelDB straight into the live dir (rusty-leveldb's disk env needs the `fs` feature, and a crash mid-save would corrupt the only copy; also impossible in wasm) and a zip writer (out of scope, and `SavedWorld` hands bytes to any zip writer). Backups for in-place saves are the viewer's job (mem-5219).

Decision: all-air sections above the highest block are overwritten with air only if the DB already has the key, otherwise left absent. Found in the sample: PE keeps an all-air section 5 in some columns and not others (history, not content), so a content-only rule cannot reproduce 64 records. Rejected: always delete (changes unedited bytes), always write up to a fixed count (adds records PE never wrote), storing a section count on `Column`.

Decision: `Column::recompute_heights()` uses an explicit id table (`passes_sky_light`) validated against the sample: it reproduces PE's stored heights for all 256 columns (test). Plain "highest non-air" matched only 92/256: PE ignores glass, rails, plants, torches, stairs, slabs, fences, carpets... but counts leaves, water, ice, farmland (60) and cauldron (118). Ids absent from the sample follow Java light opacity 0 and are unchecked. `put_column` writes `heights` as stored; it does not recompute them by itself.

Decision: light is written as stored and NOT recomputed after edits (legacy columns keep stale sky/block light around edited blocks until PE relights). Rejected for this yak: porting `mcpe2java`'s lighting (Java ids, cross-chunk) - follow-up. A legacy column with `sky_light`/`block_light` = None is an error, since the record cannot be written without it; missing heights/biomes/tint get defaults (computed / 0).

Decision: wasm. `cargo check -p mcpe-world --target wasm32-unknown-unknown` passes with the writer. Found that rusty-leveldb calls `SystemTime::now` (via `Env::micros`) when it writes/compacts a table, which panics on wasm32-unknown-unknown; the reader already hit that path on open (log recovery). Added `ClocklessEnv` (MemEnv + counter for `micros`, no-op sleep) used for both reading and writing, so the browser path cannot panic. Not run in a browser/wasm runtime (no runner installed): only compiled. The Pages download fallback can use `SavedWorld.db_files`.

Decision: new tables are compressed with zlib (id 2) like PE. Only matters when rusty-leveldb compacts; in the sample no compaction runs, writes are appended to the existing write-ahead log (`000752.log` grows 651 KB -> 824 KB), `.ldb` files are untouched byte for byte. Rejected: forcing `compact_range` after saving (extra work, more files rewritten, not needed for correctness). Unverified: that real PE / Mojang LevelDB opens the appended log; our own reader and mcpe2java do.

Decision: no new NBT writer was needed; `crates/nbt` already has `write`/`compound` with round-trip tests. No Cargo changes. Added `PartialEq` on `Column` for tests.

---
▸ 2026-11-01T16:12:12Z [delta:writer-1]

Evidence (all run in /home/user/memcraft-4 on base 48ed523 + my files; start 2026-11-01T15:33:28Z; correction: the earlier progress note said ~16:15Z, the real time was ~15:42Z).

1. Round-trip on the sample world, `cargo test -p mcpe-world` (8 unit + 4 + 3 integration): `writer::tests::encoding_a_decoded_column_reproduces_its_records` (every record of all 256 columns re-encodes to the stored bytes), `recomputed_heights_match_the_stored_ones`, `untouched_columns_keep_their_exact_records` (saves 7 edited columns; every other record in the DB is byte-identical, level.dat identical), `tests/save_world.rs::edited_world_reads_back_as_edited` (edits a legacy column, a sub-chunk column and entities; saved to a temp dir, reloaded; every field of all 256 columns `==` edited/original), `saving_in_place_replaces_db_and_keeps_other_files` (two saves over the same dir, levelname.txt/players kept), `a_column_without_light_cannot_be_written_as_legacy_terrain`. All `test result: ok`.
2. mcpe2java: test `crates/mcpe2java/tests/saved_world.rs` (convert(saved+reloaded) region files byte-equal to convert(edited in memory), 256 chunks) ok; CLI on a saved dir: `cargo run -p mcpe2java -- target/tmp/edited_world target/tmp/java_out` -> `converted "My World": 256 chunks, 90 block entities, 265 entities` / `wrote target/tmp/java_out`, exit 0.
3. `cargo clippy --workspace --all-targets -- -D warnings`: Finished, no warnings. `cargo test --workspace`: every `test result: ok` (mcpe-world 8/4/3, mcpe2java 7+7+1, viewer 55, nbt 3, doctests 1), 0 failed. `cargo check -p mcpe-world --target wasm32-unknown-unknown`: Finished.
Not run: `cargo fmt` (the base is not rustfmt-clean: 25 diffs in mcpe-world before my change), real PE/Minecraft opening the saved world, a wasm runtime test.

---
▸ 2026-11-01T16:12:12Z [delta:writer-1]
kind: move
from: shaving
to: shorn
---

moved: shaving -> shorn

---
▸ 2026-11-01T16:12:48Z [delta:herd]
kind: landed
commit: 00c5b18084d24430fdf5db77838db7e1b52adcde
source: 2c7ab1ebc604e042f4a6537b19ebfeb84307f896
branch: main
---

landed: 00c5b18 mem-a86b: world writer saves edited columns as PE world records

---
▸ 2026-11-01T16:25:13Z [delta:herd]

Re-verified on main 00c5b18 (delta:herd, Sun Nov  1 16:25:13 UTC 2026): cargo test -p nbt -p mcpe-world -p mcpe2java all ok (save_world 3 passed, saved_world 1 passed, my_world 4+7), clippy on those crates -D warnings clean, cargo check -p mcpe-world --target wasm32 ok. Viewer build not re-run by herd (worker's workspace test/clippy passed); will be covered by the next viewer landing's full run. Pushed.
