# Current Task — CONFIG-WINDOW-EFFECTS-ADAPTER-001

**Mode:** IMPLEMENTATION
**Task ID:** `054-CONFIG-WINDOW-EFFECTS-ADAPTER-001`
**Prerequisite:** `053-CONFIG-UIKIT-DESIGN-TOKENS-001`
**Evidence class:** automated code/product contract.

## Goal

Add a Rust-owned Windows visual capability adapter for progressive enhancement across Win7,
Win10, and Win11 without scattering OS-version checks through the Config UI.

## Specification references

- `docs/spec-v1.8.md` §5.5.8 Config UI/UX.
- `docs/tasks/settings-uiux-operation-integration-plan.md` Config platform architecture.

## Required behavior / implementation contract

- Define a bounded `WindowEffects`/capability model in Rust Config for:
  - native baseline;
  - dark titlebar where available;
  - corner preference where available;
  - system backdrop/Mica where available.
- Capability detection must fail soft and keep Win7-compatible startup.
- Actual DWM calls, if added in this slice, must be runtime guarded and optional.
- Do not add WinUI/WPF/WebView runtime dependency.

## Required validation

- Rust unit tests for capability mapping using fake OS/capability inputs.
- Existing Rust Config UI preview/self-check source contracts remain green.
- Runtime security/source checks still pass.

## Done when

- Later Settings UI slices can request visual effects through one adapter boundary.
- Unsupported OS builds keep the native Settings window with no startup failure.
