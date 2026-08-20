# Current Task — RUST-R2-02 Rust Control + process-execution boundary

**Mode:** CHANGE
**Task ID:** `RUST-R2-02`
**Prerequisite:** 008 green

## Goal

Migrate Control CLI/shared process execution to Rust after the authoritative C++ drain/timeout/cancel semantics are frozen.

## Specification references

- Rust R2 matrix
- Control/process sections
- `REG-PROC-PIPE-001` / `REG-RUST-DIFF-001`

## Required behavior / implementation contract

- Preserve exact exit/result/output-limit/cancel/process-tree semantics.
- Use safe typed command construction; no generic shell interpolation.
- Keep Config calling the typed Control API rather than duplicating business logic.

## Required validation

- 64 KiB/1 MiB/>limit/hung/early-close/nonzero/invalid-output differential corpus.
- Packaged Config→Control smoke.
- PE/license/SBOM checks.

## Done when

- Rust Control/process execution is authoritative.
- No wait-before-drain regression.
- No permanent C++/Rust selector.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
