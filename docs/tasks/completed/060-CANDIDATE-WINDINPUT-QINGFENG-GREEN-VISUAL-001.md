# Task 060 — Candidate WindInput/Qingfeng green visual adoption

**Mode:** CODE-ONLY
**Task ID:** `CANDIDATE-WINDINPUT-QINGFENG-GREEN-VISUAL-001`
**Prerequisite:** `059-CANDIDATE-LABEL-SLOT-RUST-DRAWING-001` label-slot contract green

## Goal

Replace PoC-level Candidate visuals with a WindInput/Qingfeng-derived Rust visual path and make the
project default palette match WeChat IME style: green/white in light mode and green/black in dark
mode.

## Specification references

- 13.3 Config / Candidate UI / Rust migration boundary
- 13.9 Theme system
- Reference policy: `huanfeng/WindInput` and `huanfeng/wind-ui-rust`

## Required behavior / implementation contract

- Candidate visual code must directly adopt or port the MIT-licensed WindInput `wind-ui` candidate
  window/view/theme behavior where local renderer quality is not comparable.
- Preserve Qingfeng/WindInput upstream theme sources and variants. Project default palette overrides
  must not delete or rewrite third-party themes.
- Config default accent is WeChat-style green `#07C160`.
- Candidate default theme uses:
  - light: green/white, with `#07C160` selected text/accent;
  - dark: green/black, with `#181818` background and `#07C160` selected text/accent.
- Candidate default font chain is CJK-first: `Microsoft YaHei`, then `Microsoft YaHei UI`, then
  `system`; Latin-first defaults are not acceptable for Chinese candidates.
- Rime/鼠须管 theme lessons, including `eosphoros-keytao` Squirrel color schemes, are allowed for
  appearance tuning: candidate format spacing, label font size, candidate font size, line/candidate
  spacing, border width, corner radius, light/dark scheme colors, and highlight colors must resolve
  through typed theme tokens before drawing.
- Candidate label slots from 059 remain configurable and stable: hidden labels still reserve space,
  revealed item/row/column labels do not move candidate text columns, and long candidate text never
  consumes the label slot.
- No new C++ product-owned Candidate/Config logic. C++ renderer/window code, if still present, is
  adapter-only until Rust renderer cutover evidence is equivalent.

## Required validation

- x64 and x86 Rust tests for candidate-core and config-poc affected tests.
- x64 and x86 CTest source-contract.
- Candidate PoC screenshots for vertical/horizontal/grid in light and dark modes.
- Evidence JSON must include WindInput/Qingfeng visual source markers and WeChat-green palette
  markers.
- Text formatting and diff checks.

## Done when

- The reachable automated checks above pass.
- `docs/tasks/status.md` records source commit, changed files, tests, and remaining manual visual
  limits, if any.
