# MemCraft

Resurrecting old Minecraft: Pocket Edition worlds so they can be revisited in a browser.

## Play "My World" in the browser

```sh
cargo run --release -p mcpe2java -- "worlds/My World" out/my-world.zip
```

Then either drag `out/my-world.zip` onto <https://mcraft.fun>, or host the zip at a CORS-enabled URL and open `https://mcraft.fun/?map=<url>`. You spawn where the player last stood.

## Fly through it in our own viewer

A Bevy app that reads the PE world directly; it runs natively or in the browser (WebGL2).

```sh
cargo run --release -p memcraft-viewer          # native window
crates/memcraft-viewer/web/build.sh              # browser build -> crates/memcraft-viewer/web/dist
python3 -m http.server -d crates/memcraft-viewer/web/dist
```

Controls: click to capture the mouse, WASD to move, Q/E for down/up, Shift to go fast, scroll to change speed. Blocks are drawn as flat colours because there are no textures yet. The browser build needs `rustup target add wasm32-unknown-unknown` and a `wasm-bindgen-cli` matching the version in `Cargo.lock`.

On a fresh Debian/Ubuntu machine (e.g. a cloud runner), `sh scripts/runner-setup.sh` installs everything above, plus Playwright's Chromium for a headless smoke test of the browser build:

```sh
NODE_PATH=~/tools/drive/node_modules node crates/memcraft-viewer/web/drive.js crates/memcraft-viewer/web/dist out/shots
```

It reports whether a click takes the pointer lock and whether releasing it works, and screenshots the view before and after turning.

## Crates

- `crates/nbt`: NBT reader/writer for both Bedrock (little-endian) and Java (big-endian).
- `crates/mcpe-world`: reads LevelDB-era Pocket Edition worlds (0.9 through 0.17: `LegacyTerrain` and v0 sub-chunks). It works from an in-memory copy, so the source world is never modified, and it builds for `wasm32-unknown-unknown`.
- `crates/memcraft-viewer`: the Bevy 0.19 fly-through viewer. The world is embedded at build time; set `MEMCRAFT_WORLD=<dir>` to use a different one.
- `crates/mcpe2java`: converts those worlds to Java Edition 1.12.2 Anvil, the last numeric-id version, so the conversion is close to lossless. It recomputes lighting, because PE sub-chunks store none.

`worlds/My World` is a real PE ~0.16 world (2016) and serves as the test fixture: `cargo test`.
