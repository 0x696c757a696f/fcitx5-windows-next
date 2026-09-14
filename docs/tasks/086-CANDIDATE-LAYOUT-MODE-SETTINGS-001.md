# Task 086 - Candidate layout-mode Settings

**Task ID:** `CANDIDATE-LAYOUT-MODE-SETTINGS-001`
**Mode:** RUST SETTINGS / CANDIDATE INTEGRATION
**Prerequisites:** `085` complete.

## Goal

Expose the existing Rust-owned candidate model as five understandable, persistent Settings choices: `automatic`, `stacked`, `flow`, `scroll`, and `vertical_text`. The embedded preview must call the production candidate layout/render path.

## Frozen behavior

- `automatic`: stable composition-scoped policy decides the existing axes; it must not oscillate while composition identity is unchanged.
- `stacked`: vertical item arrangement with paging or scrolling; candidate glyphs remain horizontal.
- `flow`: horizontal item arrangement with natural measured width and wrapping before paging.
- `scroll`: six-cell major axis, scrollable grid; presentation direction selects `6×N` or `N×6`, while `page_size` remains the maximum visible candidate count.
- `vertical_text`: one candidate per vertical glyph column; writing direction selects left-to-right or right-to-left columns.
- No waterfall layout, arbitrary user-programmed grid, theme script, or second preview renderer.

## Scope

- Rust `config-core` typed layout settings and validation, only where an existing field cannot represent the frozen behavior.
- Rust Settings shell labels, descriptions, conditional controls, persistence, and production-path live preview.
- Rust candidate/config tests and x64 screenshot/interaction evidence.

## Acceptance

1. The five user-facing choices map deterministically to the existing three-axis model and persist through the typed config boundary.
2. The UI shows only relevant controls: scroll direction/page capacity for `scroll`; column direction for `vertical_text`; no raw renderer engineering values in the primary page.
3. Live preview uses the shipping candidate layout/render FFI path and proves all five modes with CJK, emoji, labels, comments, and selected state.
4. Keyboard focus, UIA names, high contrast, and invalid config fail-soft behavior are covered by Rust-authoritative automated checks. Real Narrator/NVDA evidence remains manual-pending.
5. x64 checks only unless the user later reopens an x86 lane.
