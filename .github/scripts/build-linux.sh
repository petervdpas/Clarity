#!/usr/bin/env bash
# Builds Clarity's Linux package inside a container of the distribution it is
# for, so the program links that distribution's glibc and WebKitGTK:
# debian:bookworm for the .deb (it runs on Debian 12 and 13, Raspberry Pi OS
# and Ubuntu 24.04 and newer), fedora:43 and fedora:44 for their .rpm.
#
#   build-linux.sh <deb|rpm> <artifact name>
#
# Runs as root in the container with the repository at /src. VERSION (the tag
# without its "v", or empty) and NODE_VERSION come from the workflow. Writes
# out/<artifact name>.<deb|rpm> and, for a .deb, a portable tarball.
set -euo pipefail

format="$1"
name="$2"

if command -v apt-get >/dev/null; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update
    apt-get install -y --no-install-recommends \
        ca-certificates curl file git jq build-essential pkg-config \
        libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
        librsvg2-dev libxdo-dev libssl-dev
    curl -fsSL "https://deb.nodesource.com/setup_${NODE_VERSION}.x" | bash -
    apt-get install -y --no-install-recommends nodejs
else
    dnf -y install \
        curl file git jq gcc gcc-c++ make pkgconf-pkg-config \
        webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel \
        librsvg2-devel libxdo-devel openssl-devel
    curl -fsSL "https://rpm.nodesource.com/setup_${NODE_VERSION}.x" | bash -
    dnf -y install nodejs
fi
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
. "$HOME/.cargo/env"

cd /src

# The installers carry the tag's version: tauri.conf.json and package.json agree with it.
if [ -n "${VERSION:-}" ]; then
    jq --arg v "$VERSION" '.version = $v' app/src-tauri/tauri.conf.json > /tmp/tauri.conf.json
    mv /tmp/tauri.conf.json app/src-tauri/tauri.conf.json
    (cd app && npm version "$VERSION" --no-git-tag-version --allow-same-version)
    echo "version = $VERSION"
fi

cd app
npm ci
npm run tauri build -- --bundles "$format"
cd /src

# The workspace's target folder is at the repository's root.
mkdir -p out
cp target/release/bundle/"$format"/*."$format" "out/$name.$format"
if [ "$format" = deb ]; then
    # The program on its own (it still needs the system's WebKitGTK).
    stage="$(mktemp -d)/$name"
    mkdir -p "$stage"
    cp target/release/clarity-app "$stage/"
    cp README.md "$stage/"
    tar -C "$(dirname "$stage")" -czf "out/$name.tar.gz" "$(basename "$stage")"
fi
ls -la out
