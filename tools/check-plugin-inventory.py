#!/usr/bin/env python3
"""Check the plugin package inventories against what the build actually installed.

tools/release-plugin-inventory.json lists the payload of each plugin this
repository builds by hand. Only the release builds those packages, so a stale
name (an addon library that installs as libfoo.dll, not foo.dll) or a missing
file (the input method registration) used to surface only when a release ran.

This derives each package's payload from the Fcitx metadata in the package
stage, starting from the addon .conf files the inventory names:

  * a SharedLibrary addon needs lib/fcitx5/<Library>.dll;
  * a Lua addon needs share/fcitx5/lua/<addon>/<Library>;
  * every share/fcitx5/inputmethod/*.conf whose Addon= is one of the package's
    addons belongs to the package.

The inventory payload must be exactly that set plus any extra files it lists
that exist in the stage, and every DLL it lists must be some addon's library.

Packages that are not bundled leave the core: tools/stage-package.ps1 moves
their payload to plugin-payloads next to the stage, so they are checked there,
and none of their files may be left in the stage. An inputmethod-data package
lists input method files whose addon the stage ships, and every table
dictionary they name (File=, relative to share/fcitx5).

With --fork-root, each prepared fork plugin payload (tools/prepare-fork-plugins.ps1)
is also checked: its addon libraries and input method addons must resolve
within the package or the stage.

Exits 1 and prints the corrected payload when the inventory is out of date.
"""

import argparse
import json
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
INVENTORY_FILE = REPO / "tools" / "release-plugin-inventory.json"
ADDON_DIR = "share/fcitx5/addon"
INPUTMETHOD_DIR = "share/fcitx5/inputmethod"
LIBRARY_DIR = "lib/fcitx5"
LUA_DIR = "share/fcitx5/lua"


def read_conf(path):
    """Parse a Fcitx .conf file into {section: {key: value}}, skipping localized keys."""
    sections = {}
    current = sections.setdefault("", {})
    for raw in path.read_text(encoding="utf-8-sig").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("[") and line.endswith("]"):
            current = sections.setdefault(line[1:-1], {})
            continue
        key, sep, value = line.partition("=")
        if sep and "[" not in key:
            current[key.strip()] = value.strip()
    return sections


def addon_files(root, rel, addon):
    """Files (relative to root) that the addon conf at rel needs, or None when unknown."""
    info = read_conf(root / rel).get("Addon", {})
    kind = info.get("Type", "SharedLibrary")
    library = info.get("Library", "")
    if kind == "SharedLibrary":
        if library.startswith("export:"):
            library = library[len("export:"):]
        return [f"{LIBRARY_DIR}/{library}.dll"] if library else []
    if kind == "Lua":
        return [f"{LUA_DIR}/{addon}/{library}"] if library else []
    return []


def inputmethods_by_addon(root):
    result = {}
    folder = root / INPUTMETHOD_DIR
    if folder.is_dir():
        for conf in sorted(folder.glob("*.conf")):
            addon = read_conf(conf).get("InputMethod", {}).get("Addon", "")
            result.setdefault(addon, []).append(f"{INPUTMETHOD_DIR}/{conf.name}")
    return result


def addon_name(rel):
    return Path(rel).stem


def check_data_package(root, stage, payload):
    problems = []
    stage_addons = {p.stem for p in (stage / ADDON_DIR).glob("*.conf")}
    methods = [rel for rel in payload if rel.startswith(INPUTMETHOD_DIR + "/") and rel.endswith(".conf")]
    if not methods:
        problems.append(f"lists no {INPUTMETHOD_DIR}/*.conf")
    for rel in methods:
        if not (root / rel).is_file():
            continue
        conf = read_conf(root / rel)
        addon = conf.get("InputMethod", {}).get("Addon", "")
        if addon not in stage_addons:
            problems.append(f"{rel} uses addon {addon!r}, which the stage does not ship")
        table = conf.get("Table", {}).get("File", "")
        if table and (table.startswith("/") or ":" in table or f"share/fcitx5/{table}" not in payload):
            problems.append(f"{rel} names table {table!r}, which is not a share/fcitx5 file of the package")
    return problems


