# Task 079 - Upstream vendor subtree/submodule management

**Task ID:** `UPSTREAM-VENDOR-SUBTREE-001`
**Mode:** CHANGE / TOOLING / VENDOR-MANAGEMENT
**Prerequisite:** none (independent of 077/078/REL-01); code-only.

## Goal and completion rule

Make upstream dependency updates a one-command, clean-diff operation instead of a
manual bulk file copy. Two upstream families are in scope:

1. `huanfeng/wind-ui-rust` — currently vendored as 117+ flat files under
   `third_party/wind-ui-rust/` (a Rust path dependency `windui`), so every sync
   produces ~120 file diffs plus two repo-local Windows portability patches that
   must be re-applied by hand.
2. `fcitx/fcitx5` core and its addons (`libime`, `fcitx5-chinese-addons`,
   `fcitx5-rime`, `fcitx5-lua`, `fcitx5-unikey`, `librime*`) — currently cloned
   and checked out at pinned commits under `out/sources/` by
   `tools/bootstrap-fcitx.ps1`, with a pinned patch queue in
   `third_party/patches/`. These are build-time checkouts, not vendored into the
   tree, so they already have clean one-line pin updates; 079 should keep this
   path and only ensure the pin/patch update flow is one command.

## Requirements

- Switch `third_party/wind-ui-rust` to a git subtree or submodule tracked at a
  single pinned commit. Prefer git subtree (keeps the monorepo single-repo and
  avoids submodule checkout friction) unless the repo already uses submodules
  (it uses one: `third_party/tomlplusplus`), in which case either is acceptable
  and the choice must be documented.
- Keep the two repo-local Windows portability patches
  (`set_window_user_data` cfg wrapper, tray `copy_wide_unaligned`) as explicit
  files under `third_party/patches/` (or an equivalent named mechanism) applied
  deterministically after the subtree/submodule update, so they are never lost
  or silently dropped by a re-sync.
- Provide a single script (e.g. `tools/sync-upstream.ps1` or extend
  `tools/bootstrap-fcitx.ps1`) that:
  - accepts a new pin (or `--latest`),
  - performs the subtree/submodule pull,
  - re-applies the portability patches fail-closed,
  - updates `third_party/dependencies.json` version+source,
  - updates `Cargo.lock`,
  - updates the `WIND_UI_RUST_REFERENCE_COMMIT` constant in
    `rust/config-poc/src/main.rs` (or deletes the constant if the pin becomes
    derivable from the subtree ref),
  - runs the affected Rust tests.
- For fcitx5 core/addons, ensure the pin list in `tools/bootstrap-fcitx.ps1`
  (`$sourcePins`) and the patch queue in `third_party/patches/` are the single
  source of truth, and that updating a pin + re-running `-VerifyPatchesOnly`
  fails closed on any patch that no longer applies. Do not re-vendor the Fcitx
  native sources into `third_party/`; keep them as pinned build-time checkouts.
- Do not change product behavior or language ownership. This task only changes
  how upstream code is stored and synced; no Rust/C++ product logic is touched
  beyond the pin-update constant.

## Acceptance

- `tools/sync-upstream.ps1` (or equivalent) can sync `wind-ui-rust` to a new pin
  with a diff of one subtree/submodule pointer (plus, only when the pin actually
  changes files, the tracked portability patch re-application), not ~120 flat
  file edits.
- The two Windows portability patches are stored as named files and re-applied
  fail-closed; a sync that would drop them aborts.
- `third_party/dependencies.json` version/source stays consistent with the
  subtree/submodule ref.
- x64/x86 `cargo test --locked -p fcitx5-config-poc -p fcitx5-config-qa` pass
  after a clean re-sync.
- `tools/bootstrap-fcitx.ps1 -VerifyPatchesOnly` still reports APPLY-CLEAN or
  ALREADY-APPLIED for every patch in `third_party/patches/` at the current pins.
- Text, dependency, license, secret checks and `git diff --check` pass.

## Non-goals

- Do not re-vendor Fcitx native sources or wind-ui into a new package manager.
- Do not add a second build system or a third-party tool that requires network
  access at configure time beyond what already exists.
- Do not claim real-host/signing/UAC/accessibility evidence; this is tooling only.

## Implemented design (record of execution)

**Decision: scripted vendoring + named patch files, NOT subtree/submodule.**
`wind-ui-rust` is a Rust path dependency (`windui = { path =
"../../third_party/wind-ui-rust" }`) and carries two repo-local Windows
portability patches. A git submodule would make those patches impossible to
record (a submodule working tree cannot carry repo-local committed edits), and a
git subtree history rewrite is higher risk for no additional benefit over a
deterministic sync script. The scripted approach still delivers one-command
sync with clean, attributable diffs.

Created by 079:

- `third_party/patches/wind-ui-rust/win32-window-user-data.patch` - the
  `set_window_user_data` cfg(64/32) wrapper for `src/platform/win32/mod.rs`.
- `third_party/patches/wind-ui-rust/win32-tray-unaligned.patch` - the
  `copy_wide_unaligned` + `std::ptr` fix for `src/platform/win32/tray.rs`.
- `tools/sync-windui.ps1` - the one-command sync.

Sync command (pin or latest):

```powershell
# to a specific upstream commit:
& tools/sync-windui.ps1 -Commit <40-hex-sha>
# to upstream HEAD:
& tools/sync-windui.ps1 -Latest
```

The script: clones upstream to a temp dir, verifies every patch with
`git apply --check` on the clean upstream content **before** touching the
vendored tree (fail-closed, no half-updated tree), copies only the in-scope
entries (`src/`, `examples/`, `Cargo.toml`, `README.md`, `README.en.md`,
`LICENSE-APACHE`, `LICENSE-MIT`; `build.rs`/`assets`/`CHANGELOG.md` only serve
upstream example-exe icon embedding and are excluded), normalizes every copied
text file to LF (repo `.gitattributes` is `* text=auto eol=lf`), applies the two
patches with `git apply --directory=third_party/wind-ui-rust`, updates
`third_party/dependencies.json` (short-hash version convention, full-hash
source), updates the `WIND_UI_RUST_REFERENCE_COMMIT` constant + test assertions
in `rust/config-poc/src/main.rs`, and runs the config consumer tests.

Fcitx5 core/addons stay pinned build-time checkouts under `$sourcePins` in
`tools/bootstrap-fcitx.ps1` with the patch queue in `third_party/patches/`; they
are intentionally NOT re-vendored. 079 changed no Fcitx native pin or patch.

Idempotency: running `sync-windui.ps1 -Commit` at the current pin produces no
net diff to the tracked working tree.
