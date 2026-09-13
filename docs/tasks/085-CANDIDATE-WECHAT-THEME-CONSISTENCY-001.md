# Task 085 - Candidate WeChat-theme consistency

**Task ID:** `CANDIDATE-WECHAT-THEME-CONSISTENCY-001`
**Mode:** CODE-ONLY / VISUAL-CONTRACT REPAIR
**Prerequisites:** `060`, `061`, `062`, `080` through `084` complete. `074` external release evidence remains active but does not block this code-only correction.

## Goal

Make the shipped builtin light/dark candidate selection, Settings preview, Rust render fallback, and Candidate PoC use one coherent project-owned WeChat-IME semantic: selected surface `#07C160`, selected candidate/label/comment text `#FFFFFF`. Hover remains a separate subdued surface. Do not alter High Contrast semantics.

## Root cause frozen before edit

The shipping host parses alpha but the paint-color ABI resolves RGB only. The old builtin TOML therefore became a solid green surface while retaining green selected text. The visual PoC, embedded preview, and Rust fallback used separate pale/non-WeChat colors, so screenshots could pass without representing the shipping selection contrast.

## Scope

- `resources/themes/default/theme.toml`: production builtin selected tokens.
- `rust/candidate-core`: project-owned tokens, renderer fallback, UI plan, Settings preview plan, and PoC must agree. Qingfeng/WindUI remain replaceable renderer/layout backends, never the product palette authority.
- No C++ product code, no new theme engine, no changes to user-installed theme parsing, and no High Contrast change.

## Acceptance

1. Light and dark builtin candidate selected tokens are solid `#07C160FF` with white selected text.
2. Candidate paint-color resolution, Qingfeng plan, renderer fallback, embedded Settings preview, and generated PoC snapshot agree on RGB `7,193,96` and white selected glyphs.
3. Rust regression tests fail on the old palette and pass after the correction.
4. Fresh x64 Candidate snapshot visibly shows a solid green selected item with complete white CJK glyphs.
5. This task makes no real-host, accessibility, signing, UAC, CI, or release-readiness claim; those remain Task `074` evidence.
