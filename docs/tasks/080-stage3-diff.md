# 080 Stage 3 — Rust renderer geometry vs C++ goldens (differential record)

**Date:** 2026-09-04
**Files changed:** `rust/candidate-core/src/bin/candidate_poc.rs` (`window_smoke::render_golden` only).
**Not committed** (per Stage-3 task contract).

## Why the Rust Stage-2 output was much larger than the C++ goldens

Stage 2 rendered through `qingfeng_candidate_visual_plan`, whose default typography is the
WindInput/Qingfeng theme (**22 DIP candidate font, 42 DIP row height**) and whose cell widths are
**character-count estimates** (`(font+2) px` per CJK char, `comment_font*0.72` per Latin char).
The C++ shipping renderer (`ui_main.cpp`) measures every label/text/comment with DirectWrite
(`IDWriteTextLayout::GetMetrics`) and lays out with the Rust presentation geometry from
`resources/config.toml` (**18 DIP candidate font, label scale 0.85, annotation/comment scale 0.80,
padding 10/6, item padding 8/4, row gap 1, column gap 12, corner radius 12, YaHei**).

## What Stage 3 changed

`render_golden` no longer uses the qingfeng typography/estimates. It now:

1. Measures each label / text / comment with the real DirectWrite engine
   (`windui::text::DWriteEngine::measure` → `IDWriteTextLayout::GetMetrics`, family
   `Microsoft YaHei UI`), sizes: candidate `18.0`, label `18.0*0.85`, comment `18.0*0.80` —
   the exact C++ config defaults at DPI 1.0.
2. Builds item sizes with the C++ formula:
   `width = label_w + text_w + comment_w(+2-space comment prefix) + label_gap(4) + item_padding_x*2`,
   `height = max(label,text,comment height) + item_padding_y*2`.
3. Lays out through the Rust presentation `layout()` (the same function the C++ renderer calls)
   with `resources/config.toml` geometry; the window size is `layout().window`.
4. Renders with tiny-skia + windui DWrite (unchanged); colors stay qingfeng-theme light/dark
   (white / rgb(24,24,24) background, WeChat-green selection/text).

## Before / after (rendered BMP dimensions)

| State | Stage-2 qingfeng-estimated | Stage-3 DWrite-measured | C++ golden (target) |
|---|---|---|---|
| vertical / vertical-dark (3 candidates, sel 1) | 201 x 144 | **166 x 107** | 96 x 62 |
| scroll / scroll-dark (60 candidates) | 534 x 520 | **275 x 203** | 219 x 118 |

Artifacts: `out/080-rust/rust-golden{,-dark,-scroll,-scroll-dark}.bmp` (regenerated after the
change; dimensions printed by `--render-golden`). C++ goldens remain under
`out/build/windows-x64-dev/080-golden/`.

## Remaining deltas and why they are not closed in this slice

- **vertical height and width and scroll height scale ~1.73x versus the C++ golden.** Both sides
  use the same DWrite metric source at the same nominal 18 DIP, so a fixed ~1.73x factor means the
  C++ golden was captured under an effective text scale ≈ 18/1.73 ≈ 10.4 px (a capture/DPI artifact
  of the `fcitx5-ui.exe --demo` + `PrintWindow` harness at `caret dpi=96`, or an environment scale
  the golden manifest did not record). Closing this exactly requires either dumping the C++ text
  formats' effective size at capture time or re-capturing the goldens on a controlled 100% host.
- **scroll width 275 vs 219 (1.26x)** is non-linear: the scroll window width is driven by the
  presentation scroll state (page window, visible columns, cell width 96, max-width clamps inside
  `layout()`). Stage 3 passes the demo input (`6/page`, selected 18, 60 items); an exact match needs
  the real presentation scroll viewport parameters captured from the C++ side, not just the page
  input.
- **D2D vs tiny-skia text cannot byte-match.** The C++ renderer draws with Direct2D +
  ClearType-subbed DWrite glyphs on an HWND render target; the Rust renderer is tiny-skia + windui
  DWrite. Even with identical geometry, glyph rasterization/antialiasing differs per-pixel. The
  Stage-3 contract is geometry (window dimensions + item placement), which is now driven by the same
  DWrite metrics + presentation layout as the C++ path.

## Validation run in this slice

- `cargo test --locked -p fcitx5-candidate-core --target x86_64-pc-windows-msvc` → 40 passed.
- `cargo fmt -p fcitx5-candidate-core -- --check` → clean (applied `cargo fmt` once).
- `git diff --check` → clean.
- `fcitx5-candidate-poc --render-golden {vertical,vertical-dark,scroll,scroll-dark}` → 4 BMPs
  regenerated at the Stage-3 dimensions above.
- No tests were added (no new public behavior; `render_golden` is a screenshot tool). No staged
  files. Remaining lint notes: `theme_mode` and `candidate_count` are now unused inside
  `render_golden` (kept as the surrounding code still reads them); harmless warnings.
