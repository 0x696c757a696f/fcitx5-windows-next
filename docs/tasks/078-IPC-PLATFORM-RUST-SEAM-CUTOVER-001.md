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

## Record of execution (078 reclassification slice)

This slice is an evidence-backed reclassification of the five target groups. It does NOT delete or
rewrite any C++ file; it verifies against the current tree that each group only marshals Rust-owned
C ABI results and records the exact Rust ABI each depends on, so a later slice can move consumers
onto the Rust ABI directly.

Evidence collected from `src/` and `rust/windows-common-core/src/lib.rs` at
`a5efceb9208b9ab6230c2b9ccb4bf24f24072692`:

- **Provably policy-free native seams (verified `KEEP`; only `extern "C"` decls + marshalling):**
  - `src/ipc/peer_verification.{cpp,h}` depends only on
    `fcitx5_windows_common_verify_pipe_server_peer_utf16` and
    `fcitx5_windows_common_verify_pipe_client_peer_utf16` (two-pass query/fill). No product branch
    beyond a compile-time macro read. Consumers: `fcitx_engine_main.cpp` pipe-host accept,
    `ui_main.cpp` `servePresentation`.
  - `src/platform/pipe_security.{cpp,h}` depends only on
    `fcitx5_windows_common_pipe_security_create_utf16`/`_attributes`/`_destroy` (RAII holder).
  - `src/platform/runtime_identity.{cpp,h}` depends only on the `fcitx5_windows_common_*` identity/
    generation/executable-match/root functions listed in the inventory row, all invoked through the
    `rustWide` two-pass wrapper. `mayLaunchUserEngine` is a direct delegation to
    `fcitx5_windows_common_may_launch_user_engine_utf16`.
- **Thin transport adapters with local result mirrors (verified `KEEP` for now; delete only after a
  future renderer/IPC migration):**
  - `src/ipc/pipe_client.{cpp,h}` marshals the pipe transport, deadline, request-id, UTF-8→UTF-16,
    and response-scalar ABI (`fcitx5_windows_common_pipe_transact`, `_open/close_pipe_client_utf16`,
    `_deadline_after_milliseconds`, `_next_pipe_client_request_id`, `_utf8_to_wide_utf16`,
    `_utf8_offset_to_wide`, `_apply_hello/key/candidate_select/engine_status_*`, `_accept_*`) and the
    `fcitx5_protocol_core_*` codec. All accept/reject/revision policy is Rust-owned; the C++
    `KeyResult`/`CaretRect`/`EngineStatusResult` mirrors and per-context `ContextState` are transport
    request bookkeeping. Consumers: `fcitx_engine_main.cpp`, `ui_main.cpp` Candidate renderer
    (`selectCandidate`, `candidateClient_`), `tests/integration/ipc_*`.
  - `src/ipc/launcher_client.{cpp,h}` marshals
    `fcitx5_windows_common_pipe_transact_with_error`, `_open/close_pipe_client_utf16`,
    `_deadline_has_time`, `_apply_launcher_response_scalars`, `_next_launcher_request_id`,
    `_ipc_status_ok`, `_set_last_error` and `fcitx5_protocol_core_encode/decode_launcher_*`; local
    `LauncherResponse` mirror. Consumers: `ui_main.cpp` and `tests/integration/launcher_*`.

The three native-seam groups add no product policy and are already the approved Win32/C-ABI seam;
no further migration is required for them. The two transport adapters are the final mixed-binary IPC
surface; their deletion is gated on a future renderer/IPC migration that moves `ui_main.cpp`, the
engine dispatch path, and the integration tests onto the Rust windows-common/protocol ABI directly.
This slice does not complete 078's deletion acceptance; it closes the reclassification prerequisite.
