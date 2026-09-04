# 080 layout survey: candidate window entry / renderer / geometry / config / settings / paging / input / change path

Survey basis: HEAD `24c444ebacfe33553568ad2248e19bee3c1452cf`, clean tree. Read-only task; no code changed.
Companions: `docs/tasks/080-layout-research.md` (rabbit + fcitx5 scroll geometry), `docs/tasks/080-layout-naming-design.md` (frozen `layout_type` surface). This survey maps the frozen plan onto the actual current code.

Reading map for quick orientation:

- Shipping candidate window today: **C++** `src/ui/ui_main.cpp` (2993 lines, only file in `src/ui/`). Rust `candidate-core` is linked in as a **static lib**; the C++ window consumes it through low-level FFI only (`layout_run`, `render_segments`, model, presentation). Actual pixels are D2D/DWrite in C++.
- Rust side has: (1) geometry + presentation state machine `rust/candidate-core/src/lib.rs`; (2) a **not-yet-wired** whole-window state machine + plan types `rust/candidate-core/src/ui_plan.rs` with FFI `rust/candidate-core/src/candidate_abi.rs` (no consumer today); (3) a prototype WindInput-style visual planner `rust/candidate-core/src/qingfeng.rs` (test/demo only).
- Config authority is `rust/config-core/src/config_core.rs` + compiled default `resources/config.toml` + theme `resources/themes/default/theme.toml`. Settings shell is the windui app `rust/config-poc/src/main.rs`.

---

## 1. Candidate window entry: process, IPC, KeyResponse → model flow

### Process and data flow

- Process **fcitx5-ui.exe**, spawned by the Rust launcher `rust/launcher-core/src/lib.rs` `spawn_ui` (:915-943) with `--parent-pid/--generation`; path resolved by `resolve_default_process_paths` (:404-421). Launcher is the parent watcher (ui_main watches `parentId`, ui_main.cpp:2976-2986).
- UI **publishes nothing out**; it is the **server** of a `presentation` named pipe (`servePresentation`, `src/ui/ui_main.cpp:2859-2919`) and the **client** of the engine `engine` pipe for click-to-select (`CandidateWindow::create`, ui_main.cpp:1228-1250; client handle from `fcitx5_windows_common_candidate_select_client_create_utf16`).
- **Engine** (fcitx5-engine.exe) writes every state-changing key/select result into the `presentation` pipe. Wire is Rust protocol-core `keyResponse` frames, **wire type 4**, header 64 bytes (`kKeyResponseMessageType`, ui_main.cpp:519-520; frame cap 256 KB at :2859+). Engine side publish: C++ `PresentationPublisher` wrapper over Rust `engine-core` publisher (`src/engine/fcitx_engine_main.cpp:113-126, 392-393`), Rust worker `rust/engine-core/src/presentation_publisher.rs` (`VerifiedPipeClient::connect_exact` to the pipe, verifying the peer is the UI exe, :68-98).
- Key events themselves never reach the candidate window: it is `WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST | WS_EX_LAYERED`, `WS_POPUP` (ui_main.cpp:1260-1271). Keys go app → TSF → engine; engine owns paging/selection; UI only re-paints the resulting state.

### Entry points inside ui_main.cpp

- `wWinMain` (ui_main.cpp:2922-2993): parse flags → self-test/demo modes or `CandidateWindow window; window.create(...); thread(servePresentation, window.handle(), testOnce)` (:2991) → `window.run()` message loop.
- `servePresentation` (:2859-2919): accept one engine connection, loop-read frames, `decodePresentationFrame` (:2784-2856, DTO mirrors `KeyResponse`), then `PostMessageW(window, kSnapshotMessage, …)`.
- `windowProcedure` (ui_main.cpp:2564-2700) `kSnapshotMessage` handler (2584-2591) → `self->update(*response)`.
- `CandidateWindow::update(const KeyResponse&)` (ui_main.cpp:2088-2411): single funnel for all candidate data.

### KeyResponse → model flow (update(), ui_main.cpp:2088-2411)

Key DTOs (file-local mirrors of Rust DTOs; ui_main.cpp):
`Metadata` (:529), `CaretRect` (:529-534, has `dpi`), `CandidateRecord {id,labelUtf8,textUtf8,commentUtf8}` (:537-542), `KeyResponse` (:545-572: metadata, status, preeditUtf8, contentLocaleUtf8, candidates, selectedCandidate `UINT32_MAX`=none, candidatePage, candidatePageSize, candidateTotal, candidateVisibility, candidateBulk, candidateEnd, caret, popupAllowed).

