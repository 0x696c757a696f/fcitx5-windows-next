# Task 081 - Candidate renderer D2D→tiny-skia cutover + HWND migration

**Task ID:** `CANDIDATE-RENDERER-D2D-TINY-SKIA-CUTOVER-001`
**Mode:** CHANGE / RENDERER-CUTOVER
**Prerequisite:** 080 (three-axis model wired through snapshot ABI + axis_layout + paintOnce + wheel).

## Goal

Replace the C++ D2D/DWrite candidate renderer in `src/ui/ui_main.cpp` with a Rust
tiny-skia/windui renderer. The Rust renderer must produce equivalent visual output (geometry +
colors + layout) for all three-axis layout combinations. The HWND window + message loop must
also move to Rust. Delete `ui_main.cpp` when done.

## Frozen acceptance

- Geometry: same axis_layout rects (orientation x overflow x writing).
- Colors: WeChat-green selection, light/dark theme, high-contrast.
- Fonts: Microsoft YaHei UI, CJK-first, label/comment scaling.
- DPI: 100/125/150/200% correct scaling.
- Window: WS_EX_TOOLWINDOW|NOACTIVATE|TOPMOST, layered opacity, hit-test.
- Wheel: Scrolling=viewport, Paging=VK_PRIOR/NEXT.
- Highlight: selection rounded rect + font color swap.

## Slices

A. Extract paintOnce D2D logic into a testable render function (C++ side refactor).
B. Rust tiny-skia renderer: replace D2D with tiny-skia + windui DWrite for the same rects.
C. HWND + message loop to Rust (create window, message pump, mouse/keyboard dispatch).
D. Delete ui_main.cpp + update tests + verify.

## C++ files affected

- `src/ui/ui_main.cpp` (3007 lines) — deleted at end

## Rust files affected

- `rust/candidate-core/src/bin/candidate_poc.rs` (extend to shipping renderer)
- `rust/candidate-core/src/lib.rs` (FFI for renderer)
- New: `rust/candidate-core/src/renderer.rs` or similar
