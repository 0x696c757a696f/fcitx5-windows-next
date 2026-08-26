# Current Task — CONFIG-UIKIT-DESIGN-TOKENS-001

**Mode:** IMPLEMENTATION
**Task ID:** `053-CONFIG-UIKIT-DESIGN-TOKENS-001`
**Prerequisite:** `052-CONFIG-STAGE4-VISUAL-REDRAW-001` completed; `REL-01` remains parked on external release evidence.
**Evidence class:** automated code/product contract.

## Goal

Establish a Rust-owned Settings UI design-token foundation so the Config window stops
evolving through scattered Win32 constants and has a stable base for modern, non-overlapping,
DPI-aware Settings slices.

## Specification references

- `docs/spec-v1.8.md` §5.5.8 Config UI/UX.
- `docs/spec-v1.8.md` §5.5.10 Appearance 页面与 Live Preview.
- `docs/tasks/settings-uiux-operation-integration-plan.md` Config platform architecture,
  shared visual contract, WindInput/清风 reference refresh, and implementation slices.

## Required behavior / implementation contract

- Rust Config must declare one small design-token source for:
  - spacing scale;
  - sidebar/nav/content geometry;
  - control heights;
  - typography heights/weights;
  - palette for background/sidebar/content/header/accent/text;
  - focus-ring and disabled-surface tokens for later owner-drawn controls.
- The Rust layout contract and Win32 adapter must consume that token source for shared
  Settings geometry and palette instead of independently duplicating key values.
- Keep native HWND controls for behavior-sensitive controls; this task does not migrate to
  WinUI/WPF/WebView or create a generic UI framework.
- Preserve the embedded candidate preview path and no-overlap contract.
- Document that no mature trusted Win32+Rust+Direct2D Config skill was adopted; external UI/UX
  guidance is input to this product-specific token system only.

## Required validation

- `cargo fmt -p fcitx5-config-poc -- --check`
- `cargo test --locked -p fcitx5-config-poc --target x86_64-pc-windows-msvc`
- `cargo test --locked -p fcitx5-config-poc --target i686-pc-windows-msvc`
- `cmake --build out/build/windows-x64-dev --config Debug --target fcitx5_config_app fcitx5_source_contract_test --parallel`
- `ctest --test-dir out/build/windows-x64-dev -C Debug --output-on-failure -R "rust-config-ui-preview-qa|source-contract"`

## Done when

- Rust Settings design tokens are present and consumed by both layout evidence and the Win32
  adapter surface.
- Existing Rust Config UI/self-check/preview tests remain green.
- Source contract prevents regression to scattered tokenless Settings constants.
- `docs/tasks/status.md` records HEAD, files changed, tests, and result.
