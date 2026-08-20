# Current Task — REG-KEY-INTL-001 Language-neutral Windows KeyEvent contract

**Mode:** CHANGE
**Task ID:** `REG-KEY-INTL-001`
**Prerequisite:** 002 / CandidateModel task completed or independently verified not to conflict

## Goal

Replace the narrow `virtualKey + modifier flags` contract with a Windows key-event representation sufficient for real multilingual Fcitx input, while preserving fail-open behavior for unhandled keys.

## Specification references

- §0.5 Stabilization Gate item 3
- §4 IPC / Dispatcher contract
- Phase 2 acceptance
- Regression `REG-KEY-INTL-001`

## Required behavior / implementation contract

- Represent scan code, extended-key state, key press/release, logical/physical identity needed by the current Windows→Fcitx adapter, complete modifier state, and keyboard-layout/AltGr/dead-key semantics.
- Keep Windows-reserved/system combinations explicitly outside engine handling where required; otherwise let Fcitx decide handled vs passthrough instead of expanding a hand-written VK whitelist.
- Update protocol producer/consumer/fixtures atomically; use one current protocol version only.
- Do not grow a giant VK→KeySym switch as the platform abstraction.

## Out of scope

- Profile redesign
- Rust migration
- Candidate visual changes
- Game-specific input bypasses

## Required validation

- `REG-KEY-INTL-001`: AltGr, dead keys, extended/scancode, non-US layout, key-up.
- Regression for ordinary letters, punctuation/QuickPhrase trigger paths, Ctrl/Alt/Shift combinations, and passthrough.
- x64 + x86 TSF/protocol affected builds.

## Done when

- No duplicate/ swallowed unhandled key in the covered corpus.
- Fcitx receives the normalized event needed for multilingual engines.
- No old/new protocol dual stack remains.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
