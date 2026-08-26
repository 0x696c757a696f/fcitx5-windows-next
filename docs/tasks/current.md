# CONFIG-RUST-CUTOVER-001 Shipping Settings Rust cutover

**State:** IN-PROGRESS / SIDE-BY-SIDE-DIFFERENTIAL-GREEN / RUST-SETTINGS-UI-PREVIEW-QA-GREEN / RUST-CANDIDATE-PREVIEW-FIDELITY-QA-GREEN / RUST-NUMERIC-APPEARANCE-QA-GREEN / RUST-SYSTEM-FONT-PICKER-QA-GREEN / RUST-FONT-PERSISTENCE-RESTART-QA-GREEN / RUST-ADVANCED-NUMERIC-APPEARANCE-QA-GREEN / RUST-PAGE-VISIBILITY-INPUTMETHODS-QA-GREEN / REAL-CANDIDATE-PREVIEW-HOST-REQUIRED

## Context

The owner has explicitly authorized the non-Engine Rust direction: except for the direct
Fcitx-facing Engine island and truly necessary temporary native adapter seams, product-owned Windows
components must move to Rust. `fcitx5-config.exe` still ships through a C++ WTL/Win32 shell, but the
current `CONFIG-UX-009` work is freezing the Settings theme, appearance, package, localization,
embedded candidate preview, and no-overlap behavior contract that a Rust shipping Config must
preserve.

The existing C++ Config implementation is therefore a regression baseline and temporary adapter, not
a durable architecture choice.

## Staged shipping gates

This task allows an explicit intermediate shipping milestone without weakening the final cutover:

- Stage 0 — Legacy Config: the C++ GUI remains the shipping authority.
- Stage 1 — Rust Config Core: Rust owns typed models, schemas, validation, migrations, and
  operation planning, but shipping may still be side-by-side.
- Stage 2 — Rust Config Backend Shipped: non-interactive production paths are Rust-owned and may
  ship under the product binary/automation surface. This includes headless/test CLI, package
  install/update/remove state handling, import/export, config read/write, schema validation,
  migration, diagnostics, CI automation, atomic writes, rollback, permissions, and abnormal-input
  regressions. Once this stage is claimed, these paths must not fall back to the legacy C++ GUI
  implementation.
- Stage 3 — Rust Settings UI Preview: a real Rust settings window is available for QA, but the
  interactive GUI is not yet declared complete.
- Stage 4 — Rust Config Cutover Complete: the real interactive Settings GUI, navigation, controls,
  candidate preview, plugin configuration pages, DPI/dark-mode/keyboard/accessibility behavior,
  crash containment, persistence/restart consistency, and real Windows QA are green.
- Stage 5 — Legacy Config Removed: the old authoritative C++ Config shell is deleted or reduced to
  explicitly non-authoritative transitional assets.

Release notes may describe Stage 2 only as “Rust configuration backend is now shipping; interactive
settings UI migration is still in progress.” They must not claim “Config fully migrated to Rust”
until Stage 4 is green. Candidate preview belongs to the Stage 4 full-GUI cutover gate because it
validates the full Rust config model → UI binding → theme/config serialization → candidate renderer
product chain.

## Scope

Replace the shipping Settings executable with a Rust-owned implementation in vertical slices:

- freeze the current `fcitx5-config.exe` visible behavior corpus from `CONFIG-UX-009`, including
  navigation, theme library, embedded candidate preview, enabled input methods, font selection,
  advanced appearance, package state, diagnostics/repair, localization, keyboard/focus behavior, and
  no-overlap/no-clipping geometry;
- preserve the useful Settings/theme lessons from `huanfeng/WindInput`: task-oriented common vs
  advanced settings, typed theme schema/resolve before render, no dead theme fields, Light/Dark token
  parity, unit-aware geometry, and editor/preview/real-window rendering from one resolved snapshot;
- retain the useful Hi-DPI requirements from the removed historical plan: DPI-derived DIP layout,
  scaled fonts, cross-monitor reflow on DPI changes, no fixed-physical-pixel control grids, and
  candidate-preview DPI parity with the real candidate window;
- retain the useful IPC/security constraints from the removed historical Phase 2 threat model:
  Config must call typed Control APIs only, must not talk to the input hot path or expose realtime
  input data, must not shell out to arbitrary commands, and must preserve peer/session/path trust
  boundaries when invoking product helpers;
- implement a Rust Settings executable that preserves the product binary name and consumes the
  existing typed Control/config/package/theme boundaries instead of reimplementing them in C++;
- replace model-only/synthetic Settings preview evidence with an embedded real Candidate UI
  renderer/preview-host path: fixed sample candidates are allowed as input, but layout, theme,
  DPI, font fallback, emoji rendering, and final pixels must come from the same candidate UI
  renderer path used by the product, inside the Config window;
- implement editable numeric controls for appearance values such as font size, opacity, spacing,
  corner radius, and candidate width: slider/spinbox/text entry must stay synchronized, validate
  through the Rust-owned typed schema, report localized errors, and never write invalid or
  half-parsed values;
- implement font selection from the current system font family inventory through the Rust system font
  boundary; persist stable family names, show fallback status when a font disappears or lacks glyphs,
  and keep Config preview and the real candidate window on the same DWrite/system fallback path;
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
- Numeric appearance input checks cover valid typed entry, invalid text, paste, IME cancellation,
  min/max/out-of-range values, localized error text, keyboard focus, and rollback to the last valid
  value without corrupting `config.toml`.
- Font picker checks prove the list is populated from current system fonts, persisted selection
  round-trips, missing fonts degrade to fallback with a visible Settings status, and emoji/CJK glyphs
  remain visible in both preview and real candidate UI.
- Embedded candidate preview consumes the same CandidateModel/layout/render contract as the shipping
  candidate UI, is rendered inside Config by the real Candidate UI renderer/preview host, and never
  uses a Settings-only fake renderer, static screenshot, or external floating preview as the preview
  path.
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
