#!/usr/bin/env bash
# Builds the clarity command-line tool (clarity-cli) in release for this system
# and puts it where the IDE's installer takes it from (externalBin in
# tauri.clarity-ui.conf.json): app/src-tauri/binaries/clarity-<target triple>,
# with .exe on Windows. The installer ships it as `clarity` beside the IDE:
# /usr/bin/clarity on Linux, the install folder on Windows. VERSION (the tag
# without its "v") makes `clarity --version` say the IDE's version. Run from
# the repository's root, on Linux, macOS or Windows (Git Bash).
set -euo pipefail

if [ -n "${VERSION:-}" ]; then
    export CLARITY_VERSION="$VERSION"
fi
cargo build --release -p clarity-cli

triple="$(rustc -vV | sed -n 's/^host: //p')"
case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*) ext=.exe ;;
    *) ext= ;;
esac
stage=app/src-tauri/binaries
rm -rf "$stage"
mkdir -p "$stage"
cp "target/release/clarity$ext" "$stage/clarity-$triple$ext"
ls -l "$stage"
