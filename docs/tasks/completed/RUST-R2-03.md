# Current Task — RUST-R2-03 Rust diagnostics/repair model where justified

**Mode:** CHANGE
**Task ID:** `RUST-R2-03`
**Prerequisite:** R2-02; 012 register/bootstrap contract

## Goal

Move diagnostics/repair data modeling to Rust only where it benefits from typed parsing/state handling; keep Windows registration/bootstrap system calls in the minimal C++ layer unless evidence says otherwise.

## Specification references

- Rust R2 matrix
- Diagnostics/Repair sections
- Register/bootstrap Keep-C++ decision

## Required behavior / implementation contract

- Diagnostics never exposes live input text/history.
- Reuse authoritative package/control/registration owners; do not create shadow state.
- Do not migrate thin register/bootstrap simply for language consistency.

## Required validation

- Good/bad health fixtures.
- Repair dry-run/plan/result schema tests.
- Config diagnostics integration smoke.

## Done when

- Only justified management logic migrates.
- System-side C++ helpers remain minimal and authoritative where specified.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
