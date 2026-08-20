# Current Task — REG-WARMUP-001 Side-effect-free Engine warmup

**Mode:** CHANGE
**Task ID:** `REG-WARMUP-001`
**Prerequisite:** 005 InputContext semantics fixed first

## Goal

Remove synthetic user-text keystrokes from generic warmup and use only explicit preload/loading mechanisms that cannot mutate user learning, history, commit state, or global addon state.

## Specification references

- §0.5 item 5
- Phase 3
- `REG-WARMUP-001`

## Required behavior / implementation contract

- Remove the synthetic `n`/text-key warmup path.
- Prefer explicit engine/addon preload APIs; if none exists, allow no warmup rather than simulating user input.
- Document any addon-specific preload hook and prove it has no input/learning side effect.

## Required validation

- `REG-WARMUP-001`: startup/preload produces no commit, composition, learned word, history, or user-dictionary mutation.
- Cold-start smoke without warmup remains bounded/fail-open.
- Repeat warmup and restart tests.

## Done when

- No generic synthetic text key remains.
- Warmup has no observable user-state mutation.
- Cold start still respects the input timeout contract.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
