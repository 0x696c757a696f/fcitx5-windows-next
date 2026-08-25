# CONFIG-UX-009 WindInput-inspired Settings theme library and embedded live preview

**State:** IN-PROGRESS / RUST-THEME-PREVIEW-CONTRACT-GREEN / RUST-FIRST-DOCS-AND-THEME-IMPORT-I18N-GREEN / THEME-ACTIONS-AFFORDANCE-NO-CLIP-GREEN

## Context

The current Settings surface is visually acceptable as a direction, but it is still too close to a
traditional Win32 configuration tool. User review specifically called out:

- skin/theme management is too thin;
- the candidate preview is not live enough and must stay inside the Settings window;
- enabled input methods, font selection, advanced appearance, localization, plug-in/download
  states, and dialogs need complete operation logic;
- candidate text, labels, emoji, and controls must never overlap or clip.

Reference refresh performed on 2026-08-25:

- `huanfeng/WindInput` at `2214bede43b4153f0fdc463928cf3c50184ec2ef`;
- `huanfeng/wind-ui-rust` at `8ce94a46900a414612ead96438c770cb49eefdea`;
- `huanfeng/wind-setting` was not publicly available from GitHub at review time.

These references may guide product structure and tests. Do not copy non-trivial code unless the
copying task explicitly records license attribution and NOTICE updates.

Language baseline for this task: already Rust-owned Settings/Candidate/TSF/package/control logic
must not be moved back to C++. New Settings product logic, preview contracts, theme/package
operation models, validation, and testable UI-domain behavior default to Rust. The existing
WTL/Win32/D2D Config code is only a shipping host/adapter unless a later task records explicit
cutover evidence for replacing it.

## Scope

Implement the next Settings UX vertical slice that makes theme/appearance changes feel like a real
product surface:

- Theme Library page with built-in/user/package theme inventory, metadata, source badges, safety
  status, duplicate/import/export/delete affordances, and trusted-package provenance where present.
- Embedded candidate preview surface inside `fcitx5-config.exe`; it must use the production
  CandidateModel/layout/render contract rather than a separate fake drawing path.
- Live preview draft state for theme, light/dark mode, layout, page size, scroll mode, font family,
  font size, opacity, corner radius, spacing/padding, label style, comment style, and preedit
  placement where those renderer keys are already supported.
- Font selection through a real picker or system font enumeration, including CJK display names when
  available.
- Emoji fallback evidence: preview sample must include color-emoji candidates and detect/report
  monochrome fallback as a visible limitation.
- High-DPI behavior is automatic by default; do not add a “high DPI mode” switch. Advanced scale
  overrides are allowed only as user preference, not as the primary fix.
- Localized inline status/dialog text for every new operation.
- No-overlap and no-clipping tests for all added page states.

## Must not do

- Do not start a full Settings framework rewrite unless a later Rust Config cutover task explicitly
  authorizes it.
- Do not show fake online “official plug-ins.” Online items require signed repository metadata and
  trusted keys.
- Do not use global hooks, `SendInput` emulation, process injection, credential access, or external
  exploitation.
- Do not keep an external floating candidate preview as the only preview path.

## Required validation

- Settings visual contract at 100%, 150%, and 200% DPI.
- English and Simplified Chinese localization check for every added label/status/dialog.
- Interaction coverage for every new command button, combo/list item, slider/edit, and destructive
  confirmation.
- Candidate preview parity check proving the embedded surface consumes the same model/layout/render
  contract as the shipping candidate UI.
- Theme import/delete path safety tests: path traversal, unknown executable/script hooks, remote
  assets, invalid TOML, missing base theme, and cyclic base references.
- Font selection persistence and preview refresh test.
- Package build smoke for the same artifact lineage before handing a package to the user.

## Done when

- A user can browse, select, preview, duplicate/import/export/delete themes from Settings with clear
  source/trust labels.
- Changing any supported Appearance or Advanced Appearance field updates the embedded preview
  immediately without opening the floating candidate window.
- The preview visibly covers Chinese, Latin, punctuation, labels with the configured suffix, comments,
  preedit text, and emoji.
- Font selection is mouse/keyboard usable and persists after reopening Settings.
- High-DPI readability is automatic and covered by tests.
- All new UI strings and operation results are localized.
- Added controls and preview content pass no-overlap/no-clipping checks.
