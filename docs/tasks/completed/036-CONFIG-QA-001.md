# CONFIG-QA-001 Rust-first Settings page-by-page QA harness

**State:** COMPLETED
**Mode:** REVIEW / TEST

## Goal

Verify the existing Settings UI like a software tester, one page at a time, while adding any new test automation in Rust first.

## Scope

- Add a Rust black-box QA harness for `fcitx5-config.exe`.
- Launch the real Settings executable.
- Navigate every modern Settings page.
- Check key visible HWND bounds and overlap.
- Capture per-page screenshots for manual visual review.
- Do not replace the shipping Config app in this slice.

## Evidence

- x64/x86 Rust QA runs:
  - `out/config-ui-qa/x64/config-ui-qa.md`
  - `out/config-ui-qa/x86/config-ui-qa.md`
- Captured pages:
  - input methods
  - appearance
  - shortcuts
  - updates
  - diagnostics/repair
  - add-ons
- Rust checks:
  - `cargo fmt --all -- --check`
  - x64/x86 `cargo test -p fcitx5-config-qa --target ...`
  - x64 `cargo clippy -p fcitx5-config-qa --target x86_64-pc-windows-msvc -- -D warnings`
- Existing Settings contracts:
  - x64/x86 `config-ui-headless-smoke`
  - x64/x86 `config-ui-behavior-contract`
  - x64/x86 `config-ui-visual-contract`
  - x64/x86 `config-ui-live-preview-contract`
  - x64/x86 `config-ui-interaction-coverage`

## Result

Completed as a Rust-first QA/evidence slice. No shipping C++ Config UI change was required by this pass.
