# Task 061 — Candidate Microsoft YaHei UI Rust text renderer

**Mode:** CODE
**Task ID:** `CANDIDATE-MICROSOFT-YAHEI-RUST-TEXT-RENDERER-001`
**Prerequisite:** `CANDIDATE-WINDINPUT-QINGFENG-GREEN-VISUAL-001`
**Evidence class:** `AUTOMATED`

## Goal

Make the Rust Candidate visual screenshots render text through the Qingfeng `windui`
DirectWrite path with a CJK-first Microsoft YaHei UI font family that is visible in the bitmap
output, not only selected as a hidden verification font.

## Specification references

- §13.9 Candidate visual/theming hardening
- Candidate default CJK-first font requirement

## Required behavior / implementation contract

- Candidate screenshot text rendering remains Rust-owned.
- The label-slot screenshots must use the Qingfeng `windui` `DWriteEngine` plus
  `SkiaCanvas::with_text` path with a CJK-first `Microsoft YaHei UI` / `微软雅黑 UI` font family
  for the actual text drawn into the bitmap, then transfer the Rust bitmap with
  `SetDIBitsToDevice`.
- The generated report must continue to record the actual selected Windows text face.
- Label-slot visual goldens use the repository's 150% DPI smoke scale so the 18px logical
  Candidate text is rendered at its intended 27px physical size instead of as low-DPI small text.
- Automated source contracts must prevent silent regression away from the windui DirectWrite path
  and its `WINDOW_WINDUI_DWRITE_TEXT_DRAW_USED` evidence for the Candidate PoC screenshots.

## Required validation

- `cargo fmt --all -- --check`
- x64/x86 `fcitx5-candidate-core` tests
- x64/x86 Candidate label-slot screenshot CTest slices
- x64/x86 `source-contract`

## Done when

- Fresh label-slot screenshots are generated.
- Each generated label-slot JSON report records `candidate_visual_text_face` as `Microsoft YaHei UI`,
  `微软雅黑 UI`, `Microsoft YaHei`, or `微软雅黑`.
- Source-contract guards the Rust renderer evidence.
