# Task 069 - Plugin ecosystem build matrix

**Mode:** COMPLETED / CODE-ONLY / EXTERNAL-EVIDENCE-PARTIAL
**Task ID:** `PLUGIN-ECOSYSTEM-BUILD-MATRIX-001`
**Prerequisite:** `067` provenance/data boundary and `068` repository freshness contracts green.
**Evidence class:** deterministic x64 build/package/install/load smokes; production publication remains manual.

## Goal

Extend the existing signed Rime production-input path with separate real Lua and one non-Chinese
addon vertical slices: pinned upstream source -> standard Fcitx addon/build-farm build -> package ->
sign-input -> install/load smoke.

## Constraints and acceptance

- Read `rust-skills`; use worktree-local `CARGO_TARGET_DIR`. Add one addon
  per vertical slice, use upstream standard Fcitx addon semantics, and do not create a second plugin API.
- Keep Engine native-addon architecture claims exact: only build/test architectures actually supported
  by the Engine are declared. Do not claim ARM64 or x86 Engine native-addon support.
- Preserve the ML-DSA-65 v2 sign-input path. Each slice records pinned source/provenance, declared
  `runtime_abi` and `runtime_build`, dependencies, and deterministic x64 install/load smoke evidence.
- Real online publication, protected signing material, Authenticode, UAC, and production-host lifecycle
  remain `MANUAL-PENDING`.

## Completion evidence

- Added pinned `fcitx5-lua@05db9ee519d448a64ccbe216044e8e0342e8c536` and
  `fcitx5-unikey@53f82a1e01dc0484f46dc8ed419d586cebd2f114` to the existing MSYS2/CLANG64
  standard Fcitx build path. Only x64 native-addon packages are declared.
- Extended the existing ML-DSA-65 v2 package generator to create, sign-input, verify, and
  install every inventory package while retaining source provenance, runtime ABI/build, and
  dependency metadata in the reviewed inventory.
- Contract, dependency, license, text-format, x64/x86 Cargo package-core tests, and x64/x86
  clippy checks passed. The local PQC fixture script could not run because the worktree has no
  built fixture signer/package binaries; production signing, online publication, Authenticode,
  UAC, and real Windows host lifecycle evidence remain `MANUAL-PENDING`.
