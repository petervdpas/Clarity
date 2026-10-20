#!/usr/bin/env bash
# Builds the engines of the examples from their sources: each crate in
# <example>/engines/<name>/ becomes <example>/engines/<name>.wasm. Needs Rust
# with the wasm32-unknown-unknown target (rustup target add wasm32-unknown-unknown).
set -euo pipefail
cd "$(dirname "$0")"
for crate in */engines/*/Cargo.toml; do
    dir=$(dirname "$crate")
    name=$(basename "$dir")
    echo "building $dir"
    (cd "$dir" && cargo build --quiet --release --target wasm32-unknown-unknown)
    cp "$dir/target/wasm32-unknown-unknown/release/$name.wasm" "$(dirname "$dir")/$name.wasm"
done
