# Current Task — RUST-R1-04 Rust provider policy/runner

**Mode:** CHANGE
**Task ID:** `RUST-R1-04`
**Prerequisite:** 008 process execution semantics + R1-01

## Goal

Migrate the isolated provider-management boundary to Rust only after its command/input/output policy is frozen and bounded.

## Specification references

- Rust R1 provider sections
- Package/provider security policy
- `REG-RUST-DIFF-001`

## Required behavior / implementation contract

- Provider input/output schema is bounded and validated.
- No arbitrary shell proxy or unvalidated command construction.
- Timeout/cancel/process containment follows the authoritative process-exec contract.
- Provider never receives live key/preedit/candidate/commit content unless a separately approved product requirement exists.

## Required validation

- Differential provider fixtures.
- Malformed/oversize/hung provider cases.
- Package/artifact smoke and dependency gates.

## Done when

- Rust provider runner is authoritative and bounded.
- No new input-data-plane network path exists.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
