# Task 085 - Candidate WeChat-theme consistency

**Task ID:** `CANDIDATE-WECHAT-THEME-CONSISTENCY-001`
**Mode:** CODE-ONLY / VISUAL-CONTRACT REPAIR
**Result:** `COMPLETED / CANDIDATE-PILL-AND-GLYPH-GREEN`

## Goal

Align the builtin candidate selection, Settings preview, Rust renderer fallback, and Candidate PoC to solid WeChat green `#07C160` with white selected glyphs. Preserve High Contrast semantics.

## Delivered

- Project-owned light/dark selected colors are `#07C160` and white text.
- The 12-DIP outer candidate surface and 10-DIP inset selection pill are independent tokens.
- Flow candidates use a full CJK/emoji glyph budget; scroll-grid columns use only their own widest candidate.
- Ordinal and comment text use the candidate baseline instead of independent vertical centering.

## Evidence

- Commit `bfb418d`.
- x64 candidate-core Rust tests: 94/94.
- x64 light/dark stacked, flow, and grid Candidate PoC snapshots: 6/6.
- No manual-host or release evidence claimed.
