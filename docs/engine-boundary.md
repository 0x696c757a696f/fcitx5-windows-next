# Engine Rust/C++ Boundary

Date: 2026-08-24

Status: accepted boundary; implementation still in migration.

## Goal

The Engine is a mixed boundary by design:

```text
Rust Engine Product Core
  protocol / state / validation / revision / generation / deadline / IPC / diagnostics
        |
        | narrow C ABI with plain data only
        v
C++ Fcitx Adapter
  direct Fcitx object manipulation
        |
        v
upstream Fcitx5 core and upstream addons
```

This keeps upstream Fcitx and addons consumable without binding the full C++ object model into Rust.

## Fcitx Object Owners

These stay C++ owned:

- `fcitx::Instance`
- `fcitx::InputContext`
- `InputContextManager`
- `FocusGroup`
- `InputContextProperty`
- Fcitx event objects
- `InputPanel`
- `CandidateList`
- `StatusArea`
- `AddonManager`
- `AddonInstance`
- `InputMethodEngine`
- Fcitx config objects

Fcitx headers should converge toward the Fcitx adapter layer. Rust must not hold `fcitx::*` pointers or emulate Fcitx inheritance/vtables.

## Rust Product-State Owners

These move to Rust authority:

- IPC protocol version, message validation, codec and payload budgets
- `ContextId`
- `CompositionId`
- `Revision`
- per-context product state: last-known `CaretRect`, `popupAllowed` policy,
  candidate-highlight `selectedOverride`, `inputMethodOverridden` marker
- `EngineEpoch` (engine-process handshake scalar; E4 session scope)
- runtime/update `Generation` (release platform attribute; E4 scope)
- request freshness and stale rejection
- deadline, timeout and fail-open policy
- context/composition ledger
- canonical preedit/candidate/surrounding-text snapshot DTO (E5; `pendingStates`
  stays a C++-owned derived cache until then)
- candidate action intent DTO
- diagnostics and health reporting

If C++ keeps a derived cache during migration, the authoritative owner must be documented and the old C++ owner deleted at cutover.

## Current C++ Product State To Shrink

Current `src/engine/fcitx_runtime.cpp` still owns product state maps:

- `contexts`
- `nextCompositionId`
- `revisions`
- `compositions`
- `carets`
- `popupAllowed`
- `pendingStates`
- `selectedOverride`
- `inputMethodOverridden`

These are migration candidates for Rust Engine Product Core after the call graph and corpus are frozen.

## Current Engine Call Graph

Current `HEAD` still routes the Engine hot path through C++:

```text
fcitx5-engine.exe
  -> src/engine/fcitx_engine_main.cpp
     -> named-pipe frame read
     -> protocol::decodeFrame / protocol::decode(...)
     -> handshake and metadata checks
     -> FcitxDispatcher
        -> fcitx::EventDispatcher thread
        -> FcitxRuntime
           -> EngineInputContext
           -> keyFromRequest
           -> context.keyEvent / candidate.select / pending state
           -> collectResult
     -> makeStateResponse / makeEngineStatusResponse
     -> protocol::encode(...)
```

Current request handlers:

- `helloRequest`: establishes one client handshake and validates process/session identity.
- `keyRequest`: calls `FcitxDispatcher::processKey`, then publishes a `KeyResponse`.
- `candidateSelectRequest`: accepts only from the product UI executable, calls `selectCandidate`, publishes updated state, then signals the UI event.
- `stateRequest`: drains a pending state snapshot for a context when the UI missed an event.
- `engineStatusRequest`: returns current input-method display metadata.

The dispatcher deadline check is part of the current contract: if queued work reaches the Fcitx event loop after the caller's deadline, it is dropped before touching Fcitx state.

## Current IPC Schema Freeze

The current Engine IPC schema is C++ owned in `protocol/protocol.h` and `protocol/protocol.cpp`.

Frozen frame constants at this baseline:

- Magic: `FCW4`
- Version: `14`
- Header size: `64`
- Hot frame limit: `256 KiB`
- Control frame limit: `1 MiB`
- Commit/preedit/surrounding text limit: `16 KiB`
- Candidate count limit: `128`
- Candidate field limit: `4096 bytes`
- Logical key text limit: `64 bytes`
- Input-method id limit: `64 bytes`
- Input-method display-name limit: `128 bytes`
- Locale tag limit: `35 bytes`

Message types currently in the shared protocol:

