# Task 086 - Candidate layout-mode Settings

**Task ID:** `CANDIDATE-LAYOUT-MODE-SETTINGS-001`
**Mode:** RUST SETTINGS / CANDIDATE INTEGRATION
**Prerequisites:** `085` complete.

## Goal

Expose the existing Rust-owned candidate model as five understandable, persistent Settings choices: `automatic`, `stacked`, `flow`, `scroll`, and `vertical_text`. The embedded preview must call the production candidate layout/render path.

## Acceptance

1. The five choices map deterministically to the existing three-axis model and persist through the typed config boundary.
2. Relevant conditional controls only: scroll direction/page capacity for `scroll`; column direction for `vertical_text`.
3. Live preview uses the shipping candidate layout/render FFI path and proves every mode with CJK, emoji, labels, comments, and selection. Horizontal flow keeps a DirectWrite-measured 4px glyph-clip guard for `gjpqy`, emoji, combining marks, CJK mixing, and ZWJ emoji. The default native rounded shell is the only outer outline; it must not have an inset gray border.
4. Rust tests cover keyboard focus, UIA names, high contrast, and fail-soft invalid config. Real Narrator/NVDA remains manual-pending.
5. x64 validation only unless the user reopens x86.
