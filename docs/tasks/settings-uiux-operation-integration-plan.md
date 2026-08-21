# Settings UI/UX and operation integration plan

**Product:** Fcitx5 for Windows Next
**Created from HEAD:** `d557e4809cb26c0697169c49294fff2cd8126061`
**State:** PLANNED / not yet executed
**Scope:** Settings application behavior, visual design, localization, live preview, input-method management, add-on/package UX, update UX, diagnostics, and no-overlap verification.

This is the single planning document for the modern Settings surface. Every visible control must have a defined operation, every operation must have localized feedback, and every page must satisfy the no-overlap visual contract.

## External UI/UX reference

Reference reviewed: `nextlevelbuilder/ui-ux-pro-max-skill`.

Use it only as product/design guidance. Do not vendor or install external skill
code into this repository as part of the Settings task queue.

Applicable guidance for Fcitx5 for Windows Next:

- text, chips, badges, status rows, and labels must reflow without clipping;
- long identifiers and package names must wrap, elide with a full-detail path,
  or move into a scrollable/details region;
- state badges must not rely on color alone;
- keyboard focus states must remain visible;
- rapid interactions may cancel animation, but final semantic state and focus
  must remain correct;
- interaction timing must respect platform expectations and reduced-motion
  preferences where animation is introduced;
- every pre-delivery Settings check must include contrast, focus, scaling,
  localization, and no-overlap verification.

## User-visible gaps being addressed

This plan directly covers the gaps reported during review:

- enabled input methods are not clearly visible in Settings;
- candidate preview is not rendered live inside the Settings window;
- font selection is missing or not usable as a real picker;
- advanced appearance controls are incomplete or hidden;
- plug-ins/add-ons cannot be downloaded from a trusted online source;
- Settings does not show which official Fcitx5-compatible Windows packages are actually available for download;
- dialogs and status messages still contain English-only strings;
- the UI has no explicit language selector;
- controls, labels, candidate text, emoji, status text, and dialogs must never overlap or clip in supported layouts.

## Product decisions

- Keep the new modern Settings visual direction.
- Keep WTL/Win32 hosting plus the product-specific D2D/DWrite Settings layer.
- Do not expose the old raw Win32/package-manager surface as the default UX.
- Implement full operation coverage behind the modern surface.
- Use localized inline status for routine feedback.
- Use localized dialogs only for destructive, trust-sensitive, or restart-needed actions.
- Treat no-overlap as a hard release gate, not a visual polish issue.
- Treat official online add-ons as unavailable until a trusted signed Windows repository exists and verifies successfully.

## Goals

- Make the Settings window fully usable without exposing the old raw Win32 configuration surface.
- Show current enabled input methods and make the active/default method understandable.
- Provide an in-window candidate preview that updates immediately as appearance settings change.
- Provide real font selection, not manual text entry as the main path.
- Expose advanced appearance settings clearly and safely.
- Make add-on/package state understandable: bundled, installed, update available, disabled, repository unavailable, repository trust failure, incompatible, pending restart, and online refresh failure.
- Localize all user-facing strings, including dialogs, status messages, empty states, package errors, repository errors, diagnostics, and repair flows.
- Provide an explicit language selector in Settings.
- Prevent overlap, clipping, hidden text, or inaccessible controls at minimum size, high DPI, Chinese/English text, emoji fallback, and high contrast.

## Non-goals and constraints

- Do not invent an unsigned or untrusted online plug-in feed.
- Do not pretend upstream Linux Fcitx5 add-ons are directly installable Windows packages.
- Do not bypass the package trust model. Online packages must remain signed, manifest-verified, hash-verified, and covered by repository anti-rollback state.
- Do not add hooks, input emulation, process injection, SendInput-based simulation, anti-cheat bypass, credential access, or external exploitation.
- Do not migrate Settings to a different UI framework in this task series.

## Canonical Settings information architecture

The modern Settings window should expose these top-level pages:

1. **Overview**
   - product identity;
   - running status;
   - current input method;
   - update/add-on trust summary;
   - quick links to common repairs.
2. **Input methods**
   - enabled input methods;
   - active/default method;
   - add/remove/reorder entry points when supported.
3. **Appearance**
   - live candidate preview;
   - theme/layout/font/basic candidate controls.