- `helloRequest`
- `helloResponse`
- `keyRequest`
- `keyResponse`
- `launcherRequest`
- `launcherResponse`
- `candidateSelectRequest`
- `candidateSelectResponse`
- `stateRequest`
- `engineStatusRequest`
- `engineStatusResponse`

Engine-facing metadata currently carries:

- `requestId`
- `responseTo`
- `engineEpoch`
- `sessionId`
- `contextId`
- `compositionId`
- `revision`

Current key request payload carries virtual key, key flags, scan code, extended-key bit, popup policy, keyboard layout, logical UTF-8 text, input-method id, surrounding text/cursor/anchor, and caret rectangle.

Current key response payload carries handled/commit/preedit/candidates, selected candidate, page/total/visibility/page-size/bulk/end flags, delete-surrounding-text, forward-key, caret rectangle, popup policy, and content locale.

This schema is the compatibility corpus for `E1`. Moving it to Rust must preserve decode rejection behavior, payload budgets, metadata semantics, and fuzz/unit coverage before C++ protocol ownership is deleted.

## Current Engine C ABI

The Engine now has two Rust ABIs:

- `fcitx5-protocol-core` (`protocol/protocol_ffi.h`, E1 cutover): the FCW4
  frame codec is Rust authoritative; `protocol/protocol.cpp` is a thin
  marshalling bridge over 23 typed C functions. Wire compatibility is frozen
  by `tests/unit/protocol_wire_golden.inc` and verified by
  `protocol-differential-contract`.
- `fcitx5-engine-core` (`src/engine/engine_core_ffi.h`, E2 cutover): the
  per-context composition/revision ledger is Rust authoritative and
  `FcitxRuntime::Impl` is cut over: the C++ `nextCompositionId`/
  `compositions`/`revisions` maps are deleted and the runtime calls the C ABI
  (`begin_key`/`select_candidate`/`end_result`/`forget`). The frozen C++ ledger
  semantics are pinned by `tests/unit/engine_core_contract_test.cpp`
  (`engine-core-contract`) and by the real `fcitx5-engine.exe` integration
  acceptance (`fcitx5_engine_integration_test` baseline/typing-fuzz/safe-mode/
  chttrans on the native-engine lane).

Adjacent Rust ABIs used around the Engine remain non-Engine boundaries:

- `rust/windows-common-core` backs shared IPC transport, peer verification, deadlines, identity, path, UTF conversion, and response scalar validation used by C++ clients.
- `rust/candidate-core` backs Candidate model/layout/interaction DTO validation and UI-side state.
- `rust/launcher-core` backs launcher state/path/tray/command/frame policy.

The Engine Rust ABI must stay a narrow boundary: it must pass only plain
event/action DTO data and opaque Rust handles; it must not expose Fcitx object
pointers or share allocator-owned C++ containers.

## Current Upstream Fcitx/Addons Patch Inventory

The repository keeps upstream Fcitx/addon changes as an explicit small patch queue under `third_party/patches`:

- `fcitx5-windows-user-data-root.patch`
- `libime-windows-model-dirs.patch`
- `fcitx5-chinese-addons-msys2-clang-libcxx.patch`
- `fcitx5-lua-windows-lua54.patch`
- `fcitx5-rime-windows-paths.patch`
- `librime-msys2-clang-windows.patch`

These patches are integration patches for upstream consumption/build behavior. They are not authority to fork Fcitx core semantics, rewrite upstream addons in Windows-private Rust, or bind the full Fcitx C++ object model into Rust.

Any new Fcitx/addon patch must be recorded here with owner, upstream target, reason, and removal/upstreaming condition.

## Protocol Capability Model

Engine protocol must be versioned and capability-aware. Do not freeze the current Fcitx frontend API as a closed action enum.

Required capability vocabulary includes:

- `TEXT_COMMIT`
- `TEXT_COMMIT_WITH_CURSOR`
- `TEXT_DELETE_SURROUNDING`
- `TEXT_REPLACE_SURROUNDING`
- `CANDIDATE_ACTION`
- `FORWARD_KEY`

Unsupported capabilities fail closed at protocol validation or fail open at host-input boundaries according to the request type.

## Candidate Action Flow

```text
Fcitx CandidateList
  -> candidate/action DTO
  -> Rust Candidate state
  -> user-triggered action intent
  -> C++ Fcitx adapter
  -> upstream candidate action API
```

Rust may own transport, identity, freshness, UI state and user intent. Plugin candidate semantics remain upstream-owned.

## Addon Model

Addon model:

