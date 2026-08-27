# Task 064 - Config Candidate and plugin usability correction

**Mode:** CODE / PRODUCT CORRECTION
**Task ID:** `CONFIG-CANDIDATE-PLUGIN-USABILITY-CORRECTION-001`
**Result:** `MANUAL-PENDING / AUTOMATED-CORRECTION-GREEN / PRODUCTION-PUBLISH-INPUTS-PREPARED`

## Goal

Correct the Candidate layout regression and prepare one real supported Windows plugin lifecycle
through a production signed repository path.

## Completed automated scope

- Restored Automatic, Horizontal, Vertical, Scroll, `6 x N`, and `N x 6` in the default WindUI
  Appearance page. `N` derives from authoritative `candidate.page_size`; layout/page size persist
  through typed Control and authoritative reread.
- Added a reviewed x64 `fcitx5-rime` production inventory and immutable GitHub Release repository
  endpoint.
- Added Rust-owned strict v2 ML-DSA-65 envelope formatting/verification, release-only signing,
  package/index artifact generation, anti-rollback sequence checks, and protected release promotion.
- Kept C++ limited to the direct Fcitx Engine adapter and thin Rust ABI DTO consumption. Engine
  addon discovery accepts only Rust-verified enabled package directories.

## Validation

- x64/x86 Config Rust tests: 20/20 per architecture; x64/x86 source contract passed.
- x64/x86 package-core library tests: 30/30 per architecture.
- x64/x86 release signer smoke and focused CTest: 7/7 per architecture.
- GNU Engine build; source/no-secret, secret scan, runtime security source, dependency, license,
  text, Rust format, and diff checks passed.

## Manual pending

- Provision the protected 4032-byte ML-DSA-65 secret and matching scoped/channel v2 keyring.
- Provision Authenticode PFX/password and GitHub protected `release` environment/publication rights.
- Publish immutable `fcitx5-rime-<version>-x64.fcpkg`, `index.json`, and `index.sig.json` assets.
- On real x64/x86 hosts, refresh, install, enable, disable, update, remove, and restart through
  Settings. No local fixture or unrun host flow is accepted as production evidence.

The reachable implementation is complete. `docs/tasks/current.md` returns to `RELEASE-01`; release
readiness remains blocked by the external evidence above plus the release gate's UAC, accessibility,
and real-host requirements.
