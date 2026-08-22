# RUST-R1-05 Conditional Rust elevated deployer

**State:** FUTURE-GATED / DECISION-GATED
**Canonical task:** `docs/tasks/rust/R1-05-DEPLOYER-CONDITIONAL.md`

## Gate

Start only after installer/register/bootstrap semantics are stable and Rust updater/deployer evidence justifies changing the privileged boundary.

## Can prepare now

- Record the privilege-boundary operation list.
- Prepare an ADR template for either Rust cutover or keep-minimal-C++ decision.

## Prepared decision record

- `docs/adr/0007-rust-r1-deployer-decision.md`

## Must not do before gate opens

- Do not enlarge the elevated operation set.
- Do not force Rust if Win7/installer/toolchain evidence is insufficient.