def check_stage_packages(stage, inventory):
    errors = []
    pool = stage.parent / "plugin-payloads"
    for package in inventory["packages"]:
        pid = package["id"]
        payload = list(package["payload"])
        root = stage if package["bundled"] else pool
        problems = [f"listed but not installed: {rel}" for rel in payload if not (root / rel).is_file()]
        if not package["bundled"]:
            problems += [f"left in the core payload: {rel}" for rel in payload if (stage / rel).exists()]
        if package["type"] == "inputmethod-data":
            problems += check_data_package(root, stage, payload)
            if problems:
                errors.append((pid, problems, None))
            continue
        methods = inputmethods_by_addon(root)
        addons = [rel for rel in payload if rel.startswith(ADDON_DIR + "/") and rel.endswith(".conf")]
        if not addons:
            problems.append(f"lists no {ADDON_DIR}/*.conf")
        expected = set(addons)
        libraries = set()
        for rel in addons:
            if not (root / rel).is_file():
                continue
            needed = addon_files(root, rel, addon_name(rel))
            for need in needed:
                if not (root / need).is_file():
                    problems.append(f"{rel} needs {need}, which the build did not install")
            libraries.update(n for n in needed if n.startswith(LIBRARY_DIR + "/"))
            expected.update(needed)
            expected.update(methods.get(addon_name(rel), []))
        for rel in sorted(expected - set(payload)):
            if (root / rel).is_file():
                problems.append(f"missing from payload: {rel}")
        for rel in payload:
            if rel.startswith(LIBRARY_DIR + "/") and rel.endswith(".dll") and rel not in libraries:
                problems.append(f"{rel} is not the library of any addon the package lists")
        if problems:
            extras = [rel for rel in payload if rel not in expected and (root / rel).is_file()
                      and not (rel.startswith(LIBRARY_DIR + "/") and rel.endswith(".dll"))]
            corrected = sorted(r for r in expected | set(extras) if (root / r).is_file())
            errors.append((pid, problems, corrected))
    return errors


def check_fork_packages(stage, fork_root):
    errors = []
    stage_addons = {p.stem for p in (stage / ADDON_DIR).glob("*.conf")} if (stage / ADDON_DIR).is_dir() else set()
    for root in sorted(p for p in fork_root.iterdir() if p.is_dir()):
        problems = []
        addon_dir = root / ADDON_DIR
        addons = {p.stem for p in addon_dir.glob("*.conf")} if addon_dir.is_dir() else set()
        for addon in sorted(addons):
            rel = f"{ADDON_DIR}/{addon}.conf"
            for need in addon_files(root, rel, addon):
                if not (root / need).is_file() and not (stage / need).is_file():
                    problems.append(f"{rel} needs {need}, which neither the package nor the stage has")
        for addon, confs in inputmethods_by_addon(root).items():
            if addon and addon not in addons and addon not in stage_addons:
                problems += [f"{conf} uses addon {addon!r}, which is not installed" for conf in confs]
        if problems:
            errors.append((root.name, problems, None))
    return errors


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--stage", required=True, type=Path, help="verified package stage")
    parser.add_argument("--fork-root", type=Path, help="prepared fork plugin payloads")
    args = parser.parse_args()

    stage = args.stage.resolve()
    if not (stage / "bin" / "fcitx5-engine.exe").is_file():
        sys.exit(f"Not a package stage: {stage}")
    inventory = json.loads(INVENTORY_FILE.read_text(encoding="utf-8"))
    errors = check_stage_packages(stage, inventory)
    if args.fork_root:
        errors += check_fork_packages(stage, args.fork_root.resolve())

    if not errors:
        count = len(inventory["packages"])
        print(f"Plugin inventory matches the stage ({count} built packages"
              + (", fork payloads checked)." if args.fork_root else ")."))
        return 0
    for pid, problems, corrected in errors:
        print(f"::error::Plugin package {pid} does not match the build output:")
        for problem in problems:
            print(f"  - {problem}")
        if corrected is not None:
            print(f"  Payload for {pid} in tools/release-plugin-inventory.json should be:")
            print("  " + json.dumps(corrected, indent=2).replace("\n", "\n  "))
    return 1


if __name__ == "__main__":
    sys.exit(main())
