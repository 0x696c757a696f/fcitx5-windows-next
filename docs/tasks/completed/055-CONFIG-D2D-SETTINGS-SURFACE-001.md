# Current Task — CONFIG-D2D-SETTINGS-SURFACE-001

**Mode:** IMPLEMENTATION
**Task ID:** `055-CONFIG-D2D-SETTINGS-SURFACE-001`
**Prerequisite:** `053-CONFIG-UIKIT-DESIGN-TOKENS-001`, `054-CONFIG-WINDOW-EFFECTS-ADAPTER-001`
**Evidence class:** automated code/product contract plus screenshot evidence.

## Goal

Move the Rust Settings visual surface from GDI/owner-draw polish toward a bounded D2D/DWrite
Settings Surface for cards, section rows, navigation items, and preview containers while preserving
native HWND controls for behavior-sensitive input.

## Specification references

- `docs/spec-v1.8.md` §5.5.8 Config UI/UX.
- `docs/spec-v1.8.md` §5.5.10 Appearance 页面与 Live Preview.
- `docs/tasks/settings-uiux-operation-integration-plan.md` Shared visual contract.

## Required behavior / implementation contract

- Use the design tokens from `053`; do not introduce new raw visual constants.
- Render only bounded product components: NavigationItem, Section/Card, SettingRow container,
  Banner/status row, and PreviewSurface.
- Keep native Edit/ComboBox/ListBox/ListView where IME, keyboard, font picker, selection, or UIA
  behavior matters.
- Ensure every custom-painted area clears its assigned rect before drawing.
- Device-loss/repaint must fail soft; stale pixels/重影 are regressions.

## Required validation

- Existing x64/x86 Rust Config preview QA remains green.
- Screenshot evidence proves page surfaces do not overlap or leave stale pixels.
- Source contract prevents growing a generic UI framework.

## Done when

- Main Settings chrome/cards are visually modern through the product Settings Surface, not through
  bare VC6-style label/control grids.
- Candidate preview remains embedded and uses the production candidate preview contract.
