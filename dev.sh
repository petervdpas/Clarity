#!/usr/bin/env bash
# Starts the Clarity app in development mode, from any directory.
#
#   ./dev.sh                     opens the folder used last time
#   ./dev.sh examples/shop/spec  opens a folder of .clarity files
#
# The designer writes its changes into the spec files, so point it at a
# copy when you only want to experiment.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ $# -gt 0 ]; then
    if [ ! -d "$1" ]; then
        echo "dev.sh: '$1' is not a directory" >&2
        exit 1
    fi
    # Absolute, because the app runs from app/.
    CLARITY_SPEC_DIR="$(cd "$1" && pwd)"
    export CLARITY_SPEC_DIR
fi

cd "$root/app"
if [ ! -d node_modules ]; then
    npm install
fi
exec npm run tauri dev
