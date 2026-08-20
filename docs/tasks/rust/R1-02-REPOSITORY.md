# Current Task — RUST-R1-02 Rust repository metadata / anti-rollback model

**Mode:** CHANGE
**Task ID:** `RUST-R1-02`
**Prerequisite:** RUST-R1-01 + 009 complete

## Goal

Migrate repository metadata verification and sequence-state model to Rust using the already-correct corruption/channel/signature contract.

## Specification references

- Rust R1 rules
- Repository sections
- `REG-REPO-STATE-001` / `REG-RUST-DIFF-001`

## Required behavior / implementation contract

- Preserve channel binding, canonicalization, signature verification inputs, sequence monotonicity, corrupt-state fail-closed behavior and explicit repair/reset.
- Use one atomic persistence model.
- Do not merge network download behavior into this task unless the current boundary already owns it.

## Required validation

- C++↔Rust differential repository corpus.
- Corrupt/missing/rollback/channel/signature fixtures.
- Artifact/SBOM/license checks.

## Done when

- Repository Rust implementation is authoritative and differential-green.
- Old C++ authoritative repository implementation removed.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
