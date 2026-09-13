# Task 085 - Candidate WeChat-theme consistency

**Task ID:** `CANDIDATE-WECHAT-THEME-CONSISTENCY-001`
**Mode:** CODE-ONLY / VISUAL-CONTRACT REPAIR
**Prerequisites:** `060`, `061`, `062`, `080` through `084` complete. Task `074` remains the external release-evidence campaign.

## Goal

Align the builtin candidate selection, Settings preview, Rust renderer fallback, and Candidate PoC to solid WeChat green `#07C160` with white selected glyphs. Keep hover subtle and High Contrast unchanged.

## Acceptance

1. Builtin light/dark selection is green with white candidate, label, and comment text.
2. Qingfeng, the renderer fallback, preview, and PoC agree on that semantic.
3. Rust regressions and a fresh x64 visual snapshot prove the selected CJK glyphs are complete and white.
4. No manual host/release evidence is claimed; Task `074` remains pending.
