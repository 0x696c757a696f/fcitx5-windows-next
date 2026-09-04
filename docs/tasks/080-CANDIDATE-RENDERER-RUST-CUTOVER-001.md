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

## Acceptance standard (revised 2026-09-04)

The Rust renderer cannot pixel-clone the C++ D2D output: Direct2D ClearType vs tiny-skia
rasterize glyphs differently, and the frozen C++ goldens were captured under a display-DPI
artifact (96x62 window renders ~10.4px text). Per user decision the cutover is gated on Rust
functional correctness plus user visual confirmation, not byte/pixel parity with the C++ goldens:

- **Full-glyph visibility**: CJK/emoji/comment render completely (no clip/overlap/truncation)
  across every layout × HiDPI (100/125/150/200%) × light/dark — the user's core requirement
  (see `docs/tasks/080-layout-naming-design.md` §2).
- **Geometry**: window/item geometry driven by the same DWrite metrics + Rust presentation
  `layout()` the C++ renderer consumed (18 DIP candidate, label 0.85, comment 0.80); C++ golden
  dimensions remain a reference (geometric sanity), not a pixel gate.
- **Visual confirmation**: the user reviews Rust screenshots (vertical/scroll light+dark at
  minimum; later stacked/flow/vertical_text + HiDPI).

The C++ goldens are kept as a behavioral reference corpus; they are NOT a pixel-parity gate
(D2D vs tiny-skia glyph rasterization cannot byte-match — recorded in `080-stage3-diff.md`).

## TDD cutover sequence

1. **Stage 1 — freeze corpus + inventory the C++ render path.** Capture golden BMPs from the
   shipping `fcitx5-ui.exe` render for each layout/selection/scroll/DPI/high-contrast state; record
   the exact render inputs (CandidateModel snapshot + render plan) that produced each golden.
2. **Stage 2 — Rust renderer parity.** Extend `fcitx5-candidate-core`'s existing `tiny-skia` +
   `windui` DWrite path to draw the full candidate window (preedit divider, candidate rows,
   labels/comments, selection rounded rect, scrollbar, borders) and emit BMPs for the same inputs.
3. **Stage 3 — renderer geometry + full-glyph parity.** Rust BMPs reproduce the C++ window
   geometry via the same DWrite metrics + presentation layout; full-glyph visibility (CJK/emoji/
   comment) asserted on Rust screenshots; C++ goldens kept as geometric reference (recorded in
   `080-stage3-diff.md`), not a pixel gate.
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

- Full-glyph visibility screenshots per layout state (CJK/emoji/comment complete, no clip/
  overlap) across HiDPI 100/125/150/200% where the target supports it; recorded under a documented
  artifact path with the visual-confirmation state noted per state.
- `cargo test --locked -p fcitx5-candidate-core --target ...` green (x64/x86 where the target
  supports it); `cargo fmt --all -- --check` (for Rust-owned crates, excluding the vendored
  `wind-ui-rust` pre-existing tray.rs whitespace); `git diff --check`.
- x64/x86 CMake/Ninja builds green; candidate screenshot/contract CTests green; the old
  `fcitx5_ui` C++ renderer tests are migrated to Rust or classified `KEEP`/`DELETE` with reason.
- `src/ui/ui_main.cpp` deleted; no remaining C++ candidate drawing/window authority.
