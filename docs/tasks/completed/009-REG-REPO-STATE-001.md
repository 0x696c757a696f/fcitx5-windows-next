# Current Task — REG-REPO-STATE-001 Robust repository anti-rollback state

**Mode:** CHANGE
**Task ID:** `REG-REPO-STATE-001`

## Goal

Make accepted repository sequence state atomic and corruption-aware so an established installation cannot silently fall back to sequence zero after truncation/deletion/corruption.

## Specification references

- §0.5 item 9
- Package/repository security sections
- Phase 7 prerequisites
- `REG-REPO-STATE-001`

## Required behavior / implementation contract

- Distinguish never-initialized from expected-but-missing/corrupt state.
- For an established installation, corruption/missing state fails closed to an explicit repair/reset workflow.
- Publish sequence state atomically with bounded, parseable format.
- Keep repository channel binding and signature verification semantics unchanged.

## Out of scope

- Rust migration itself
- Network UI

## Required validation

- Accept high sequence, then truncate/corrupt/delete state; stale repository must not become acceptable.
- First-run state works.
- Interrupted atomic write simulation.
- Explicit repair/reset path test.

## Done when

- No established state path maps corruption to zero.
- Atomic publication is used.
- `REG-REPO-STATE-001` passes and becomes Rust R1 differential corpus.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
