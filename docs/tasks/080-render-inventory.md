# 080 Stage 1 — C++ shipping candidate renderer inventory + golden corpus

**Task 080 stage:** freeze corpus + inventory the C++ render path (read-only; no product code changed).
**Artifacts:** goldens + manifest under `out/build/windows-x64-dev/080-golden/` (gitignored `out/`).

## 1. Render path inventory — `src/ui/ui_main.cpp` (2993 lines)

The candidate window lives in class `CandidateWindow` (starts line ~1208). Product semantics
(model/layout/presentation/selection) are Rust-owned (`fcitx5-candidate-core` + the
`fcitx5_candidate_presentation_*` C ABI). The C++ file is the **window/D2D/DWrite adapter**.

| Function | Lines | Role |
|---|---|---|
| `CandidateWindow::create` | 1221-1283 | HWND: `RegisterClassW`/`CreateWindowExW` (`WS_EX_TOOLWINDOW\|NOACTIVATE\|TOPMOST`, layered unless interactionTest), `SetTimer`, `enableNativeWindowEffects`, layered opacity, `createDeviceResources`; builds the opaque `candidate_select_client` for engine selection IPC |
| `CandidateWindow::run` | 1284-1293 | Win32 message loop |
| `showSyntheticPreview` | 1297-1354 | Builds a synthetic `KeyResponse` (3-candidate vertical, or 60-candidate scroll bulk 6/page), `update()` + `RedrawWindow` + `paintOnce` + `paintTestSurfaceOverlay`. This is the offscreen-usable driver behind `--demo`/`--scroll-demo` |
| `refreshVisualConfig` / `applyContentLocale` / `applyScrollLabelReservations` / `reloadVisualConfig` | 1698-1768 | Re-apply config/locale/label-reservation changes |
| **`paintOnce`** | **1769-1992** | **Main D2D paint.** High-contrast probe (`SPI_GETHIGHCONTRAST` + `GetSysColor`) overrides `visualConfig_.colors`; creates 9 `ID2D1SolidColorBrush`; `Clear(background)`; preedit `DrawTextW` + divider line; scroll-mode cell separator lines; per visible candidate: selected `FillRoundedRectangle` (inflated, `cornerRadiusDip`) then label/text/comment `DrawTextW` (clip, no wrap) from Rust `renderSegments`; `hasScrollbar_` track+thumb; outer `DrawRoundedRectangle` border; `EndDraw` with `D2DERR_RECREATE_TARGET` recovery |
| `paintToDeviceContext` | 1993-2077 | GDI fallback paint (used by the interaction-test overlay path) |
| `paintTestSurfaceOverlay` | 2078-2087 | GDI overlay when `interactionTest_` (demo/self-test) |
| `update` | 2088-2411 | `KeyResponse` → `candidate::CandidateModel.apply` → `fcitx5_candidate_presentation_apply` → `render_plan`; UTF-8→UTF-16 into `CandidateVisual` rows; measure via `IDWriteTextLayout::GetMetrics`; caret/monitor/work-area-driven `fcitx5_candidate_presentation_resolve_orientation`; builds `itemRects_` + placement |
| `reflowCurrentModel` | 2412-2451 | Model → presentation reset → `update` |
| `windowProcedure` | 2563-2666 | WM_PAINT→`paintOnce`, mouse hit-test/dispatch (`dispatchCandidate`), dismiss, config-reload, placement messages |
| `createDeviceResources` | 2667-2730 | `D2D1CreateFactory` + `DWriteCreateFactory` + three `IDWriteTextFormat`s (candidate/label scaled/annotation, `NO_WRAP` + character-trimming ellipsis) + `CreateHwndRenderTarget` + **`renderTarget_->SetDpi(96,96)`** |

Non-member render helpers: `loadVisualConfig` (1132-1207) pulls `NativeRenderConfig` through
`fcitx5_config_snapshot_load_visual_utf16` (Rust config-core: orientation/preeditMode/labelStyle/
scrollMode/labelVisible, paddings/gaps/border/corner radii/font sizes/scales/weight/families, and the
color table incl. theme layering); `systemUsesDarkAppearance` (990, delegates to Rust
`fcitx5_windows_common_system_uses_dark_appearance`, reads `AppsUseLightTheme`); `nativeColor`
(1063) parses the resolved colors; Rust `renderSegments` maps layout → label/text/comment rects.

