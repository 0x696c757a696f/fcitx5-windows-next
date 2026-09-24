# 086 — WindUI Settings locale plumbing, first shipping slice

**Status:** completed; REL-01 remains manual-pending.

## Goal

Connect the real Rust/WindUI Settings shell to the existing eight locale
catalogs, resolve the Windows display language, and let the user persist an
explicit locale through Config Core.

## Scope and results

- Consumed all eight existing `locales/*.json` catalogs without creating a
  second translation source. Windows system display language is resolved via
  the existing mapping; unsupported locale falls back to English and missing
  keys fall back to English then the key.
- Localized the six Settings navigation labels and corresponding page headings.
- Added an Appearance language selector that persists through Config Core and
  refuses to overwrite unrelated Draft edits. The screenshot-only
  `--locale-preview` option supports visual review without changing the user's
  selected locale.
- Shortened the system-default option labels after screenshots exposed wrapping
  in English, Vietnamese, and Sinhala.
- Captured and visually reviewed 32 real WindUI window screenshots in
  `out/evidence/task-086-locales`: eight locales, Input and Appearance pages,
  at 1040x700 and 900x620. Headings and language controls fit at both sizes.
- Page-body and operation text remains mostly Simplified Chinese. Full Settings
  localization is explicitly incomplete and handed to task 087.

## Validation

- x64: Config UI 36, Config Core 29, Control Core 68 tests passed.
- x86: Config UI 36, Config Core 29, Control Core 68 tests passed.
- All eight strict JSON locale catalogs and matching key sets passed.
- Package-scoped Rust format check, invalid preview-mode rejection, and
  `git diff --check` passed.
- Full-workspace `cargo fmt --all -- --check` reports pre-existing formatting
  differences under vendored `third_party/wind-ui-rust`; no vendored file was
  changed. One signed-package E2E remains ignored because it requires protected
  signer and shipping binary paths.

REL-01 remains manual-pending for production signing, UAC, real-host and
screen-reader evidence, online signed plugin lifecycle, constrained-host cases,
and externally verifiable CI/publication.
