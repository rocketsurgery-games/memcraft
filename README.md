# MemCraft

Resurrecting old Minecraft: Pocket Edition worlds so they can be revisited in a browser.

## Play "My World" in the browser

```sh
cargo run --release -p mcpe2java -- "worlds/My World" out/my-world.zip
```

Then either drag `out/my-world.zip` onto <https://mcraft.fun>, or host the zip at a CORS-enabled URL and open `https://mcraft.fun/?map=<url>`. You spawn where the player last stood.

## Fly through it in our own viewer

A Bevy app that reads the PE world directly; it runs natively or in the browser (WebGL2).

The browser build of `main` is hosted at <https://rocketsurgery-games.github.io/memcraft/>; `.github/workflows/pages.yml` rebuilds it on every push.

```sh
cargo run --release -p memcraft-viewer          # native window, embedded world
cargo run --release -p memcraft-viewer -- "path/to/world"   # native, a world directory
crates/memcraft-viewer/web/build.sh              # browser build -> crates/memcraft-viewer/web/dist
python3 -m http.server -d crates/memcraft-viewer/web/dist
```

Natively, a world directory argument (the folder holding `level.dat` and `db/`) opens that world instead of the embedded one, and Save (in the menu, or Ctrl+S) writes the edited columns back into it. The previous `db/` is first copied to `db.backup-<UTC time>` in the same directory; backups are never deleted. Heights are recomputed for edited columns, but lighting is not, and the saved world has not yet been tried in Pocket Edition itself. The embedded world and the browser build cannot save.

Controls: click to capture the mouse (Esc releases it and opens the menu: Resume, Save (native, with a world directory), and a controls help; the other controls only act while the mouse is captured), WASD to move, Q/E for down/up, Shift to go fast, scroll to change speed, F to switch between flying and walking (walking has gravity and solid blocks; Space jumps), 1-9 to choose a block in the hotbar, left click to break the block in the crosshair, right click to open or close a door, trapdoor or fence gate, flip a lever or press a button (it pops back after a second or so), edit the four lines of a sign (type, Enter for the next line and to save after the last, Esc to cancel; the text is drawn on the sign), or else to place the selected block against the block in the crosshair. Blocks are textured with Pixel Perfection by XSSheep (CC BY-SA 4.0, see `crates/memcraft-viewer/assets/ATTRIBUTION.md`); blocks the pack doesn't cover fall back to flat colours. Sign text, paintings and item frames in the world are drawn too (paintings and item frames are static: they cannot be targeted or removed). In the browser, `?cam=x,y,z,yaw,pitch` after the page address (natively, the `MEMCRAFT_CAM` environment variable) starts the camera at that spot. The browser build needs `rustup target add wasm32-unknown-unknown` and a `wasm-bindgen-cli` matching the version in `Cargo.lock`.

On a fresh Debian/Ubuntu machine (e.g. a cloud runner), `sh scripts/runner-setup.sh` installs everything above, plus Playwright's Chromium for a headless smoke test of the browser build:

```sh
NODE_PATH=~/tools/drive/node_modules node crates/memcraft-viewer/web/drive.js crates/memcraft-viewer/web/dist out/shots
```

It reports whether a click takes the pointer lock and whether releasing it works, and screenshots the menu (`menu.png`, `menu-controls.png`, `menu-back.png`) and the view before and after turning.

To look at one place instead, start the page (or a native run, through the
`MEMCRAFT_CAM` environment variable) with `?cam=x,y,z,yaw,pitch`, as in
`CAM=119.5,66,12.5,180,12 LEVER=1`: the drive then right-clicks the crosshair
once and screenshots `lever-before.png` and `lever-after.png` (that spot is a
lever of the sample world).

## Crates

- `crates/nbt`: NBT reader/writer for both Bedrock (little-endian) and Java (big-endian).
- `crates/mcpe-world`: reads LevelDB-era Pocket Edition worlds (0.9 through 0.17: `LegacyTerrain` and v0 sub-chunks) and writes edited columns back (`WorldWriter`; see the crate docs). It works from an in-memory copy, so the source world is never modified until you `write_dir`, and it builds for `wasm32-unknown-unknown`.
- `crates/memcraft-viewer`: the Bevy 0.19 fly-through viewer. The world is embedded at build time; set `MEMCRAFT_WORLD=<dir>` to use a different one.
- `crates/mcpe2java`: converts those worlds to Java Edition 1.12.2 Anvil, the last numeric-id version, so the conversion is close to lossless. It recomputes lighting, because PE sub-chunks store none.

`worlds/My World` is a real PE ~0.16 world (2016) and serves as the test fixture: `cargo test`.
