# MemCraft

Resurrecting old Minecraft: Pocket Edition worlds so they can be revisited in a browser.

## Play "My World" in the browser

```sh
cargo run --release -p mcpe2java -- "worlds/My World" out/my-world.zip
```

Then either drag `out/my-world.zip` onto <https://mcraft.fun>, or host the zip at a CORS-enabled URL and open `https://mcraft.fun/?map=<url>`. You spawn where the player last stood.

## Crates

- `crates/nbt`: NBT reader/writer for both Bedrock (little-endian) and Java (big-endian).
- `crates/mcpe-world`: reads LevelDB-era Pocket Edition worlds (0.9 through 0.17: `LegacyTerrain` and v0 sub-chunks). It works from an in-memory copy, so the source world is never modified, and it builds for `wasm32-unknown-unknown`.
- `crates/mcpe2java`: converts those worlds to Java Edition 1.12.2 Anvil, the last numeric-id version, so the conversion is close to lossless. It recomputes lighting, because PE sub-chunks store none.

`worlds/My World` is a real PE ~0.16 world (2016) and serves as the test fixture: `cargo test`.
