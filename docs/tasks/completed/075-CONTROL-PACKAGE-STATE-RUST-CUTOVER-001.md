# Task 075 - Control package-state Rust cutover

**Task ID:** `CONTROL-PACKAGE-STATE-RUST-CUTOVER-001`
**Result:** COMPLETED / RUST-DEPENDENCY-SAFE-STATE-GREEN / MANUAL-PENDING

Installed package state transitions now read installed manifests and apply Rust-owned dependency
policy before atomically publishing `packages.lock`. Disabling a package required by an active
dependent returns `package_in_use`; enabling a package with an unavailable or inactive exact
dependency returns `dependency_unavailable`. The existing narrow package FFI and C++ native
reload adapter remain; no C++ package policy was added.

## Evidence

- Final implementation HEAD: `648f7957ee031adbd6d99e10cf293ea9db49a4be`.
- Changed product file: `rust/package-core/src/lib.rs`.
- Rust tests: x64/x86 package-core `38/38` each, x64/x86 clippy with `-D warnings`.
- `cargo fmt --all -- --check` and `git diff --check` passed.

## Limitations

Correct-path x64 CMake configure and source-contract test build passed. Resource/plugin CTest
contracts passed; `control-package-stopped-service-contract` still fails on the pre-existing
missing `--set-presentation-font` CLI behavior, and `source-contract` still fails on a pre-existing
Candidate UX marker. The pinned CMake is at
`out/toolchains/fast/cmake-3.31.8/cmake-3.31.8-windows-x86_64/bin/cmake.exe`. No real-host,
production-signing, UAC, accessibility, offline, low-resource,
Windows 7, or online CI evidence is inferred. Task 074 and `REL-01` remain gated.
