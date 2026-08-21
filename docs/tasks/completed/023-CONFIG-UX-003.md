# CONFIG-UX-003 In-window live candidate preview

**State:** COMPLETED
**Plan source:** `docs/tasks/settings-uiux-operation-integration-plan.md`

## Goal

Render a deterministic live candidate preview inside Settings and update it immediately as appearance values change.

## Acceptance

- Preview is drawn inside the Settings content area.
- Preview reflects mode, layout, font, size, and supported basic appearance values.
- Preview includes Chinese, Latin, punctuation, and emoji samples.
- Color emoji fallback is preferred where available; black-and-white emoji is tracked as a regression to investigate.
- No candidate labels, text, comments, emoji, preedit text, or controls overlap.
