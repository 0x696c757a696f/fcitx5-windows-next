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

## Migration plan

Consumer audit at `fa83803465e30a1d324f0d66e676f969f54517e9` (078 selected, reclassification
complete).

### Consumer inventory (exact findings)

Production consumers (ship in the product):

- `src/ui/ui_main.cpp` — the only production caller of `pipe_client.h`. Uses exactly two
  `PipeClient` operations:
  1. `runCandidateSelectionTest` (line ~1005): constructs `PipeClient`, calls `selectCandidate`,
     returns only the `bool`.
  2. `CandidateWindow::candidateClient_` (created ~1226; select call ~2498): interactive
     candidate click. `selectCandidate` is called with seven plain scalars from a
     `CandidateSelectionIntent` (targetProcessId, engineEpoch, contextId, compositionId, revision,
     candidateId); the result is only the `bool`. `ui_main.cpp` never reads `KeyResult`,
     `EngineStatusResult`, `CaretRect`, or `LauncherResponse` fields, and never calls
     `processKey`/`pollState`/`queryEngineStatus`/launcher commands.
  - `ui_main.cpp` also uses `ipc::verifyPipeClient` (servePresentation ~2871) and
    `platform::*` identity/pipe-security — those are the peer_verification / runtime_identity /
    pipe_security native seams already classified `KEEP`.
- `src/engine/fcitx_engine_main.cpp` — NOT a transport consumer. It includes `peer_verification.h`
  (KEEP seam); its key/candidate paths go through `FcitxDispatcher` (`dispatcher.processKey` /
  `dispatcher.selectCandidate`), which is the direct Fcitx adapter island, and it calls
  `ipc::verifyPipeClient` in the pipe-host accept path. It does not include `pipe_client.h` or
  `launcher_client.h` (rg-confirmed no include).
- `src/engine/fcitx_runtime.cpp` — not a consumer; drives Fcitx objects.
- No Rust code links the C++ `PipeClient`/`LauncherClient`. Rust `tsf-poc` already implements its
  own safe `EngineClient` (connect/launcher-start/handshake/process-key/per-context revision map)
  directly over the `fcitx5_windows_common_*` C ABI; `launcher-core` uses `VerifiedPipeClient`;
  `protocol-core` owns the wire codec. There is no cross-language dependency on the C++ client.

Test consumers (drive the real binaries over the real pipe):

- `tests/integration/ipc_roundtrip_test.cpp` — `PipeClient` + `KeyResult` +
  `queryEngineStatus`/`EngineStatusResult`. Validates wire round-trip vs the Rust
  `fcitx5-mock-engine.exe` fixture (miss/connect/stall/key commit/status). Reads commit, handled.
- `tests/integration/ipc_multi_client_test.cpp` — many concurrent `PipeClient::processKey`.
- `tests/integration/ipc_idle_client_test.cpp` — idle + concurrent `processKey` commit checks.
- `tests/integration/ipc_generation_routing_test.cpp` — `processKey` commit echo vs generation.
- `tests/integration/ipc_late_response_test.cpp` — late/mismatch response behavior, also imports
  `protocol_ffi.h` to craft server frames.
- `tests/integration/fcitx_engine_integration_test.cpp` — drives the real `fcitx5-engine.exe`
  (x64/x86) through `PipeClient::processKey` + reads `KeyResult` fields extensively (105 field
  accesses) for baseline/typing-fuzz/chttrans/safe-mode/rime-lua; drives candidate selection by
  launching `fcitx5-ui.exe` with a `--candidate-select-test`. Runs in `tools/test-fcitx.ps1` as the
  real-Fcitx release acceptance gate.
- `tests/integration/launcher_integration_test.cpp` — `LauncherResponse` reads
  (status/launcherState/engineState/startDisposition/currentInputMethodId) for
  startDemand/status/userStop/resume/shutdown against the shipping Rust `fcitx5-launcher.exe` +
  mock engine (real-process lifecycle E2E).
- `tests/integration/launcher_crash_loop_test.cpp` — launcher Safe-Mode crash-loop E2E vs real
  launcher + crash fixture, reads `LauncherResponse` scalars.

`KeyResult` local DTO is read 143 times across all tests. All launcher/IPC/engine integration tests
link `fcitx5::ipc_client` (the C++ adapter library).

### State owned by C++ `PipeClient`

