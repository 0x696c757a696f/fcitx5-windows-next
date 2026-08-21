# Current Task — CONFIG-UX-001 Settings operation inventory and localized status

**State:** COMPLETED
**Plan source:** `docs/tasks/settings-uiux-operation-integration-plan.md`
**Scope:** Settings operation inventory, localized status/dialog string coverage, and language selector model.

## Goal

Make the modern Settings surface explicit about every operation it exposes and ensure user-facing status/dialog strings are localizable before deeper page work continues.

## In scope

- Inspect the current Settings implementation and tests before changing code.
- Add or update the Settings operation inventory used by the modern UI.
- Add missing locale keys for operation status, empty states, dialog text, package trust states, and language selection.
- Add the language selector model/persistence entry point if not already present.
- Add or update tests that ensure locale key parity and no hard-coded normal Settings user strings for the touched surface.

## Out of scope

- Full input-method management UI.
- Full live candidate preview implementation.
- Full font picker.
- Full advanced appearance page.
- PQC signing implementation.
- Real online official add-on repository enablement.

## Acceptance

- Settings has a defined operation/status inventory for its current modern pages.
- All newly visible operation/status strings exist in `locales/en-US.json` and `locales/zh-CN.json`.
- A Settings language option model exists for system default, English, and Simplified Chinese, even if immediate runtime language reload is deferred.
- Missing trusted official repository is represented as a localized unavailable state, not as fake downloadable plug-ins.
- Focused Config i18n/resource/behavior tests pass on x64 and x86 where available.

## Evidence

- JSON locale validation passed for `locales/en-US.json` and `locales/zh-CN.json`.
- x64/x86 build passed for `fcitx5_config_app` and `fcitx5_config_parser_test`.
- x64/x86 tests passed:
  - `config-ui-i18n-check`
  - `config-ui-resource-check`
  - `config-ui-behavior-contract`
  - `config-toml-contract`
- Explicit language override diagnostics passed:
  - x64 `fcitx5-config.exe --lang=zh-CN --ui-contract-test`
  - x64 `fcitx5-config.exe --lang=en-US --ui-contract-test`
  - x86 `fcitx5-config.exe --lang=zh-CN --ui-contract-test`
  - x86 `fcitx5-config.exe --lang=en-US --ui-contract-test`

## Result

Completed.
