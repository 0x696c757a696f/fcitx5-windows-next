# 080 layout: three-axis candidate layout model (frozen 2026-09-04)

**Sources:** rimeinn/rabbit (flow/stacked/vertical_text visuals), fcitx5-macos#164 (scroll),
user design spec (orientation/overflow/writing-mode separation).

## 1. Three orthogonal concepts (do not collapse)

```rust
enum CandidateOrientation { Horizontal, Vertical }   // candidate items' overall arrangement
enum OverflowBehavior    { Paging, Scrolling, Wrapping } // what happens past the visible space
enum WritingMode         { Horizontal, VerticalRl, VerticalLr } // the candidate TEXT's own direction
```

**CandidateOrientation != WritingMode.** A "vertical candidate list" (1你好/2你们/3你的 top-to-
bottom) is still Horizontal writing with orientation=Vertical. VerticalRl/Lr is traditional CJK
vertical typesetting (glyphs top-to-bottom). Never merge the two.

## 2. Effective combination matrix + UI visibility

| orientation | overflow | writing | UI shows? | semantics |
|---|---|---|---|---|
| Horizontal | Paging | Horizontal | 普通页 ✓ (default) | classic single-row candidate bar, page up/down at page capacity |
| Vertical | Paging | Horizontal | 普通页 ✓ | traditional vertical list (row per candidate) |
| Horizontal | Scrolling | Horizontal | 普通页 ✓ | single row, viewport scrolls (not wraps); highlighted always scroll-into-view |
| Vertical | Scrolling | Horizontal | 普通页 ✓ (our natural extension; fcitx5-macos limits scroll to H+H, do not claim theirs) | fixed max height, viewport scrolls vertically |
| Horizontal | Wrapping | Horizontal | 普通页 ✓ | rabbit flow: real measured width breaks to next row; no wrap at max rows → page |
| Vertical | Wrapping | Horizontal | ✗ not shown | no meaningful semantics |
| any | any | VerticalRl/VerticalLr | 高级页 (文字方向) | rabbit vertical_text: text top-to-bottom, columns left→right (Lr) or right→left (Rl); independent of orientation/overflow |

UI rule: `Vertical + Wrapping` is not displayed. Writing-mode is an advanced setting, never shown
as an orientation option.

## 3. Config model + legacy compatibility

New canonical fields (single source inside the model):

```rust
struct CandidateLayoutOptions {
    orientation: CandidateOrientation, // default Horizontal
    overflow: OverflowBehavior,        // default Paging
    writing_mode: WritingMode,         // default Horizontal
    // page_size etc. stay existing fields
}
```

Legacy mapping (never delete old keys abruptly; decode once at the model boundary):
- `layout_type=stacked` → orientation=Vertical, overflow=Paging, writing=Horizontal
- `layout_type=flow` → orientation=Horizontal, overflow=Wrapping
- `layout_type=scroll` → overflow=Scrolling (orientation from the grid shape / page_size)
- `layout_type=vertical_text` → writing=VerticalRl (or VerticalLr per a direction flag) with a
  column arrangement
- `layout_type=automatic` → presentation-decided orientation (Horizontal/Vertical from work area)
- legacy `orientation=vertical|horizontal` + `scroll_mode=true|false` → same expansion
- saving prefers the new canonical fields; the renderer/layout engine reads ONLY the unified model
  (no `if horizontal / if scroll / if flow` scatter).

## 4. Settings UI

Ordinary page (config-poc):
```
候选窗口
布局:  [横向预览卡片] [纵向预览卡片]   ← orientation, clickable live/static preview cards
候选溢出: [分页 ▾]                    ← overflow, options filtered by orientation
候选数量: [5]
高级设置 >
```
Advanced page: 文字方向 (横排 / 竖排从右到左 / 竖排从左到右), 候选间距, 窗口边距, 最大宽度/高度,
圆角, 阴影, 透明度, 候选编号样式 — reuse existing settings where present; do not duplicate.

Preview must update instantly with orientation/overflow (real candidate-core geometry, not only a
schematic), staying inline with the plan for a later real renderer preview.

## 5. Renderer invariant (one paint path)

Renderer receives only geometry: x/y/w/h, visibility, viewport offset, per-item writing token.
Different layouts only change the layout engine's output rects; label/text/comment/highlight paint
is one shared path. No per-combination renderer copies.

## 6. Interaction invariants

- Keyboard selection index is data-stable: candidate index N stays N across wrap/scroll/orientation.
- Paging changes the candidate page; Scrolling changes only the visible viewport.
- Highlighted candidate is always scroll-into-view in Scrolling.
- Wheel: a single wheel semantics (scroll viewport in Scrolling; page prev/next in Paging) — check
  existing wheel handling before adding any.

## 7. Window stability + HiDPI + font

- Re-check placement vs caret + monitor work area whenever orientation/overflow/writing changes the
  size; no flow-expansion off-screen, no viewport jitter, no resize-on-highlight-change.
- Real font measurement (DWrite), per-monitor DPI; no `chars × fixed-width` estimates.
- Layout must scale under 100/125/150/200%; highlight geometry must equal text geometry.

## 8. Priority scope this round (do not gold-plate)

1. Horizontal+Paging, 2. Vertical+Paging, 3. Horizontal+Scrolling, 4. Vertical+Scrolling,
5. Horizontal+Wrapping, 6. WritingMode vs CandidateOrientation decoupling,
7. settings UI + preview, 8. config persistence + legacy, 9. DPI/window-bounds correctness.
No new theme/animation/config framework; no big renderer rewrite unless blocked (state the block).

## 9. Acceptance

Config decode → unified model tests; layout() geometry per combination; settings UI + preview;
legacy config still loads; DPI screenshots; user visual review of previews/screenshots; and the
15 acceptance items from the user spec (paging/scroll-into-view/wrap/measure/monitor-bounds/mixed
glyphs/DPI/live-switch/writing-vs-orientation/legacy/edge-cases/click index/highlight geometry).
