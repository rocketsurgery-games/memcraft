---
id: mem-a86b
title: 'World writer: save edited columns as PE world records'
status: shaving
type: task
priority: 3
created: '2026-11-01T01:16:34Z'
updated: '2026-11-01T15:32:23Z'
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
