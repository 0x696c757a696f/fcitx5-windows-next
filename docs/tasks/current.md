# Current Task - RELEASE-01 Stable release pipeline / Build Once evidence

**Mode:** RELEASE
**Task ID:** `RELEASE-01`
**Prerequisite:** All stabilization tasks + required external evidence + intended Rust cutovers
**Evidence class:** `EXTERNAL_EVIDENCE` - never claim unrun real-host evidence passed.

## Goal

Execute the final release gate only after stabilization, required external host evidence, and selected Rust cutovers are complete.

## Specification references

- Phase 8
- Build Once principle
- Signing/SBOM/provenance sections

## Required behavior / implementation contract

- Build each declared Modern/Legacy lineage once from one source commit and locked toolchain.
- Test/promote the same artifacts; do not recompile in signing/publish jobs.
- Generate final hash/manifest/attestation for actual signed release bytes.
- Unify C++/MSYS2/Cargo dependencies in SBOM/notices/provenance.
- Validate channel identity, key rotation/revocation, rollback and package-manager ownership.
- Retain the useful Phase 8 requirements from removed historical docs: `package` compiles one
  x64-with-x86-TSF lineage and records its source commit; `release` promotes exactly that stage,
  injects protected public keyring material, Authenticode-signs/timestamps PE files when
  credentials exist, signs packages/installers, and never recompiles in the signing/publish job.
- Release artifacts must include final hashes, signed manifest, SPDX SBOM from actual staged files
  and dependency inventory, SLSA-shaped provenance, WinGet metadata, Chocolatey metadata, and final
  smoke that rechecks hash/signature/SBOM consistency.

## Required validation

- Full declared host/release matrix.
- Authenticode/timestamp where credentials are available.
- Install/update/rollback/uninstall from final packaged bytes.
- SBOM/provenance/hash/signature consistency.

## Done when

- No unresolved required MANUAL-PENDING compatibility evidence.
- Final published artifacts trace to source commit and locked toolchains.
- No signing-stage recompilation.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
