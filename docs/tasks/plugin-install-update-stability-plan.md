# Plugin install/update/uninstall stability plan

**Scope:** Fcitx5 for Windows Next package/add-on lifecycle stability.
**Created from HEAD:** `d557e4809cb26c0697169c49294fff2cd8126061`
**Execution state:** QUEUED as `049-PLUGIN-LIFECYCLE-STABILITY-001` / not yet executed.

This plan is intentionally separate from the current `CONFIG-UX-009` and
`CONFIG-RUST-CUTOVER-001` tasks. It should be executed after the Config theme/preview behavior
contract and Rust Config cutover are green, unless a package/add-on failure blocks Config itself.

## User concern

The highest-risk product area to verify after the Settings rewrite is:

- online plug-in/add-on installation;
- update followed by uninstall;
- normal-use stability after install/update/remove operations.
- official Fcitx5 add-ons/plugins should be compiled by this project for Windows, published as
  signed GitHub Release package assets, and discovered by Settings through signed repository
  metadata rather than by scraping source repositories.

## Boundaries

In scope:

- repository refresh from a controlled fixture or configured package endpoint;
- official add-on build/release inventory: supported upstream add-ons are built into Windows package
  artifacts, reviewed for license/dependencies/ABI/architecture support, and attached to the
  matching GitHub Release with signed repository metadata;
- signed package verification;
- install/update/remove state transitions;
- package repair/verification;
- preservation of package-owned user data during uninstall;
- Config UI wiring for refresh/install-or-update/enable-disable/remove actions;
- x64 and x86 automated checks.

Out of scope for this plan:

- new package format design;
- Rust migration;
- unreviewed dynamic source builds on the user's machine;
- displaying upstream source repositories as installable packages before this project has built,
  signed, and published corresponding Windows artifacts;
- installer/TSF in-use update generation semantics covered by `REG-UPDATE-TSF`;
- prohibited techniques such as hooks, input emulation, process injection, anti-cheat bypass, credential access, or external exploitation.

## Existing evidence to reuse

The repository already has focused lifecycle coverage:

- `package-core-contract`
  - package path and manifest semantics;
  - install/remove state handling;
  - pending removal and user-data preservation.
- `control-package-stopped-service-contract`
  - add-on descriptor inventory;
  - package install while the launcher service is not running;
  - package detail;
  - disable/enable;
  - update to a newer package version;
  - package repair;
  - remove after update;
  - executable payload removal;
  - package-owned user data preservation.
- `deployment-previous-known-good`
  - previous-known-good deployment/rollback behavior.
- `updater-cleanup-previous`
  - updater cleanup of previous generations/artifacts.
- `package-manifest-path-fuzz-smoke` and `package-path-corpus-fuzz-smoke`
  - package path hardening smoke coverage.

Retained requirements from the removed historical Phase 7 acceptance record:

- package core must reject unknown fields, traversal, ADS/UNC/absolute paths, symlinks, encrypted
  entries, duplicates, undeclared files, excessive counts/sizes, revoked keys, hash/signature
  mismatch, architecture/Core API/addon ABI mismatch, missing/cyclic/inexact dependencies, and
  rollback/downgrade sequences;
- downloader remains the only component with the package network boundary; package, updater,
  deployer, provider, engine, UI, Config, and TSF must not receive realtime input data through the
  package path;
- update followed by uninstall must remove executable payloads while preserving user-owned and
  policy-protected package data;
- online refresh remains blocked unless a production endpoint and trusted keyring are provisioned
  and verified by the package gate;
- the first production endpoint may be a GitHub Release-backed signed repository: CI/release builds
  the official plugin packages, uploads immutable assets, signs metadata with the configured trusted
  key, and Settings only displays packages whose hashes/signatures and compatibility checks pass.

## Execution order

### 1. Finish current Config task first

Complete and verify `REG-CONFIG-VISUAL-001` before changing package internals.

Required x64/x86 Config checks:

```powershell
cmake --build 'out/build/windows-x64-dev' --config Debug --target fcitx5_config_app fcitx5_source_contract_test fcitx5_release_identity_test fcitx5_config_parser_test --parallel
ctest --test-dir 'out/build/windows-x64-dev' -C Debug --output-on-failure -R 'config-ui-(i18n-check|resource-check|visual-contract|live-preview-contract|behavior-contract|interaction-coverage)|source-contract|release-identity-contract|config-toml-contract'

cmake --build 'out/build/windows-x86-dev' --config Debug --target fcitx5_config_app fcitx5_source_contract_test fcitx5_release_identity_test fcitx5_config_parser_test --parallel
ctest --test-dir 'out/build/windows-x86-dev' -C Debug --output-on-failure -R 'config-ui-(i18n-check|resource-check|visual-contract|live-preview-contract|behavior-contract|interaction-coverage)|source-contract|release-identity-contract|config-toml-contract'
```

Acceptance:

- the modern Settings surface has no visible default raw package-manager layout;
- all modern package/add-on actions exposed by Config are wired to existing package commands;
- visual contracts still prove no overlap at required DPI/minimum-size cases.

### 2. Run package lifecycle regression suite

Run the package/update/deployment tests on x64:

```powershell
ctest --test-dir 'out/build/windows-x64-dev' -C Debug --output-on-failure -R 'package-core-contract|control-package-stopped-service-contract|deployment-previous-known-good|updater-cleanup-previous|package-(manifest-path|path-corpus)-fuzz-smoke'
```

Run the same suite on x86:

```powershell
ctest --test-dir 'out/build/windows-x86-dev' -C Debug --output-on-failure -R 'package-core-contract|control-package-stopped-service-contract|deployment-previous-known-good|updater-cleanup-previous|package-(manifest-path|path-corpus)-fuzz-smoke'
```

Acceptance:

- install succeeds from the controlled package repository fixture;
- update publishes the newer verified payload atomically;
- remove after update removes executable package payloads;
- remove after update does not delete package-owned user data;
- repair verifies the installed set;
- prior generation cleanup behavior remains green;
- x64 and x86 results match.

### 3. Verify Config-to-package UI paths

After the modern Settings implementation exposes package actions, add or extend automated interaction coverage so the default UI can exercise:

- package refresh;
- package card selection/details;
- install or update action;
- enable/disable action;
- remove action;
- diagnostics repair action.

Acceptance:

- the test uses the public Config interaction contract, not private implementation details;
- package actions are reachable without showing overlapping or clipped controls;
- native raw listbox/details controls remain hidden on the default Add-ons/Updates pages.

### 4. Manual online check, only if automated tests are green

Use a disposable data root and the intended signed package endpoint. For the initial official
catalog, the endpoint should resolve to GitHub Release assets generated by this project's release
workflow, not to raw upstream plugin source trees.

Manual checklist:

- refresh online package index;
- install one non-core add-on;
- restart Fcitx5 for Windows Next normally;
- confirm candidate/input behavior remains usable;
- update the same add-on if a newer fixture/version is available;
- remove the add-on after update;
- restart again;
- confirm the add-on payload is gone and package-owned user data policy is respected.

Record in `docs/tasks/status.md`:

- OS build;
- architecture;
- app build identity;
- endpoint used;
- package id/version installed;
- package id/version updated to;
- uninstall result;
- whether normal input continued working before and after restart.

If no real online package update is available, mark only that manual subcase as `MANUAL-PENDING`; do not mark it passed from fixture-only tests.

## Blockers to report immediately

- package verification accepts an unsigned or wrongly signed package;
- repository rollback is accepted without an explicit downgrade path;
- update succeeds but uninstall leaves executable payloads behind;
- uninstall deletes user-owned/package-owned data outside the documented policy;
- Config exposes install/update/remove controls that can overlap, clip, or trigger the wrong package;
- x64 and x86 package lifecycle results diverge.

## Done when

- current Config visual task is green;
- package lifecycle regression suite is green on x64 and x86;
- Config package actions are covered by interaction tests;
- any real online check gap is recorded as `MANUAL-PENDING` with the exact missing evidence;
- `docs/tasks/status.md` contains the final evidence summary.