```text
Addon
  static or built-in
  dynamic or package-loaded
```

The package model must not assume every addon is a dynamic DLL. Upstream addons are not rewritten as Windows-private Rust addons.

## FFI Rules

Allowed across Engine Rust/C++ ABI:

- fixed-width integers
- `size_t`
- pointer + length slices
- `repr(C)` POD structs
- UTF-8 and UTF-16 slices with explicit length
- opaque Rust handles owned and freed by Rust

Forbidden across Engine Rust/C++ ABI:

- `std::string`
- `std::vector`
- `std::optional`
- `std::variant`
- C++ class pointers
- Rust `String`
- Rust `Vec`
- Rust `Result`
- trait objects
- shared allocator ownership

## Migration Order

1. `E0`: freeze current Engine call graph, owner matrix, protocol schema and corpus. **DONE** (`docs/engine-boundary.md`, `docs/tasks/rebaseline.md`).
2. `E1`: move Engine protocol DTO, IDs, validation and codec to Rust. **DONE — CUTOVER-GREEN** (2026-08-24): `fcitx5-protocol-core` is the authoritative codec; `protocol/protocol.cpp` is a thin bridge; wire frozen by `protocol_wire_golden.inc`; `protocol-differential-contract` + 79/79 CTest green; Cargo/clippy/fmt green.
3. `E2`: move context/composition/revision/generation state to Rust. **DONE — CUTOVER-GREEN** (2026-08-24): `fcitx5-engine-core` owns the per-context ledger (composition id allocation starting at 1 with reserved-0 wrap, per-context composition/revision, `processKey`/`selectCandidate` stale checks, candidate id validation) **and the remaining per-context product state maps** (`carets`, `popupAllowed`, `selectedOverride`, `inputMethodOverridden` — 9 more C ABI functions in `engine_core_ffi.h`) with a narrow C ABI; `FcitxRuntime::Impl` is cut over and the C++ `nextCompositionId`/`compositions`/`revisions`/`carets`/`popupAllowed`/`selectedOverride`/`inputMethodOverridden` maps are deleted; `engine-core-contract` pins the frozen semantics; source-contract guards both the ledger calls and the absence of the old C++ maps; real `fcitx5-engine.exe` integration acceptance (baseline/typing-fuzz/safe-mode/chttrans) passes on the native-engine lane with an isolated, pre-deployed `FCITX_USER_DATA_ROOT`. Explicitly deferred: `pendingStates` (a full `RuntimeResult` derived cache — moves with the E5 snapshot DTO, not as a standalone ABI), and `EngineEpoch`/`Generation` (process-level session/release attributes in the E4 IPC scope).
4. `E3`: converge Fcitx event -> plain EngineEvent -> Rust -> EngineActionBatch -> C++ adapter.
5. `E4`: move Engine IPC transport/framing/session/deadline policy to Rust.
6. `E5`: move snapshot and surrounding-text canonicalization to Rust.
7. `E6`: delete replaced C++ product state, leaving only the Fcitx adapter and Windows process shell.

Every step requires regression evidence, no duplicate authoritative state, no new Fcitx object pointer exposure, and unchanged upstream addon behavior.

## Known integration gap: native-engine (MSYS2) lane — RESOLVED

`tools/bootstrap-fcitx.ps1` / `tools/test-fcitx.ps1` build the upstream-facing
`native-engine/CMakeLists.txt` project (the `fcitx5-engine.exe` that links
`Fcitx5::Core`). Since E1, `protocol/protocol.cpp` calls the Rust
`fcitx5_protocol_core_*` ABI and the runtime calls `fcitx5_engine_core_*`, so
`native-engine/CMakeLists.txt` now wires the Windows Rust toolchain's GNU-Abi
staticlibs directly: a `fcitx_add_rust_staticlib` helper builds
`fcitx5-protocol-core`, `fcitx5-engine-core`, `fcitx5-windows-common-core`, and
`fcitx5-control-core` with the `x86_64-pc-windows-gnu` target (rust-std for
that target is installed under `out/toolchains/rust`) into
`out/toolchains/rust/target-gnu`, and `fcitx5_engine` links them plus
`ntdll`/`ws2_32`/`userenv`. Verified: `fcitx5-engine.exe` builds, installs, and
passes the real-engine integration acceptance
(`fcitx5_engine_integration_test` baseline / typing-fuzz 4000 iterations /
safe-mode / chttrans) with `FCITX_USER_DATA_ROOT` isolated.
