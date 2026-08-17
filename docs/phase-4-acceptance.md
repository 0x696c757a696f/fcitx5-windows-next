# Phase 4 implementation and acceptance record

Date: 2026-08-17
Status: automated acceptance passed; external host/DPI matrix pending

## Delivered contract

- Protocol v4 carries an immutable engine-owned candidate snapshot: identity/revision, labels,
  text, comments, selection, page, total, explicit visibility and the last valid TSF caret rect.
  Empty preedit does not hide a prediction snapshot. Caret coordinates come from
  `ITfContextView::GetTextExt`; `TS_E_NOLAYOUT` preserves the last valid rect instead of moving the
  popup to an unrelated mouse position.
- `CandidateModel` rejects invalid, stale and conflicting snapshots. Layout and paint remain
  separate from selection/commit semantics.
- `fcitx5-ui.exe` is an independent Win32 + Direct2D + DirectWrite process. Its presentation pipe
  is same-user/session, protected by the Phase 2 DACL and exact sibling engine/UI path checks.
  Engine publication is a coalescing background queue with 25 ms bounded overlapped writes; UI
  absence or failure cannot block key processing.
- The popup uses `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW`, does not enter Alt+Tab, paints only on
  invalidation, recreates its D2D target on device loss, uses per-monitor-v2 when available with a
  Win7-safe capability fallback, handles `WM_DPICHANGED`, clamps to the monitor work area, and
  honors High Contrast system colors.
- TSF also exposes `ITfCandidateListUIElement` semantics for UILess hosts and accessibility clients.
- `config.toml` and `theme.toml` use pinned toml++ 3.4.0 syntax parsing plus project-owned strict
  typed validation. Unknown/duplicate/wrong-type/out-of-range fields fail; page size and key
  semantics are not accepted as appearance settings.
- The renderer consumes the typed config rather than parsing an independent schema. Its fixed
  precedence is built-in defaults → theme common → active light/dark branch → user override;
  High Contrast remains the final runtime override. Font family/size/weight, semantic colors,
  opacity, label style and layout are applied by the DWrite/D2D renderer.
- The checked-in default TOML files contain clear Chinese comments for purpose, units, ranges,
  enum choices, inheritance and ownership. Canonical rewriting may regenerate comments instead of
  preserving their original positions.

## Automated evidence

| Result | Evidence | Status |
|---|---|---|
| Candidate merge/reset/revision | model contract with duplicate/stale/invalid/prediction/context-switch cases | Pass |
| Theme merge/reset | common → branch → user golden test; removing override restores inheritance | Pass |
| Vertical/horizontal and stable placement | pure layout contract at 125/150/200%, negative monitor coordinates and work-area clamp | Pass |
| D2D/DWrite creation and device path | hidden UI self-test forces target discard and recreation | Pass |
| Independent presentation | authenticated real engine snapshot causes `ui.exe --test-once` to exit | Pass |
| UI failure isolation | real engine integration passes with UI absent and with UI exiting after one frame | Pass |
| UILess semantics | count/selection/string/page/update flag COM contract | Pass |
| Strict TOML | annotated config/theme golden cases plus duplicate/unknown/type/range/path invalid cases | Pass |
| Candidate hot model | x64 Release baseline: 0.767 us per 10-candidate immutable apply | Pass |
| Candidate model → paint | x64 Release immediate-present D2D/DWrite baseline: 0.814 ms per 9-candidate frame | Pass |
| Hidden UI idle | 60 s: 0 ms CPU delta; private bytes 9,952 → 9,932 KiB | Pass |
| Dual architecture | Debug `/analyze` x64 and x86: 18/18 CTest plus policy gates | Pass |

## Pending environmental evidence

This machine has Notepad but not Word, Chrome or VS Code, and no physical 125/150/200/300% or
multi-monitor harness. Pure layout coverage is present, but the real host/display matrix rows are
not claimed as passed. They must be recorded during the Dogfood/Phase 5 environment rotation
before a public compatibility claim.
