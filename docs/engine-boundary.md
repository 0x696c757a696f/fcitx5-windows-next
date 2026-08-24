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
- `EngineEpoch`
- runtime/update `Generation`
- request freshness and stale rejection
- deadline, timeout and fail-open policy
- context/composition ledger
- canonical preedit/candidate/surrounding-text snapshot DTO
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

1. `E0`: freeze current Engine call graph, owner matrix, protocol schema and corpus.
2. `E1`: move Engine protocol DTO, IDs, validation and codec to Rust.
3. `E2`: move context/composition/revision/generation state to Rust.
4. `E3`: converge Fcitx event -> plain EngineEvent -> Rust -> EngineActionBatch -> C++ adapter.
5. `E4`: move Engine IPC transport/framing/session/deadline policy to Rust.
6. `E5`: move snapshot and surrounding-text canonicalization to Rust.
7. `E6`: delete replaced C++ product state, leaving only the Fcitx adapter and Windows process shell.

Every step requires regression evidence, no duplicate authoritative state, no new Fcitx object pointer exposure, and unchanged upstream addon behavior.