4. **Advanced appearance**
   - detailed candidate renderer controls behind a clear advanced page or expandable panel.
5. **Add-ons**
   - bundled/installed/available components;
   - trusted online repository state;
   - install/update/enable/disable/remove flows.
6. **Updates**
   - product and package update state.
7. **Diagnostics**
   - TSF/engine/candidate/config/package health and repair.
8. **Language**
   - system default / English / Simplified Chinese selector.

If screen height is insufficient, pages scroll inside the content region. The sidebar must remain stable and must not squeeze content into overlap.

## Global operation model

Every operation must follow the same visible lifecycle:

```text
idle
  -> validating input
  -> running
  -> success | warning | failure
  -> refreshed state
```

Rules:

- the initiating control is disabled while its operation is running;
- unrelated safe navigation remains available;
- the affected page shows a localized status row;
- success status clears after a reasonable interval or when the next operation starts;
- failure status remains visible until dismissed or replaced;
- page data is refreshed after any successful state-changing command;
- partial failure must say exactly which subsystem failed;
- the UI must return to the last confirmed state after failed package operations.

Examples:

- changing the candidate font updates preview immediately, then persists;
- refreshing online add-ons shows local packages even if online refresh fails;
- uninstall failure must not remove the package card until inventory refresh confirms removal;
- language change must either apply immediately or clearly say restart is required.

## Shared visual contract

The visual contract applies to every page, dialog, and preview state:

- no control can overlap another control;
- no text can overlap adjacent text or icons;
- candidate labels, candidate text, comments, emoji, and preedit text must each have distinct measured boxes;
- status rows must reserve height before drawing text;
- wrapped text must push following content down or use an explicit scroll area;
- elided text must use a visible ellipsis and an accessible/full detail path;
- disabled actions must keep their explanatory text visible;
- focus rings must not be clipped;
- high contrast mode must not draw low-contrast custom backgrounds over system text colors;
- 100%, 150%, and 200% DPI must be tested;
- English and Simplified Chinese strings must both be tested;
- minimum supported window size is 860 x 600 unless a task explicitly changes the product minimum.

Do not rely on manual inspection only. Add or extend visual contract tests for each page state introduced by implementation slices.

## Page map and required operations

### 0. Overview

Visible content:

- product name: `Fcitx5 for Windows Next`;
- current version/build identity;
- engine running state;
- TSF registration state;
- current enabled/default input method summary;
- active candidate appearance summary;
- add-on repository trust state;
- last update/check result.

Required operations:

- open the relevant detailed page from each summary card;
- run a lightweight refresh;
- open diagnostics/repair when a subsystem is unhealthy.

Acceptance:

- the first screen answers “is it working?” without making the user open logs;
- no summary claims online/offline state that has not actually been checked.

### 1. Input methods

Visible content:

- startup toggle;
- enabled input method list;
- active/default input method marker;
- input method display name and native name;
- empty state when Control cannot read input methods;
- repair hint when no input methods are available.

Required operations:

- toggle startup;
- refresh enabled input methods;
- select current/default input method;
- save input-method selection through `fcitx5-control --set-input-method`;
- refresh list after save;
- show localized success/failure status.

Future operations, separate from the first slice:

- add input method;
- remove input method;
- reorder enabled input methods;
- per-input-method configuration entry point when typed config metadata is available.

Acceptance:

- enabled input methods are visible without opening a combo box;
- the active method is visible without opening a combo box;
- the combo/list state and modern card state never disagree;
- if backend enumeration fails, the page says so in the selected UI language;
- a user can tell which input methods are enabled, which one is active, and what will change before pressing save.

### 2. Appearance

Visible content:

- in-window candidate preview;
- mode selector: system, light, dark;
- candidate layout selector: automatic, horizontal, vertical;
- text size selector;
- font selector;
- advanced appearance entry;
- reset appearance action.

Required operations:

- changing mode/layout/size/font updates the in-window preview immediately;
- changes are saved through `fcitx5-control --set-presentation`;
- existing external candidate demo remains available as an explicit preview action, but it is not the only preview;
- reset restores defaults and refreshes the page and preview.

In-window preview requirements:

- draw inside the Settings content area, not as a separate floating candidate window;
- reflect current mode, layout, font family, font size, opacity, corner radius, page size, label style, scroll mode, and preedit placement when supported;
- use sample content with Chinese, Latin, punctuation, and emoji so font fallback problems are visible;
- emoji should prefer color emoji-capable fallback when available; black-and-white rendering should be treated as a visible regression to investigate;
- never overlap labels, candidates, comments, emoji, preedit text, or action rows.

Candidate preview sample set:

- Chinese candidates: `你`, `你好`, `输入法`;
- Latin candidates: `fcitx`, `Windows`, `Next`;
- punctuation: `，。！？`;
- emoji: `😀`, `🎉`, `⌨️`;
- mixed preedit: `ni hao 😊`.

The preview does not need to fake real typing. It must be a deterministic rendering contract for layout, colors, fonts, fallback, and high DPI behavior.

### 3. Advanced appearance

Visible content:

- candidate max width;
- page size;
- scroll mode;
- scroll cell width;
- corner radius;
- opacity;
- shadow;
- preedit display mode;
- candidate font size;
- candidate font family;
- annotation scale;
- label visibility/style/font scale/gap, when exposed by typed config;
- selected-candidate colors;
- normal-candidate colors;
- comment/annotation colors;
- background/border colors;
- spacing and padding, if supported by the renderer contract;
- theme metadata and active theme summary.

Required operations:

- expand/collapse advanced appearance without shifting controls into overlap;
- every advanced control must either save live or clearly show an unapplied state;
- invalid values must be blocked before save and reported in localized text.

Acceptance:

- the advanced page is useful at 860 x 600;
- if all advanced controls cannot fit at minimum height, the page must use a real scroll/viewport model rather than clipping;
- visual contract must test both collapsed and expanded states;
- every visible advanced setting has a matching storage key or an explicit “not supported by this renderer yet” disabled explanation.

### 4. Font selection

Visible content:

- current candidate font;
- current size;
- font fallback hint;
- font picker action.

Required operations:

- provide a real font selection path;
- allow common presets such as system, Microsoft YaHei, Segoe UI, Noto Sans CJK if installed, and Cascadia/Consolas for monospace-related display;
- use a Windows font dialog or an enumerated font list; do not rely on manual typing as the main interaction;
- save selected font to `fonts.candidate.families`;
- preview selected font immediately.

Acceptance:

- a user can choose a font with the mouse/keyboard;
- the selected font is persisted and visible after reopening Settings;
- the preview proves fallback still displays Chinese and emoji samples;
- if the chosen font lacks CJK or emoji coverage, Settings explains that fallback fonts will be used.

### 5. Language and localization

Visible content:

- language selector;
- current language;
- restart-required or immediate-apply explanation, depending on implementation.

Required operations:

- choose system default, English, or Simplified Chinese;
- persist the language choice in user configuration;
- load matching locale on next launch, or apply immediately if safe;
- localize every visible Settings string;
- localize status messages, dialog captions, dialog buttons where custom dialogs are used, empty states, package failures, repository unavailable messages, and repair/update messages.

Language application policy:

- if safe immediate reload is implemented, update current window text and preview/status content without restart;
- if immediate reload is not safe, show a localized restart-required dialog;
- the language selector itself must remain understandable after a failed language switch.

Required localized string categories:

- page titles;
- sidebar items;
- labels;
- button text;
- tooltips;
- empty states;
- inline success/failure messages;
- dialog titles;
- dialog body text;
- destructive action confirmations;
- package trust errors;
- repository unavailable/refresh failure states;
- diagnostics and repair messages.

Acceptance:

- a Chinese Windows system shows Chinese by default;
- a non-Chinese Windows system can switch to Chinese from Settings;
- `--lang=zh-CN` and `--lang=en-US` remain useful diagnostic overrides;
- `config-ui-i18n-check` covers all new keys;
- English fallback is allowed only for missing developer diagnostics, not for normal Settings UI, dialogs, package errors, or user-facing status.

### 6. Add-ons and extensions

Visible content:

- bundled components;
- installed components;
- disabled/enabled state;
- update available state;
- repository state;
- selected component details;
- trusted repository unavailable state;
- repository trust failure state;
- online refresh failure state.

Required operations:

- refresh local package state;
- refresh online repository through `fcitx5-control --packages-refresh`;
- install/update only when a signed trusted repository entry is available;
- enable/disable installed non-bundled packages;
- remove installed non-bundled packages;
- restart or notify the engine after package state changes when required;
- show package details:
  - type;
  - summary;
  - available version;
  - installed version;
  - state;
  - permissions;
  - dependencies;
  - config surface;
  - source commit, when available;
  - manifest hash/signature summary.

