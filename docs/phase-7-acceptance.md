# Phase 7 acceptance

Date: 2026-08-17  
Specification: Frozen v1.5  
Scope: Package, addon, repository, provider, plugin manager, candidate scroll mode

## Evidence

- The package core strictly parses signed manifests and repository indexes, rejects unknown fields,
  traversal, ADS/UNC/absolute paths, symlinks, encrypted entries, duplicates, undeclared files,
  excessive counts/sizes, revoked keys, hash/signature mismatch, architecture/Core API/addon ABI
  mismatch, missing/cyclic/inexact dependencies, and downgrade sequences.
- Downloader alone links WinHTTP. Package, updater, deployer, provider, engine, UI, and TSF do not.
  The minimal elevated deployer consumes only an already verified fixed-root transaction.
- Staging and lockfile publication are atomic. Enable/disable requires an engine restart; uninstall is
  `pending_remove` then restart/finalize and does not delete external user dictionaries/configuration.
- Config's plugin manager refreshes the signed online index and supports install, update, enable,
  disable, uninstall, repair and explicit error state through typed Control commands.
- Rime/Plum is an isolated provider with an explicit user-data directory. A `fcitx5-rime` package and
  optional `librime-lua` package must declare exact runtime dependencies; incompatibility fails closed.
- Candidate protocol v5 exposes bounded bulk candidates. C++ D2D/DWrite renders the 6×6 viewport,
  current-row labels, selected-item reveal and scrollbar. It starts collapsed, expands across a page
  boundary, and collapses on page 1 → 0, following the mature macOS and Rabbit behavior.
- Unit, archive/repository signature, crash-consistency, removal/user-data, provider policy,
  candidate-layout and fuzz smoke tests run on x64 and x86 in the package gate.

No addon code enters the TSF host process and no package component sees real-time input data.
