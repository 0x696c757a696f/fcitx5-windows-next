# Current Task — REG-BRAND-001 Penguin-first product/TSF identity

**Mode:** CHANGE
**Task ID:** `REG-BRAND-001`
**Prerequisite:** 004 single profile identity complete; 016 resource/visual system available

## Goal

Ship one coherent Fcitx5 for Windows Next visual identity: an original or license-cleared simplified penguin family, with a dedicated micro-penguin TSF glyph and stable user-visible names.

## Specification references

- §5.6.4A Product/TSF icons
- §9 single profile
- Phase 6
- `REG-BRAND-001`

## Required behavior / implementation contract

- Product/Start/Config icon uses Penguin-first identity; no abstract F5/keyboard/Windows-pane default replacement.
- TSF picker icon uses a manually simplified language-neutral micro-penguin for 16/20/24px instead of shrinking a detailed master.
- Do not embed language characters, flags, Rime/Mozc logos, or dynamic engine state into TSF identity.
- Windows profile name stays `Fcitx5`; Config/Start label stays `Fcitx5 for Windows Next`.
- Use one resource manifest and stable AppUserModelID.
- Candidate/launcher/engine/background helpers stay out of taskbar/Alt+Tab; no default extra tray icon.

## Required validation

- `REG-BRAND-001` at common DPI/Light/Dark/High Contrast.
- ICO contains required sizes and shell picker remains recognizable.
- Switch internal engines and confirm TSF name/icon identity does not change.
- Taskbar/Alt+Tab surface test for background processes.

## Done when

- No default blank EXE icon remains on visible product surfaces.
- Product and micro TSF icons are recognizably the same family.
- Brand asset licensing/originality is documented.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