Repository UX rules:

- if no trusted repository keys are provisioned, say: online repository is not configured; bundled components remain usable;
- if refresh fails, say: online refresh failed; local components are still shown;
- if a package is bundled, actions must explain that it is managed by product updates;
- do not show fake “official downloadable plug-ins” unless they are present in a trusted Windows package repository;
- do not treat the upstream Linux Fcitx5 add-on list as directly installable Windows packages;
- “official” means: signed repository metadata, trusted official key, supported Windows package manifest, compatible architecture, and successful anti-rollback checks.

Acceptance:

- install/update/remove buttons are disabled when unsafe or impossible;
- the UI never implies unsigned packages can be installed;
- the existing `plugin-install-update-stability-plan.md` remains the lifecycle test plan for install/update/remove stability;
- the Add-ons page can distinguish at least:
  - bundled;
  - installed;
  - disabled;
  - update available;
  - available online;
  - repository unavailable;
  - repository trust failure;
  - incompatible package;
  - pending restart.

Add-on operation state machine:

```text
not installed
  -> available online
  -> installing
  -> installed
  -> disabling
  -> disabled
  -> enabling
  -> installed
  -> updating
  -> installed updated version
  -> removing
  -> not installed
```

Failure from any running state returns to the last confirmed inventory state and shows a localized failure reason.

### 7. Updates

Visible content:

- product version;
- channel;
- component repository state;
- check for updates action;
- link to add-ons/extensions.

Required operations:

- refresh product/component metadata;
- show localized online unavailable/failure states;
- do not claim the system is up to date unless the relevant endpoint was actually checked;
- if repository signing/trusted key validation fails, block update and route users to the trust error explanation rather than generic network failure.

### 8. Diagnostics and repair

Visible content:

- TSF registration status;
- engine status;
- candidate UI status;
- configuration status;
- package trust/install status;
- recheck action;
- repair action.

Required operations:

- recheck status through Control;
- start repair through the packaged bootstrap/repair entry point;
- show localized result;
- do not delete user dictionaries or user package data during ordinary diagnostics.

Acceptance:

- repair dialogs describe exactly what is changed and what is preserved;
- repair errors are localized;
- diagnostics refresh cannot visually stack old and new status rows.

## Dialog rules

- Prefer inline status for routine success/failure.
- Use dialogs only for destructive or potentially disruptive actions:
  - uninstall package;
  - reset appearance;
  - repair;
  - language change requiring restart;
  - failed online repository trust/verification when user action is needed.
- Dialog text must be localized.
- Dialog title must use `Fcitx5 for Windows Next`.
- Dialog body must say what will happen and what data is preserved.
- Dialog buttons must be localized when custom dialogs are used.
- Native Windows buttons are acceptable only if they follow the selected system language; otherwise use a localized custom confirmation surface.

## Official add-on repository requirements

Settings may show downloadable official add-ons only after the package trust chain is implemented and provisioned:

- trusted public key exists in `security/trusted-keys.json`;
- repository index is signed and verifies;
- repository channel matches the selected product channel;
- repository generation passes anti-rollback checks;
- package manifest is signed and verifies;
- package payload hashes verify;
- package architecture and product compatibility match the current install;
- package id, display name, summary, version, license/source metadata, and restart requirement are available for display.

Until then, the correct UX is an honest unavailable state, not an empty broken store:

```text
Official add-on repository is not configured yet.
Bundled components are still available.
```

## Data and command boundary

Settings should not duplicate package business logic. It should call the public Control/package boundary and render the returned state.

Required state returned to Settings:

- enabled input methods;
- active/default input method;
- appearance values;
- supported advanced appearance fields;
- local package inventory;
- online repository state;
- package trust/verification error category;
- operation progress/result;
- diagnostics state.

Required command categories:

- refresh input methods;
- set active/default input method;
- set appearance value;
- reset appearance;
- set language preference;
- refresh local packages;
- refresh online repository;
- install/update package;
- enable/disable package;
- remove package;
- check updates;
- run diagnostics;
- run repair.

Each command must have a deterministic success/failure result suitable for localized UI rendering and automated interaction tests.

