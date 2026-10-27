#!/usr/bin/env python3
"""Pin tools/fork-plugin-inventory.json to the newest fcitx5-plugins release.

Finds the newest dated windows-YYYYMMDD-N release of the fork named in the
inventory, reads its SHA256SUMS, and rewrites the release tag, the commit and
the hash of every archive a package uses. Archives the release no longer has
are an error; archives no package uses yet are listed so they can be added by
hand (a new package needs a reviewed title, summary and license).

Set GITHUB_TOKEN to raise the API rate limit. Prints a short summary; exits 0
when the inventory is already current.
"""

import argparse
import json
import os
import re
import sys
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
INVENTORY_FILE = REPO / "tools" / "fork-plugin-inventory.json"
BUILT_HERE_FILE = REPO / "tools" / "release-plugin-inventory.json"
# Built into the base package by tools/bootstrap-fcitx.ps1 rather than shipped as a plugin.
BASE_ADDONS = {"chinese-addons"}
RELEASE_TAG = re.compile(r"^windows-(\d{8})-(\d+)$")
SHA256_LINE = re.compile(r"^([0-9a-f]{64})\s+\*?(\S+)$")
COMMIT = re.compile(r"^[0-9a-f]{40}$")


def fetch(url, accept="application/vnd.github+json"):
    request = urllib.request.Request(url, headers={"Accept": accept, "User-Agent": "fcitx5-windows-next"})
    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")
    if token and url.startswith("https://api.github.com/"):
        request.add_header("Authorization", f"Bearer {token}")
    with urllib.request.urlopen(request, timeout=60) as response:
        return response.read()


def newest_release(slug):
    releases = json.loads(fetch(f"https://api.github.com/repos/{slug}/releases?per_page=50"))
    dated = []
    for release in releases:
        match = RELEASE_TAG.match(release["tag_name"])
        if match and not release["draft"]:
            dated.append(((match.group(1), int(match.group(2))), release))
    if not dated:
        sys.exit(f"{slug} has no windows-YYYYMMDD-N release.")
    return max(dated, key=lambda item: item[0])[1]


def release_commit(slug, release):
    target = release["target_commitish"]
    if COMMIT.match(target):
        return target
    commit = json.loads(fetch(f"https://api.github.com/repos/{slug}/commits/{target}"))["sha"]
    if not COMMIT.match(commit):
        sys.exit(f"Could not resolve the commit of {release['tag_name']}.")
    return commit


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--release", help="pin this release tag instead of the newest one")
    args = parser.parse_args()

    inventory = json.loads(INVENTORY_FILE.read_text(encoding="utf-8"))
    match = re.match(r"^https://github\.com/([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+?)\.git$", inventory["repository"])
    if not match:
        sys.exit("Fork plugin inventory has an invalid repository URL.")
    slug = match.group(1)

    if args.release:
        release = json.loads(fetch(f"https://api.github.com/repos/{slug}/releases/tags/{args.release}"))
    else:
        release = newest_release(slug)
    tag = release["tag_name"]
    if not RELEASE_TAG.match(tag):
        sys.exit(f"{tag} is not a dated windows release.")
    if tag == inventory["release"]:
        print(f"Fork plugins already pinned to {tag}.")
        return 0

    sums = {}
    text = fetch(f"https://github.com/{slug}/releases/download/{tag}/SHA256SUMS", "application/octet-stream")
    for line in text.decode("utf-8").splitlines():
        if line.strip():
            entry = SHA256_LINE.match(line.strip())
            if not entry:
                sys.exit(f"Unexpected SHA256SUMS line in {tag}: {line!r}")
            sums[entry.group(2)] = entry.group(1)

    used = sorted({archive for package in inventory["packages"] for archive in package["archives"]})
    missing = [archive for archive in used if archive not in sums]
    if missing:
        sys.exit(f"{tag} no longer ships archives the inventory uses: {', '.join(missing)}")

    old = inventory["archives"]
    previous = inventory["release"]
    inventory["release"] = tag
    inventory["commit"] = release_commit(slug, release)
    inventory["archives"] = {archive: sums[archive] for archive in used}
    INVENTORY_FILE.write_text(json.dumps(inventory, indent=2) + "\n", encoding="utf-8")

    changed = [archive for archive in used if old.get(archive) != sums[archive]]
    built_here = BASE_ADDONS | {
        package["id"].removeprefix("fcitx5-")
        for package in json.loads(BUILT_HERE_FILE.read_text(encoding="utf-8"))["packages"]
    }
    unused = sorted(
        name for name in sums
        if name not in inventory["archives"] and "arm64" not in name
        and re.sub(r"-(any|x86_64)\.tar\.bz2$", "", name) not in built_here
    )
    print(f"Fork plugins: {previous} -> {tag} ({inventory['commit'][:7]})")
    print(f"Changed archives ({len(changed)}): {', '.join(changed) or 'none'}")
    if unused:
        print(f"Not packaged yet ({len(unused)}): {', '.join(unused)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
