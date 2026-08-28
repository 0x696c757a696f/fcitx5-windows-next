# Task 075 - Control package-state Rust cutover

**Task ID:** `CONTROL-PACKAGE-STATE-RUST-CUTOVER-001`
**Mode:** CHANGE / RUST-FIRST
**Prerequisite:** 067-069 and 071; 074 external rows may remain `MANUAL-PENDING`.

## Goal

Move the `fcitx5-control.exe --packages-state ID enabled|disabled` behavior to the Rust-owned
Control/package boundary. Preserve the existing command contract while making validation,
dependency constraints, lockfile mutation, atomic publication, and error classification explicit
Rust behavior.

## Scope

- Freeze the current enabled/disabled, invalid ID/state, missing package, dependency-blocked,
  failed-write, and successful atomic-publication behavior.
- Add Rust behavior tests in `rust/control-core` and `rust/package-core` for the package-state
  command and its lifecycle effects.
- Extend the existing narrow Rust action ABI only as required; C++ may marshal UTF-16/native
  process or reload calls, but may not own package-state policy or duplicate lifecycle logic.
- Update `src/control/control_main.cpp` around `packageAction`, package action dispatch, and
  `wmain` only as a thin adapter.
- Keep `tests/integration/control_package_integration_test.cpp` only as a final mixed-binary
  boundary test; Rust is the behavior authority.

## Hard constraints

- Use PowerShell 7 at `D:\Program Files\PowerShell\7\pwsh.exe`.
- Toolchain root is `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains`; prefer
  `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast`.
- Use pinned Cargo at
  `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home\bin\cargo.exe`.
- `CARGO_TARGET_DIR` must be inside the implementation worktree.
- New product code and tests are Rust. C++ is limited to native/ABI adapters and final mixed E2E.
- Do not modify Candidate, Launcher, Engine, UI, repository trust, or unrelated plugin commands.
- Do not claim real-host, signing, UAC, accessibility, Windows 7, offline, low-resource, or CI
  evidence from local tests.

## Acceptance

- Rust tests cover enabled/disabled, invalid ID/state, missing package, dependency blocking,
  failure without lockfile mutation, and atomic publication.
- x64 and x86 Rust tests/fmt/clippy where clean pass; relevant `package-core-contract`,
  `control-package-stopped-service-contract`, source-contract, and mixed smoke are run or their
  exact environmental blockers are recorded.
- Source-contract evidence prevents reintroducing C++ package-state authority.
- `docs/tasks/status.md` records HEAD, files, tests, results, and limitations.

On completion, archive this task and select the next eligible Rust-first migration task. Keep
074 and `REL-01` release-gated while external evidence remains unavailable.
