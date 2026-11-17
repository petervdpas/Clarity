#!/usr/bin/env bash
# Fetches the runtimes the installed Clarity carries, so a project builds
# with no internet (docs/ledger/offline.md, "The installer";
# crates/clarity-gen/src/carried.rs). The projects in runtimes/carried/
# name every runtime a generated project can ask for, at the versions this
# Clarity generates, published before the IDE's tag. Into <out> (default
# out/runtimes), each packed with zstd, which a build with no internet
# unpacks into the person's cache folder the first time:
#   rust.tar.zst   the crates, as cargo vendor writes them (every system's)
#   go.tar.zst     the modules, as a Go module proxy in files
# C# carries none: its packages hold every system's native libraries, too
# big for the installer, so its first build needs the internet.
# They are the same on every system: the release's runtimes job fetches
# them once, and each installer job takes them. Needs cargo, go and zstd.
# The registries can take a while to serve what was just published, so
# each fetch tries again for up to an hour.
set -euo pipefail

out="${1:-out/runtimes}"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
rm -rf "$out"
mkdir -p "$out"
out="$(cd "$out" && pwd)"
stage="$work/stage"
mkdir -p "$stage"
cp runtimes/carried/* "$work/"

# Tries a command again every minute, for up to an hour.
again() {
    for try in $(seq 1 60); do
        if "$@"; then
            return 0
        fi
        echo "not there yet (try $try of 60): $*"
        sleep 60
    done
    return 1
}

# Rust: a program of its own that needs every crate, vendored.
mkdir -p "$work/src"
echo 'fn main() {}' > "$work/src/main.rs"
(cd "$work" && again cargo generate-lockfile && cargo vendor --versioned-dirs --locked "$stage/rust" > /dev/null)

# Go: the runtime module and every module it needs, from the module cache.
export GOMODCACHE="$work/gomod"
export GOFLAGS=-mod=mod
(cd "$work" && again go mod download all)
cp -R "$GOMODCACHE/cache/download" "$stage/go"
rm -rf "$stage/go/sumdb"
chmod -R u+w "$GOMODCACHE"

# Each stack's folder packed on its own, its contents at the archive's top.
for name in rust go; do
    tar -C "$stage/$name" -cf - . | zstd -19 -T0 -q -o "$out/$name.tar.zst"
done
du -sh "$stage"/* "$out"/*
