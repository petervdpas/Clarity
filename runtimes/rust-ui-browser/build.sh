#!/usr/bin/env bash
# Builds Clarity.UI's browser files (DESIGN 7.9, round B) into one folder:
#   clarity-ui.js  clarity-ui.wasm  clarity-ui-worker.js  clarity-ui-page.js
# Usage: runtimes/rust-ui-browser/build.sh [output folder]
# The folder is target/clarity-ui-browser by default. Needs the Emscripten SDK
# (EMSDK, else ~/emsdk) and `rustup target add wasm32-unknown-emscripten`.
# Skia comes from skia-safe's prebuilt binaries for that target; it is never
# built from source (the crate's skia-bindings "no-compile" stops a build that would).
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
out="${1:-$repo/target/clarity-ui-browser}"
emsdk="${EMSDK:-$HOME/emsdk}"

if [ ! -f "$emsdk/emsdk_env.sh" ]; then
  echo "No Emscripten SDK at $emsdk: install it (https://emscripten.org/docs/getting_started/downloads.html) or set EMSDK." >&2
  exit 1
fi
# shellcheck disable=SC1091
source "$emsdk/emsdk_env.sh" > /dev/null 2>&1
export EMSDK="$emsdk"

cd "$repo"
cargo build -p clarity-ui-browser --target wasm32-unknown-emscripten --profile browser

built="$repo/target/wasm32-unknown-emscripten/browser"
mkdir -p "$out"
cp "$built/clarity-ui.js" "$out/clarity-ui.js"
cp "$built/clarity_ui.wasm" "$out/clarity-ui.wasm" 2> /dev/null || cp "$built/clarity-ui.wasm" "$out/clarity-ui.wasm"
cp "$here/js/clarity-ui-worker.js" "$here/js/clarity-ui-page.js" "$out/"
ls -l "$out"
