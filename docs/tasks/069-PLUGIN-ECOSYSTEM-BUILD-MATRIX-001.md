# Task 069 - Plugin ecosystem build matrix

**Mode:** CHANGE / CODE-ONLY / EXTERNAL-EVIDENCE-PARTIAL
**Task ID:** `PLUGIN-ECOSYSTEM-BUILD-MATRIX-001`
**Prerequisite:** `067` provenance/data boundary and `068` repository freshness contracts green.
**Evidence class:** deterministic x64 build/package/install/load smokes; production publication remains manual.

## Goal

Extend the existing signed Rime production-input path with separate real Lua and one non-Chinese
addon vertical slices: pinned upstream source -> standard Fcitx addon/build-farm build -> package ->
sign-input -> install/load smoke.

## Constraints and acceptance

- Read `ponytail`, `rust-skills`, and `tdd`; use worktree-local `CARGO_TARGET_DIR`. Add one addon
  per vertical slice, use upstream standard Fcitx addon semantics, and do not create a second plugin API.
- Keep Engine native-addon architecture claims exact: only build/test architectures actually supported
  by the Engine are declared. Do not claim ARM64 or x86 Engine native-addon support.
- Preserve the ML-DSA-65 v2 sign-input path. Each slice records pinned source/provenance, declared
  `runtime_abi` and `runtime_build`, dependencies, and deterministic x64 install/load smoke evidence.
- Real online publication, protected signing material, Authenticode, UAC, and production-host lifecycle
  remain `MANUAL-PENDING`.
