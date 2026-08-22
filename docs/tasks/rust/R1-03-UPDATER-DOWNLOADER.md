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

## Current C++ boundary to freeze for differential tests

The Rust cutover should mirror these C++ transaction surfaces first and only
then remove them:

- `src/package/downloader_main.cpp`
  - `download`
  - `--download`
  - `--download-signed-metadata`
- `src/updater/deployment_core.cpp`
  - `read_deployment_state`
  - `write_deployment_state`
  - `begin_activation`
  - `mark_current_healthy`
  - `rollback_target`
  - `finish_rollback`
  - `clear_previous_known_good`
- `src/updater/updater_main.cpp`
  - `--activate`
  - `--health`
  - `--rollback`
  - `--cleanup-previous`
  - `--install-tsf-dll`
  - `--cleanup-old-tsf-dlls`
  - `--activate-runtime-generation`
  - `--publish-generation`
  - `--generation-status`

## Transaction/failure fixtures to keep visible

- interrupted download before publication
- invalid download destination/hash
- bad signature or hash
- repository rollback rejection
- generation activation failure after staging
- generation cleanup with pending previous-known-good state
- TSF in-use DLL staging and cleanup without host kill

Current regression coverage lives in:

- `tests/unit/deployment_core_test.cpp`
- `tests/unit/updater_cleanup_test.cpp`
- `tests/unit/control_repository_rollback_test.cpp`
- `tests/integration/control_package_integration_test.cpp`

## Done when

- Rust updater/downloader passes all security/transaction regressions.
- No permanent C++/Rust runtime selector remains.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
