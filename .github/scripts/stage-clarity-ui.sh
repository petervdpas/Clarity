#!/usr/bin/env bash
# Builds Clarity.UI's library (clarity-ui-ffi) in release for this system and
# puts it where the IDE's installer takes it from (tauri.clarity-ui.conf.json):
# app/src-tauri/clarity-ui/, the folder CLARITY_UI names (DESIGN 7.9):
#   include/clarity_ui.h
#   lib/libclarity_ui.so | libclarity_ui.dylib | clarity_ui.dll and clarity_ui.dll.lib
# The installed Clarity hands it to Go, C and C++ builds. Beside it, the
# browser files every language's web server serves (round B) go to
# app/src-tauri/clarity-ui-browser/:
#   clarity-ui.js  clarity-ui.wasm  clarity-ui-worker.js  clarity-ui-page.js
# They are the same on every system: a folder already holding them (the
# release workflow's browser job puts them there) is kept, else they are
# built here, which needs the Emscripten SDK (EMSDK, else ~/emsdk). Run from
# the repository's root, on Linux, macOS or Windows (Git Bash).
set -euo pipefail

browser=app/src-tauri/clarity-ui-browser
browser_files="clarity-ui.js clarity-ui.wasm clarity-ui-worker.js clarity-ui-page.js"
browser_ready=1
for f in $browser_files; do
    [ -f "$browser/$f" ] || browser_ready=0
done
if [ "$browser_ready" = 0 ]; then
    rm -rf "$browser"
    bash runtimes/rust-ui-browser/build.sh "$browser"
fi
cp LICENSE "$browser/"

cargo build --release -p clarity-ui-ffi

stage=app/src-tauri/clarity-ui
rm -rf "$stage"
mkdir -p "$stage/include" "$stage/lib"
cp runtimes/rust-ui-ffi/include/clarity_ui.h "$stage/include/"
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) files="clarity_ui.dll clarity_ui.dll.lib" ;;
    Darwin) files="libclarity_ui.dylib" ;;
    *) files="libclarity_ui.so" ;;
esac
for f in $files; do
    cp "target/release/$f" "$stage/lib/"
done
cp LICENSE "$stage/"
ls -lR "$stage" "$browser"
