#!/usr/bin/env bash
# Builds Clarity.UI's library (clarity-ui-ffi) in release for this system and
# puts it where the IDE's installer takes it from (tauri.clarity-ui.conf.json):
# app/src-tauri/clarity-ui/, the folder CLARITY_UI names (DESIGN 7.9):
#   include/clarity_ui.h, include/clarity_audio.h
#   lib/libclarity_ui.so | libclarity_ui.dylib | clarity_ui.dll and clarity_ui.dll.lib
#   lib/ the same of clarity_audio: Clarity.Audio, for programs that play songs
#   lib/ the same of clarity_video, include/clarity_video.h: Clarity.Video
#   lib/ the same of clarity_image, include/clarity_image.h: Clarity.Image,
#        for programs whose code calls picture functions
#   lib/ Cisco's openh264 for this system, which every app with video carries
#        (fetched from Cisco and checked, docs/VIDEO-PLAN.md)
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

cargo build --release -p clarity-ui-ffi -p clarity-audio-ffi -p clarity-video-ffi -p clarity-image-ffi -p clarity-formats-ffi -p clarity-auth-ffi -p clarity-app-mcp-ffi

stage=app/src-tauri/clarity-ui
rm -rf "$stage"
mkdir -p "$stage/include" "$stage/lib"
cp runtimes/rust-ui-ffi/include/clarity_ui.h runtimes/rust-audio-ffi/include/clarity_audio.h runtimes/rust-video-ffi/include/clarity_video.h runtimes/rust-image-ffi/include/clarity_image.h runtimes/rust-formats-ffi/include/clarity_formats.h runtimes/rust-auth-ffi/include/clarity_auth.h runtimes/rust-app-mcp-ffi/include/clarity_app_mcp.h "$stage/include/"
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) files="clarity_ui.dll clarity_ui.dll.lib" ;;
    Darwin) files="libclarity_ui.dylib" ;;
    *) files="libclarity_ui.so" ;;
esac
for f in $files; do
    cp "target/release/$f" "$stage/lib/"
    cp "target/release/${f/clarity_ui/clarity_audio}" "$stage/lib/"
    cp "target/release/${f/clarity_ui/clarity_video}" "$stage/lib/"
    cp "target/release/${f/clarity_ui/clarity_image}" "$stage/lib/"
    cp "target/release/${f/clarity_ui/clarity_formats}" "$stage/lib/"
    cp "target/release/${f/clarity_ui/clarity_auth}" "$stage/lib/"
    cp "target/release/${f/clarity_ui/clarity_app_mcp}" "$stage/lib/"
done
case "$(uname -s)-$(uname -m)" in
    MINGW*-x86_64|MSYS*-x86_64|CYGWIN*-x86_64) rid=win-x64 ;;
    MINGW*-aarch64|MSYS*-aarch64|MINGW*-arm64|MSYS*-arm64) rid=win-arm64 ;;
    Darwin-arm64) rid=osx-arm64 ;;
    Darwin-*) rid=osx-x64 ;;
    *-aarch64|*-arm64) rid=linux-arm64 ;;
    *) rid=linux-x64 ;;
esac
python3 .github/scripts/fetch_openh264.py "$rid" "$stage/lib"
cp LICENSE "$stage/"
ls -lR "$stage" "$browser"
