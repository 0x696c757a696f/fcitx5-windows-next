# RUST-R1-02 Rust repository metadata / anti-rollback model

**State:** COMPLETED
**Historical source task:** archived here; the obsolete standalone R1 repository task source was removed after the Rust-first queue cleanup.

## Gate

Start only after `RUST-R1-01` is complete and repository-state semantics remain green.

## Can prepare now

- Keep repository rollback/channel/signature fixtures visible and documented.
- Identify the exact C++ repository-state API boundary for future differential tests.

## Must not do before gate opens

- Do not merge downloader/network behavior into repository metadata work.
- Do not change rollback policy while preparing Rust migration.
