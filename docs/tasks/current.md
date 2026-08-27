# Current Task - 065 Rust Config Core transaction contract

**Mode:** CHANGE / CODE-ONLY
**Task ID:** `CONFIG-CORE-TRANSACTION-CONTRACT-001`
**Prerequisite:** `064` automated Config/package evidence green; `RELEASE-01` remains parked.
**Evidence class:** automated x64/x86 contract, differential, and fault-injection evidence; real-host
Settings evidence remains owned by `056`/`RELEASE-01`.

## Goal

Make the existing Rust Config Core the only Config state/transaction authority for `Current`, `Draft`,
and `Defaults`, including validation, diff, atomic commit, last-known-good recovery, and the shared
GUI/CLI/test contract.

## Specification references

- 0.1 principles 1, 10, 13, 28, 32, 33
- 5.4, 6.2, 7.2, 10.1-10.2, 13.9.10-13.9.14, 13.10.13
- `docs/current.md` 2026-08-27 integration freeze

## Constraints

- Before implementation, fully read `ponytail`, `rust-skills`, and `tdd`; use a worktree-local
  `CARGO_TARGET_DIR`.
- New product logic is Rust. C++ may only remain a thin Windows ABI/renderer adapter; do not revive
  the legacy Config shell or add a generic GUI framework, a second Config truth, or a permanent
  protocol dual stack.
- Keep Fcitx-owned settings in the Fcitx Config API. Candidate preview reads a Draft snapshot through
  the existing real renderer path and never commits it. Preserve ML-DSA-65 v2 package behavior.

## Required behavior

- GUI, CLI, and tests use one typed model and the same `validate`/`diff`/transaction entry points.
- CLI is a first-class frontend of that Core. Expose `get`/`set`/`validate`/`diff`/`reset`/`import`/
  `export`/`doctor` and plugin/theme commands incrementally where the underlying capability exists;
  commands must not parse/write files or duplicate defaults independently, and this task need not
  implement every command.
- `Current` is the committed state, `Draft` is editable and discardable, and `Defaults` has explicit
  reset/inheritance semantics. Apply validates the complete Draft before a staged write, reread,
  validation, and atomic replace; failure leaves Current unchanged.
- Successful commits preserve one usable last-known-good recovery record. Startup validation,
  migration, or plugin-config failure follows current, last-known-good, then compiled safe defaults;
  recovery preserves basic input and accessibility and does not rewrite a bad file speculatively.

## Acceptance

- Rust unit/property or fault tests cover apply/cancel/reset, invalid Draft, interrupted staged write,
  reread mismatch, recovery order, and GUI/CLI semantic equivalence on x64 and x86.
- Existing Config/candidate preview/package smoke remains green. Run focused formatting, diff, and
  source-contract checks; record unavailable Narrator/NVDA/Win7/10/11 evidence as `MANUAL-PENDING`.
- Update `status.md`, archive this task, and advance only to task 066 when automated acceptance is green.