`PipeClient` members: `pipeName_`, `launcherGeneration_`, `peerPolicy_`, `identity_`, `pipe_`
(HANDLE), `handshakeComplete_`, `engineEpoch_`, `sessionId_`, and a per-context
`unordered_map<uint64_t, ContextState{compositionId, revision}>`. `processKey` reads/updates the
map; `selectCandidate` reads `engineEpoch_`/`sessionId_` after handshake and does not touch the
map; `pollState`/`queryEngineStatus` use the map/epoch. Every stale/revision accept-or-reject and
scalar projection is delegated to Rust
(`fcitx5_windows_common_accept_*`/`apply_*_scalars`, `fcitx5_protocol_core_decode_*`); the C++
body holds no product decision. The same state is already owned by the Rust `tsf-poc`
`EngineClient` (pipe, handshake flags, engine epoch, per-context composition/revision map). The
C++ mirror is therefore a duplicate transport client whose state Rust can hold in an opaque handle.

### Migration stages

#### Stage 1 — Rust opaque candidate-select client ABI (enables `ui_main.cpp` cutover)

Goal: give the C++ renderer host a single, stateless-in-C++ select call so `PipeClient`'s
production use disappears without a D2D rewrite.

- Rust: extend `rust/windows-common-core/src/lib.rs` (named low-level Win32 exception, already in
the safety-policy allowlist) with a safe opaque client object that owns connect + peer-verify +
handshake + engine-epoch/session state + (optionally) the per-context map, and expose a minimal
`extern "C"` ABI:
  - `void* fcitx5_windows_common_candidate_select_client_create_utf16(pipe_name, pipe_name_len,
    expected_peer_path_utf16, expected_peer_path_len)` (performs lazy connect on first call,
    mirrors `PeerPolicy::exact`; returns null on invalid input)
  - `u8 fcitx5_windows_common_candidate_select_client_select(handle, target_process_id,
    expected_engine_epoch, context_id, composition_id, revision, candidate_id)`
  - `void fcitx5_windows_common_candidate_select_client_destroy(handle)`
  The select body is exactly today's `PipeClient::selectCandidate`: verify peer, handshake,
  `fcitx5_windows_common_accept_candidate_select_request`, encode via
  `fcitx5_protocol_core_encode_candidate_select_request`, `pipe_transact`, decode,
  `accept_candidate_select_response`. No C++ product logic is introduced; C++ holds only the
  opaque pointer + plain scalar args and gets a `u8`.
- New Rust exports go in the existing allowlisted `windows-common-core/src/lib.rs`; a new file (if
  split out, e.g. `src/ipc_client_abi.rs`) must be added to `UNSAFE_EXCEPTIONS` in
  `rust/measurement-core/tests/rust_safety_policy.rs` and carry
  `#![deny(unsafe_op_in_unsafe_fn)]`. Prefer one module with narrow `unsafe` blocks and `SAFETY`
  comments; wrap the safe logic in a `forbid(unsafe_code)` inner module where possible.
- RED gate: Rust unit/property tests in `windows-common-core` asserting (1) create rejects null/
  zero-length pipe name and empty expected path, (2) select returns 0 on a never-connected handle
  (no crash), (3) select with an expected-epoch mismatch returns 0 (stale) via a fake/mock engine
  pipe endpoint, (4) destroy(null) is a no-op. Deterministic barriers, no arbitrary sleeps.
- C++ header: `src/ipc/candidate_select_client.h` (declarations only; no DTO, no state).

#### Stage 2 — migrate `src/ui/ui_main.cpp` consumers

- Replace the two `PipeClient` constructions (`runCandidateSelectionTest`, `candidateClient_`) with
  the opaque handle: create once per `CandidateWindow::create` and in `runCandidateSelectionTest`,
  call `select`, `destroy`. Keep the `PeerPolicy::exact(engine)` path by passing the exact peer
  executable path into `_create_utf16`.
- `ui_main.cpp` keeps its direct `peer_verification.h` / `runtime_identity.h` / `pipe_security.h`
  includes (those are `KEEP` native seams). After stage 2 no `pipe_client.h`/`launcher_client.h`
  include remains in `src/`.
- Add direct includes that `pipe_client.h` previously provided transitively (peer_verification.h,
  runtime_identity.h, protocol_ffi.h where still needed).
- Delete `src/ipc/pipe_client.h`, `pipe_client.cpp`, `launcher_client.h`, `launcher_client.cpp`, and
  remove `fcitx5_ipc_client` source entries / the now-unused `fcitx5::ipc_client` static-lib wiring
  in `CMakeLists.txt` if no test still links it (see stage 3).

#### Stage 3 — migrate integration tests to Rust clients

