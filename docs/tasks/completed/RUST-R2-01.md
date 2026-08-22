# Current Task — RUST-R2-01 Rust launcher state machine

**Mode:** CHANGE
**Task ID:** `RUST-R2-01`
**Prerequisite:** 007 launcher C++ contract green; 020 generation model if launcher supervises drains

## Goal

Migrate launcher supervision/state logic to Rust only after the C++ crash-ledger/recovery contract is fully green.

## Specification references

- Rust R2 matrix
- Launcher sections
- `REG-LAUNCHER-LEDGER-001` / `REG-RUST-DIFF-001`

## Required behavior / implementation contract

- Encode recovery states with strong enums/types.
- Preserve persisted crash ledger, healthy-window reset, job containment, generation supervision and SafeMode semantics.
- Do not change recovery policy during language migration.

## Required validation

- C++↔Rust model/differential corpus for crash/restart/backoff/SafeMode.
- Child crash/job failure/generation supervision fixtures.
- Packaged startup smoke.

## Done when

- Rust launcher is differential-green and authoritative.
- Old authoritative C++ launcher logic removed after cutover.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
