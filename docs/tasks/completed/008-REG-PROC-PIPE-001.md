# Current Task — REG-PROC-PIPE-001 Authoritative bounded child-process execution

**Mode:** CHANGE
**Task ID:** `REG-PROC-PIPE-001`
**Prerequisite:** C++ behavior frozen before R2

## Goal

Replace duplicate Config/Control child execution paths with one authoritative implementation that drains stdout/stderr concurrently, bounds output, supports timeout/cancel, and contains the child process tree.

## Specification references

- §0.5 item 8
- Phase 5
- `REG-PROC-PIPE-001`

## Required behavior / implementation contract

- Reuse/factor the already-correct concurrent drain pattern rather than keeping Control's wait-then-read implementation.
- Bound captured output and define truncation/result semantics.
- Timeout/cancel must terminate or contain the intended child/process tree; parent must not report final failure while a mutating child keeps running.
- Keep handle inheritance minimal.

## Out of scope

- Rust R2 rewrite (later)
- Unrelated CLI redesign

## Required validation

- Child writes 64 KiB, 1 MiB, and > configured limit.
- Hung child timeout/cancel.
- Early exit/pipe close/non-zero exit.
- Invalid UTF-8/binary-ish output handled according to current API contract.
- No pipe-buffer deadlock.

## Done when

- Config and Control call one authoritative process-exec primitive.
- No wait-before-drain deadlock path remains.
- Process tree is bounded/reaped according to contract.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
