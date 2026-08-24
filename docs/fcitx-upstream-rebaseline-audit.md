# Fcitx5 Upstream Rebaseline Audit

Date: 2026-08-24 (audit against real repository HEAD `e993dfc6cbd0d1688ef67f153cb1164a0e144955`)

Status: audit only. No shipping baseline was changed by this document.

## Why this audit exists

`docs/engine-boundary.md` fixes the Engine boundary as "Rust Engine Product Core +
thin C++ Fcitx adapter" and states upstream Fcitx core/addons are consumed as-is
with only a small audited patch queue. Before starting Engine E2/E3 (which touches
`InputContext`, `CandidateList`, `Instance` adapters), the repository must know what
Fcitx5 version it actually builds against, how far that baseline has drifted from
current upstream, and which Windows-local changes still need to be carried.

## Current shipping pin (verified locally)

`tools/bootstrap-fcitx.ps1` pins:

```text
Name   fcitx5
Url    https://github.com/gaboolic/fcitx5.git
Commit 50a3069a2f1bb8647abef713d98ad10d0713b752
```

Verified in the local checkout `out/sources/fcitx5`:

- `50a3069a2f1bb8647abef713d98ad10d0713b752` is a single Windows commit whose
  parent is the official upstream commit `ebf24ddc8a2afe331df2b2f4cbe538f73a4a9b5f`
  (2025-07-04, Fcitx 5.1.14 era).
- The fork is **not** a private semantic fork: relative to its official parent it
  changes only 4 files, +41/-5 lines:

```text
 src/lib/fcitx-utils/eventdispatcher.cpp     | 13 +
 src/lib/fcitx-utils/eventdispatcher.h       | 12 +
 src/lib/fcitx-utils/standardpaths_p_win.cpp | 19 +, 4 -
 test/teststandardpaths_win.cpp              |  2 +, 1 -
```

## What the fork's 41 lines do

1. `EventDispatcher::dispatchPending()` (`eventdispatcher.{h,cpp}`, `@since 5.1.15`):
   runs up to 64 functors already queued by `schedule()` on the calling thread, so
   TSF-style hosts without a running event-loop thread can apply deferred work
   before reading shared state.
2. `standardpaths_p_win.cpp`:
   - maps the Windows install layout to CMake `GNUInstallDirs`
     (`data/` -> `share/`, `data/fcitx5` -> `share/fcitx5`, `data/locale` -> `share/locale`);
   - reads `FCITX_DATA_DIRS` (semicolon-separated) and prepends those directories to
     the `fcitx5` package data search path.
3. `teststandardpaths_win.cpp`: test expectation updated for the `share/fcitx5` layout.

## Current upstream master (verified locally)

Fetched from `https://github.com/fcitx/fcitx5.git`:

```text
master 442edbc9...  (2026-08-20)  project(fcitx VERSION 5.1.22)
```

Relative to the fork's official parent `ebf24ddc` (2025-07-04), master has advanced
about one hundred commits. Key interfaces used by this repository's Engine adapter
grew in that window:

```text
 src/lib/fcitx/candidatelist.h | 123 +++++  (ActionableCandidateList, candidateaction.h, TabbedCandidateList)
 src/lib/fcitx/event.h         |  51 +++
 src/lib/fcitx/inputcontext.h  |   9 +, 3 -
 src/lib/fcitx/inputpanel.h    |  38 +++
 src/lib/fcitx/instance.h      |  37 +++
```

`ActionableCandidateList` / `src/lib/fcitx/candidateaction.h` are the upstream
candidate-action surface the Engine capability model (`CANDIDATE_ACTION` in
`docs/engine-boundary.md`) should target when E2/E3 adapter work happens.

## Are the fork's Windows changes upstream yet? No.

Checked `upstream/master` (442edbc9):

