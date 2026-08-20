# Current Task — REG-CONFIG-LIVE-001 Appearance live production preview and progressive disclosure

**Mode:** CHANGE
**Task ID:** `REG-CONFIG-LIVE-001`
**Prerequisite:** 016 visual component system complete

## Goal

Make Appearance use the production Candidate renderer as an inline/live preview and apply reversible visual changes immediately, while hiding renderer-engineering knobs from the default user path.

## Specification references

- §5.6 Appearance
- §13.9 Config ownership
- Phase 6
- `REG-CONFIG-LIVE-001`

## Required behavior / implementation contract

- Preview must reuse the real Candidate layout/rendering code; do not build a separate fake renderer.
- Theme, font size, layout and other low-risk reversible appearance changes update preview immediately and propagate according to the current typed config contract.
- Reset removes the override and returns to inherited theme/default semantics.
- Default Appearance shows high-value controls only; max width/scroll cell geometry and similar renderer knobs move behind Advanced/More appearance.
- Avoid repeated Apply→Preview loops for reversible visual settings.

## Required validation

- `REG-CONFIG-LIVE-001`: theme/font/layout changes update the inline production preview.
- Reset/inheritance test.
- Invalid theme/config fails safe without blocking input.
- Keyboard and High Contrast interaction.

## Done when

- Appearance can be evaluated without launching a separate demo window for every change.
- No second renderer implementation exists.
- Default page meets cognitive-load limits.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
