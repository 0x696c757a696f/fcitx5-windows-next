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

## Current C++ boundary to freeze for differential tests

The future Rust cutover should treat these C++ entry points as the compatibility
surface to mirror and then delete:

- `src/package/repository.cpp`
  - `parse_repository_index`
  - `verify_repository_index`
  - `find_repository_package`
- `src/control/control_main.cpp`
  - `repositoryFiles`
  - `repositorySequencePath`
  - `readSequenceState`
  - `readMaxSequence`
  - `writeMaxSequence`
  - `indexMaxSequence`
  - `loadRepository`
  - `refreshRepository`
  - `repairRepositorySequenceState`

The current Rust mirror already exists in `rust/package-core/src/lib.rs`:

- `RepositoryIndex`
- `parse_repository_index`
- `verify_repository_index`
- `verify_repository_index_envelope`
- `find_repository_package`

## Fixture inventory to keep visible

- Channel binding:
  - stable build rejects a beta index
  - beta channel index remains valid only for beta builds
- Anti-rollback:
  - accepted sequence state present
  - missing sequence state after prior acceptance
  - truncated sequence state
  - corrupt sequence state
  - newer accepted sequence after repair
- Signature:
  - valid repository index signature
  - tampered repository bytes
  - revoked key
  - wrong trusted key id
  - wrong envelope object / algorithm

The authoritative regression coverage for those fixtures currently lives in:

- `tests/unit/control_repository_rollback_test.cpp`
- `tests/unit/package_core_test.cpp`

## Done when

- Repository Rust implementation is authoritative and differential-green.
- Old C++ authoritative repository implementation removed.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