Flow in `update()`:
1. Build `candidate::Snapshot` (ui_main.cpp:2088-2113) and apply to the Rust model via C++ `CandidateModel` wrapper (:738-770 → `fcitx5_candidate_model_apply`, FFI `rust/candidate-core/src/lib.rs:1123`). Returns applied/duplicate/stale/invalid; stale/invalid drops the frame.
2. Apply presentation metadata through FFI `fcitx5_candidate_presentation_apply` with `Fcitx5CandidatePresentationUpdate` (ui_main.cpp:2115-2133; struct def :78-88; Rust side `CandidatePresentationState::apply`, lib.rs:2216-2294). Returns 1/duplicate, 2/stale → frame dropped.
3. Get the render window via `fcitx5_candidate_presentation_render_plan` (ui_main.cpp:2189-2195; Rust `lib.rs:1252-1289`) → `renderIndices_` (which candidates exist for this presentation).
4. Hide if `visibility==hidden`/empty/no caret; `hidePopup()` if `!popupAllowed` (2187-2200).
5. Re-measure everything with DirectWrite (see §2), compute candidate item `Size`s, resolve orientation, call Rust `layout()` (§3), place window, stash `itemRects_/visibleIndices_`, `SetWindowPos` + `InvalidateRect` (2200-2406).
6. `WM_PAINT`/`WM_PRINT` → `paintOnce()` (1769-1991) / `paintToDeviceContext` (1993-2076) rasterize (§2).
7. Reconfig: `WM_SETTINGCHANGE/WM_THEMECHANGED/WM_SYSCOLORCHANGE` → `reloadVisualConfig()` (2634-2638) → re-read config + `reflowCurrentModel()` (:2412-2452); `WM_DPICHANGED` (:2624-2631).
- Synthetic entry: `showSyntheticPreview(bool scrollDemo)` (ui_main.cpp:1297-1340) builds a `KeyResponse` by hand (42 words scroll demo with 60 bulk candidates vs 3-candidate demo) and calls `update()`; used by `--demo`, `--scroll-demo`, reload and interaction self-tests.

---

## 2. Candidate renderer: where the pixels come from, input/output types

### Shipping renderer: C++ D2D + DirectWrite (ui_main.cpp)

- Device resources: `createDeviceResources` (ui_main.cpp:2667-2742): `ID2D1Factory`, `IDWriteFactory`, three `IDWriteTextFormat`s (text `textFormat_`, label `labelFormat_`, comment/annotation `annotationFormat_`), one `ID2D1HwndRenderTarget`. Fonts from `NativeRenderConfig`; sizes = `candidateFontSizeDip * labelFontScale * fontDpiScale_` (2736-2745); wrapping disabled + ellipsis trimming (2745-2762). **HiDPI: render target forced to `SetDpi(96,96)` (:2723) and again on `WM_DPICHANGED` (:2621)** — the hardcode the naming doc §3 removes.
- Text paint: `paintOnce()` (1769-1991). Background/border/selection via `visualConfig_.colors` (light/dark/high-contrast resolution at 1787-1880), preedit panel (1888-1902), per-candidate: selected pill `FillRoundedRectangle` (1946-1956) then three `DrawTextW` calls for label/text/comment clipped into segment rects (1957-1980); scroll-grid dividers (:1899-1928); scrollbar track+thumb (1981-1987); window border (1988-1996).
- So today the Rust core supplies **geometry/segments**; **all glyph pixels are D2D/DWrite in C++**. There is no Rust-side rasterizer in the shipping path.

### Rust inputs to the C++ painter

- Layout FFI: `ui::layout()` C++ shim (ui_main.cpp:381-409) → `fcitx5_candidate_layout_run` (`rust/candidate-core/src/lib.rs:1691-1774`); C++ mirror types `ui::LayoutInput/LayoutResult` (ui_main.cpp:293-320). Input includes resolved orientation, per-item measured `Size`, caret, work area, paddings/gaps, placement, scrollMode/scrollColumns/scrollVisibleRows/selected/scrollCellWidth. Output = window rect + item rects + itemIndices + scrollbar rects + firstVisible.
- Segment FFI: `renderSegments()` shim (ui_main.cpp:411-445) → `fcitx5_candidate_render_segments` (`lib.rs:2039-2068`, core `candidate_render_segments` `lib.rs:1975-2037`). Input `Fcitx5CandidateRenderItemInput {bounds, labelWidth, labelGap, textWidth, commentWidth, hasLabel, reserveLabel}`; output `Fcitx5CandidateRenderItemOutput {label,text,comment rect, drawComment}`. It is **label-column math only** (label rect = shared per-screen column width for scroll when any item reserves a label; comment dropped when it does not fit, `draw_comment=0`). This is where label/comment sub-rect geometry currently lives (lib.rs:1975-2037).
- Hit test FFI for clicks: `fcitx5_candidate_hit_test` (`lib.rs:1782-1806`).
- Model/presentation/label FFI used by C++: `fcitx5_candidate_model_*` (lib.rs:1074-1179), `fcitx5_candidate_presentation_*` (:1182-1399), `fcitx5_candidate_format_label_utf16` (:1942-1973), `fcitx5_candidate_scroll_label_policy` (:1926-1933), `fcitx5_candidate_locale_prefers_compact_horizontal_utf8` (:1913-1923).

### Rust candidate-core as renderer-plan layer (future/parallel)

