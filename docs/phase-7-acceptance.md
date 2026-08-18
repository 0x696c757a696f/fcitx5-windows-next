# Phase 7 acceptance

> Status: historical v1.5 evidence; Phase 7 is not currently accepted under v1.6.

Date: 2026-08-17  
Specification: Frozen v1.5  
Scope: Package, addon, repository, provider, plugin manager, candidate scroll mode
Status: local implementation/package acceptance passed; production online repository blocked

## Evidence

- The package core strictly parses signed manifests and repository indexes, rejects unknown fields,
  traversal, ADS/UNC/absolute paths, symlinks, encrypted entries, duplicates, undeclared files,
  excessive counts/sizes, revoked keys, hash/signature mismatch, architecture/Core API/addon ABI
  mismatch, missing/cyclic/inexact dependencies, and downgrade sequences.
- Downloader alone links WinHTTP. Package, updater, deployer, provider, engine, UI, and TSF do not.
  The minimal elevated deployer consumes only an already verified fixed-root transaction.
- Staging and lockfile publication are atomic. Enable/disable requires an engine restart; uninstall is
  `pending_remove` then restart/finalize and does not delete external user dictionaries/configuration.
- Config's plugin manager and typed Control commands implement signed-index refresh, install,
  update, enable, disable, uninstall and repair. The production endpoint and non-empty trusted
  keyring are release infrastructure, not repository defaults; until both are provisioned and the
  package gate has consumed a signed fixture, online refresh is **not accepted** and must report an
  explicit unavailable/error state.
- Rime/Plum is an isolated provider with an explicit user-data directory. A `fcitx5-rime` package and
  optional `librime-lua` package must declare exact runtime dependencies; incompatibility fails closed.
- Candidate protocol v6 exposes bounded bulk candidates and explicit Shift/Control/Alt/Super key
  semantics. C++ D2D/DWrite renders the 6×6 viewport,
  current-row labels, selected-item reveal and scrollbar. It starts collapsed, expands across a page
  boundary, and collapses on page 1 → 0, following the mature macOS and Rabbit behavior.
- Unit, archive/repository signature, crash-consistency, removal/user-data, provider policy,
  candidate-layout and fuzz smoke tests run on x64 and x86 in the package gate. Real Fcitx
  acceptance additionally exercises Pinyin, `fcitx5-lua`, Rime, Rime Lua, and the
  `fcitx5-chttrans` Ctrl+Shift+F simplified/traditional conversion path.

No addon code enters the TSF host process and no package component sees real-time input data.

## Open release blocker

- Publish and operate `https://packages.fcitx5-windows.org/v1/<channel>` (or update the pinned
  endpoint through an ADR), inject the protected public keyring, then pass refresh/download/
  update/uninstall against the real service. An empty development keyring intentionally fails
  closed and cannot be described as a working online store.
