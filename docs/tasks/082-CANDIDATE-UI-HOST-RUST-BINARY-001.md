# Task 082 - Rust shipping candidate-UI host (fcitx5-ui.exe → Rust binary)

**Task ID:** `CANDIDATE-UI-HOST-RUST-BINARY-001`
**Mode:** CHANGE / HOST-RUST-CUTOVER
**Prerequisite:** 081 (renderer/layout/window/state/decisions all Rust-owned via candidate-core C ABI; `ui_main.cpp` deleted, shell renamed `ui_host.cpp`).

## Goal

Make `fcitx5-ui.exe` a Rust shipping binary (precedent: `fcitx5-config.exe`)
and delete `src/ui/ui_host.cpp`. The C++ shell that remains after 081 is still
~2.8k lines of Rust-ABI call stitching, a Win32 pipe server, a config adapter,
and the self-test harness — none of it requires C++.

## Why (user challenge recorded)

081D deleted `ui_main.cpp` by renaming the shell to `ui_host.cpp`; the user
correctly flagged that the host itself is still C++. This task finishes the
migration for real.

## Slices

1. Rust presentation pipe server: named-pipe inbound server with the frozen
   peer-verification semantics (port from src/platform pipe_security +
   ipc::verifyPipeClient) + protocol-core decode → KeyResponse; C++
   decodePresentationFrame/servePresentation deleted.
2. Rust frame orchestration ABI: decoded response + config snapshot + caret →
   model apply → presentation apply → visual build → measure → axis layout →
   window assembly in one call; C++ update() stitching deleted.
3. Rust config consumption: config-core snapshot → render theme/geometry
   directly; NativeRenderConfig and loadVisualConfig deleted.
4. Rust window host integration: window_host.rs owns snapshot/config-changed/
   dismiss/timer dispatch internally; the C++ candidate callback shrinks to
   nothing (candidate-select client is already Rust).
5. Rust binary main: command-line parsing (parse_candidate_command_line
   already Rust), parent-watch, self-test entry points; new cargo bin +
   CMake target switch; ui_host.cpp deleted; harness moves to Rust E2E or a
   temporary C++ differential with a recorded deletion condition.

## Frozen acceptance

- No behavior change: pipe handshake, frame limits (256 KiB, 64-byte header),
  focus-watch timing (100 ms), click guard (750 ms), dismissal filtering,
  wheel/paging, DPI, layered window effects, WS_EX_TOOLWINDOW|NOACTIVATE|
  TOPMOST all preserved.
- x64/x86 builds green; all six self-tests + parent-close smoke exit 0 in
  both lanes; candidate-core Rust tests green.
- `ui_host.cpp` deleted; no permanent C++/Rust dual stack for this component.

## Files affected

- C++ deleted at end: `src/ui/ui_host.cpp`
- Rust: `rust/candidate-core` (or new `rust/ui-host` crate), new cargo bin
- CMake: `fcitx5_ui` target → Rust bin

## Validation

Same as existing: x64/x86 builds, six self-tests, message-loop smoke,
cargo tests; new Rust unit tests for the pipe server and frame orchestration.