- `rust/candidate-core/src/ui_plan.rs` — Rust-owned whole-presentation state machine + **renderer-neutral plan**. Key types: `CandidateUiColors` (:32-41), `CandidateTheme {Light,Dark,HighContrast}` (:86-90), `CandidateUiConfig` (:94-110, carries `orientation: PresentationOrientation`, `scroll_mode: bool`, `page_size`, geometry paddings/gaps, fonts, theme, opacity), `CandidateUiMeasurement` (:136-141, DWrite label/text/comment width + height), `CandidateUiInput` (:166-174), `CandidateRenderItem` (:187-198, per-item rects incl. `item_rect/label_rect/text_rect/comment_rect`), `CandidateUiaPlan` (:210-213), `CandidateUiPlan` (:217-230: popup_visible, orientation, placement, window, preedit rect, colors, opacity, scrollbar rects, items, uia), `CandidateUiState` (:253-258). `render_plan` (:331-465) = resolve orientation (`CompositionLayoutState`), call the same `layout()` from lib.rs, split item sub-rects. `visible_indices` (:513-522), `item_size` (:524-549), `render_item` (:551-610).
- FFI: `rust/candidate-core/src/candidate_abi.rs` — `Fcitx5CandidateUiInput` (:16-38), `Fcitx5CandidateUiPlanOutput` (:69-95), `Fcitx5CandidateUiRenderItemOutput` (:99-111), lifecycle `fcitx5_candidate_ui_create/apply/measurement_texts/build_plan` (:146-329). **No consumer in `src/`, `tsf-poc`, or elsewhere** (verified by grep); it is the planned replacement seam for the C++ window, matching the "renderer cutover" language in config projections and the naming doc acceptance.
- `rust/candidate-core/src/qingfeng.rs` — prototype WindInput (青风) visual planner, `qingfeng_candidate_visual_plan` (:235-373), `QingfengOrientation {Horizontal,Vertical,Grid}` (:8-12), `QingfengCandidateVisualItem` with full label/text/comment rects (:212-221). Not wired to the shipping window; used by `candidate_poc` window smoke and its own unit tests (:375-445).
- POC/prototype runner with real pixels: `rust/candidate-core/src/bin/candidate_poc.rs` — on Windows creates real HWNDs, paints through `windui::render::SkiaCanvas`/`DWriteEngine` (imports :282-296, screenshots :1194-1380), renders "golden" (`render_golden` :2418+), layout/label-slot snapshots (:3331+). It is the current **real-render evidence harness**, not a shipping path. Deps: `tiny-skia` + vendored `windui` (`rust/candidate-core/Cargo.toml`).

### Paint-input contract summary

| Layer | Gives renderer |
|---|---|
| `layout()` (lib.rs:2389-2706) | window + item rects (x/y/w/h) + scrollbar rects |
| `candidate_render_segments` (lib.rs:1975-2037) | per-item label/text/comment sub-rects + draw-comment flag |
| D2D painter (ui_main.cpp:1769-1991) | draws glyphs into those rects |
| ui_plan `CandidateUiPlan` (ui_plan.rs:217-230) | everything above in one Rust struct (unwired) |

---

## 3. Layout/geometry code

All geometry lives in `rust/candidate-core/src/lib.rs` plus one C++ mirror (`ui::LayoutInput`, ui_main.cpp:293-323).

### Core enums & input

- `Orientation {Vertical, Horizontal}` (`lib.rs:61-64`) — meaning today: **reading/stacking direction** of the page.
- `PresentationOrientation {Automatic, Vertical, Horizontal}` (lib.rs:67-71) — configured or resolved orientation.
- `LayoutInput` (lib.rs:81-98): `orientation`, `items: Vec<Size>`, `caret`, `caret_height`, `work_area`, `max_width`, `padding_x/y`, `row_gap`, `column_gap`, `placement`, `scroll_mode: bool`, `scroll_columns`, `scroll_visible_rows`, `selected`, `scroll_cell_width`. Defaults (:100-121): vertical, max width 720, padding 8/6, row gap 2, col gap 8, scroll columns 6, visible rows 6, cell 96.
- `LayoutResult` (lib.rs:124-133): window, items, item_indices, scrollbar_track/thumb, has_scrollbar, first_visible, placement.
- FFI `Fcitx5CandidateLayoutInput` (:160-176) mirrors it; `layout` returns `LayoutResult`.

### The single dispatcher: `layout()` (lib.rs:2389-2706)

One function decides everything today, branches in order:

1. **scroll** = `input.scroll_mode && !items.empty()` (:2393-2542), then orientation-split:
   - **Vertical-oriented scroll** (`Orientation::Vertical`, :2395-2516): `rows_per_column = scroll_columns.clamp(1,9)`, `visible_columns = scroll_visible_rows.clamp(1,6)` → it lays a **page-size tall column group, 6 columns wide**; viewport columns anchored by `selected/rows_per_column`; scrollbar vertical. Cell width clamped to `scroll_cell_width.max(40)`.
   - **Horizontal-oriented scroll** (:2518-2542): `columns = scroll_columns.clamp(1,9)` = page size per row, `visible_rows = scroll_visible_rows.clamp(1,6)`; **6 rows visible**, remainder scrolls; equal `cell_width = usable/columns`.
   - So current runtime grid = **page_size × 6**, orientation decides which axis is the paged one; `page_size` doubles as "columns per row (H)" or "rows per column (V)" and grid thickness 6 is a hardcoded constant (`scroll_visible_rows`).
2. **Non-scroll** (:2544-2597): single axis — Vertical: rows stacked, width = max item width; Horizontal: one row (no wrap), height = max item height.
3. Placement/geometry window clamp (:2544-2706): below/above auto flip at `Unlocked`, clamp into work area, items translated into the window.

### Helpers added for the new layouts (pure geometry, zero-origin; NOT in the live renderer yet)

