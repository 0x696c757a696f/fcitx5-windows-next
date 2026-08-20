# Current Task — REG-PKG-WINPATH-001 Freeze Windows hostile-path corpus before Rust R1

**Mode:** CHANGE
**Task ID:** `REG-PKG-WINPATH-001`

## Goal

Define one authoritative Windows package-path acceptance policy and corpus covering reserved device names, separator/case/reparse edge cases, traversal, trailing-dot/space and control characters.

## Specification references

- §0.5 item 14
- Package path/security sections
- Phase 7 prerequisite
- `REG-PKG-WINPATH-001`

## Required behavior / implementation contract

- Add explicit rejection for DOS device components such as CON/PRN/AUX/NUL/COM1..9/LPT1..9 including extension forms where Windows resolves them as devices.
- Define case-collision, separator, trailing dot/space, control-character, absolute-path, colon, dot/dotdot, and reparse/symlink policy.
- Use one corpus consumed by current C++ implementation and later Rust implementation.
- Verify extracted/staged filesystem objects after creation to prevent reparse escape.

## Out of scope

- Starting Rust before the corpus is green

## Required validation

- `REG-PKG-WINPATH-001` complete corpus.
- Archive extraction/property/fuzz corpus around path normalization.
- Case-collision and reserved-device fixtures.
- Current C++ implementation produces deterministic expected decisions.

## Done when

- Path policy is documented and machine-testable.
- C++ baseline passes the corpus.
- Corpus is ready as mandatory R1 differential input.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
