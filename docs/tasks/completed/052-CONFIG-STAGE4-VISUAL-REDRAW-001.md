# Task — CONFIG-STAGE4-VISUAL-REDRAW-001

**Mode:** CONFIG-RUST-CUTOVER / STAGE4-GUI-QA
**State:** COMPLETED / VISUAL-REDRAW-QA-GREEN
**Task ID:** `052-CONFIG-STAGE4-VISUAL-REDRAW-001`
**Prerequisite:** `048` Stage 2 Rust Config Backend Shipped green; `REL-01` remains parked on external evidence.

## Goal

Fix the real Rust Settings window visual redraw/ghosting regression and strengthen the Stage 4 GUI
QA evidence for in-window candidate preview placement.

This is not the full `Rust Config Cutover Complete` gate. It is the next focused Stage 4 slice for
the user-reported defect: the Settings window looks visually rough, still reads as an old VC6-era
property sheet, and shows ghosting/重影.

## Specification references

- `docs/spec-v1.8.md` §5.5.8 Config UI/UX
- `docs/spec-v1.8.md` §5.5.10 Appearance page and Live Preview
- `docs/spec-v1.8.md` §5.6.6 DPI, cache, and runtime cost
- `docs/tasks/PLAN.md` Config migration stage definitions
- `docs/tasks/settings-uiux-operation-integration-plan.md`
- WindInput reference refresh:
  - `huanfeng/WindInput` `2214bede43b4153f0fdc463928cf3c50184ec2ef`
  - `huanfeng/wind-ui-rust` `62241e25e762df154c1b1f855b4db57533e516fc`
  - `wind-setting` source was not publicly available, so do not claim source-level Settings
    implementation parity.

## Required behavior / implementation contract

- The normal Rust `fcitx5-config.exe` window must launch as a real top-level Settings window, not a
  mock.
- The visual language must move toward the v1.8 modern native Settings surface: navigation, cards,
  spacing, section hierarchy, readable typography, and focused native controls. A plain
  label/control grid with a white card wrapper is not acceptable as the target look.
- Apply the useful WindInput/wind-ui-rust lessons without importing its input architecture:
  `measure -> arrange -> paint`, logical DIP coordinates, theme/design tokens, card/setting-row/list
  composition, explicit dirty repaint, and no stale pixels after runtime page/theme/control changes.
- Navigation, resize, and visible control changes must not leave stale pixels or 重影 in the client
  area.
- Parent painting must clear the full intended background and must not draw over child HWND content.
- Child candidate-preview painting must clear its own surface before drawing each frame.
- Visible controls, labels, and user-entered text must not overlap or clip at supported minimum
  sizes and tested DPI scales.
- The candidate preview must remain embedded inside the Config content area. It must not render into
  the desktop, an external popup, or a static screenshot path.
- Do not migrate unrelated Config features in this slice and do not reintroduce C++ product logic.

## Evidence notes

- Legacy comparison was captured from
  `out/build/windows-x64-dev/Debug/fcitx5-config-legacy.exe` as
  `out/build/windows-x64-dev/legacy-config-capture.png`. The legacy shell already has a useful
  modern Settings shape: left navigation, large page title, content cards, and localized text. The
  Rust Settings preview must not regress to raw VC6-style button grids.
- The observed red-box defect was reproduced in the Rust screenshot as stale text from the previous
  page over the Appearance page. The verified root cause class was transparent STATIC child painting
  combined with page visibility changes and parent repaint/capture. The fix makes parent surfaces
  repaint explicitly, returns stable opaque background brushes for STATIC controls, keeps the child
  candidate preview responsible for clearing its own surface, and adds a pixel-level ghosting guard.
- No dedicated installed Win32/Rust modern UIUX skill was available in this session. The design
  direction is therefore anchored in the project UIUX plan plus Microsoft Windows desktop
  modernization guidance: full modern Windows UI should eventually be a richer Rust-owned UI layer
  or WinUI-style frontend when compatibility gates allow it; this slice keeps the Win7-compatible
  Win32 HWND adapter but moves visual ownership toward Rust design tokens, layout contracts, and
  owner-drawn navigation.

## Required validation

- Inspect the current Rust Config window implementation before changing it.
- Add or update automated QA that exercises repaint-sensitive transitions:
  - page navigation;
  - resize/minimum-size relayout;
  - appearance control changes that invalidate preview;
  - candidate preview paint count / in-window bounds.
- Run affected Rust Config QA and CMake source/visual contract tests for x64 and x86 where practical.
- If real screenshot automation is available in the repo, capture before/after evidence from the
  actual executable under `out` or package stage. Otherwise record the exact missing manual visual
  evidence.

## Done when

- The user-reported ghosting/重影 class has a concrete implementation fix or a narrower verified
  root-cause note.
- Automated redraw/non-overlap/embedded-preview tests pass.
- `docs/tasks/status.md` records files changed, HEAD, tests, and any manual visual evidence still
  needed.