- `flow_paged_bounds` (lib.rs:2715-2759): L→R with **wrap to a new row when a row would exceed `row_content_width`**; returns (rects, tight content w, h). Doc comment claims rabbit `flow`/`flow_paging` semantics.
- `vertical_text_columns` (lib.rs:2771-2818): one column per candidate; column width = item width budget, columns L→R or R→L via `left_to_right`; vertical glyph typesetting explicitly deferred to a later renderer slice (:2776-2784 comment).
- Label-slot engine: `CandidateLabelStyle/Display/Scope/Align/WidthStrategy` (lib.rs:268-302), `CandidateLabelSlotConfig` (:305-313), `format_candidate_label` (:881-913), `candidate_label_slot_plan` (:915-980), scroll row-label policy `scroll_label_policy` (:864-879) + FFI (:1926-1933). Scroll label policy = renumber selected row/column 1..9 in scroll grids.
- Used **only** by `candidate_poc.rs` + unit tests (verified: no other references to `flow_paged_bounds`/`vertical_text_columns`).

### Automatic orientation

- `resolve_automatic_orientation` (lib.rs:3373-3399) + `locale_prefers_compact_horizontal` (:3401-3404) — candidate text / locale heuristic cached per composition in `CompositionLayoutState` (lib.rs:2327-2387, `resolve_orientation` :2348-2372). C++ path calls the equivalent FFI `fcitx5_candidate_presentation_resolve_orientation` (ui_main.cpp:2239-2251) with caret-x/scale/page size, plus its own width-vs-`max_width` fallback for auto-horizontal (ui_main.cpp:2297-2304).

### Who feeds it (C++ measurements → items)

- ui_main.cpp `update()`: DirectWrite `CreateTextLayout`/`GetMetrics`, sums label+text+comment with `labelGap`/paddings → `ui::Size` per candidate (2280-2320); preedit measured separately (:2322-2334); auto-horizontal natural width (:2306-2312); LayoutInput built at :2340-2350 with `scrollVisibleRows` hardcoded `6U` (:2346).
- ui_plan path: `item_size` (ui_plan.rs:524-549) same sum but from `CandidateUiMeasurement`; `scroll_visible_rows: 6` hardcoded (ui_plan.rs:401).

---

## 4. Config: current candidate layout fields

Authority: `rust/config-core/src/config_core.rs`. Resolved `CandidateConfig` at :122-137; compiled defaults = `resources/config.toml` (`COMPILED_DEFAULTS`, config_core.rs:20, used at :1124). User overrides layer over it; appearance colors/fonts additionally layered from the selected theme (`resources/themes/default/theme.toml`).

### `[candidate]` (CandidateConfig, config_core.rs:122-137)

| field | accessor | allowed values | range/default (validated :2205-2320) |
|---|---|---|---|
| `layout_type` | `layout_type()` :140-144 | `automatic`/`stacked`/`flow`/`scroll`/`vertical_text` (validation :2205-2208) | default fallback `"automatic"` (`default_candidate_layout_type`, :2172-2176); compiled default file says `scroll` |
| `page_size` | `page_size()` :146-149 | 1..9 | compiled default 5 |
| `max_width_dip` | `max_width_dip()` :172-175 | 160..2048 | 860 |
| `scroll_cell_width_dip` | `scroll_cell_width_dip()` :178-181 | 40..160 | 96 |
| `opacity` | `opacity()` :193-196 | 0.2..1.0 | 1.0 |
| `preedit_mode` | `preedit_mode()` :184-187 | `inline`/`panel` | inline |
| `geometry` | `geometry()` | `CandidateGeometry` (:206-230+) | see below |
| `label` | `label()` | `CandidateLabel` | see below |
| `colors` | `colors()` | `BTreeMap<String,String>` color names | keys consumed by C++ `assignNativeColor` (ui_main.cpp:1040-1058): `background/border/candidate_text/label_text/comment_text/selected_background/selected_candidate_text/selected_label_text/selected_comment_text/preedit_text`, `#RRGGBB[AA]` |

`CandidateGeometry` (config_core.rs:206-…): `padding_x_dip` (0..64; default 10), `padding_y_dip` (0..64; 6), `item_padding_x_dip` (0..64; 8), `item_padding_y_dip` (0..64; 4), `row_gap_dip` (0..64; 1), `column_gap_dip` (0..64; 12), `border_width_dip` (0..8; 1), `corner_radius_dip` (0..64; 12), `shadow: bool` (true). Validation ranges :2213-2273.

`CandidateLabel`: `visible` (true), `style` ∈ `plain|dot|paren|bracket|circled` (:2287-2291), `sequence` (9 labels), `font_scale` 0.5..1.5 (0.85), `gap_dip` 0..64 (4).

### Layout-type representation: one key + two legacy projections

- Storage key is only `layout_type`; the class projects two legacy values for old ABI consumers:
  - `scroll_mode()` (config_core.rs:151-155): `layout_type=="scroll"`.
  - `orientation()` (:157-171): `stacked→"vertical"`, `flow→"horizontal"`, everything else (`automatic`, `scroll`, `vertical_text`) → `"automatic"` ("their direction is resolved at presentation time").
