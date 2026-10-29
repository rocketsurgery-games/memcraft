---
id: mem-0d8e
title: Rust workspace + MCPE LevelDB world decoder crate
status: shorn
type: task
priority: 3
created: '2026-10-29T01:33:57Z'
updated: '2026-10-29T01:45:30Z'
parent: mem-0908
---

---
▸ 2026-10-29T01:34:25Z [delta:Start investigation for yak mem-0908]

moved: hairy -> shaving

---
▸ 2026-10-29T01:45:29Z [delta:Start investigation for yak mem-0908]

Landed in commit `331cc2e` on local `main` (not pushed).

- `crates/nbt`: a little/big-endian NBT codec, so the same code reads Bedrock and writes Java.
- `crates/mcpe-world`: a world reader.
  - `World::open_dir` / `World::from_files` load the `db/` files into rusty-leveldb's `MemEnv`, so the source world is never touched and the same path works in wasm.
  - Registers zlib (id 2) and raw-deflate (id 4) compressors.
  - Decodes `LegacyTerrain` and v0–7 sub-chunks into one `Column` (128-high, ids + data + biomes + block entities + entities), plus players.
- `worlds/My World` is now committed as the test fixture.

Evidence:

```sh
cargo test                                               # 15 tests pass, incl. 4 against My World
cargo build -p mcpe-world --target wasm32-unknown-unknown  # ok (needs getrandom/js)
cargo clippy --all-targets                               # clean
```

The fixture test asserts 256 columns (54 legacy + 202 sub-chunk), spawn (128, 64, 128), at least one Chest, and that more than 99% of floor blocks are bedrock. A gold block at y=0 in chunk (4, 5) proves the floor isn't 100% bedrock.

---
▸ 2026-10-29T01:45:30Z [delta:Start investigation for yak mem-0908]

moved: shaving -> shorn
