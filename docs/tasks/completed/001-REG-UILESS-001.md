# Current Task — REG-UILESS-001 PresentationPolicy

**Mode:** CHANGE  
**Priority:** first unfinished v1.8 Stabilization Gate item  
**Task ID:** `REG-UILESS-001`

## Goal

Make the host's TSF UILess presentation decision apply to the independent candidate popup **for the same input context**, without losing TSF `ITfUIElement` candidate/accessibility semantics.

When the host causes `BeginUIElement(..., show=false)`, `fcitx5-ui.exe` must not show its own candidate popup for that context. Candidate data must still continue to update through the TSF UIElement path so the host and assistive technology can consume it.

## Specification references

Read only the directly relevant parts of `docs/spec-v1.8.md` first:

- §0.5 Stabilization Gate item 1: `UILess / PresentationPolicy`
- §5.4: `UILess、无障碍与独立 UI 进程`
- Phase 4 acceptance for CandidateModel + independent UI + UILess
- regression table entry `REG-UILESS-001`

Read additional sections only if current code proves they are required to complete this task.

## Known audit observation — verify against current HEAD

The reproducible audit baseline observed that the TSF-side `show=false` decision affected only the in-process UIElement state, while the independent UI process consumed presentation snapshots separately and could still show the popup.

This is **evidence to investigate, not permission to assume the code is unchanged**. Trace the current HEAD before editing.

## Required behavior

1. Normal host behavior remains unchanged: when popup presentation is allowed, candidate popup behavior works as before.
2. If the host's TSF UIElement decision for a context is `show=false`, the independent candidate popup is suppressed for that context.
3. Suppressing the popup does **not** suppress candidate model/UIElement updates. Candidate count, selection, text/change semantics, and accessibility-facing state continue to update.
4. Presentation policy is context-scoped. A UILess decision for context A must not incorrectly suppress a normal context B.
5. Focus/context transitions, composition end, engine epoch changes, reconnects, and context destruction must not leave stale presentation policy attached to another context.
6. A UI-process crash/restart must not block composition or commit. Presentation behavior after reconnect must be derived from authoritative current context state, not a stale global boolean.
7. The change must not add disk I/O, process launch, or unbounded synchronous work to the TSF key hot path.
8. If the cross-process wire contract must change, make it a current-version breaking contract update across all in-repo producer/consumer/fixtures in the same change. Do not add an old-protocol compatibility decoder.

## Preferred model

Treat popup permission as context presentation state, for example conceptually:

```text
PresentationPolicy
  context identity
  popup_allowed / presentation_mode
```

The exact representation should follow the current code and existing protocol patterns. Do not introduce a new framework or generic policy system for this one requirement.

The popup renderer consumes presentation policy; the TSF UIElement/accessibility path continues to consume candidate semantics. Do not conflate "do not draw our popup" with "candidate list does not exist".

## In scope

Only files directly required to propagate, store, consume, reset, and test the UILess presentation decision, typically within the current equivalents of:

- TSF UIElement / TextService presentation handling;
- existing IPC/protocol model if cross-process propagation is required;
- Engine/presentation snapshot plumbing if it is the authoritative bridge;
- independent `fcitx5-ui.exe` popup visibility decision;
- Candidate/UIElement tests and protocol fixtures directly affected by the change.

Use the actual current paths from HEAD; do not create parallel implementations just to match this list.

## Out of scope

Do **not** use this task to implement or redesign:

- the single Windows TSF profile work;
- the general KeyEvent contract;
- Fcitx surrounding-text/delete/forward-key capabilities;
- Candidate A→B→A ordering fixes unless current UILess correctness directly depends on the same minimal change;
- Config UI/UX or candidate visual styling;
- Rust R1/R2 migration;
- package/update/installer/generation-draining work;
- Win7 support expansion;
- unrelated cleanup, renaming, formatting, abstraction, or dependency changes.

If an out-of-scope defect blocks `REG-UILESS-001`, report the blocker with evidence instead of silently turning this task into the next project phase.

## Required tests

Add or update deterministic regression coverage for at least:

### `REG-UILESS-001`

Given a context with candidates and a TSF `BeginUIElement` result of `show=false`:

- independent popup stays hidden;
- UIElement candidate state continues updating;
- selected candidate/change state remains correct.

Also cover:

- `show=true` normal popup path;
- context A `show=false` → context B normal → A again, proving no global leakage;
- context/composition end or destruction clears the correct policy state;
- reconnect/restart does not resurrect stale popup permission if the existing architecture exposes this deterministically.

Prefer deterministic state/IPC tests. Do not rely on arbitrary `Sleep()` timing.

## Validation

Run the smallest relevant build/test set first. Because this crosses TSF/presentation/IPC boundaries, include relevant x64 and x86 compile/test coverage if those targets are affected.

Do not run unrelated full release, package, Rust, Win7 VM, or installer matrices unless a directly affected gate requires them.

## Done when

The task is complete only when:

- `REG-UILESS-001` passes;
- normal popup behavior is preserved;
- candidate/UIElement semantics remain available while popup is suppressed;
- context isolation is covered by regression tests;
- any changed protocol contract is updated end-to-end with no compatibility shim;
- affected builds/tests pass;
- no unrelated feature/refactor was included.

Then stop. Do not automatically start Stabilization Gate item 2.