- These two projections are what reach the C++ window via `Fcitx5ConfigSnapshot`: `candidate_orientation` / `candidate_scroll_mode` (`rust/config-core/src/config_snapshot_abi.rs:45-54`, filled at :149-153 from `candidate.orientation()`/`scroll_mode()`). **The ABI does not expose `layout_type` itself** — so `vertical_text` is currently indistinguishable from `automatic` at the renderer.
- C++ decode: `loadVisualConfig` (ui_main.cpp:1132-1210) reads `snapshot.candidateOrientation` etc. into `NativeRenderConfig {orientation (NativeOrientation{automatic,vertical,horizontal}, :835), scrollMode, ...}` (:852-873). So today's shipping renderer only ever sees the **3-state orientation + bool scroll** vocabulary, even though config-core has the frozen 5-state key.

### Legacy read-time migration (old config compatibility)

- `CandidateOverrides` (config_core.rs:618-660) still accepts `orientation` + `scroll_mode` keys; `normalize_layout_type()` (:668-690) maps `horizontal→flow`, `vertical→stacked`, `scroll_mode=true→scroll`, else `automatic`, then clears legacy keys. CLI field parsing (`candidate.orientation`/`candidate.scroll_mode`) goes through the same mapping (:1952-1969).
- `ConfigEdit::CandidateLayoutType` (:1334-1337) and `ConfigField::CandidateLayoutType` reset (:1294-1298) clear all three keys.

### What the config.toml file actually contains today

- `resources/config.toml` (compiled defaults): `[candidate]` with `layout_type = "scroll"`, `page_size = 5`, `max_width_dip = 860`, `scroll_cell_width_dip = 96`, `opacity`, `preedit_mode = "inline"`, `[candidate.geometry]`, `[candidate.label]` (:10-35). (Comments there still describe the old `orientation` vocabulary and promise direction-key scroll for bulk lists.)
- `resources/themes/default/theme.toml`: `[common.candidate]` `layout_type="scroll"`, `max_width_dip=860`, `opacity`, `[common.candidate.geometry]`, `[common.candidate.label]`, `[common.fonts.candidate]` / `[common.fonts.annotation]`, and `[light|dark].candidate.colors` (:11-63).

---

## 5. Settings UI (config-poc windui app)

All in `rust/config-poc/src/main.rs`. Five-way layout segmented control already exists and matches the frozen `layout_type` vocabulary.

- `CandidateLayoutMode {Automatic, Stacked, Flow, Scroll, VerticalText}` (main.rs:1176-1234): `control_value()` (:1183-1197) maps to `automatic|stacked|flow|scroll|vertical_text`; `label()` (:1198-1204) = 自动/纵排/横排/卷轴/竖排文字; `preview_description(page_size)` (:1205-1233).
- Rendered on the 外观设置 page (`windui_settings_root`, appearance card, main.rs:1703-1710) via `windui_config_core_candidate_layout_controls` (main.rs:2014-2106): segmented buttons built from `config_core_candidate_layout_button` (:1936-1980) — every click fires `update_candidate_draft(ConfigEdit::CandidateLayoutType(value))` (:1949-1950, :1965-1966); page-size buttons 1..9 (:1982-2012, `ConfigEdit::CandidatePageSize`); then 应用/取消/重置 buttons → `apply_candidate_draft/cancel_candidate_draft/reset_candidate_draft` (:2040-2106 → adapter apply/cancel/reset_candidate_layout).
- State: `WindUiConfigAdapter {path, store, core: ConfigCore}` (main.rs:2997-3057) holds a Draft/Current; `layout_mode()` (:3021-3029) maps `snapshot.candidate().layout_type()` back to the enum via `candidate_layout_mode` (:3059-3075); `set()` runs `ConfigCore.execute(ConfigCommand::Set(...))`; apply persists through FileStore; reset clears `CandidateLayoutType` + `CandidatePageSize` (main.rs:3040-3047). Loader/signal: `windui_candidate_config_manager` (main.rs:1884-1891).
- Preview plumbing: preview card is `windui_candidate_preview_panel(layout_signal, page_size_signal, theme_mode_signal, draft_summary_signal)` (main.rs:647-839). `draft_summary` derives `PreviewRenderContext::from_draft(adapter.preview(), 150)` → font family / effective font px / preedit mode (main.rs:2019-2025, type at :2955-2996).
- Config-core CLI `fcitx5_config` edit parsing for the same keys: `candidate.layout_type/page_size/max_width_dip/scroll_cell_width_dip/opacity/preedit_mode` + legacy `candidate.orientation`/`candidate.scroll_mode` (config_core.rs:1939-2018).

### Current preview = schematic, not the candidate renderer

- `windui_candidate_preview_panel` (main.rs:647-838) shows **hard-coded rows** (`windui_candidate_preview_row`, :594-646, fixed 38 px rows: "1. 是 / 2. 识 / 3. 实 / 4. 水~b …") for `Automatic|Stacked|Scroll|VerticalText`, and **hard-coded chips** (`windui_candidate_preview_chip`, :567-593, 32 px) in one row for `Flow`. Selection pill = first slot. Visibility only filters slots by `page_size` (`candidate_preview_slot_visible`, :563-565).
- It is driven by windui element-tree layout inside the settings shell — real pixels from the windui rasterizer, but the geometry is a fixed mock (identical rows for Stacked/Scroll/VerticalText; single-row chips for Flow; no grid, no wrap, no scrollbar, no vertical text). It never calls `candidate-core::layout()`/ui_plan/qingfeng (`config-poc` imports only `run_candidate_poc_self_check` from candidate-core, main.rs:9).
- The `CandidatePreviewHostEvidence`/`candidate-preview-surface` machinery (main.rs:2935-2960, :4798-4918, and the layout-element "candidate-preview-surface" in `layout_elements_for_scenario`, :3940+) is **QA/contract evidence** (rect placement, DPI parity 100-300, renderer-contract strings) around that embedded preview surface, not a separate renderer. Flags there record the desired contract: embedded in config content, not an external popup, not a fake/static renderer.
- Separate real-pixel evidence lives only in `fcitx5-candidate-poc` (candidate-core bin, §2) and its self-check.

