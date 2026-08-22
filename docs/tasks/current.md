# Current Task — RUST-R3-CONFIG-POC Rust Settings/Config differential PoC

**State:** CURRENT / INITIAL-POC-GREEN / VIEW-MODEL-SELF-CHECK-GREEN / LAYOUT-NONOVERLAP-SELF-CHECK-GREEN / OPERATION-STATE-MACHINE-GREEN

## Gate

Start only after the Settings operation model, typed config model, package UI states, localization/dialog behavior, and Control/package/update boundaries are frozen.

## Scope

- Build an isolated Rust Settings/Config PoC as a normal user-facing EXE.
- Consume typed config/control/package boundaries; do not duplicate business logic or shell out to unsafe commands.
- Preserve product UI/UX rules: no overlap at supported DPI/minimum size, localized dialogs, language selector, live Candidate preview, font selection, advanced appearance controls, input-method list, add-ons/update states, and diagnostics actions.
- Prefer Rust domain models for settings, validation, package actions, and UI state.

## Must not do

- Do not replace the shipping Config app during the PoC.
- Do not fake online plugin availability without signed trusted repository metadata.
- Do not regress accessibility, localization, package safety, or existing typed Control behavior.

## Done when

- Rust Config PoC matches the frozen operation/visual contracts and records package/update/settings evidence.
- A later cutover task can decide whether to replace the WTL/D2D implementation and delete the old authoritative path.
