# 058-CONFIG-WINDUI-SETTINGS-SHELL-001

## Status

COMPLETED / WINDUI-SETTINGS-SHELL-DEFAULT-GREEN

## Scope

Make the normal interactive Rust Config window use the actual vendored
`huanfeng/wind-ui-rust` settings shell code path, visually aligned with
upstream `settings-input.png`, instead of opening the old Win32/D2D preview host
by default.

## Completed Behavior

- `RunMode::Interactive` now routes through `run_default_interactive_window`.
- No-argument user launch opens the real `windui::App` Settings shell unless the
  QA preview-state environment is present.
- `--screenshot PATH` is accepted and delegated to windui screenshot handling.
- The windui shell uses a frameless titlebar, left search/navigation, right
  settings pages, grouped cards, `setting_row_desc`, `segmented`, `switch`,
  `stepper`, `slider`, `dropdown`, `text_input`, and bottom status/actions.
- The old Win32/D2D preview host remains only as
  `rust-config-win32-qa-preview-host` behind `FCITX5_CONFIG_RUST_PREVIEW_STATE`
  or explicit smoke/test modes.

## Evidence

- x64/x86 `cargo test --locked -p fcitx5-config-poc`.
- x64/x86 Debug CMake build of `fcitx5_config_app` and
  `fcitx5_source_contract_test`.
- x64/x86 CTest `rust-config-ui-preview-qa`, `rust-config-poc-contract`,
  `rust-config-poc-window-smoke`, and `source-contract`.
- `tools/check-dependencies.ps1`.
- `tools/check-licenses.ps1`.
- `tools/check-runtime-security.ps1 -SourceOnly`.
- `tools/check-text-format.ps1`.
- `git diff --check`.
- Screenshot:
  `out/build/windows-x64-dev/windui-settings-shell.png`.

## Non-Goals

This does not claim release readiness or Stage 4 real-host accessibility
completion. Narrator/NVDA, real Win7/Win10/Win11 host evidence, production
signing/key evidence, and release asset evidence remain external gates.
