# Task 078 - IPC and platform C++ seam to Rust cutover

**Task ID:** `IPC-PLATFORM-RUST-SEAM-CUTOVER-001`
**Mode:** CHANGE / RUST-MIGRATION / SEAM-CUTOVER
**Prerequisite:** 077 source-cutover (protocol DTO bridge deleted, inventory closed);
`REL-01` external evidence remains parked and does not block code migration.

## Goal and completion rule

Eliminate the remaining thin C++ marshalling seams that no longer hold any product policy. After 077
deleted the C++ protocol DTO bridge, five groups of C++ files remain that only marshal Rust flat ABI
results into local mirror structs. Their semantics are already Rust-owned (`fcitx5_windows_common_*`,
`fcitx5_protocol_core_*`, `fcitx5_engine_core_*`). 078 moves those consumers to the Rust ABI directly
and deletes the redundant C++ mirror, leaving only the direct Fcitx adapter island and the
Candidate renderer host.

078 is not complete while any of the target C++ files still exists with product-relevant DTO/state,
or while any new permanent C++/Rust dual authority is introduced.

## Target C++ files

- `src/ipc/pipe_client.cpp`, `pipe_client.h` (641 + 129 lines) — the largest seam. `PipeClient`
  connect/handshake/transact/decode all wrap `fcitx5_windows_common_*` and `fcitx5_protocol_core_*`.
  `KeyResult`/`CaretRect`/`EngineStatusResult` are local DTO mirrors.
- `src/ipc/launcher_client.cpp`, `launcher_client.h` (250 + 45 lines) — launcher command client,
  pure FFI marshalling; `LauncherResponse` is a local mirror.
- `src/platform/runtime_identity.cpp`, `runtime_identity.h` (418 + 65 lines) — identity/generation/
  executable-match/path semantics already in Rust; C++ only marshals and mirrors structs.
- `src/ipc/peer_verification.cpp`, `peer_verification.h` (128 + 27 lines) — two-phase query/fill
  wrapper around Rust `verify_pipe_*`.
- `src/platform/pipe_security.cpp`, `pipe_security.h` (59 + 21 lines) — RAII wrapper around Rust
  `pipe_security_*`.

The consumers of these seams are `src/engine/fcitx_engine_main.cpp`, `src/engine/fcitx_runtime.cpp`,
`src/ui/ui_main.cpp`, and the mixed-binary integration tests. The engine-side consumers are the
approved direct Fcitx adapter island and may keep calling the Rust ABI through the header; the
Candidate renderer and integration tests are the "final mixed binary" consumers that justify a thin
transport adapter until a renderer migration task lands.

## Permanent C++ boundary (unchanged)

Long-term C++ is limited to direct Fcitx object/addon access (`fcitx::Instance`, `InputContext`,
`Addon`, `InputPanel`, `CandidateList`), a necessary thin Win32/COM/C-ABI seam with no product state,
and upstream native addon integration. The renderer (`src/ui/ui_main.cpp`) stays C++ until an
equivalent-visual/DPI renderer migration task lands.

## TDD cutover sequence

For each file group: `frozen Rust public-behavior corpus -> RED boundary test on the Rust owner ->
minimal Rust API -> move consumers off the C++ mirror -> delete the C++ mirror -> x64/x86 mixed
evidence -> source gate`. No permanent old/new selector.

## Acceptance

- `src/ipc/pipe_client.*` and `launcher_client.*` are either deleted (consumers use the Rust
  windows-common/protocol ABI directly) or reduced to a single non-stateful transport call with no
  local DTO mirror; same for `runtime_identity.*`, `peer_verification.*`, `pipe_security.*`.
- Every retained C++ symbol maps to the direct Fcitx adapter, the renderer host, or a necessary
  Win32/COM/C ABI with no product policy.
- Rust public behavior/unit/property/fault/fuzz coverage owns the moved semantics; the mixed
  IPC/launcher/candidate integration/E2E tests still pass on x64 and x86.
- x64/x86 Cargo test, clippy, and fmt pass for affected crates.
- x64/x86 CMake/Ninja builds and affected CTest routes pass.
- Text, dependency, license, source-structure, runtime-security checks and `git diff --check` pass.
- Real-host visual/Accessibility/Win7/signing/UAC evidence remains external and is not claimed.

Do not archive 078 while any target C++ mirror retains product-relevant DTO/state.