**Render inputs that drive each pixel:** `visualConfig_` (colors + geometry + fonts from the Rust
config snapshot incl. the system-dark flag), the Rust `CandidateModel` snapshot + presentation
`render_plan` (`itemRects_`, `visibleIndices_`, `renderIndices_`, scroll state), `fontDpiScale_`
(caret dpi / 96), and the OS high-contrast/syscolor state (when active).

## 2. Frozen goldens (captured offscreen, dev x64 Debug)

Method: `fcitx5-ui.exe <flag>` shows its synthetic window (demo/interactionTest mode makes it
non-layered), then a PowerShell `PrintWindow(PW_RENDERFULLCONTENT)` helper captures the HWND class
`Fcitx5WindowsNext.Stable.Candidate` to BMP. Helper scripts live next to the goldens
(`capture-demo.ps1`, `capture-dark.ps1`).

| File | State | Theme |
|---|---|---|
| `cpp-golden.bmp` | vertical, 3 candidates, candidate 1 selected | light |
| `cpp-golden-dark.bmp` | vertical, 3 candidates, candidate 1 selected | dark |
| `cpp-golden-scroll.bmp` | scroll mode, 60 candidates, 6/page bulk, candidate 18 selected | light |
| `cpp-golden-scroll-dark.bmp` | scroll mode, 60 candidates, 6/page bulk, candidate 18 selected | dark |

Pixel check: light = white bg `(255,255,255)`, dark = `(24,24,24)` bg; both carry the WeChat-green
selected rect `(6,193,96)` + text pixels. Manifest: `080-golden/manifest.json` (exact commands,
sizes, render-input note for each).

## 3. States NOT reachable offscreen from the C++ CLI

- **horizontal / grid**: orientation is decided by Rust
  `fcitx5_candidate_presentation_resolve_orientation` from the real monitor work-area + caret +
  content. The demo caret is fixed at (100,100) in a narrow window → always vertical. Needs a real
  wide host/caret or a config-forced horizontal layout on a wide work area.
- **high-contrast**: `paintOnce` reads `SPI_GETHIGHCONTRAST` + `GetSysColor`; would require enabling
  the OS high-contrast theme (intrusive, not done).
- **150%/200% DPI**: `createDeviceResources` forces `SetDpi(96,96)` and the demo caret dpi is 96;
  `fcitx5-ui.exe` has no `--dpi` flag. Needs a real per-monitor-DPI host.

## 4. Gap vs the Rust renderer (`rust/candidate-core/src/bin/candidate_poc.rs`)

`fcitx5-candidate-poc` (tiny-skia + vendored `windui` DWrite) already emits 15 BMPs:
demo/scroll-demo/typography/host(mock-chromium|emoji-host|notepad|word)/label-slot
(vertical|horizontal|grid × light|dark)/dpi-smoke/window-smoke, plus `--dpi-scale`.

So the Rust side already covers layouts the C++ CLI cannot reach offscreen. The Stage 2/3
differential corpus that is directly comparable today is **vertical + scroll × light + dark**
(4 goldens here vs the candidate-poc demo/scroll snapshots). Horizontal/grid/high-contrast/DPI
C++-vs-Rust comparison requires the real-host capture path above (documented, not faked).

## 5. Stage 2 scope (from this gap)

Extend the candidate-core Rust renderer to draw the full C++ window vocabulary for the 4 frozen
states first: preedit panel + divider, per-row label/text/comment with no-wrap clip + ellipsis
trimming, selected inflated rounded rect with `cornerRadiusDip`, scroll cell separators, scrollbar
track/thumb, outer rounded border, WeChat-green palette + dark palette, and the exact
vertical/scroll layout from the same Rust presentation render plan — then diff byte/tolerance-wise
against these 4 goldens. High-contrast/horizontal/grid/DPI follow with the real-host captures.
