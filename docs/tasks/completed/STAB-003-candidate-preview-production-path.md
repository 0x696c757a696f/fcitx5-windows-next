# STAB-003 — Candidate Preview Production Path

Status: completed / automated evidence green / host UIA evidence manual-pending.

Goal:
Replace the Settings candidate preview's fake/demo drawing with the shipping Rust
candidate layout and rendering path, while preserving draft → Apply/Cancel behavior.

Completed scope:

- Settings calls the production Rust CandidateModel/layout/resolved-theme/render path.
- The corpus covers CJK, Latin, punctuation, emoji, comments, selection, and all
  supported layouts.
- Draft layout, axis, page size, font, and appearance mode flow through the
  same typed snapshot used by Apply/Cancel.
- Invalid or incomplete preview input clears the cached image fail-soft.
- The old hand-drawn chips/rows preview is deleted; no WindUI vendor source or
  patch was changed.

Automated evidence:

- `fcitx5-candidate-core`: 99 tests passed.
- `fcitx5-config-core`: 9 tests passed.
- `fcitx5-config-poc`: 26 tests passed for each executable entry.
- `git diff --check` passed.

Truth inventory:

`candidate.preview` and `theme.mode` are `FullyBound` in
`STAB-002-settings-control-truth-inventory.md`.

Remaining external evidence:

Real desktop visual parity, Narrator/UIA, and host accessibility remain
MANUAL-PENDING under `REL-01-RELEASE-GATE.md`.
