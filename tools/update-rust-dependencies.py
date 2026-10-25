#!/usr/bin/env python3
"""Update the pinned Rust toolchain and Cargo.lock, then resync the crate inventory.

Steps (each can be skipped):
  1. Bump the pinned toolchain to the latest stable release from
     static.rust-lang.org, rewriting every tracked pin of the old version.
  2. Run `cargo update` with the (new) pinned toolchain.
  3. Rewrite the `rust-crate-*` records in third_party/dependencies.json so
     they match the registry crates in Cargo.lock, which is what
     tools/check-dependencies.ps1 enforces.

Prints a short summary; exits 0 when nothing changed as well.
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tomllib
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TOOLCHAIN_FILE = REPO / "rust-toolchain.toml"
INVENTORY_FILE = REPO / "third_party" / "dependencies.json"
LOCK_FILE = REPO / "Cargo.lock"
STABLE_CHANNEL_URL = "https://static.rust-lang.org/dist/channel-rust-stable.toml"
CRATE_PREFIX = "rust-crate-"
REGISTRY_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"

# Files that carry the full toolchain version (X.Y.Z).
VERSION_PIN_GLOBS = [
    "rust-toolchain.toml",
    "AGENTS.md",
    "tools/*.ps1",
    ".github/workflows/*.yml",
]
# Files that carry the README badge version (X.Y).
BADGE_GLOBS = ["README.md", "README.*.md"]


def cargo_env():
    env = dict(os.environ)
    # .cargo/config.toml points rustc-wrapper at the Windows sccache binary,
    # which does not exist outside a bootstrapped Windows checkout.
    env["RUSTC_WRAPPER"] = ""
    return env


def run(args):
    subprocess.run(args, cwd=REPO, env=cargo_env(), check=True)


def pinned_toolchain():
    match = re.search(r'(?m)^channel\s*=\s*"([^"]+)"', TOOLCHAIN_FILE.read_text())
    if not match:
        sys.exit("rust-toolchain.toml has no channel pin")
    return match.group(1)


def latest_stable():
    with urllib.request.urlopen(STABLE_CHANNEL_URL, timeout=60) as response:
        manifest = tomllib.loads(response.read().decode())
    # "1.99.1 (58cf8f9c3 2026-10-13)"
    return manifest["pkg"]["rust"]["version"].split()[0]


def version_key(version):
    return tuple(int(part) for part in version.split("."))


def tracked_files(globs):
    output = subprocess.run(
        ["git", "ls-files", "--", *globs], cwd=REPO, check=True, capture_output=True, text=True
    ).stdout
    return [REPO / line for line in output.splitlines() if line]


def replace_in_files(globs, pattern, replacement):
    changed = []
    for path in tracked_files(globs):
        text = path.read_text(encoding="utf-8")
        updated = pattern.sub(replacement, text)
        if updated != text:
            path.write_text(updated, encoding="utf-8")
            changed.append(path.relative_to(REPO).as_posix())
    return changed


def bump_toolchain():
    old = pinned_toolchain()
    new = latest_stable()
    if version_key(new) <= version_key(old):
        print(f"Rust toolchain {old} is current.")
        return None
    full = re.compile(r"(?<![\d.])" + re.escape(old) + r"(?![\d])")
    changed = replace_in_files(VERSION_PIN_GLOBS, full, new)
    old_minor = ".".join(old.split(".")[:2])
    new_minor = ".".join(new.split(".")[:2])
    badge = re.compile(r"badge/Rust-" + re.escape(old_minor) + r"-")
    changed += replace_in_files(BADGE_GLOBS, badge, f"badge/Rust-{new_minor}-")
    print(f"Rust toolchain {old} -> {new}: {', '.join(changed)}")
    return old, new


def normalize(name):
    return name.replace("-", "_")


def lock_registry_packages():
    lock = tomllib.loads(LOCK_FILE.read_text())
    return sorted(
        {
            (package["name"], package["version"])
            for package in lock["package"]
            if package.get("source") == REGISTRY_SOURCE
        },
        key=lambda item: (normalize(item[0]), version_key_loose(item[1])),
    )


def version_key_loose(version):
    return tuple(int(part) if part.isdigit() else 0 for part in re.split(r"[.+-]", version))


def ensure_toolchain():
    toolchain = pinned_toolchain()
    run(["rustup", "toolchain", "install", toolchain, "--profile", "minimal", "--no-self-update"])
    return toolchain


def crate_metadata():
    output = subprocess.run(
        ["cargo", f"+{ensure_toolchain()}", "metadata", "--format-version", "1", "--locked"],
        cwd=REPO,
        env=cargo_env(),
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    metadata = {}
    for package in json.loads(output)["packages"]:
        if package.get("source") != REGISTRY_SOURCE:
            continue
        source = package.get("repository") or f"https://crates.io/crates/{package['name']}"
        metadata[(package["name"], package["version"])] = {
            "source": source.removesuffix(".git"),
            "license": package.get("license") or "UNKNOWN",
        }
    return metadata


def entry_crate(entry):
    return entry.get("crate") or entry["name"][len(CRATE_PREFIX):]


def sync_inventory():
    text = INVENTORY_FILE.read_text(encoding="utf-8")
    inventory = json.loads(text)
    packages = inventory["packages"]
    crate_positions = [i for i, entry in enumerate(packages) if entry["name"].startswith(CRATE_PREFIX)]
    insert_at = crate_positions[-1] + 1 if crate_positions else len(packages)
    entries = [packages[i] for i in crate_positions]

    pending = lock_registry_packages()
    pending_set = set(pending)
    matched = set()
    kept = []
    unmatched_entries = []
    for entry in entries:
        key = next(
            (
                (name, version)
                for name, version in pending
                if normalize(name) == normalize(entry_crate(entry)) and version == entry["version"]
            ),
            None,
        )
        if key and key not in matched:
            matched.add(key)
            kept.append(entry)
        else:
            unmatched_entries.append(entry)

    changes = []
    for entry in unmatched_entries:
        candidates = [
            key
            for key in pending
            if key not in matched and normalize(key[0]) == normalize(entry_crate(entry))
        ]
        if not candidates:
            changes.append(f"- {entry_crate(entry)} {entry['version']}")
            entry["_removed"] = True
            continue
        key = candidates[-1]
        matched.add(key)
        old_version = entry["version"]
        entry["version"] = key[1]
        suffix = "-" + old_version
        if entry["name"].endswith(suffix):
            entry["name"] = entry["name"][: -len(suffix)] + "-" + key[1]
        changes.append(f"~ {key[0]} {old_version} -> {key[1]}")
        kept.append(entry)

    added = [key for key in pending if key not in matched]
    metadata = crate_metadata() if added else {}
    new_entries = []
    for name, version in added:
        info = metadata.get((name, version), {"source": f"https://crates.io/crates/{name}", "license": "UNKNOWN"})
        same_crate = [
            entry for entry in kept + new_entries if normalize(entry_crate(entry)) == normalize(name)
        ]
        record = {
            "name": f"{CRATE_PREFIX}{name}-{version}" if same_crate else f"{CRATE_PREFIX}{name}",
            "version": version,
            "source": info["source"],
            "license": info["license"],
        }
        if same_crate:
            record["crate"] = name
        new_entries.append(record)
        changes.append(f"+ {name} {version} ({info['license']})")

    rebuilt = []
    for i, entry in enumerate(packages):
        if i == insert_at:
            rebuilt.extend(new_entries)
        if entry.get("_removed"):
            continue
        rebuilt.append(entry)
    if insert_at >= len(packages):
        rebuilt.extend(new_entries)
    inventory["packages"] = rebuilt

    covered = {(normalize(entry_crate(e)), e["version"]) for e in rebuilt if e["name"].startswith(CRATE_PREFIX)}
    missing = [key for key in pending_set if (normalize(key[0]), key[1]) not in covered]
    if missing:
        sys.exit(f"inventory sync left crates untracked: {missing}")

    updated = json.dumps(inventory, indent=2, ensure_ascii=False) + "\n"
    if updated != text:
        INVENTORY_FILE.write_text(updated, encoding="utf-8")
    unknown = [r["name"] for r in new_entries if r["license"] == "UNKNOWN"]
    if unknown:
        sys.exit(f"crates without a declared license need manual review: {unknown}")
    if changes:
        print("Crate inventory:\n  " + "\n  ".join(changes))
    else:
        print("Crate inventory matches Cargo.lock.")


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--skip-toolchain", action="store_true", help="keep the pinned Rust toolchain")
    parser.add_argument("--skip-cargo-update", action="store_true", help="do not run cargo update")
    args = parser.parse_args()

    if not args.skip_toolchain:
        bump_toolchain()
    if not args.skip_cargo_update:
        run(["cargo", f"+{ensure_toolchain()}", "update"])
    sync_inventory()


if __name__ == "__main__":
    main()