---

## 6. Paging: who owns page state

Layers, in ownership order:

1. **Engine / fcitx instance owns paging** (page mutations, next/prev/select). UI never receives or forwards page keys: candidate window is `WS_EX_NOACTIVATE` and cannot be focused (ui_main.cpp:1260-1271). Engine dispatches key requests into fcitx on the UI thread of the engine process (`FcitxDispatcher::processKey`, `src/engine/fcitx_dispatcher.cpp:57-130`, posting onto the `::fcitx` EventDispatcher; `FcitxRuntime::processKey` `src/engine/fcitx_runtime.h:85`). Windows VK→FcitxKey incl. `FcitxKey_Page_Down` mapping at `src/engine/key_event.cpp:101`. Engine replies with a state payload containing `candidatePage`, `candidatePageSize`, `candidateTotal`, `candidateBulk` (visible in the C++ mirror `KeyResponse`, ui_main.cpp:560-570, and produced by `encodeStateResponse` after every key — `kKeyRequestType` branch at `src/engine/fcitx_engine_main.cpp:286-304`, publish at :303).
2. **Rust presentation state is a render-window mirror, not a page owner.** `CandidatePresentationState` (lib.rs:2177-2324):
   - `apply` (lib.rs:2216-2294): dedup/staleness by identity+revision; stores `page_size`, `candidate_bulk`; derives `scroll_columns = page_size.clamp(1,9)`; decides `scroll_expanded/scroll_mode = configured_scroll_mode && candidate_bulk && count>page_size && (previously expanded || page>0 || selected>=page_size)` (:2274-2286); computes `ordinary_start = page*page_size` for bulk non-scroll page windows and the ordinary visible count (:2287-2295); adjusts `selected` to the absolute index for non-bulk pages (:2296-2304).
   - It never changes page; it only *windows* the bulk list for display and tells `layout()` what is visible (`ordinary_start/ordinary_count` → `visible_indices`, ui_plan.rs:513-522) and whether to scroll (`scroll_mode`/`scroll_columns`).
3. **`layout()` viewport is derived per frame from `selected`** for scroll mode (first visible row/column math, lib.rs:2395-2542) — again no mutation.
4. **Engine-initiated select** (mouse click or select key) is the only input that advances state: ui_main click → `fcitx5_candidate_select_client_select` (ui_main.cpp:2508-2530) → engine `selectCandidate` (`fcitx_dispatcher.cpp:141-187`) → engine publishes new state → UI re-renders.
5. Everything page-related in the C++ window is deterministic re-render of engine state; the self-test `runScrollExpansionSelfTest` (ui_main.cpp:1474-…) feeds synthetic `KeyResponse`s to prove expansion/paging rendering in isolation.

There is no `page` counter in the C++ window or in candidate-core that can advance pages on its own.

---

## 7. Mouse / wheel interaction today

Mouse (all in `windowProcedure`, ui_main.cpp:2564-2700; no other mouse handling exists):

- `WM_MOUSEACTIVATE` → `MA_NOACTIVATE` (:2630).
- `WM_LBUTTONDOWN` (:2632-2641): hit test against `itemRects_` via Rust `fcitx5_candidate_hit_test`, store `pressedCandidate_`, `SetCapture`.
- `WM_LBUTTONUP` (:2642-2653): hit test again; if press+release match a candidate → `dispatchCandidate(pressed)`.
- `WM_CANCELMODE`/`WM_CAPTURECHANGED` (:2655-2659): clear pressed state.
- `dispatchCandidate(localIndex)` (:2489-2536): guards `clickInFlight_`, foreground-target validity; builds a `fcitx5_candidate_selection_intent` (FFI `lib.rs:1809-1830`, C++ `makeCandidateSelectionIntent` ui_main.cpp:463-475) and sends through the engine pipe (`fcitx5_windows_common_candidate_select_client_select`); 750 ms `kClickGuardTimer` (:2532-2535); in `interactionTest_` mode it captures the intent for self-tests (`runInteractionSelfTest`, ui_main.cpp:1342-1393, synthesizes `WM_LBUTTONDOWN/UP`).

Wheel:

- **No wheel handling anywhere** in `src/ui/ui_main.cpp` (verified: no `WM_MOUSEWHEEL`/`WM_MOUSEHWHEEL`/`GetMessagePos` matches in `src/`). No hover highlighting either. Unhandled messages fall through to `DefWindowProcW` (:2695-2698); on a `WS_EX_NOACTIVATE` layered toolwindow the wheel does nothing.
- Consequence: the fcitx5-macos "wheel = page while collapsed / scrollbar scroll while scrolling" behaviors from 080-layout-research §3 have **no Windows equivalent today**; navigation is engine keys + mouse click only.

---