- `EventDispatcher::dispatchPending` — **absent** from `eventdispatcher.h`.
- `FCITX_DATA_DIRS` — **absent** from `standardpaths_p_win.cpp`.
- Windows install layout — master still maps `pkgdatadir` to `basePath / "data/fcitx5"`,
  not `share/fcitx5`.

All three remain Windows-local needs and must be carried through the rebaseline.

## Repository dependencies on the fork changes

- `src/engine/fcitx_runtime.cpp` calls `instance->eventDispatcher().dispatchPending()`
  from `dispatchPendingEvents()`, which is on the Engine hot path
  (`collectResult` and other request handlers). Upgrading to master without this API
  requires an Engine-local replacement (e.g. draining the dispatcher queue through an
  adapter) or an upstream proposal.
- `src/engine/fcitx_runtime.cpp` sets `FCITX_DATA_DIRS` (with `FCITX_ADDON_DIRS`,
  `XDG_DATA_DIRS`, `LIBIME_MODEL_DIRS`) before the Fcitx runtime reads data paths.
- Product staging and Control already assume the `share/fcitx5` layout
  (`tools/stage-package.ps1`, `native-engine/CMakeLists.txt`, `src/control/control_main.cpp`),
  which only works because of the fork's `share/` mapping.

## Patch queue applicability against official master (tested)

`git apply --check` for every `third_party/patches/*.patch` against a clean
`upstream/master` (442edbc9) worktree:

| Patch | Result on master |
|---|---|
| `fcitx5-windows-user-data-root.patch` | **APPLIES-CLEAN** |
| `fcitx5-chinese-addons-msys2-clang-libcxx.patch` | NEEDS-REWORK |
| `fcitx5-lua-windows-lua54.patch` | NEEDS-REWORK |
| `fcitx5-rime-windows-paths.patch` | NEEDS-REWORK |
| `libime-windows-model-dirs.patch` | NEEDS-REWORK |
| `librime-msys2-clang-windows.patch` | NEEDS-REWORK |

The `NEEDS-REWORK` results are expected: those upstream files moved during the last
year. Each must be re-diffed against the new pinned upstream commit during the
rebaseline, not blindly applied.

## Rebaseline plan (proposed, not yet executed)

1. **Freeze current truth first** — the shipping build stays on the current pin
   (`gaboolic/fcitx5 @ 50a3069`) until a compatibility lane proves the new baseline.
2. **Create an upstream compatibility lane** — clone/build lane against official
   `fcitx/fcitx5` master (or a pinned recent tag) in `out/`, not the shipping tree.
3. **Replay the patch queue** on the new lane in this order:
   - `fcitx5-windows-user-data-root.patch` first (already applies clean);
   - then re-diff each of the five `NEEDS-REWORK` patches against the new commit;
   - then carry the three fork changes as explicit Windows-local patches:
     `dispatchPending`, `share/` layout, `FCITX_DATA_DIRS`.
4. **Decide upstream candidates** — `dispatchPending` (TSF no-event-loop drain) and
   the GNUInstallDirs `share/` layout are plausible upstream contributions; propose
   them to `fcitx/fcitx5` before depending on them privately.
5. **Only then run Engine E2/E3** — write `InputContext`/`CandidateList`/candidate-action
   adapter code against the new upstream API (including `ActionableCandidateList`)
   instead of the 2025-07-04-era API.
6. **Record every carried patch** in `docs/engine-boundary.md`'s patch inventory with
   owner, upstream target, reason, and removal/upstreaming condition.

## Risks and decisions

- This audit does **not** change the shipping baseline. A full Fcitx5 build against
  the new upstream is a separate, heavier task requiring the MSYS2 CLANG64 lane and
  real-host verification.
- The fork is safe to keep as the shipping pin while the lane is prepared; it is a
  41-line integration commit, not a semantic fork.
- Do not start E2/E3 adapter work against the old API shape if the rebaseline is
  expected soon; keep E1 (shared `protocol-core`, Fcitx-independent) on the critical
  path instead.
