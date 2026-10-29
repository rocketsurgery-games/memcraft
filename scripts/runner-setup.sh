#!/bin/sh
# Prepares a fresh Debian/Ubuntu machine (e.g. a cloud runner) to build,
# test and browser-test this repo. Idempotent: re-run it after a reset.
#
#   sh scripts/runner-setup.sh
#
# Installs: rustup (+ clippy, rustfmt, wasm32 target), Bevy's system
# libraries, clang/lld, binaryen (wasm-opt), the wasm-bindgen-cli version
# pinned in Cargo.lock, the yaks CLI, and Playwright's Chromium (in
# ~/tools/drive) for headless browser checks of the web build.
set -eu
# The agent terminal's TMPDIR can vanish when its command ends; use our own.
export TMPDIR="$HOME/.cache/tmp"
mkdir -p "$TMPDIR"
repo=$(cd "$(dirname "$0")/.." && pwd)

if ! command -v pkg-config >/dev/null || ! dpkg -s libasound2-dev >/dev/null 2>&1; then
  sudo apt-get update -qq
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    pkg-config libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev clang lld binaryen
fi

if [ ! -x "$HOME/.cargo/bin/rustup" ]; then
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal -c clippy,rustfmt -t wasm32-unknown-unknown
fi
. "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown >/dev/null

bindgen=$(grep -A1 '^name = "wasm-bindgen"$' "$repo/Cargo.lock" | sed -n 's/^version = "\(.*\)"/\1/p')
if [ "$(wasm-bindgen --version 2>/dev/null)" != "wasm-bindgen $bindgen" ]; then
  cargo install -q wasm-bindgen-cli --version "$bindgen" --locked
fi

command -v yaks >/dev/null || npm install -g -s @j15r/yaks

# Playwright + Chromium for driving the browser build headlessly.
if [ ! -d "$HOME/tools/drive/node_modules/playwright" ]; then
  mkdir -p "$HOME/tools/drive"
  (cd "$HOME/tools/drive" && npm init -y >/dev/null && npm i -s playwright@1 && npx playwright install --with-deps chromium)
fi

# A Delta clone borrows a bare repo with no checkout, so yaks can't create
# the farm worktree itself: create it from origin/yaks.
farm="$HOME/.local/share/yaks/farms/memcraft"
if [ ! -d "$farm/.yaks" ]; then
  bare=$(git -C "$repo" config --get remote.local.url || true)
  [ -n "$bare" ] || bare=$(git -C "$repo" rev-parse --git-common-dir)
  git -C "$bare" fetch -q origin '+refs/heads/yaks:refs/remotes/origin/yaks'
  git -C "$bare" show-ref -q refs/heads/yaks || git -C "$bare" branch -q yaks origin/yaks
  mkdir -p "$(dirname "$farm")"
  git -C "$bare" worktree add -q "$farm" yaks
  git -C "$farm" branch -q --set-upstream-to=origin/yaks
fi

echo "ready: $(rustc --version), $(wasm-bindgen --version), yaks farm $(yaks path 2>/dev/null)"
