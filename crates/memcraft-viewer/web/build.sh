#!/bin/sh
# Builds the browser version into crates/memcraft-viewer/web/dist.
#
# Needs: rustup target add wasm32-unknown-unknown
#        cargo install wasm-bindgen-cli --version <the wasm-bindgen version in Cargo.lock>
# Serve the result with any static server, e.g.
#        python3 -m http.server -d crates/memcraft-viewer/web/dist
set -eu
cd "$(dirname "$0")/../../.."

# getrandom needs to be told to use the browser's crypto API.
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' \
  cargo build --profile wasm-release -p memcraft-viewer --target wasm32-unknown-unknown

out=crates/memcraft-viewer/web/dist
rm -rf "$out"
wasm-bindgen --no-typescript --remove-name-section --remove-producers-section --target web --out-dir "$out" --out-name memcraft-viewer \
  target/wasm32-unknown-unknown/wasm-release/memcraft-viewer.wasm
cp crates/memcraft-viewer/web/index.html "$out/"
if command -v wasm-opt >/dev/null; then
  wasm-opt -Oz -o "$out/memcraft-viewer_bg.wasm" "$out/memcraft-viewer_bg.wasm"
fi
ls -lh "$out"