## 8. Minimum-change path for (orientation × overflow × writing-mode) decomposition

Current vocabulary vs target:

- Today config has the frozen key `layout_type` (5 values), but the renderer surface sees only `orientation(3) + scroll_mode(bool)` projections (§4), and candidate-core geometry sees `Orientation(H/V) + scroll_mode(bool) + scroll_columns(=page_size) + scroll_visible_rows(=6) + scroll_cell_width` (lib.rs:61-121). `VerticalText` has no rendering surface at all; `flow` renderer = single non-wrapping row (ui_main + lib.rs non-scroll horizontal) while two "future" helpers (`flow_paged_bounds`, `vertical_text_columns`) already encode rabbit wrap/vertical semantics but are unreachable from the live path.
- Target (080-layout-naming-design §1, §2): renderer understands only rects; three orthogonal axes:
  - **orientation** (reading direction): Horizontal / Vertical (already `Orientation`, lib.rs:61).
  - **overflow**: Paging (page = hard break) / Scrolling (fixed grid + scrollbar) / Wrapping (rows break on width) — today: `scroll_mode` bool + `page_size` grid, one-axis page window, no wrap in live path.
  - **writing mode**: Horizontal / VerticalRl / VerticalLr (glyph run direction; today nothing — DWrite formats in ui_main are horizontal only).

Suggested split (names illustrative, aligned to existing code so the diff stays small):

- `stacked` ⇒ orientation Vertical + overflow Paging; `flow` ⇒ orientation Horizontal + overflow Wrapping/Paging; `scroll` ⇒ overflow Scrolling (grid shape from orientation × page_size × row/col budget); `vertical_text` ⇒ writing mode Vertical (rl/lr), orientation Vertical; `automatic` ⇒ orientation auto-resolve (existing `resolve_automatic_orientation`), overflow Paging.

### (a) Rust candidate-core `lib.rs` — geometry + presentation (must change)

1. Types near `lib.rs:61-98`:
   - Keep `Orientation {Vertical, Horizontal}` (:61-64) = screen/reading axis.
   - Add `WritingMode { Horizontal, VerticalRl, VerticalLr }` and `Overflow { Paging, Scrolling, Wrapping }`.
   - Replace in `LayoutInput` (:81-98): `scroll_mode: bool` + `scroll_visible_rows` with `overflow: Overflow` (+ optional `scroll_visible_rows` kept as scroll-only param); add `writing_mode`. Keep `scroll_columns`, `scroll_cell_width`, `selected`, and all paddings/gaps/placement; grid shape continues to be derived from `scroll_columns` (page_size) × visible rows, exactly as today (§3), so scroll visuals do not regress.
2. `layout()` (:2389-2706): re-key its three existing branches to `overflow`; the scroll branch keeps its code path unchanged (guarded by `Overflow::Scrolling`); non-scroll stays Paging; add a thin routing to `flow_paged_bounds` (:2715-2759, needs page-size-consistent row rule, see risks) for Wrapping and to `vertical_text_columns` (:2771-2818, add direction param from `WritingMode`) for VerticalRl/Lr.
3. Decoder: `layout_type`-string → (orientation, overflow, writing) mapping function next to `resolve_automatic_orientation` (:3373-3399), pure, unit-tested against the frozen mapping table in 080-layout-naming-design §1.
4. `CandidatePresentationState` (:2177-2324): `scroll_mode` remains a derived *render* flag, but base it on `overflow==Scrolling` instead of `configured_scroll_mode`; `CompositionLayoutState` (:2327-2387) stays (auto orientation + stable width). No page-owner semantics change.

### (b) ui_plan.rs / candidate_abi.rs — single rect-only renderer (must change, small)

- `CandidateUiConfig` (ui_plan.rs:94-110): replace `orientation: PresentationOrientation` + `scroll_mode: bool` with the decoded `layout` (or carry `layout_type` + decode once). Add `writing_mode` to what `render_plan` resolves.
- `CandidateUiPlan` (:217-230) / `CandidateRenderItem` (:187-198): keep rects exactly as today (`item_rect/label_rect/text_rect/comment_rect`), add **visibility** per item and a writing-mode token on the plan. Do **not** pass overflow/orientation/layout knowledge into the paint step — the renderer (ui_plan consumer) only iterates rects × visibility, picks text format by writing mode, draws. `item_size` (:524-549), `visible_indices` (:513-522), `render_item` (:551-610) already carry the full-glyph budgets; only the vertical/flow run placement needs writing-mode-aware split (rabbit rules: vertical label top-centered, comment bottom-anchored; label-slot machinery at lib.rs:864-980 reused for scroll row labels).
- `candidate_abi.rs` FFI structs (:16-111) then expose the same decomposed plan; this is the seam the future Rust candidate window consumes.

### (c) Config — compatible, minimal

- **Config keys stay frozen**: `layout_type` + `page_size` + `scroll_cell_width_dip` remain the persisted surface; legacy `orientation`/`scroll_mode` read-migration and `orientation()`/`scroll_mode()` projections (:151-171) can remain during transition (naming doc accepts them as legacy projections) but must be deleted together with the C++ renderer cutover.
- Add `candidate_layout_type` to the native ABI `Fcitx5ConfigSnapshot` (`config_snapshot_abi.rs:44-60`, fill ~:149-153) so C++ can stop re-projecting; **required for `vertical_text`** which today collapses to `automatic` in the ABI.
- Keep defaults/theme untouched (`resources/config.toml`, theme.toml).

