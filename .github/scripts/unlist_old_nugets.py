#!/usr/bin/env python3
"""Unlists every version of Clarity's NuGet packages on nuget.org but the newest.

The packages are those of runtimes/csharp (each csproj's <PackageId>). For
each, the script reads its versions from nuget.org, keeps the newest, and
unlists every other version that is still listed. Unlisting hides a version
from search and from new installs; projects that name it keep restoring it,
and it can be listed again on nuget.org.

It runs in the C# release workflow (release-csharp-runtime.yml), started by
hand: nuget.org's Trusted Publishing policy for that workflow,
with the scope "Unlist or relist package versions", gives it a short-lived
key (NuGet/login), which it reads from NUGET_API_KEY. Without the scope
nuget.org answers 403. Without --yes it only says what it would unlist:

    python3 .github/scripts/unlist_old_nugets.py            # what it would do
    NUGET_API_KEY=... python3 .github/scripts/unlist_old_nugets.py --yes

Package ids named on the line are done too, as the retired Clarity.Avalonia
and Clarity.Avalonia.Desktop, which no csproj makes any more:

    python3 .github/scripts/unlist_old_nugets.py Clarity.Avalonia Clarity.Avalonia.Desktop
"""

import json
import os
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
REGISTRATION = "https://api.nuget.org/v3/registration5-semver1/{id}/index.json"
PUBLISH = "https://www.nuget.org/api/v2/package/{id}/{version}"
AGENT = "clarity-unlist-old-nugets"


def package_ids():
    """The ids of the packages runtimes/csharp makes, in name order."""
    ids = set()
    for csproj in (ROOT / "runtimes" / "csharp").glob("*/*.csproj"):
        text = csproj.read_text(encoding="utf-8-sig")
        if re.search(r"<IsPackable>\s*true\s*</IsPackable>", text):
            found = re.search(r"<PackageId>\s*([^<\s]+)\s*</PackageId>", text)
            if found:
                ids.add(found.group(1))
    return sorted(ids)


def get_json(url):
    request = urllib.request.Request(url, headers={"User-Agent": AGENT})
    with urllib.request.urlopen(request, timeout=30) as answer:
        return json.load(answer)


def version_key(version):
    """Orders 0.9.0 before 0.10.0; a prerelease before its release."""
    core, _, pre = version.partition("-")
    numbers = [int(part) if part.isdigit() else 0 for part in core.split(".")]
    return (numbers, pre == "", pre)


def versions(package):
    """Every version of a package on nuget.org, with whether it is listed."""
    url = REGISTRATION.format(id=package.lower())
    try:
        index = get_json(url)
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return []
        raise
    found = []
    for page in index.get("items", []):
        # A large registration keeps its versions in pages of their own.
        items = page.get("items") or get_json(page["@id"]).get("items", [])
        for item in items:
            entry = item["catalogEntry"]
            found.append((entry["version"], entry.get("listed", True)))
    return found


def unlist(package, version, key):
    """nuget.org unlists a version on DELETE (it never deletes it)."""
    request = urllib.request.Request(
        PUBLISH.format(id=package, version=version),
        method="DELETE",
        headers={"X-NuGet-ApiKey": key, "User-Agent": AGENT},
    )
    with urllib.request.urlopen(request, timeout=60) as answer:
        return answer.status


def main():
    doing = "--yes" in sys.argv[1:]
    key = os.environ.get("NUGET_API_KEY", "")
    if doing and not key:
        sys.exit("NUGET_API_KEY is not set: the workflow's NuGet/login step gives it")
    named = [a for a in sys.argv[1:] if not a.startswith("-")]
    total = 0
    refused = 0
    for package in sorted(set(package_ids()) | set(named)):
        all_versions = versions(package)
        if not all_versions:
            print(f"{package}: not on nuget.org")
            continue
        newest = max((v for v, _ in all_versions), key=version_key)
        old = sorted((v for v, listed in all_versions if listed and v != newest), key=version_key)
        print(f"{package}: keeps {newest}, {len(old)} listed version(s) to unlist")
        for version in old:
            if doing:
                try:
                    status = unlist(package, version, key)
                    print(f"  unlisted {version} ({status})")
                except urllib.error.HTTPError as error:
                    print(f"  {version}: nuget.org answered {error.code} {error.reason}")
                    refused += 1
            else:
                print(f"  would unlist {version}")
        total += len(old)
    if not doing:
        print(f"\n{total} version(s) would be unlisted. Run again with --yes and NUGET_API_KEY to unlist them.")
    elif refused:
        sys.exit(f"\nnuget.org refused {refused} of {total} version(s)")


if __name__ == "__main__":
    main()
