# Candidate scroll mode

Status: implemented in the Phase 7 candidate vertical slice.

The requested scroll mode is the expanded multi-row candidate browser shown by
`fcitx5-macos` commit `59d1ae8cc976573fc5dc0a86043b1cf2f98d291c`. It is not a third
renderer orientation and must not be represented as `candidate.orientation = "scroll"`.

## Reference behavior to preserve

- Keep explicit `none`, `ready`, and `scrolling` states with expand/collapse transitions.
- Start with the ordinary current page, expand after the user crosses a page boundary, and collapse
  when navigating from page 1 back to page 0. This matches Rabbit's August 2026 flow viewport and
  avoids opening a large grid for every short composition.
- Enable expansion only when Fcitx exposes a `BulkCandidateList`; the engine remains the owner of
  selection, highlight, candidate order, and commit.
- Keep the transport bounded at 128 candidates and the visible D2D viewport bounded at six rows
  by six columns. Known-size `BulkCandidateList` values are copied only up to that protocol cap;
  the scrollbar communicates the visible portion without creating an unbounded UI tree.
- Use `BulkCandidateList::candidateFromAll()` for global items and
  `BulkCursorCandidateList::setGlobalCursorIndex()` for highlight intent.
- Support up/down/left/right, row start/end, page up/down, commit, and per-row numeric selection.
  Only the highlighted row receives labels 1–6.
- Keep the highlighted cell visible; lazy loading and visual scrolling must not block the input
  channel. UI callbacks are intents sent to the engine, never local commit decisions.
- Collapse back to the ordinary horizontal candidate row without losing the authoritative Fcitx
  context.

## Windows design implications

Protocol v6 carries page size, bulk availability, end-of-list state, and semantic Windows modifier
flags. The engine obtains global
items through `candidateFromAll()` and keeps selection authoritative; the UI derives only a bounded
viewport and row-local labels. Ordinary Fcitx key processing remains the sole commit path. The
production D2D renderer supplies the grid, selected-row labels, automatic selected-row reveal, and
scrollbar; no WebView or theme-only approximation is involved.

Primary references:

- <https://github.com/fcitx/fcitx5-macos/commit/59d1ae8cc976573fc5dc0a86043b1cf2f98d291c>
- <https://github.com/fcitx-contrib/fcitx5-webview/blob/a8aa4d5d43d2df36c4a8fe40b02b6f54b4e8298a/page/scroll.ts>
- <https://github.com/fcitx/fcitx5-macos/blob/59d1ae8cc976573fc5dc0a86043b1cf2f98d291c/webpanel/webpanel.cpp>
- <https://github.com/rimeinn/rabbit/commit/f72f95a58092e59c430823088e03df41db9b492e>
- <https://github.com/rimeinn/rabbit/commit/91243f1b79f8e72c557dc2623fe51648266716ca>

Rabbit independently confirms three useful policies: expand only after a page transition, preload a
bounded page window around the active page, and leave labels/highlight authoritative only on the
current page. Its animation and AutoHotkey rendering are not copied; Windows uses the existing C++
D2D/DWrite renderer and Fcitx-owned key path.
