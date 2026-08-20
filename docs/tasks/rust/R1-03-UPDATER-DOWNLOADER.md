# Current Task — RUST-R1-03 Rust updater/downloader transaction

**Mode:** CHANGE
**Task ID:** `RUST-R1-03`
**Prerequisite:** 020 generation contract + R1-01/02

## Goal

Migrate network/update management-plane logic to Rust while preserving signed manifest, hash, bounded download, staging, generation-drain and rollback contracts.

## Specification references

- Rust R1 rules
- §7 update
- §7.3.1 generation draining
- `REG-UPDATE-TSF-001/002` / `REG-RUST-DIFF-001`

## Required behavior / implementation contract

- No realtime input data enters networking.
- Verify manifest/signature/hash before activation.
- Bound sizes/timeouts and use safe temp/staging publication.
- Preserve generation-specific activation/drain semantics; do not add protocol compatibility.
- Keep Windows update/file operations in a small auditable adapter.

## Required validation

- C++↔Rust differential update fixtures.
- Interrupted/invalid download, bad signature/hash, rollback and generation-drain tests.
- Clean packaged artifact smoke with network fixture, not real public network as correctness oracle.

## Done when

- Rust updater/downloader passes all security/transaction regressions.
- No permanent C++/Rust runtime selector remains.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
