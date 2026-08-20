# Current Task — RUST-R1-01 Rust workspace + package core/path model

**Mode:** CHANGE
**Task ID:** `RUST-R1-01`
**Prerequisite:** 014 corpus green; 009 repository-state semantics available where shared

## Goal

Introduce the first real Rust R1 target by migrating the package-core/parser/path policy behind the already-frozen C++ contract and hostile-path corpus.

## Specification references

- §0.5 Rust R1
- §11.11 dual toolchain
- §13.5/13.6 Rust rules
- Appendix I
- `REG-PKG-WINPATH-001` / `REG-RUST-DIFF-001`

## Required behavior / implementation contract

- Create `rust-toolchain.toml`, committed `Cargo.lock`, and the minimal workspace only in the same change as this real target.
- Model package ID, safe relative path, verified artifact and related domain states with strong types.
- Use the exact frozen manifest/path/signature semantics; Rust is not permission to reinterpret Windows path rules.
- Keep unavoidable Win32/unsafe in a very small platform adapter; package business logic stays safe Rust where feasible.
- Integrate Cargo dependencies into license/SCA/SBOM/provenance and source scans.

## Required validation

- Run the same package/path/golden/invalid/fuzz corpus against C++ and Rust.
- `REG-RUST-DIFF-001` for this component.
- Clean-runner `cargo --locked` build and packaged artifact smoke.
- PE/min-OS/network-boundary checks for produced Rust artifact.

## Done when

- Rust package core matches correct C++ semantics.
- All Cargo dependencies appear in inventory/SBOM/notices.
- Cutover deletes the old authoritative C++ implementation rather than leaving runtime dual stack.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
