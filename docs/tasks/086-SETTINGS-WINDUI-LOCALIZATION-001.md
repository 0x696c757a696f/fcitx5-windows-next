# 086 — WindUI Settings locale plumbing, first shipping slice

**Status:** completed; REL-01 remains manual-pending.

## Goal

Connect the real Rust/WindUI Settings shell to the existing eight locale
catalogs, resolve the Windows display language, and let the user persist an
explicit locale through Config Core.

## Scope

- Consume the existing `locales/*.json` catalogs in the shipping WindUI Settings
  path; do not create a second translation source.
- Resolve `ui.language=system` through the existing Windows UI-language mapping;
  resolve explicit supported locale identifiers directly and fail soft to
  English for unsupported/missing catalog entries.
- Localize the six Settings navigation labels and page headings.
- Expose the language choice on the Appearance page, persist it through Config
  Core, and use it on the next launch. Do not silently discard unrelated Draft
  changes.
- Add a screenshot-only locale override so every catalog can be reviewed without
  editing the user's active settings.
- Add Rust tests for locale lookup/fallback and persisted language selection;
  add a source/contract check that navigation uses catalog keys.
- Build and run the shipping WindUI Settings executable. Capture and visually
  inspect the Settings shell in all eight locales at the default UI size and at
  the smallest supported window; record screenshots and any clipping.

## Out of scope

- Full Input/Appearance copy, plugin operation messages and metadata, Updates,
  Diagnostics, and remaining placeholder pages; queue these as later
  localization slices. This task must not be described as full Settings
  localization.
- Claiming screen-reader or release-host evidence from screenshots or unit tests.
- Changing WindUI vendor code or introducing a new localization dependency.

## Acceptance

- All eight catalogs remain strict UTF-8 JSON with identical required keys.
- `system`, each explicit locale, and unsupported locale behavior are covered.
- Language selection survives Settings close/reopen via the existing Config Core
  persistence path and takes effect on the next launch.
- All six navigation labels and their page headings come from locale catalogs;
  untranslated page-body copy is explicitly known and remains queued.
- Actual shipping-window screenshots are visually reviewed at the specified
  locales/sizes; automated contract checks are supporting evidence only.
- Rust code/tests use `#![forbid(unsafe_code)]` unless a narrowly documented
  platform/ABI boundary requires otherwise.

## Required environment

- PowerShell 7: `D:\Program Files\PowerShell\7\pwsh.exe`
- Toolchain root: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains`
- Fast tools: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast`
- Pinned Cargo: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home\bin\cargo.exe`
- Keep `CARGO_TARGET_DIR` inside this worktree.

## Results

- Embedded and consumed all eight existing locale catalogs; system language is
  resolved through the existing Windows UI-language mapping, unsupported locale
  values fall back to English, and missing keys fall back to English then key.
- Localized the six navigation items and corresponding page headings. Added an
  Appearance language selector that persists via Config Core and refuses to
  overwrite unrelated Draft edits. Screenshot preview uses `--locale-preview`
  and does not change the user's selected locale.
- Shortened the translated system-default option after real screenshots showed
  wrapping in English, Vietnamese, and Sinhala at minimum size.
- Built the real WindUI Settings window and captured/reviewed 32 screenshots in
  `out/evidence/task-086-locales`: all eight locales, Input and Appearance
  pages, at 1040x700 and 900x620. Navigation headings and language controls fit;
  Settings page-body text remains mostly Simplified Chinese and is explicitly
  not claimed complete (queued in task 087).
- x64 and x86 tests passed: Config UI 36 each; Config Core 29 each; Control Core
  68 each. Locale catalog validation, package-scoped Rust format check, invalid
  preview-mode rejection, and `git diff --check` passed. The full-workspace
  `cargo fmt --all -- --check` remains blocked by pre-existing formatting
  differences in vendored `third_party/wind-ui-rust`; no vendored file was
  changed. One signed package E2E is ignored because it requires the protected
  signer and shipping binaries.

Next: task 087 completes the remaining visible Settings copy across the same
eight locale catalogs. Full Settings localization, screen-reader evidence, and
release-host proof remain incomplete.
