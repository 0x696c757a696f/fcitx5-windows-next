# 080 layout research: reference geometry rules (rabbit + fcitx5 scroll)

Research companion to `docs/tasks/080-layout-naming-design.md`. Gives mirror-able geometry rules
for the four candidate layouts (`stacked` / `flow` / `scroll` / `vertical_text`) plus the selected
"label + text + comment" cell rules, taken from two working IME frontends. No repo code changed.

- **Reference A — rabbit (玉兔毫, rimeinn/rabbit, AutoHotkey + Direct2D/DirectWrite).**
  Repo root: `D:\tmp\pi-github-repos\runtime-bxaVoE\cd8d167701b44e3c7c37a0518947604ae7ee8be1e780043cda5eb9a8e93da368`
  All `Lib/*.ahk` paths below are relative to that root. HEAD of clone used as-is.
- **Reference B — fcitx5-macos PR #164 "scroll mode 📜 卷轴模式".** Merged diff of `webpanel/webpanel.cpp`
  (+240/−45) and `webpanel/webpanel.h` (+27) fetched from
  `https://patch-diff.githubusercontent.com/raw/fcitx/fcitx5-macos/pull/164.diff`. The actual grid
  layout lives in the webview submodule `fcitx-contrib/fcitx5-webview` (cloned fresh to
  `/tmp/fcw`, master HEAD). `page/scroll.ts`, `page/panel.ts`, `page/customize.ts`, `page/common.scss`,
  `page/generic.scss` relative to that clone.

PNG geometry was confirmed by pixel-band analysis of `docs/images/candidate-layouts/{stacked,flow,flow_paging,vertical_text_left_to_right,vertical_text_right_to_left}.png`
(ink/column-band extraction; screenshot text itself is not machine-readable, so exact glyph-level
facts come from code, pixel facts from band structure).

**Common shared style metrics (rabbit, `Lib/RabbitUIStyleSnapshot.ahk:30-77`, defaults used when the
YAML omits a key):** content origin `base_x/base_y = borderWidth + marginX/marginY`
(`RabbitCandidateBox.ahk:251-252`, `413-414`, `581-582`); borders drawn as an outer rounded rect with
inner rounded background (`RenderFrame`, `RabbitCandidateBox.ahk:918-932`). Defaults: border 2, corner
radius 6, highlight radius 4, margins 6, candidate padding X/Y 0, candidate spacing 6, min box 160×160,
`label_format` = `"{}. "`, fonts Microsoft YaHei UI 14pt for label/text/comment (all three sizes 14).
`label_format` supplies the visual label string, e.g. `"1. "`; labels come from select keys /
index numbers (`RabbitCandidatePresentation.ahk:46-60`).

---

## 1. `stacked` 纵排 (vertical list, horizontal glyphs)

Source of truth: `BuildStacked` `Lib/RabbitCandidateBox.ahk:250-344`.

- **Row = one candidate; rows stacked top-to-bottom.** Each row is laid out independently inside a
  shared content column.
- **Label slot (per row, natural width, NOT column-shared):** label text at
  `x = base_x + candidatePaddingX`; measured with `GetLabelTextMetrics`
  (`RabbitCandidateBox.ahk:1102-1109`), which includes **trailing whitespace** so a `"{}. "` format
  yields an inherent gap. Candidate text starts immediately after the label:
  `x = base_x + candidatePaddingX + label_width` (`:282-302`). Left-aligned; comment follows inline:
  `x = label_x + label_w + candidate_w` (`:304-313`).
- **Comment placement:** normally inline right after text; if present it is then **re-floated to the
  right edge** of the content column: `comment.x = base_x + content_width − candidatePaddingX − comment.w`
  (`:339-342`), executed only when `comment.w > 0`.
- **Text start gap**: nothing beyond `candidatePaddingX`; cell text begins right after label slot.
- **Row height:** `max(label_h, candidate_h, comment_h) + 2 * candidatePaddingY`; per-row baseline
  alignment is a single shared rule `AlignCandidateText` (`:743-749`, config `align_type`:
  `top` default / `center` / `bottom`), applied separately to label/text/comment so mixed-font rows
  align on one rule. Consecutive rows are separated by `candidateSpacing` (added between rows, not
  inside them; `:319-323`).
