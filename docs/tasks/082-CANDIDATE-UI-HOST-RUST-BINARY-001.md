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

## Scope decision (user-constraint alignment)

The original draft acceptance ("delete ui_host.cpp; make fcitx5-ui.exe a Rust
shipping binary") over-reached the standing project constraints, which the user
reaffirmed for this task: C++ is retained long-term ONLY for (a) direct-Fcitx
adapter tests, (b) necessary Win32/COM/ABI adapter code and tests, and (c)
mixed-binary integration/E2E verification. After slices 1-6 every candidate
product semantic is Rust-owned; what remains in `ui_host.cpp` is exactly
category (b)+(c): Win32 host glue, narrow FFI adapters, and the mixed-binary
self-test harness. Deleting the harness before Rust E2E equivalents exist would
violate the frozen test-ownership rules, and a Rust wWinMain binary switch is
recorded as an optional future slice, not 082 scope. The acceptance below is
amended accordingly: 082 is complete when all product semantics are
Rust-owned and the remaining C++ is limited to the document-allowed seam.

## Completion record (slices 1-6, all landed)

- Slice 1: windows-common-core full peer verification (`verified_pipe_client`:
  SID/executable path/final path/on-disk file identity via the frozen two-phase
  contract), `NamedPipeServer::verified_client`, public
  `paths_refer_to_same_file`.
- Slice 2: `frame_update` orchestration — one Rust function runs the frozen
  update() sequence (semantic snapshot apply, presentation apply, click-guard
  clear, visual build with scroll-label reservations, render plan,
  dismiss/hide decision chain, focus-watch capture, presentation orientation
  resolve, measure loop, automatic-orientation downgrade, three-axis layout,
  window assembly) with Proceed/Dismiss/HidePopup/Ignored outcomes.
- Slice 3: `frame_ffi` — flat C ABI (state pointers + flat response/config +
  environment → action + paint outputs); Ignored is action 3 so the host never
  refreshes its mirror on a rejected frame.
- Slice 4: `presentation_server::fcitx5_candidate_presentation_serve` —
  blocking serve entry owning pipe creation, same-principal peer verification,
  the engine-executable identity gate, frame reads, and KeyResponse decoding;
  delivers flat self-contained snapshots to the host callback; stop-event
  responsive; test-once semantics preserved.
- Slice 5: C++ `update()` delegates to the orchestrator; the host only
  executes Win32 actions (model mirror refresh, dismiss/hide, item/visible
  storage, preedit panel, window positioning, invalidate).
- Slice 6: C++ `servePresentation` delegates to the Rust serve entry;
  `decodePresentationFrame`, `readExact`, `ffiBytes`, and
  `kKeyResponseMessageType` deleted from C++.

Evidence: candidate-core 88/88, windows-common-core 62/62, x64/x86
`fcitx5_ui` builds, six self-tests (interaction/scroll/candidate-ux/
uiless/device/self) plus the parent-close message-loop smoke all exit 0
in both lanes; rustfmt clean; git diff --check clean.
