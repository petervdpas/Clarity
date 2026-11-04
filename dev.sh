#!/usr/bin/env bash
# Starts the Clarity app in development mode, from any directory.
#
#   ./dev.sh                         opens the project used last time
#   ./dev.sh examples/shop.clarity   opens a project file
#   CLARITY_RUNTIME= ./dev.sh         builds apps on the published runtimes
#
# The designer writes its changes into the project's files, so point it at
# a copy when you only want to experiment.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ $# -gt 0 ]; then
    if [ ! -f "$1" ]; then
        echo "dev.sh: '$1' is not a project file; give one like examples/shop.clarity" >&2
        exit 1
    fi
    # Absolute, because the app runs from app/.
    CLARITY_PROJECT="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
    export CLARITY_PROJECT
fi

# Apps generated and built here use the runtimes of this repository, not the
# published ones, so a change to Clarity.UI shows in the next Run. C# wants the
# C# runtime's folder; Rust finds its runtime beside it. Set CLARITY_RUNTIME
# to an empty value to build on the published packages and crates instead.
export CLARITY_RUNTIME="${CLARITY_RUNTIME-$root/runtimes/csharp}"

cd "$root/app"
if [ ! -d node_modules ]; then
    npm install
fi
exec npm run tauri dev
