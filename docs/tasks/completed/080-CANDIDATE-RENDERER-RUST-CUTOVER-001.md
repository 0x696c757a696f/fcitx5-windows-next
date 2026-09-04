# Task 080 - Candidate renderer (ui_main.cpp D2D/DWrite) → Rust cutover

**Task ID:** `CANDIDATE-RENDERER-RUST-CUTOVER-001`
**Mode:** CHANGE / RUST-MIGRATION / RENDERER-CUTOVER
**Prerequisite:** 078 (transport adapters deleted; `pipe_client`/`launcher_client` gone; wire/launcher/engine E2E in Rust `fcitx5-ipc-client`).
`REL-01` external evidence remains parked and does not block code migration.

## Goal and completion rule

`src/ui/ui_main.cpp` (2993 lines) is the last large product-owned C++ file. Its candidate
**semantics** are already Rust-owned (`fcitx5-candidate-core`: model, layout, presentation state,
selection, notification). What remains C++ is the **renderer/window adapter**: HWND candidate
window, message loop, Direct2D/DWrite drawing, mouse hit-testing, and the candidate-selection
intent back to the engine (the selection IPC already uses the Rust opaque
`candidate_select_client`).

080 moves the candidate window/render path to a Rust renderer with **equivalent visual/DPI
evidence**, then deletes `ui_main.cpp`. 080 is not complete while `ui_main.cpp` still owns the
candidate drawing/window path, or while any new permanent C++/Rust dual renderer authority is
introduced.

## Frozen corpus / differential gate

The Rust renderer must reproduce the C++ D2D/DWrite output. Frozen golden screenshots from the
shipping C++ renderer (vertical/horizontal/grid candidate layouts, selected/scroll states, light/
dark, 100/150/200% DPI, high-contrast) are the differential corpus. The cutover is gated on
byte-comparable or tolerance-equivalent screenshots, not on "it compiles".

## TDD cutover sequence

1. **Stage 1 — freeze corpus + inventory the C++ render path.** Capture golden BMPs from the
   shipping `fcitx5-ui.exe` render for each layout/selection/scroll/DPI/high-contrast state; record
   the exact render inputs (CandidateModel snapshot + render plan) that produced each golden.
2. **Stage 2 — Rust renderer parity.** Extend `fcitx5-candidate-core`'s existing `tiny-skia` +
   `windui` DWrite path to draw the full candidate window (preedit divider, candidate rows,
   labels/comments, selection rounded rect, scrollbar, borders) and emit BMPs for the same inputs.
3. **Stage 3 — differential.** Compare Rust BMPs vs C++ goldens per state; iterate until within the
   frozen tolerance (or byte-identical where feasible). Record the diff evidence.
4. **Stage 4 — Rust window/adapter host.** Move the HWND window, message loop, mouse hit-testing,
   and selection intent to a Rust binary (the new `fcitx5-ui`), reusing the Rust renderer from
   Stage 2 and the existing Rust opaque `candidate_select_client` for selection IPC. Keep the
   candidate window behavior/a11y/DPI contract.
5. **Stage 5 — cutover + delete.** Make the Rust binary the sole shipping `fcitx5-ui`; delete
   `src/ui/ui_main.cpp` (and any now-unused C++ candidate renderer helpers); update
   `docs/tasks/071-test-ownership-inventory.md` / `077-production-cpp-inventory.md`; register the
   Rust screenshot/contract CTests; x64/x86 verification.

## Permanent C++ boundary (unchanged)

Long-term C++ remains only the direct Fcitx adapter island (`fcitx_engine_main.cpp`,
`fcitx_runtime.cpp`, `fcitx_dispatcher.cpp`, `key_event`, `windows_keyboard`), the policy-free
native seams (`runtime_identity`/`peer_verification`/`pipe_security`), and the FFI headers that let
the C++ island call Rust. After 080, the Candidate window/renderer is Rust-owned; the old C++
D2D/DWrite renderer is deleted, not kept as a runtime selector.

## Verification contract

- Frozen golden corpus committed under `docs/tasks/` or a documented artifact path; Stage 3
  differential evidence recorded per state.
- `cargo test --locked -p fcitx5-candidate-core --target ...` green (x64/x86 where the target
  supports it); `cargo fmt --all -- --check` (for Rust-owned crates, excluding the vendored
  `wind-ui-rust` pre-existing tray.rs whitespace); `git diff --check`.
- x64/x86 CMake/Ninja builds green; candidate screenshot/contract CTests green; the old
  `fcitx5_ui` C++ renderer tests are migrated to Rust or classified `KEEP`/`DELETE` with reason.
- `src/ui/ui_main.cpp` deleted; no remaining C++ candidate drawing/window authority.
