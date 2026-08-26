# 057-CONFIG-WINDUI-ADOPTION-001

## Status

COMPLETED / WINDUI-RUST-CODE-CONSUMED-GREEN

## Scope

User explicitly required actual `huanfeng/wind-ui-rust` code adoption for the
Rust Config GUI/UX, not another design-reference-only pass.

This task vendors `windui` at upstream main commit
`62241e25e762df154c1b1f855b4db57533e516fc`, consumes it from
`fcitx5-config-poc` as a Rust path dependency, and moves the Appearance page
contract toward the windui settings shape:

- role-based palette from `windui::Theme`/`Role`;
- `Theme::form.row_height` as the comfortable row metric;
- actual `Element::setting_row`, `Element::segmented`, and `Element::switch`
  construction in the Config Rust self-check path;
- preview-first Appearance layout, with numeric controls below the preview;
- user-facing labels instead of debug `DIP` labels on the first screen.

## Acceptance

- `windui` source and licenses are present under `third_party/wind-ui-rust`.
- `rust/config-poc/Cargo.toml` depends on vendored `windui`.
- `--self-check --report` emits `windui_*` evidence fields.
- source contract requires the vendored crate, Config dependency, actual
  windui element construction, and removal of first-screen debug-DIP labels.
- x64 and x86 Rust tests compile the vendored crate.

## Non-Goals

This does not claim 056 real-host Narrator/NVDA/Win7/Win10/Win11 evidence and
does not make `REL-01` releasable.
