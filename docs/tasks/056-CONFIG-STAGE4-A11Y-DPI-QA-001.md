# Current Task — CONFIG-STAGE4-A11Y-DPI-QA-001

**Mode:** IMPLEMENTATION
**Task ID:** `056-CONFIG-STAGE4-A11Y-DPI-QA-001`
**Prerequisite:** `055-CONFIG-D2D-SETTINGS-SURFACE-001`
**Evidence class:** automated checks plus manual/real-host evidence where automation cannot prove it.

## Goal

Freeze the Stage 4 Rust Config GUI QA gate so “Rust Config Cutover Complete” cannot be claimed
until the real interactive Settings UI, controls, candidate preview, DPI, localization, and
accessibility evidence are green.

## Specification references

- `docs/spec-v1.8.md` §5.5.8 Config UI/UX.
- `docs/spec-v1.8.md` §3382 `REG-CONFIG-A11Y-001`.
- `docs/tasks/settings-uiux-operation-integration-plan.md` Verification plan.

## Required behavior / implementation contract

- Automated checks must cover keyboard tab order, focus visibility, page navigation, no-overlap,
  high-DPI geometry, high contrast fallback markers, and embedded candidate preview bounds.
- Manual evidence must be explicitly recorded for Narrator/NVDA, real Win7/Win10/Win11 host
  behavior, and any OS behavior that cannot be automated in CI.
- If manual evidence is unavailable, mark this task `MANUAL-PENDING`; do not mark Stage 4 complete.

## Required validation

- Rust Config QA and source contract tests.
- x64/x86 Debug Config build and relevant CTest filter.
- Manual evidence checklist recorded in `docs/tasks/status.md`.

## Done when

- Stage 4 Config cutover language is backed by real GUI QA evidence, not by CLI/backend tests alone.
