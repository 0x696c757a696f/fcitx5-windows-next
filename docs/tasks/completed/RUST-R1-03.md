# RUST-R1-03 Rust updater/downloader transaction

**State:** COMPLETED
**Historical source task:** archived here; the obsolete standalone R1 updater/downloader task source was removed after the Rust-first queue cleanup.

## Gate

Start only after generation-drain semantics from `REG-UPDATE-TSF` and Rust R1 package/repository prerequisites are complete.

## Can prepare now

- Preserve update-generation fixtures and deployment-core evidence.
- Document downloader transaction boundaries and failure categories.

## Must not do before gate opens

- Do not add real public-network checks as correctness oracle.
- Do not change TSF generation-drain protocol while preparing migration.