- **Selected highlight shape:** rounded rect per row (`hlCornerR`) over the **full content width** —
  after measuring, every row rect is stretched: `row.w := content_width`
  (`:330-331`, `content_width = boxWidth − 2*border − 2*margin`, `:325-328`, min-width clamp `:326-328`).
  So in `stacked` the highlight (and the per-row `candidate_back_color` background pill) is a
  uniform full-width band, one per row. Paint loop: `RenderFrame`, `RabbitCandidateBox.ahk:941-975`
  (background pill + label/text/comment colors swap on `candidate.highlighted`).
- **Box width** = widest row (preedit width and candidate rows compete) clamped to `min_width`;
  **box height** = preedit + Σ(row heights) + Σ(spacings) + chrome (`:324-333`).
- PNG `stacked.png`: 6 ink bands = preedit row + 5 candidate rows; comment text visibly sits at the
  far right of the box (pixel band at x≈290-331), confirming the right-floated comment rule.

---

## 2. `flow` 横排 (horizontal flow, rows break at page boundary)

Source of truth: `BuildFlow` `Lib/RabbitCandidateBox.ahk:579-714`; trigger/expansion state in
`BuildPresentationLayout` `:233-239`; page model in `RabbitCandidateViewport.ahk:33-105`.

- **Single row + paging is the collapsed form; multi-row "flow_paging" is the expanded form — same
  code path.** One Rime page occupies one flow row. `flow_page_size` (per row / per page) is set by
  the viewport to Rime `menu/page_size` (default 5; `RabbitCandidateViewport.ahk:36-42`).
- **Grid:** `column_count = min(flow_page_size, candidate_count)`,
  `row_count = ceil(candidate_count / flow_page_size)` (`:618-619`). Candidate order is
  **row-major**: `column = ((i−1) mod page_size)+1`, `row = ceil(i/page_size)` (`:614, 630`).
  Collapsed: candidates ≤ page_size ⇒ 1 row. Expanded (`RabbitCandidateViewport.BuildExpanded`,
  `:73-105`): the viewport preloads up to `flow_rows` pages (config default 5, clamp 1..9;
  `RabbitUIStyleSnapshot.ahk:75`, `RabbitAppearanceSettingsPage.ahk:724`) centered on the current
  page (`target_row = ceil(flow_rows/2)`, `:75-76`), so each page = one flow row and the current
  page stays near the middle; candidates on non-current rows get **empty labels** and are drawn as
  plain preview text (`RabbitCandidateViewport.ahk:93-101`).
- **Label slot in flow:** shared **per column** across rows. Before layout, for every candidate the
  label width is measured with trailing-whitespace inclusion and the maximum per column index is
  kept: `label_widths[column] := max(...)` (`:609-617`). So label column N is one fixed width for
  every row; ordinal digits align vertically, and label is left-aligned at
  `card_x + candidatePaddingX` (`:682-688`).
- **What happens when candidates exceed one row / one page:** extra pages become extra rows; a page
  never spills into a second row (page boundary = hard row break). Rows/columns are height/width
  uniform: `column_widths[col] = max card width in column`, `row_heights[row] = max card height in
  row` (`:651-652`); column x positions advance `column_width + candidateSpacing`
  (`:655-661`), row y advances `row_height + candidateSpacing` (`:663-669`). Card = column cell
  (rounded highlight pill covers exactly that cell, `rows[i] = {x: card_x, w: column_widths[col]}`,
  `:703-707`).
- **Overwidth cells (fixed max width case):** when the box is width-constrained the per-cell budget
  is `card_max_width = (available − (page_size−1)*spacing)/page_size`
  (`:589-594`); cards wider than that are ellipsized by `TruncateFlowCandidate` (`:726-741`):
  candidate text truncates with `…`, comment is dropped first if the text alone still overflows.
- **Expand/collapse animation intent:** overflow (candidates > flow_page_size) sets
  `flow_expanded`; a visible box animates height from the old row-count to the new box height with a
  cubic ease-out over 160 ms in 15 ms steps (`FLOW_ANIMATION_DURATION/INTERVAL`,
  `RabbitCandidateBox.ahk:30-31`, `StartFlowAnimation`/`RenderFlowAnimationFrame` `:793-860`),
  anchored to the caret edge (`SetFlowAnimationAnchor`); collapse to a shorter box re-uses the
  previous full frame while clipping so the window appears to shrink smoothly. Purpose: expanding
  from one row to the paged grid must not jump.
