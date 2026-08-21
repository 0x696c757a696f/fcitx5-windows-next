# RUST-R1-03 Rust updater/downloader transaction

**State:** FUTURE-GATED
**Canonical task:** `docs/tasks/rust/R1-03-UPDATER-DOWNLOADER.md`

## Gate

Start only after generation-drain semantics from `REG-UPDATE-TSF` and Rust R1 package/repository prerequisites are complete.

## Can prepare now

- Preserve update-generation fixtures and deployment-core evidence.
- Document downloader transaction boundaries and failure categories.

## Must not do before gate opens

- Do not add real public-network checks as correctness oracle.
- Do not change TSF generation-drain protocol while preparing migration.
