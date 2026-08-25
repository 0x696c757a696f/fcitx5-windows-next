# CONFIG-RUST-CUTOVER-001 Shipping Settings Rust cutover

**State:** IN-PROGRESS / INITIAL-CUTOVER-PENDING

## Context

The owner has explicitly authorized the non-Engine Rust direction: except for the direct
Fcitx-facing Engine island and truly necessary temporary native adapter seams, product-owned Windows
components must move to Rust. `fcitx5-config.exe` still ships through a C++ WTL/Win32 shell, but the
current `CONFIG-UX-009` work is freezing the Settings theme, appearance, package, localization,
embedded candidate preview, and no-overlap behavior contract that a Rust shipping Config must
preserve.

The existing C++ Config implementation is therefore a regression baseline and temporary adapter, not
a durable architecture choice.

## Scope

Replace the shipping Settings executable with a Rust-owned implementation in vertical slices:

- freeze the current `fcitx5-config.exe` visible behavior corpus from `CONFIG-UX-009`, including
  navigation, theme library, embedded candidate preview, enabled input methods, font selection,
  advanced appearance, package state, diagnostics/repair, localization, keyboard/focus behavior, and
  no-overlap/no-clipping geometry;
- retain the useful Hi-DPI requirements from the removed historical plan: DPI-derived DIP layout,
  scaled fonts, cross-monitor reflow on DPI changes, no fixed-physical-pixel control grids, and
  candidate-preview DPI parity with the real candidate window;
- retain the useful IPC/security constraints from the removed historical Phase 2 threat model:
  Config must call typed Control APIs only, must not talk to the input hot path or expose realtime
  input data, must not shell out to arbitrary commands, and must preserve peer/session/path trust
  boundaries when invoking product helpers;
- implement a Rust Settings executable that preserves the product binary name and consumes the
  existing typed Control/config/package/theme boundaries instead of reimplementing them in C++;
- keep any native Windows calls as small Rust or temporary adapter seams with explicit safety and
  failure handling;
- run side-by-side differential tests against the frozen C++ behavior corpus until the Rust
  implementation is equivalent or better;
- cut over the CMake/package target so `fcitx5-config.exe` is built from Rust;
- delete the old authoritative C++ WTL Config shell and update source-contract checks so it cannot
  silently return.

## Must not do

- Do not change Settings semantics while migrating language unless the behavior change is separately
  captured in the frozen corpus and reviewed by tests.
- Do not introduce WebView/Tauri/Qt or another heavy GUI runtime merely to move away from C++.
- Do not create a permanent runtime selector between C++ Config and Rust Config.
- Do not expose fake online plug-ins or unsigned repository content.
- Do not use global hooks, `SendInput`, process injection, credential access, or external
  exploitation.

## Required validation

- x64/x86 Rust build and package lineage produce `fcitx5-config.exe` from the Rust target.
- Differential Settings behavior corpus passes against the old C++ baseline before deletion.
- English and Simplified Chinese localization coverage remains complete.
- Keyboard-only focus order, visible focus, and basic UI Automation/accessibility markers are
  covered for all primary pages.
- 100%, 125%, 150%, 200%, and 300% DPI layout checks prove no added control, label, candidate text,
  emoji, or preview content overlaps or clips.
- Embedded candidate preview consumes the same CandidateModel/layout/render contract as the shipping
  candidate UI and never opens an external floating preview as the only preview path.
- Theme import/export/duplicate/delete, font persistence, package trust blocking, diagnostics, and
  repair operation paths remain routed through Rust-owned typed boundaries.
- Package smoke from `out/package` passes after cutover.

## Done when

- The packaged shipping `fcitx5-config.exe` is Rust-built.
- The old authoritative `src/config` C++ WTL shell is deleted or reduced to non-authoritative
  transitional assets explicitly listed by source-contract.
- Existing Settings UX from `CONFIG-UX-009` is preserved or improved with current tests proving the
  behavior.
- `docs/tasks/status.md`, `docs/current.md`, `docs/tasks/PLAN.md`, and source-contract evidence are
  updated so future work cannot treat C++ Config as the target implementation.
