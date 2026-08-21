# Current Task — REG-CONFIG-VISUAL-001 Reopened modern Settings surface

**Mode:** CHANGE
**Task ID:** `REG-CONFIG-VISUAL-001`
**State:** REOPENED
**Prerequisite:** Core stabilization 003–014 green; 015 may remain MANUAL-PENDING per `PLAN.md`

## Goal

Replace the user-facing Config surface with a coherent modern Settings experience for
Fcitx5 for Windows Next. The previous 016 implementation is not accepted as complete:
manual visual evidence showed a traditional Win32 property-sheet/tooling surface with
raw controls, sparse empty pages, copied package-manager pages, and weak product
information architecture.

This task is not a request for spacing-only polish. The intended vertical slice is a
real visual/interation-model replacement while preserving the existing WTL/Win32 host.

## Specification references

- §5.6 Config / Settings UX
- §5.6.4A Product/TSF icons, as it affects Config product surfaces
- Phase 6
- `REG-CONFIG-VISUAL-001`

## Current visual evidence

The following user-provided screenshots are the failing before-baseline:

- Input Methods page: raw checkbox, empty combobox, and isolated Apply action.
- Appearance page: raw combobox/listbox/edit controls, visible internal theme metadata,
  overlapped layout, and internal tuning controls on the default surface.
- Shortcuts page: large empty textbox-like placeholder.
- Updates page: copied package-manager layout with listbox/details textbox/buttons.
- Diagnostics & Repair page: sparse buttons plus huge empty textbox.
- Add-ons & Extensions page: copied Updates layout with clipped package rows.

## Required behavior / implementation contract

- Preserve WTL/Win32 as the application host; do not introduce a new GUI framework.
- Keep native controls only where they are genuinely useful for text input, file/system
  dialogs, accessibility-critical editing, or platform integration.
- Replace the default user-facing interaction model with D2D-rendered product
  components:
  - `NavigationItem`
  - `SettingRow`
  - `Toggle`
  - `SegmentedControl`
  - `Slider`
  - `InputMethodCard`
  - `AddonCard`
  - `ThemeCard`
  - `StatusRow`
  - `Banner`
  - `CandidatePreview`
- Navigation must no longer look like a column of bordered Win32 buttons. Use compact
  grouped navigation with a clear selected state such as a subtle background and/or
  left accent indicator.
- Product name is `Fcitx5 for Windows Next`. Window title should use the product name
  without a version suffix; version and technical details belong in Diagnostics/About.
- Default window target is roughly `1100 x 720`; minimum is roughly `860 x 600`.
  Sidebar should be roughly `220–240px`. Main content should not stretch endlessly on
  wide windows; use a max content width around `820–900px`.
- Freeze a consistent visual scale:
  - Page title: 28px / semibold
  - Section title: 16px / semibold
  - Setting title: 14px
  - Description: 12px
  - Normal UI: 14px
  - Page padding: 32px
  - Section gap: 28px
  - Row gap: 12px
  - Setting row: 52–64px
  - Control height: 32–36px
  - Corner radius: 6–8px
- Input Methods page should present startup and enabled input methods as settings/cards,
  not as an empty combobox plus global Apply button.
- Appearance page default surface should show:
  - candidate preview;
  - theme selection as cards/segmented choices;
  - candidate layout as segmented choices;
  - text size as a product control;
  - font as a setting row;
  - advanced appearance as a drill-in/expandable area.
- Appearance default surface must not show internal theme IDs, repository metadata,
  scroll-cell tuning, max-width tuning, opacity, or preedit engineering controls.
- Add-ons & Extensions should show installed/add-on cards with readable names,
  summaries, version/source/status chips, and a path to details. Do not present the
  main surface as a listbox plus details textbox.
- Updates should be a product update page, not a clone of Add-ons. It should show the
  product name/version, update status, check-update action, auto-update setting, update
  channel, and a link/card for component updates.
- Diagnostics & Repair should show status rows/cards first. Technical text dump/logs
  must be hidden behind an explicit technical details affordance.
- Keyboard navigation and focus visibility must remain usable.
- High Contrast must remain readable and deterministic.

## Required validation

- Add/extend automated visual contract tests so the old failure modes cannot pass:
  - no visible default-page raw listbox/details textbox package-manager layout for
    Updates/Add-ons;
  - no visible default Appearance theme ID/metadata panel;
  - no visible overlapped controls at 96/120/144/192 DPI;
  - navigation is owner/product-rendered rather than bordered button-looking rows;
  - minimum and default window sizes satisfy the contract.
- Run affected x64 and x86 Config tests:
  - `config-ui-i18n-check`
  - `config-ui-resource-check`
  - `config-ui-behavior-contract`
  - `config-ui-visual-contract`
  - `config-ui-live-preview-contract`
  - `config-ui-interaction-coverage`
  - `config-toml-contract`
  - `source-contract`
- Capture a before/after evidence note in `docs/tasks/status.md`. Real screenshots may
  remain manual evidence unless an automated screenshot harness already exists.

## Done when

- The six failing baseline pages no longer look like raw Win32 property-sheet forms.
- Default pages present user-facing Settings concepts rather than implementation
  internals.
- Existing Config behavior and live-preview semantics remain green.
- x64/x86 affected tests pass.
- `docs/tasks/status.md` records the reopened task result and any remaining manual
  visual evidence.
