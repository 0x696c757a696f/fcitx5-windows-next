# PLUGIN-LIFECYCLE-STABILITY-001 Signed add-on install/update/uninstall stability

**State:** TODO

## Context

The owner specifically called out plug-in online installation, update followed by uninstall, and
normal-use stability as one of the highest-risk areas. Historical Phase 7 notes had useful package
lifecycle requirements, but the old phase document was removed from `docs/` because it was stale
v1.5/v1.6 evidence. The retained current plan is `docs/tasks/plugin-install-update-stability-plan.md`.

This task must run after `CONFIG-RUST-CUTOVER-001` unless package lifecycle defects block Config
itself.

## Scope

- Verify signed repository refresh from a controlled fixture and, when available, the production
  package endpoint.
- Build official upstream Fcitx5 add-ons/plugins that this Windows product supports into reviewed
  Windows package artifacts, publish those artifacts plus signed repository metadata as GitHub
  Release assets for the matching source commit/release channel, and let Settings discover/download
  from that signed repository.
- Verify install/update/remove/repair state transitions through Rust package/control boundaries.
- Verify Config UI can reach refresh, install/update, enable/disable, remove, and repair actions
  without hidden raw package-manager UI, overlap, or clipping.
- Verify update followed by uninstall removes executable payloads and preserves user-owned or
  package-policy-protected data.
- Verify normal input remains usable before and after package lifecycle restart points.

## Must not do

- Do not show fake official online plug-ins. Online items require signed repository metadata and a
  trusted key.
- Do not point Settings at arbitrary upstream source repositories as if they were installable
  packages. The online catalog must list only compiled, reviewed, signed Windows package artifacts
  that this project published, initially via GitHub Releases.
- Do not let package/downloader/updater/provider/deployer/Config see realtime input data.
- Do not introduce hooks, `SendInput`, process injection, credential access, anti-cheat bypass, or
  external exploitation.
- Do not bypass signature, hash, anti-rollback, dependency, path, or architecture checks for UX
  convenience.

## Required validation

- x64/x86 package lifecycle regression suite:
  `package-core-contract`, `control-package-stopped-service-contract`,
  `deployment-previous-known-good`, `updater-cleanup-previous`,
  `package-manifest-path-fuzz-smoke`, and `package-path-corpus-fuzz-smoke`.
- Fixture package coverage rejects unknown fields, traversal, ADS/UNC/absolute paths, symlinks,
  encrypted entries, duplicate/undeclared files, excessive counts/sizes, revoked keys,
  hash/signature mismatch, architecture/Core API/addon ABI mismatch, missing/cyclic/inexact
  dependencies, and rollback/downgrade sequences.
- Config interaction coverage for package refresh, details, install/update, enable/disable, remove,
  diagnostics repair, busy-state guarding, and selection changes during an operation.
- Package smoke from `out/package` after lifecycle checks.
- Manual online endpoint check only if a real signed endpoint and trusted keyring are provisioned;
  otherwise record that subcase as `MANUAL-PENDING`.
- GitHub Release package catalog check: the repository metadata points to immutable release assets,
  each package hash/signature matches, architecture/Core API/addon ABI compatibility is enforced,
  and Settings shows only those verified official package entries as installable.

## Done when

- Fixture lifecycle tests are green on x64 and x86.
- Config package/add-on actions are covered by interaction tests and localized status strings.
- Official add-on/plugin packages are built by the project release workflow, attached to GitHub
  Releases with signed repository metadata, and consumed by Settings through the normal package
  verification path.
- Update-after-install and uninstall-after-update behavior is proven for executable payloads and
  user-data policy.
- Normal input smoke is green after lifecycle restart points, or the exact real-host gap is recorded
  as `MANUAL-PENDING`.
- `docs/tasks/status.md` records endpoint/key status, package id/version, install/update/uninstall
  result, and any manual blocker.
