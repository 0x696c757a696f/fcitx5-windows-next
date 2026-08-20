# Current Task — RUST-R1-05 Conditional Rust elevated deployer

**Mode:** CHANGE
**Task ID:** `RUST-R1-05`
**Prerequisite:** 011/012 installer semantics + R1-03

## Goal

Evaluate and, only if evidence supports it, migrate the minimal elevated deployer to Rust without enlarging the privilege boundary.

## Specification references

- Rust R1 matrix
- Installer/deployer privilege sections
- Legacy policy

## Required behavior / implementation contract

- First record whether migration materially reduces risk without breaking Win7/installer/toolchain constraints.
- Keep privileged surface tiny: validated staging root, exact operation set, no generic command execution.
- If Win7 Rust/toolchain evidence is insufficient, keep the proven C++ Legacy implementation and record the decision instead of forcing Rust.

## Required validation

- Privilege-boundary/validated-artifact tests.
- Modern packaged install/update smoke.
- Legacy PoC only if claiming Rust Legacy support.
- Differential operation corpus if migration proceeds.

## Done when

- Either a justified Rust cutover with all gates green, or an explicit ADR to keep the minimal C++ deployer.
- No duplicated long-term business logic.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
