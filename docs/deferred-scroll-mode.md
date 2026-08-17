# Deferred candidate scroll mode

Status: deferred until the Phase 0–8 baseline and Dogfood gates are complete.

The requested scroll mode is the expanded multi-row candidate browser shown by
`fcitx5-macos` commit `59d1ae8cc976573fc5dc0a86043b1cf2f98d291c`. It is not a third
renderer orientation and must not be represented as `candidate.orientation = "scroll"`.

## Reference behavior to preserve

- Keep explicit `none`, `ready`, and `scrolling` states with expand/collapse transitions.
- Enable expansion only when Fcitx exposes a `BulkCandidateList`; the engine remains the owner of
  selection, highlight, candidate order, and commit.
- Fetch bounded windows rather than copying an unbounded list: the reference initially requests
  42 candidates (six visible rows plus one hidden row, six columns), then prefetches batches of 36
  near the viewport end.
- Use `BulkCandidateList::candidateFromAll()` for global items and
  `BulkCursorCandidateList::setGlobalCursorIndex()` for highlight intent.
- Support up/down/left/right, row start/end, page up/down, commit, and per-row numeric selection.
  Only the highlighted row receives labels 1–6.
- Keep the highlighted cell visible; lazy loading and visual scrolling must not block the input
  channel. UI callbacks are intents sent to the engine, never local commit decisions.
- Collapse back to the ordinary horizontal candidate row without losing the authoritative Fcitx
  context.

## Windows design implications

The feature needs a separate bounded presentation/control intent contract, bulk-candidate cache
owned by the engine, keyboard and pointer accessibility semantics, D2D grid/scrollbar rendering,
and fault-injection tests for UI crash, stale fetches, unknown total size, and end-of-list races.
It will be implemented as a complete vertical slice after the current release baseline, not as a
theme-only visual approximation.

Primary references:

- <https://github.com/fcitx/fcitx5-macos/commit/59d1ae8cc976573fc5dc0a86043b1cf2f98d291c>
- <https://github.com/fcitx-contrib/fcitx5-webview/blob/a8aa4d5d43d2df36c4a8fe40b02b6f54b4e8298a/page/scroll.ts>
- <https://github.com/fcitx/fcitx5-macos/blob/59d1ae8cc976573fc5dc0a86043b1cf2f98d291c/webpanel/webpanel.cpp>
