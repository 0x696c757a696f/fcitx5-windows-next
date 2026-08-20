# Current Task — REG-CONFIG-VISUAL-001 Config visual system and accessible D2D Settings components

**Mode:** CHANGE
**Task ID:** `REG-CONFIG-VISUAL-001`
**Prerequisite:** Core stabilization 003–014 should be green; 015 may be MANUAL-PENDING per PLAN

## Goal

Replace the VC6/property-sheet visual feel with a coherent product-specific Settings system while keeping WTL/Win32 hosting and native controls where they provide better text/IME/accessibility semantics.

## Specification references

- §5.5 Config human factors
- §5.6.4 Config visual system
- §13.9 Config information architecture
- Phase 6
- `REG-CONFIG-VISUAL-001` / `REG-CONFIG-A11Y-001`

## Required behavior / implementation contract

- Create one typed DesignTokens source for spacing, typography, corner radius, surfaces, control metrics, focus, states, icon metrics and animation budget.
- Provide reusable D2D components only for NavigationItem, SettingRow, Toggle, SegmentedControl, Slider, ThemeCard, InputMethodCard, Banner and Preview as actually needed.
- Keep native HWND for EDIT/complex text/system dialogs and any control where custom UIA cannot reach equivalent usability.
- Implement System/Light/Dark/High Contrast for Config itself.
- Reorganize navigation by user task: Input Methods, Appearance, Shortcuts, Add-ons/Extensions, Updates, Diagnostics/Repair; avoid Theme/Repair as engineering-first top-level pages.
- Every custom interactive component exposes keyboard/focus and appropriate UI Automation patterns.

## Out of scope

- Changing GUI framework
- Rust rewrite
- Candidate renderer rewrite

## Required validation

- `REG-CONFIG-VISUAL-001` at 100/125/150/200% DPI in Light/Dark/High Contrast.
- `REG-CONFIG-A11Y-001` with keyboard-only and available Narrator/NVDA smoke.
- No clipping, invisible focus, color-only state, or default bare property-grid appearance.
- Canonical component screenshots plus structural assertions; do not gate every pixel of every page.

## Done when

- Config no longer looks like a collection of raw STATIC/EDIT/COMBO/CHECKBOX rows.
- Design tokens/components are reused rather than reimplemented per page.
- Accessibility remains equivalent or better than the replaced native controls.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
