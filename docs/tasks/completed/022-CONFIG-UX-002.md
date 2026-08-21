# CONFIG-UX-002 Input methods page completion

**State:** COMPLETED
**Plan source:** `docs/tasks/settings-uiux-operation-integration-plan.md`

## Goal

Make enabled input methods visible and selectable in the modern Settings surface.

## Acceptance

- Enabled input methods are visible without opening a combo box.
- Active/default input method is clearly marked.
- Refresh and save operations use the Control boundary.
- Empty/error states are localized.
- Interaction and visual contracts prove the modern card/list state does not overlap or disagree with backend state.

## Evidence

- x64/x86 build passed for `fcitx5_config_app` and `fcitx5_config_parser_test`.
- x64/x86 tests passed:
  - `config-ui-i18n-check`
  - `config-ui-resource-check`
  - `config-ui-behavior-contract`
  - `config-ui-visual-contract`
  - `config-ui-interaction-coverage`
  - `config-toml-contract`

## Result

Completed.
