# Task 072 - Candidate Win32 renderer Rust cutover

**Mode:** CHANGE / CODE-ONLY / MIGRATION
**Task ID:** `CANDIDATE-WIN32-RENDERER-RUST-CUTOVER-001`
**Prerequisite:** `071` completed with the C++ test ownership ledger green; `061`, `062`, and
`066` visual/semantic contracts remain green.
**Evidence class:** Rust renderer/window-host migration with x64/x86 automated evidence; real
Windows host, assistive technology, and release evidence remain manual.

## Goal

Move Candidate presentation state and the product-owned parts of `src/ui/ui_main.cpp` to Rust.
Retain C++ only where it is a necessary Win32/D2D/DWrite/COM or ABI adapter, and delete the old
C++ authority after equivalent evidence is complete. This task must not create a permanent
old/new renderer selector or a second Candidate semantic model.

## Required scope

- Freeze the public behavior corpus from `rust/candidate-core` and the current Candidate UI
  integration tests before changing ownership: layout, labels, comments, selection, scrolling,
  command routing, locale fallback, privacy/accessibility projection, and failure behavior.
- Add Rust-owned presentation/window state for update/reflow, selection, scrolling, work-area
  placement, and command/IPC handling currently retained in `src/ui/ui_main.cpp` around the
  CandidateWindow state and serve path. Consume the existing Rust CandidateModel/layout/render
  contracts; do not duplicate their semantics in C++.
- Implement the smallest Rust renderer host needed for equivalent Candidate output, including
  DirectWrite-compatible font fallback, color emoji behavior, theme/high-contrast colors,
  per-monitor DPI, vertical/horizontal/grid layouts, full glyph visibility beside comments, and
  device-loss recovery. Keep native drawing/window calls behind a narrow ABI seam when Rust cannot
  directly own the platform call yet.
- Remove C++ wrappers/DTOs, synthetic self-test logic, and product state that become redundant.
  Retained C++ must be explicitly documented as `KEEP-ADAPTER`; obsolete C++ tests and fixtures
  follow the 071 deletion gate.

## Evidence and tests

- Rust public-behavior tests cover the frozen corpus, invalid input, selection/scroll boundaries,
  stale updates, sensitive contexts, and fail-soft/device-loss recovery.
- Rust visual goldens cover 100/125/150/200/300 DPI, horizontal/vertical/grid and scroll modes,
  CJK/Latin/emoji/fallback fonts, annotations, and proof that every glyph fits its text rect.
- Rust accessibility contracts cover semantic bounds, focus/selection, notifications, and privacy;
  do not count rectangle non-overlap alone as glyph visibility or accessibility evidence.
- Run focused Cargo tests, format, and clippy where reachable for x64 and x86. Run only the
  affected CMake/CTest adapter and final mixed-binary tests, with no arbitrary sleeps.
- Compare memory/latency against the frozen baseline using deterministic fixtures. Record real
  Windows 7/10/11, per-monitor multi-display, Narrator/NVDA, High Contrast, and manual visual
  review as `MANUAL-PENDING` unless actually run.

## Cutover and deletion gate

No shipping cutover until Rust and the retained native adapter pass the corpus, visual/DPI,
accessibility, device-loss, performance, x64/x86, and mixed-binary checks. Then replace the
shipping Candidate path, remove C++ Candidate product state/wrappers/self-tests and tests that
served only that authority, and prove removed sources are absent from CMake/CTest. Keep only the
direct Fcitx adapter, necessary native ABI/renderer seam, and final mixed-binary E2E categories
allowed by `AGENTS.md`.

## Audit basis

The 2026-08-28 C++/header audit was supplied in the task conversation but is not present in the
current Git HEAD at the requested `docs/audits/cpp-header-migration-audit-2026-08-28.md` path.
Its inventory classifies `src/ui/ui_main.cpp` as mixed: wrappers/DTOs, Candidate state,
self-tests, and pipe serving are `MIGRATE`; native window lifecycle and D2D/DWrite paint are
temporary `KEEP-ADAPTER` seams pending this task's evidence. The full repository test inventory
and ownership migration remain task 071's responsibility.

On completion, update `docs/tasks/status.md`, archive this task, and select the next eligible task.
