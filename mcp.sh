#!/usr/bin/env bash
# Serves Clarity's tools to an AI agent (`clarity mcp`) from this checkout's
# own build: mcp.mjs does it, on every system (.mcp.json starts it with node,
# as Windows cannot start this script itself).
#
#   ./mcp.sh            the tools, on the project open in the Clarity IDE
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec node "$root/mcp.mjs" "$@"