### C++ `src/ui/ui_main.cpp` (must change only until Rust renderer cutover, not after)

- The only durable C++ renderer surface left by the 080 plan is D2D/DWrite drawing of rects; keep `paintOnce()` (:1769-1991) shape, window plumbing, wndproc, mouse hit-test/select, model+presentation FFI glue.
- Replace the mixed orientation+scroll policy code in `update()` (auto-horizontal fallback :2297-2312, scroll label reservations `applyScrollLabelReservations` :1732-1760, grid divider paint :1899-1928, scroll label column :2252-2265, hardcoded `6U` :2346) with: read the full `layout_type` (via the new ABI field), decode to the three axes, feed `ui::LayoutInput` and segment input accordingly; add vertical-text `IDWriteTextFormat`s (reading/flow TOP_TO_BOTTOM) for `VerticalRl/Lr`; delete the `SetDpi(96,96)` hardcodes (:2621, :2723) per HiDPI contract.
- Or (smaller for cutover): keep C++ consuming ui_plan's `CandidateUiPlan` through `candidate_abi` and shrink `update()` to "measure → plan → paint plan".

### Unchanged (can stay)

- IPC/protocol layer (`KeyResponse` wire, `presentation`/`engine` pipes, candidate-select client), engine paging semantics (§6), launcher process management.
- config-poc five-way control `CandidateLayoutMode` ↔ `layout_type` mapping (main.rs:1176-1234) and adapter (`WindUiConfigAdapter`, :2997-3057) — already the frozen 5 values; only the **preview panel** (main.rs:647-838) must become decomposition-aware later, and a `vertical_text` LTR/RTL knob + scroll grid/scrollbar still missing.
- Label-slot engine, `qingfeng.rs` prototype, `candidate_poc` evidence harness — reusable fixtures for geometry golden tests.

---

## Preview-UI summary

| UI | Where | Real render or schematic |
|---|---|---|
| Settings appearance page embedded candidate preview (rows/chips) | `rust/config-poc/src/main.rs:647-838` (`windui_candidate_preview_panel` etc.) | **Schematic**: hand-built windui rows/chips; Stacked/Scroll/VerticalText share the identical row list, Flow is one chip row; driven by `layout_mode` + `page_size` only; geometry never comes from candidate-core. Pixels are real (windui rasterizer draws the settings shell) but layout is mocked. |
| QA "candidate-preview-surface" + `CandidatePreviewHostEvidence` | config-poc main.rs:2935-2960, :4798-4918 | Contract evidence around the embedded surface (placement/DPI/renderer-contract flags); asserts the desired *production-renderer* path, not a separate renderer. |
| Rust candidate-core POC window smoke / goldens / self-check | `rust/candidate-core/src/bin/candidate_poc.rs` (SkiaCanvas + DWriteEngine, golden :2418+, snapshots :3331+); `run_candidate_poc_self_check` lib.rs:2876-2882 | **Real render** (test harness only): paints qingfeng/ui_plan-based candidate surfaces on real HWNDs, DPI 0.5-4.0, label-slot and layout snapshots. |
| ui_plan `CandidateUiPlan` | ui_plan.rs:217-230 | Renderer-neutral **plan**, no pixels, no consumer yet. |

---

## Open design points / risks found in survey

1. **Flow semantics conflict.** 080-layout-research §2 + naming doc: rabbit "flow" = page boundary = hard row break, one page per row (`row_count=ceil(n/page_size)`, `column_count=min(page_size,n)`); but `flow_paged_bounds` (lib.rs:2715-2759) wraps on a **width budget**, and research §"Implications" item 2 explicitly says *do not* implement a glyph-width reflow default. Freeze which rule before wiring `flow`.
2. **Scroll grid defaults.** Current code derives `scroll_columns` from `page_size` (clamped 1..9) and fixes 6 visible rows/cols (ui_main.cpp:2346, ui_plan.rs:401, LayoutInput defaults lib.rs:115-120); research §3 + naming doc §1 want default grid 6×6 / cell 65 with overridable row/col/cell config (`ScrollMode.MaxRowCount/MaxColumnCount/ShowScrollBar`, `Size.ScrollCellWidth`). No config key exists for rows/cols today; page_size does double duty.
3. **`vertical_text` needs a direction setting** (naming doc §1/rabbit `vertical_text_left_to_right`) and the full-glyph/vertical-DWrite render slice — both absent; ABI collapses the type to `automatic` today (config_core.rs:157-171).
4. **HiDPI**: C++ `SetDpi(96,96)` at ui_main.cpp:2621/2723 vs DPI contract (naming doc §3) — golden coverage (100/125/150/200) must run against the future Rust/windui renderer.
5. **Settings preview drift**: five-button UI is already the frozen surface, but its preview is schematic (identical rows for 4/5 modes, no scrollbar/vertical text) — user cannot tell 纵排 vs 卷轴 vs 竖排文字 apart from the caption.
6. **Dead seam**: ui_plan + candidate_abi have no consumer; the C++ `CandidateWindow` (all of `src/ui/ui_main.cpp`) is the renderer until the 080 cutover task replaces it — any early decomposition must keep the FFI projections working or land with the cutover slice.
