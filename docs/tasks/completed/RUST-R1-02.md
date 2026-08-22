# RUST-R1-02 Rust repository metadata / anti-rollback model

**State:** FUTURE-GATED
**Canonical task:** `docs/tasks/rust/R1-02-REPOSITORY.md`

## Gate

Start only after `RUST-R1-01` is complete and repository-state semantics remain green.

## Can prepare now

- Keep repository rollback/channel/signature fixtures visible and documented.
- Identify the exact C++ repository-state API boundary for future differential tests.

## Must not do before gate opens

- Do not merge downloader/network behavior into repository metadata work.
- Do not change rollback policy while preparing Rust migration.