The integration tests exercise the wire and process lifecycle over the real pipe. Rust already owns
the wire corpus (`protocol-core` frozen corpus), the mock engine (`mock-engine-core`), and a safe
client (`tsf-poc` `EngineClient`); 071 established Rust test authority. Migrate each C++ test that
currently links `fcitx5::ipc_client` to a Rust transport E2E that spawns the same real binaries
and drives a Rust client, deleting the C++ client dependency:

- `ipc_roundtrip`, `ipc_multi_client`, `ipc_idle_client`, `ipc_generation_routing`,
  `ipc_late_response`: MIGRATE to Rust tests under `rust/windows-common-core/tests/` (or a new
  `rust/ipc-client-core` crate) that spawn `fcitx5-mock-engine.exe` and assert the same commit/
  handled/late/multi-client/idle/revision semantics with a Rust client. Reuse the frozen
  `protocol-core` wire corpus where it already covers the frame semantics.
- `launcher_integration_test` and `launcher_crash_loop_test`: KEEP (final mixed-binary
  integration/E2E). They spawn the real shipping `fcitx5-launcher.exe` + mock/crash engine and
  assert the real process lifecycle (launcherState/engineState/startDisposition/Safe-Mode); the
  C++ client is the legitimate cross-language probe, not a product authority. Per AGENTS.md
  long-term C++ test allowlist item 3 these stay C++.
- `fcitx_engine_integration_test` (real `fcitx5-engine.exe` x64/x86 acceptance in
  `tools/test-fcitx.ps1`): KEEP (final mixed-binary integration/E2E). It drives the real
  Fcitx C++ binary + `fcitx5-ui.exe` and is the release-gate real-Fcitx acceptance corpus; the
  C++ client over the real pipe is the cross-language probe. Do NOT rewrite it to a Rust harness
  (that would replace a mixed-binary E2E with a Rust-only one and shrink release coverage).
- Classification: the five `ipc_*` wire tests are `MIGRATE` (pure protocol, mock-engine fixture,
  Rust `protocol-core` owns the wire). `fcitx_engine_integration_test` and the two `launcher_*`
  tests are `KEEP` (final mixed-binary integration/E2E, allowlist item 3). Consequently
  `pipe_client.*`/`launcher_client.*` remain as the necessary C++ client for the KEEP E2E tests
  (no product policy, only transport); they are NOT deleted. 078 completion is: ui_main.cpp
  production consumer migrated to the Rust opaque client (done, Stage 1+2), the five `ipc_*` wire
  tests migrated to Rust (Stage 3), and the KEEP E2E tests + their C++ client reclassified.

#### Stage 4 — delete the five migrated C++ wire tests and their CTest entries

- Delete `tests/integration/ipc_roundtrip_test.cpp`, `ipc_multi_client_test.cpp`,
  `ipc_idle_client_test.cpp`, `ipc_generation_routing_test.cpp`, `ipc_late_response_test.cpp` after
their Rust replacements are green on x64 and x86, and remove their CTest registrations in
  `CMakeLists.txt` once the Rust CTest routes are registered.
- `src/ipc/pipe_client.*`/`launcher_client.*` stay as the KEEP mixed-binary E2E client (used by
  `fcitx_engine_integration_test` and `launcher_*_test`). They hold no product policy; update
  `docs/tasks/077-production-cpp-inventory.md` rows to `KEEP (final mixed-binary E2E client)`.
- The five deleted test `.cpp` files were the only consumers that no longer qualify; `fcitx5::ipc_client`
  remains linked by the KEEP E2E tests.

#### Stage 5 — x64/x86 verification

- `cargo test --locked` for `windows-common-core` (new ABI tests) and every migrated Rust test
  crate on x64 and x86; `cargo fmt --all -- --check`; strict clippy `-D warnings`.
- x64/x86 CMake/Ninja Debug + Release build and affected CTest (Rust candidate-select contract,
  migrated IPC/launcher/engine E2E routes; run the real-Fcitx acceptance through
  `tools/test-fcitx.ps1` exactly as the release gate does).
- `tools/check-runtime-security.ps1`, secrets/licenses/dependencies/locales/text-format,
  `rust_safety_policy.rs`, and `git diff --check`.

### Consumers that cannot migrate without a renderer rewrite

None block 078. `src/ui/ui_main.cpp` stays a C++ HWND/D2D/DWrite renderer host; it needs a way to
send candidate selection to the engine, which Stage 1's opaque `fcitx5_windows_common_candidate_select_client_*`
ABI provides without moving renderer drawing to Rust. Every other consumer is a test or the engine
adapter island (already Rust/ABI-direct) and migrates in Stages 2-4. A full D2D renderer rewrite is
NOT a prerequisite for 078.
