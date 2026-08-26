# 056-CONFIG-STAGE4-A11Y-DPI-QA-001

## Status

MANUAL-PENDING / AUTOMATED-STAGE4-QA-GREEN

## Scope

Freeze the Stage 4 Rust Config GUI QA gate so "Rust Config Cutover Complete"
cannot be claimed until the real interactive Settings UI, controls, candidate
preview, DPI, localization, and accessibility evidence are green.

## Specification References

- `docs/spec-v1.8.md` §5.5.8 Config UI/UX.
- `docs/spec-v1.8.md` §3382 `REG-CONFIG-A11Y-001`.
- `docs/tasks/settings-uiux-operation-integration-plan.md` verification plan.

## Completed Automated Evidence

- Keyboard tab order, focus visibility, page navigation, no-overlap, high-DPI
  geometry, high-contrast fallback markers, and embedded candidate preview
  bounds are covered by automated Rust Config QA/source-contract checks.
- `--self-check --report` emits Stage 4 evidence and explicitly blocks a full
  Stage 4 cutover claim while manual evidence is missing.

## Manual Evidence Still Pending

- Narrator smoke.
- NVDA smoke.
- Real Windows 7 host startup/native-baseline behavior.
- Real Windows 10 1809+ host behavior.
- Real Windows 11 window effects/high-DPI behavior.

## Advancement Note

This task remains manual-pending, but PLAN permits later code-only Settings GUI
work to continue. It is no longer the active `current.md` implementation item.