- PNG `flow.png` = preedit + **one** candidate band (collapsed single row); `flow_paging.png` =
  preedit + 5 stacked bands (expanded grid, page per row) — same band count as `stacked.png` but
  rows carry multiple candidates each.

---

## 3. `scroll` / 卷轴 (fcitx5-webview, fcitx5-macos#164)

C++ seam (fcitx5-macos merged PR #164): `webpanel/webpanel.cpp` `scroll(start,count)`, `expand()`,
`collapse()`, plus scroll key interception in the PreInputMethod key handler; defaults for every key
are in the new `ScrollConfig` (`webpanel/webpanel.h`, PR hunk +135): expand = `Down`, collapse = `[]`,
up/down/left/right = arrows, row start/end = Home/End, page up/down = PgUp/PgDn, commit = Space.
While collapsed, `Down` expands (`webpanel.cpp`, state `ready`). Digit keys **1-6** select position
1-6 of the highlighted row (`selectMap`, PR hunk +140-160). All other grid math is JavaScript in the
webview submodule; current master `page/scroll.ts`.

- **Activation:** scroll mode only when engine layout hint = horizontal AND writing mode =
  horizontal-tb AND `EnableScroll` is on AND the candidate list is a bulk list (`webpanel.cpp`
  `update()` hunk +258; state machine `scroll_state_t`: `none` / `ready` / `scrolling`).
- **Grid defaults:** `MAX_ROW = 6`, `MAX_COLUMN = 6`, `UNIT_WIDTH = 65` px (comment: `(400−8)/6`),
  `ROW_HEIGHT = 28` px — `page/scroll.ts:12-15`. All four are theme-configurable through
  `setScrollParams(row, column, width, height)` (`:17-21`), fed from style JSON:
  `cellWidth = 65` default, override `Size.ScrollCellWidth`; `candidateHeight =
  max(24, TextFontSize) + TopPadding + BottomPadding + 2*Margin` (`page/customize.ts:55-69,167-170,
  191-195`), themes "macOS 26" 28 px vs "macOS 15" 30 px. Panel **inline-size is fixed**:
  `cell-width * max-column + 2px redundancy + scrollbar-width(8px)` ⇒ 400 px default
  (`page/common.scss:393-395`, scrollbar-width vars `customize.ts:196-197`).
- **Rows/columns and cell sizing:** CSS wraps items: `.fcitx-scroll-area { flex-wrap: wrap;
  overflow-y: auto }`, every `.fcitx-candidate { min-inline-size: cell-width }`
  (`generic.scss:215-239`, `common.scss:530-532`). JS then **quantizes to the unit grid** after
  layout: each cell's measured width is rounded up `nUnits = min(ceil(width/UNIT_WIDTH), MAX_COLUMN)`
  and pinned to `width = nUnits*UNIT_WIDTH`; a row closes when the running unit count would exceed
  `MAX_COLUMN`; the divider after the last item of a full row is given `flex-grow:1` to absorb the
  remainder (`scroll.ts recalculateScroll :145-167`). Consequence: columns always sit on unit-grid
  boundaries; a wide candidate can legally span several units; per-column alignment is not enforced
  beyond the unit grid. Row height is the uniform `candidate-height` CSS var
  (`common.scss` `$candidate-height`, ~:387-388); scroll area `max-block-size = MAX_ROW *
  candidate-height` (`common.scss:526-528`) so **exactly MAX_ROW rows are visible**, everything
  beyond scrolls in a native-style webkit scrollbar (8 px, `macos.scss:67-73`), not pagination.
- **Data windowing:** expand requests `(MAX_ROW + 1) * MAX_COLUMN` candidates (`scroll.ts:54-55`;
  macos `expand()` hard-codes 42 in PR #164) — visible rows plus one hidden row so scrolling is
  smooth. Scrolling near the bottom fetches another chunk of `MAX_ROW * MAX_COLUMN` starting at the
  current item count (`scroll.ts:249-260, 275-295`); C++ returns candidates from the bulk list by
  global index range (`webpanel.cpp scroll()`), and selection/actions during `scrolling` route
  through `bulk->candidateFromAll(index)` (global index, not page-local).
- **Highlight + labels in the grid:** navigation model is **row-based**.
  - `renderHighlightAndLabels` adds class `fcitx-highlighted-row` to the whole highlighted row and
    only to it; the exact cell additionally gets `fcitx-highlighted`. Non-highlighted rows fade
    labels (`label opacity:0`, `.fcitx-highlighted-row .fcitx-label {opacity:1}`,
    `generic.scss:230-234`).
  - Row labels are **re-numbered per row** from 1..9,0 by column offset:
    `label(i) = formatter((i − rowStart + 1) % 10)` (`scroll.ts:113-134`); the label element for
    each cell exists but is blanked (`renderLabel(candidate,0)` for non-highlighted rows) — this is
    the webview equivalent of rabbit's empty labels on preview rows.
  - Mark/selection visual only on the focused cell (`page/panel.ts:139-152` adds a `.fcitx-mark`
    placeholder cell for vertical/scroll so cells keep equal size).
- **Navigation keys inside scrolling:** arrow Up/Down move to the vertically nearest cell in the
  adjacent row by midpoint overlap; Left/Right are strict neighbors; Home/End jump to row start/end;
  PgUp/PgDn step exactly `MAX_ROW` rows (looping Up/Down up to MAX_ROW times);
  `Up`/`PgUp` on the first row collapses back to ready state; `commit` (Space) selects the
  highlighted cell (`scroll.ts:170-273`). While collapsed, wheel over the panel = page prev/next
  (`scroll.ts:279-285`); while scrolling, wheel scrolls the inner scrollbar with auto-fetch.
- **Visual difference from rabbit's plain flow row:** rabbit flow is a rigid page grid (fixed
  columns per page, hard page breaks, no scrollbar, rows equal per column widths) with page-key
  navigation; fcitx scroll is one continuous unit-quantized wrap grid capped at MAX_ROW visible rows
  in a fixed-width panel, with an internal scrollbar, incremental fetch, row-restart numbering and a
  moving highlight row. Collapsed "ready" look ≈ rabbit single flow row + expand chevron
  (`page/panel.ts:188-194`; `page/common.scss:388-391` for the pill end-radius, and paging buttons
  on the right like rabbit/arrow paging).

---

## 4. `vertical_text` 竖排文字

Source of truth: `BuildVerticalText` `Lib/RabbitCandidateBox.ahk:412-505`; vertical typesetting
`GetVerticalTextMetrics` `:1111-1119` and `DrawLayoutText` `:1121-1144`.

- **Column direction:** config `style/vertical_text_left_to_right` (default `false` ⇒
  right-to-left). RTL: first candidate starts at the right edge (`x = base_x + content_width`, then
  `x -= card.width` per candidate); LTR: first candidate at the left edge and columns advance
  `+card.width + candidateSpacing`; RTL advances `−(candidateSpacing)` between columns
  (`:452, 463-465, 499`). PNG pair confirms: identical 5-column extents in both, mirror content only
  in column order (pixel-diff not a pure x-mirror because glyphs are not symmetric).
- **Typesetting:** every text run (label, candidate, comment) is measured and drawn with DirectWrite
  `reading_direction = TOP_TO_BOTTOM`, `flow_direction = RIGHT_TO_LEFT`
  (`:1111-1119, 1130-1138`); a `card.width` per column = `max(label_w, cand_w, comment_w) +
  2*candidatePaddingX`, `card.height = label_h + cand_h + comment_h + 2*candidatePaddingY`
  (`:436-443`). Box height `= max(min_height(160 default), content + chrome)`
  (`:456-459`); width = Σ card widths + spacing between cards + chrome, plus a vertical preedit
  column when shown (`:445-455`).
- **Ordinal/label placement:** at the **top of each card, horizontally centered over the vertical
  text**: `x = card_x + (card.width − label.w)/2`, `y = base_y + candidatePaddingY`
  (`:476-482`); the candidate text begins right under the label (`y += label.h`, `:484-489`);
  comment is **bottom-anchored and centered**: `y = content_bottom − candidatePaddingY − comment.h`
  (`:491-496`, `content_bottom = boxHeight − borderWidth − marginY`, `:459`).
  PNG: top band (y≈62-85) carries the ordinal row at the same x-clusters as the columns below;
  candidates occupy the tall vertical strip beneath.
- **Highlight:** per-card rounded pill `{x: card_x, y: base_y, w: card.width, h: content_bottom −
  base_y}` (`:498`) — one full card column, spanning the whole inner height.
- `align_type` (top/center/bottom) does **not** apply (settings page disables it for vertical_text,
  `RabbitAppearanceSettingsPage.ahk:657`); vertical stacking is always top-anchored.
- Vertical flow direction RTL with preedit: preedit column is offset to sit left of the candidate
  columns (`:452-464` + `OffsetLayoutX` `:544-558`).

---

## Implications for our Rust candidate-core (exact rules to mirror)

Mirror these rules 1:1 in the Rust layout engine (see also `080-layout-naming-design.md` §1 for the
unified `layout_type` surface):

1. **Label slot rule — three variants, do not unify them away:**
   - `stacked`: label slot is *per row, natural width*, measured **including trailing whitespace of
     `label_format`** (rabbit `GetLabelTextMetrics` / `include_trailing_whitespace`,
     `RabbitCandidateBox.ahk:1102-1109`); text x = label end; no inter-row label alignment.
   - `flow`/grid: label slot width is shared **per column index across rows** (max per column)
     ⇒ ordinal columns line up in every row (`:609-617`).
   - `scroll` (fcitx): labels re-number per highlighted row `1..9,0`, only the highlighted row's
     labels visible, label box text-align center (webview `.fcitx-label`,
     `generic.scss:154-158, 230-234`).
   - Model the label string as already-formatted (`Format(label_format, label)`); measure with
     trailing whitespace so "1. " yields an automatic gap.
2. **Flow page break rule:** page boundary = hard row break; `row_count = ceil(n/page_size)`,
   `column_count = min(page_size, n)`; row-major indexing `column=((i−1) mod ps)+1`; column widths
   and row heights are the maxima of their cells; inter-cell gap `candidateSpacing` between columns
   and rows; overflow beyond one page ⇒ more rows (rabbit) — do not implement a reflow "one long
   line that wraps at arbitrary glyph width" as the default break.
3. **Expansion is a height animation, not a relayout.** Collapsed height animates to the multi-row
   grid height (cubic ease-out, ~160 ms in rabbit; 300 ms max-block-size in webview), anchored to
   the caret edge.
4. **`vertical_text` ordinal centering:** label above the vertical run, horizontally centered in
   the card; comment bottom-anchored, centered; text top-anchored directly under the label; column
   order flag `vertical_text_left_to_right` (default RTL); column gap = `candidateSpacing`; box
   height floor `min_height`. Highlight = whole card column, not text width.
5. **Comment rules:** `stacked` floats the comment right (`right − paddingX − comment_w`, only when
   width > 0); `flow` keeps it inline after text and truncates the card (drop comment first, then
   ellipsize candidate text) under a width budget; `scroll` keeps comment inline within the cell,
   wraps with `overflow-wrap:anywhere` instead of truncating; `vertical_text` places it bottom.
6. **Selected-cell geometry:** highlight rect ≠ text bounding box. `stacked`: full content width;
   `flow`: the cell's column width; `scroll`: the quantized cell (`nUnits*UNIT_WIDTH`); label/text
   color triple swaps with the pill (label_color, text_color, comment_color).
7. **When mirroring fcitx scroll:** implement panel width = `maxColumn * cellWidth + scrollbar` with
   `min-inline-size: cellWidth`, unit-quantized wrap (`nUnits = min(ceil(measuredWidth/cellWidth),
   maxColumn)`), `maxRow` visible rows + internal scroll + incremental fetch window of
   `maxRow*maxColumn` from the global index, digit select 1..6 within the highlighted row,
   Up/Down/Left/Right/Home/End/PgUp/PgDn(=maxRow step)/Space-commit, Up on row 0 = collapse.
   Defaults to freeze in Rust config: rows 6, cols 6, cell 65 px, row height 28 px (each overridable
   — cf. webview `ScrollMode.MaxRowCount/MaxColumnCount/ShowScrollBar`, `Size.ScrollCellWidth`).

Key references:
- rabbit box + viewport: `Lib/RabbitCandidateBox.ahk:229-248, 250-344, 412-505, 579-749, 941-975,
  1095-1144`; `Lib/RabbitCandidatePresentation.ahk:33-60`;
  `Lib/RabbitCandidateViewport.ahk:33-105`; `Lib/RabbitUIStyleSnapshot.ahk:30-77`.
- fcitx: merged PR #164 `webpanel/webpanel.cpp` (scroll/expand/collapse + key map) and
  `webpanel/webpanel.h` (ScrollConfig); webview master `page/scroll.ts:12-22,54-55,113-167,170-295`,
  `page/panel.ts:94-240`, `page/customize.ts:55-69,167-197`, `page/common.scss:387-395,515-532`,
  `page/generic.scss:215-243`.
