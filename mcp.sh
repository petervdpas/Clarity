#!/usr/bin/env bash
# Serves Clarity's tools to an AI agent (`clarity mcp`) from this checkout's
# own build, so the agent works with the Clarity of this repository, not an
# installed one. It builds the command first when it is behind its source
# (cargo does nothing when it is not), then runs it. .mcp.json names it for
# Claude Code; any agent that starts a command over stdio can use it.
#
#   ./mcp.sh            the tools, on the project open in the Clarity IDE
#
# Only MCP goes to standard output; the build says what it does on stderr.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$root"
CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}" cargo build --quiet -p clarity-cli 1>&2
exec "$root/target/debug/clarity" mcp "$@"
