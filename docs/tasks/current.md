# Current Task — RELEASE-01 Stable release pipeline / Build Once evidence

**Mode:** RELEASE
**Task ID:** `RELEASE-01`
**Prerequisite:** All stabilization tasks + required external evidence + intended Rust cutovers
**Evidence class:** `EXTERNAL_EVIDENCE` — never claim unrun real-host evidence passed.

## Goal

Execute the final release gate only after stabilization, required external host evidence, and selected Rust cutovers are complete.

## Specification references

- Phase 8
- Build Once principle
- Signing/SBOM/provenance sections

## Required validation

- Full declared host/release matrix.
- Authenticode/timestamp where credentials are available.
- Install/update/rollback/uninstall from final packaged bytes.
- SBOM/provenance/hash/signature consistency.

## Done when

- No unresolved required MANUAL-PENDING compatibility evidence.
- Final published artifacts trace to source commit and locked toolchains.
- No signing-stage recompilation.
