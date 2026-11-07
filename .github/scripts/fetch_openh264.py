#!/usr/bin/env python3
"""Fetches Cisco's openh264 binary for one system into a folder, as every
app with video carries it (docs/VIDEO-PLAN.md): downloaded from Cisco,
unpacked, and kept only when its SHA-256 is one Clarity.Video's openh264
knows (runtimes/rust-video/openh264.sha256). Run from the repository's root
on any runner (Linux, macOS, Windows):

    python3 .github/scripts/fetch_openh264.py <rid> <folder>

with rid linux-x64, linux-arm64, win-x64, win-arm64, osx-x64 or osx-arm64.
"""

import bz2
import hashlib
import pathlib
import sys
import urllib.request

VERSION = "2.6.0"
NAMES = {
    "linux-x64": f"libopenh264-{VERSION}-linux64.8.so",
    "linux-arm64": f"libopenh264-{VERSION}-linux-arm64.8.so",
    "win-x64": f"openh264-{VERSION}-win64.dll",
    "win-arm64": f"openh264-{VERSION}-win-arm64.dll",
    "osx-x64": f"libopenh264-{VERSION}-mac-x64.dylib",
    "osx-arm64": f"libopenh264-{VERSION}-mac-arm64.dylib",
}


def main() -> int:
    rid, folder = sys.argv[1], pathlib.Path(sys.argv[2])
    name = NAMES[rid]
    known = {}
    for line in pathlib.Path("runtimes/rust-video/openh264.sha256").read_text().splitlines():
        parts = line.split()
        if len(parts) == 2:
            known[parts[1].lstrip("*")] = parts[0]
    url = f"https://ciscobinary.openh264.org/{name}.bz2"
    request = urllib.request.Request(url, headers={"User-Agent": "Clarity release (github.com/petervdpas/Clarity)"})
    with urllib.request.urlopen(request, timeout=120) as reply:
        binary = bz2.decompress(reply.read())
    digest = hashlib.sha256(binary).hexdigest()
    if known.get(name) != digest:
        print(f"::error::{name} from Cisco has SHA-256 {digest}, not the one Clarity.Video knows", file=sys.stderr)
        return 1
    folder.mkdir(parents=True, exist_ok=True)
    (folder / name).write_bytes(binary)
    print(f"{name}: {len(binary)} bytes, {digest}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
