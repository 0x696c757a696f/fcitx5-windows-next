# Task 085 - Candidate WeChat-theme consistency

**Task ID:** `CANDIDATE-WECHAT-THEME-CONSISTENCY-001`
**Mode:** CODE-ONLY / VISUAL-CONTRACT REPAIR
**Prerequisites:** `060`, `061`, `062`, `080` through `084` complete. Task `074` remains the external release-evidence campaign.

## Goal

Align the builtin candidate selection, Settings preview, Rust renderer fallback, and Candidate PoC to solid WeChat green `#07C160` with white selected glyphs. Keep hover subtle and High Contrast unchanged. Freeze the shared visual-layout rules used by the existing `stacked`, `flow`, `scroll`, and `vertical_text` model: an outer floating surface is distinct from an inset selected pill; every CJK/emoji glyph receives its full measured budget; and a scroll grid sizes each column independently.

## Acceptance

1. Builtin light/dark selection is green with white candidate, label, and comment text.
2. Qingfeng, the renderer fallback, preview, and PoC agree on that semantic.
3. The outer surface keeps a 12-DIP rounded outline and the selected candidate is an inset 10-DIP green pill, never a square fill; smaller label/comment text shares the candidate baseline.
4. `flow` gives every visible item its natural width; `scroll` grid gives every column its own widest-item width. Neither may clip glyphs or create global-longest-item gaps.
5. Rust regressions and fresh x64 snapshots prove selected CJK/emoji glyphs are complete, white, baseline-aligned, and visually consistent across stacked, flow, and scroll-grid paths.
6. This task does not add new Settings controls or claim real-host, accessibility, signing, UAC, CI, or release-readiness evidence; those remain separate work, including Task `074`.
