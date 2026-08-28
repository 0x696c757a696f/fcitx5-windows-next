# Task 075 - Control package-state Rust cutover

**Task ID:** `CONTROL-PACKAGE-STATE-RUST-CUTOVER-001`
**Mode:** CHANGE / RUST-FIRST
**Prerequisite:** 067-069 and 071; 074 external rows may remain `MANUAL-PENDING`.

## Goal

Move `fcitx5-control.exe --packages-state ID enabled|disabled` behavior to the Rust-owned
Control/package boundary while preserving its command contract.

## Scope and acceptance

- Freeze enabled/disabled, invalid ID/state, missing package, dependency-blocked, failed-write,
  and successful atomic-publication behavior.
- Add Rust behavior tests in `rust/control-core` and `rust/package-core`.
- Extend only the narrow Rust action ABI required; C++ may marshal native calls but may not own
  package-state policy or duplicate lifecycle logic.
- Keep `tests/integration/control_package_integration_test.cpp` only as a final mixed-binary
  boundary test; Rust is the behavior authority.

## Hard constraints

- Use PowerShell 7 at `D:\Program Files\PowerShell\7\pwsh.exe`.
- Toolchain root: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains`; prefer `...\fast`.
- Pinned Cargo: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home\bin\cargo.exe`.
- `CARGO_TARGET_DIR` must be inside the implementation worktree.
- New product code and tests are Rust. C++ is limited to native/ABI adapters and final mixed E2E.
- Do not modify Candidate, Launcher, Engine, UI, repository trust, or unrelated plugin commands.
- Do not claim real-host, signing, UAC, accessibility, Windows 7, offline, low-resource, or CI
  evidence from local tests.

## Acceptance

- Rust tests cover enabled/disabled, invalid ID/state, missing package, dependency blocking,
  failure without lockfile mutation, and atomic publication.
- x64/x86 Rust tests, fmt, clippy where clean, relevant CTest/source-contract and mixed smoke
  pass or exact environmental blockers are recorded.
- Source-contract evidence prevents reintroducing C++ package-state authority.
- `docs/tasks/status.md` records HEAD, files, tests, results, and limitations.

On completion, archive this task and select the next eligible Rust-first migration task. Keep
074 and `REL-01` release-gated while external evidence remains unavailable.