## Implementation slices

### CONFIG-UX-001 — Settings operation inventory and localized status

- add missing locale keys;
- replace hard-coded English user-facing Settings strings;
- add language selector model and persisted preference;
- add tests for locale key parity;
- define the operation result categories used by inline status and dialogs.

### CONFIG-UX-002 — Input methods page completion

- make enabled input methods visible in modern cards;
- wire current/default selection to Control;
- add empty/error states;
- add interaction test coverage.

### CONFIG-UX-003 — In-window live candidate preview

- draw live preview in Settings;
- reflect mode/layout/font/size/basic advanced values;
- include Chinese, Latin, punctuation, and emoji samples;
- keep external demo preview as an explicit action;
- extend live preview and visual no-overlap contracts.

### CONFIG-UX-004 — Font picker

- add real font picker or enumerated font list;
- persist selected candidate font;
- preview immediately;
- add regression coverage for non-empty font selection and reopen persistence.

### CONFIG-UX-005 — Advanced appearance completion

- expose all supported candidate/renderer settings;
- add scroll/viewport if needed;
- validate no overlap in collapsed and expanded states.

### CONFIG-UX-006 — Add-ons/package UX completion

- improve repository state messaging;
- make bundled/installed/update/disabled/trust-failure states visually distinct;
- disable unsafe actions with visible explanation;
- cover refresh/install-or-update/enable-disable/remove paths in interaction tests;
- reuse `docs/tasks/plugin-install-update-stability-plan.md` for deeper lifecycle checks.

### CONFIG-UX-007 — Dialog and destructive action localization

- localize all dialogs/statuses;
- add confirmation flows for reset/remove/repair/language restart;
- add tests or source contracts for no hard-coded dialog strings in Settings.

### CONFIG-UX-008 — Package smoke cleanup

- ensure portable smoke and package tests shut down any launcher/UI/engine processes they start;
- keep package output unlocked after a successful gate.

## Verification plan

Run focused Settings checks on x64 and x86:

```powershell
cmake --build out/build/windows-x64-dev --config Debug --target fcitx5_config_app fcitx5_config_parser_test fcitx5_source_contract_test --parallel
ctest --test-dir out/build/windows-x64-dev -C Debug --output-on-failure -R "config-ui-(i18n-check|resource-check|behavior-contract|visual-contract|live-preview-contract|interaction-coverage)|config-toml-contract|source-contract"

cmake --build out/build/windows-x86-dev --config Debug --target fcitx5_config_app fcitx5_config_parser_test fcitx5_source_contract_test --parallel
ctest --test-dir out/build/windows-x86-dev -C Debug --output-on-failure -R "config-ui-(i18n-check|resource-check|behavior-contract|visual-contract|live-preview-contract|interaction-coverage)|config-toml-contract|source-contract"
```

Run package lifecycle checks after add-on UX changes:

```powershell
ctest --test-dir out/build/windows-x64-dev -C Debug --output-on-failure -R "package-core-contract|control-package-stopped-service-contract|control-repository-rollback|package-(manifest-path|path-corpus)-fuzz-smoke"
ctest --test-dir out/build/windows-x86-dev -C Debug --output-on-failure -R "package-core-contract|control-package-stopped-service-contract|control-repository-rollback|package-(manifest-path|path-corpus)-fuzz-smoke"
```

Run package gate before handing a new build to users:

```powershell
& 'D:\Program Files\PowerShell\7\pwsh.exe' -NoLogo -NoProfile -ExecutionPolicy Bypass -File .\tools\build.ps1 package -Architecture all -Configuration Release
```

## Done when

- every Settings page has visible, localized empty/error/success states;
- enabled input methods are visible and selectable;
- candidate preview is in the Settings window and updates live;
- font selection works without manual text entry;
- advanced appearance exposes the full supported setting set without overlap;
- emoji preview uses color emoji fallback where available or reports the fallback limitation as a regression to investigate;
- Add-ons page explains bundled, installed, disabled, update, repository unavailable, repository trust failure, incompatible, pending restart, and refresh failure states;
- official downloadable plug-ins are shown only from a trusted signed Windows package repository;
- all dialogs/statuses are localized and use the product name;
- visual and interaction contracts cover the final UI;
- package smoke leaves no lingering staged launcher/UI/engine processes.
