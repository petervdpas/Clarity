#!/usr/bin/env python3
"""Copies Microsoft's C++ runtime beside Clarity's Windows libraries.

    python3 vc_runtime.py <rid> <folder>

Clarity's Windows libraries (clarity_ui.dll and the others) are built with
MSVC and need VCRUNTIME140.dll (clarity_ui.dll also MSVCP140.dll, which needs
VCRUNTIME140_1.dll), which a fresh Windows has only with the Visual C++
Redistributable. Microsoft lets a program carry these files beside itself
(app-local deployment), so the libraries ship with them and every program
that copies the libraries beside itself runs without the Redistributable.
Linking the runtime in (+crt-static) would need a Skia built so, which
rust-skia does not publish. Taken from the Visual Studio of the Windows
runner (vswhere); check_dll_imports.py then proves nothing is missing.
"""

import shutil
import subprocess
import sys
from pathlib import Path

FILES = ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"]
ARCH = {"win-x64": "x64", "win-arm64": "arm64"}


def redist_folder(arch: str) -> Path:
    vswhere = Path(r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe")
    install = subprocess.run([str(vswhere), "-latest", "-products", "*", "-property", "installationPath"], check=True, capture_output=True, text=True).stdout.strip()
    # VC\Redist\MSVC\<version>\<arch>\Microsoft.VC<toolset>.CRT, the newest version first.
    for version in sorted((Path(install) / "VC" / "Redist" / "MSVC").iterdir(), reverse=True):
        for crt in sorted((version / arch).glob("Microsoft.VC*.CRT"), reverse=True):
            if (crt / "vcruntime140.dll").is_file():
                return crt
    raise SystemExit(f"no Microsoft.VC*.CRT for {arch} in {install}")


def main() -> int:
    if len(sys.argv) != 3 or sys.argv[1] not in ARCH:
        print(__doc__)
        return 2
    folder = Path(sys.argv[2])
    crt = redist_folder(ARCH[sys.argv[1]])
    for name in FILES:
        shutil.copy2(crt / name, folder / name)
        print(f"{crt / name} -> {folder / name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
