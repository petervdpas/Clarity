#!/usr/bin/env python3
"""Fails when a Windows library in a folder needs a DLL that is neither part
of Windows nor shipped in the same folder.

    python3 check_dll_imports.py <folder>

A program on a fresh Windows finds a DLL beside itself or in Windows' own
folders; anything else (Microsoft's C++ runtime, VCRUNTIME140.dll and
MSVCP140.dll, above all) must be shipped beside the libraries
(.github/scripts/vc_runtime.py). Reads each .dll's import table itself, so it
needs nothing but Python, on any system.
"""

import struct
import sys
from pathlib import Path

# What every Windows 10 and 11 has: the system's own DLLs the libraries use,
# and the API sets (api-ms-win-*, ext-ms-*), which Windows resolves itself.
WINDOWS = {
    "advapi32.dll", "bcrypt.dll", "bcryptprimitives.dll", "combase.dll", "comctl32.dll", "comdlg32.dll",
    "crypt32.dll", "d3d11.dll", "d3d12.dll", "d3dcompiler_47.dll", "dwmapi.dll", "dwrite.dll", "dxgi.dll",
    "gdi32.dll", "gdiplus.dll", "imm32.dll", "iphlpapi.dll", "kernel32.dll", "kernelbase.dll", "mf.dll",
    "mfplat.dll", "mfreadwrite.dll", "mmdevapi.dll", "msvcrt.dll", "ncrypt.dll", "ntdll.dll", "ole32.dll",
    "oleaut32.dll", "opengl32.dll", "propsys.dll", "rpcrt4.dll", "secur32.dll", "setupapi.dll", "shell32.dll",
    "shlwapi.dll", "ucrtbase.dll", "uiautomationcore.dll", "user32.dll", "userenv.dll", "uxtheme.dll",
    "version.dll", "winhttp.dll", "winmm.dll", "ws2_32.dll", "wsock32.dll",
}


def imports(data: bytes) -> list[str]:
    """The DLL names a PE file's import table names, lower case."""
    if data[:2] != b"MZ":
        raise ValueError("not a PE file")
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("not a PE file")
    sections, optional_size = struct.unpack_from("<H", data, pe + 6)[0], struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    magic = struct.unpack_from("<H", data, optional)[0]
    directories = optional + (112 if magic == 0x20B else 96)
    import_rva = struct.unpack_from("<I", data, directories + 8)[0]
    table = optional + optional_size
    spans = [struct.unpack_from("<IIII", data, table + 40 * i + 8) for i in range(sections)]

    def offset(rva: int) -> int:
        for size, address, raw_size, raw in spans:
            if address <= rva < address + max(size, raw_size):
                return rva - address + raw
        raise ValueError(f"address {rva:#x} is in no section")

    names = []
    if import_rva:
        entry = offset(import_rva)
        while True:
            name_rva = struct.unpack_from("<I", data, entry + 12)[0]
            if name_rva == 0:
                break
            start = offset(name_rva)
            names.append(data[start:data.index(b"\0", start)].decode("ascii").lower())
            entry += 20
    return names


def missing(folder: Path) -> dict[str, list[str]]:
    """Per DLL in `folder`, what it needs that Windows lacks and the folder does not ship."""
    shipped = {p.name.lower() for p in folder.iterdir() if p.suffix.lower() == ".dll"}
    found = {}
    for path in sorted(folder.glob("*.dll")):
        lacking = sorted(
            n for n in set(imports(path.read_bytes()))
            if n not in WINDOWS and n not in shipped and not n.startswith(("api-ms-win-", "ext-ms-"))
        )
        if lacking:
            found[path.name] = lacking
    return found


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__)
        return 2
    folder = Path(sys.argv[1])
    lacking = missing(folder)
    for name, needs in lacking.items():
        print(f"{name} needs {', '.join(needs)}, which Windows lacks and {folder} does not ship")
    if lacking:
        return 1
    print(f"every library in {folder} finds what it needs on a fresh Windows")
    return 0


if __name__ == "__main__":
    sys.exit(main())
