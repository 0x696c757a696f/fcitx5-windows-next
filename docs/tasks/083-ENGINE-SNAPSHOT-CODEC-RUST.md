# 083 — ENGINE-SNAPSHOT-CODEC-RUST

## Task
Delete the C++ wire-codec dual stack: `serializeSnapshot`/`deserializeSnapshot`
(~180 lines of hand-rolled little-endian primitives in
`src/engine/fcitx_runtime.cpp`) duplicated the Rust-authoritative snapshot
codec. Move the boundary to a flat self-contained ABI projection so no
serialization format crosses the FFI edge.

## Changes (53c3cb4)
- rust/engine-core: blob codec deleted; new pure-safe flat ABI DTOs
  (`Fcitx5EngineSnapshotFlatC`/`RecordC` + `FlatSnapshotArena` in
  `snapshot.rs`, `#![forbid(unsafe_code)]`); ledger `snapshot_take_flat`
  returns an arena-backed projection (pointers valid until the next flat
  take or ledger drop); FCI `snapshot_store_put_flat`/`take_flat` replace
  the blob put/take/required-size trio.
- src/engine: hand-rolled codec deleted; `selectCandidate`/`takePendingState`
  marshal the flat projection; `engine_core_ffi.h` carries the frozen flat
  structs.
- tests: C++ contract E5-3 error paths moved to the flat ABI (fail-closed:
  null snapshot, null candidate string with non-zero length, absent take);
  Rust C-ABI roundtrip test added.
- Pipeline hygiene: fcitx5-unikey builds with `-DENABLE_QT=OFF` (Qt macro
  editor is outside the engine acceptance path); test-fcitx.ps1 builds
  `fcitx5_ui_rustbin` (082 rename).

## Unsafe-boundary audit (all Rust crates)
30 files `#![forbid(unsafe_code)]` (pure safe), 26 FFI/ABI files with
`#![deny(unsafe_op_in_unsafe_fn)]` at the crate root (candidate-core's five
FFI modules covered by its crate-root deny), narrow unsafe blocks with
SAFETY comments throughout; no unsafe outside the FFI/Win32/COM/ABI
categories.

## Evidence
- engine-core 129/129 (incl. `snapshot_store_flat_c_abi_roundtrip`),
  candidate-core 88/88, windows-common-core 62/62.
- Real-Fcitx engine acceptance (baseline, typing-fuzz 4000 iterations,
  chttrans, safe-mode, rime-lua) green on x64 and x86.
- x64 CTest 79/79; x86 CTest 78/79 (pre-existing documented 071 x86 updater
  UAC os-error-740 known failure).
- rustfmt clean; `git diff --check` clean. HEAD 53c3cb4.

## Environment fixes recorded during verification
- MSYS package pins (cmake/boost/lua54/pkgconf/libwinpthread) had drifted via
  `pacman -Syu`; restored to the pinned versions and the bootstrap now runs
  with `-SkipToolchainSync`.
- fcitx5-unikey: `-DENABLE_QT=OFF` (missing Fcitx5Qt6WidgetsAddons package;
  not needed by the engine acceptance path).
- `src/engine/windows_keyboard.cpp` restored: it is the `windowskeyboard`
  Fcitx addon target in native-engine/CMakeLists.txt (an earlier orphan scan
  only checked the root CMakeLists).
