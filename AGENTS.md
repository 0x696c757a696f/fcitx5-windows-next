# Fcitx5 for Windows Next — Codex Queue Rules

This repository is an authorized open-source Windows input-method project. Work only in this repository and in public documentation/reference code needed by the currently selected task.

## Sources of truth

1. `docs/spec-v1.8.md` — long-term engineering specification.
2. `docs/tasks/PLAN.md` — ordered task queue and gate dependencies.
3. `docs/tasks/current.md` — the one task authorized for implementation right now.
4. `docs/tasks/status.md` — execution evidence and pending external verification.

The full specification is **not** one giant implementation prompt. Read the current task first, then only the specification sections that task references.

Security/compatibility terms in the specification describe defensive constraints, prohibited behavior, or regression requirements. They do not authorize unrelated offensive functionality or work outside the repository.

## Before every task

1. Record:
   - `git rev-parse HEAD`
   - `git status --short`
2. If the work tree contains user changes, preserve them.
3. Read `docs/tasks/current.md`.
4. Read only its referenced sections of `docs/spec-v1.8.md`.
5. Inspect the current implementation/tests before assuming an older audit finding is still present.
6. State the smallest subsystem/file set required.

## Implementation rule

Implement the smallest correct vertical slice that satisfies the current task. Necessary producer/consumer changes, protocol changes, fixtures, and regressions are in scope only when required for correctness.

Do not perform unrelated cleanup, framework migration, dependency churn, naming changes, or future tasks.

Architecture defaults:
- Fcitx-facing Engine stays C++ as the long-term Fcitx5 island.
- Everything else that is owned by this Windows product should move toward Rust when an explicit gated task exists and the relevant C++ behavior corpus is frozen.
- TSF DLL currently stays C++/Win32/COM/TSF and minimal as the shipping stable baseline, but it is not permanently exempt from Rust. Rust TSF work requires an explicitly gated TSF Rust PoC task with panic/COM/host-matrix evidence before cutover.
- Candidate renderer currently stays C++/Win32/D2D/DWrite until UX/layout/UILess contracts are frozen; future Candidate Rust migration should use IPC/differential tests, not C++ FFI.
- Config currently keeps WTL/Win32 hosting plus the product-specific D2D/DWrite settings layer until the Settings operation model is frozen; future Config Rust migration should consume the typed Control/config/package boundaries.
- Rust starts only in tasks explicitly marked `RUST-R1`, `RUST-R2`, or a later explicitly gated Rust PoC/migration task.
- Do not create a permanent old/new protocol dual stack.

## Testing

Run affected tests first. Expand only when the changed boundary requires it.

Use deterministic barriers/fake clocks/fixtures where possible instead of arbitrary sleeps. A reproducible bug fix should leave a regression test.

## Automatic queue advancement

After a task meets all automatable acceptance criteria:

1. Append its HEAD, files changed, tests, and result to `docs/tasks/status.md`.
2. Copy the completed `docs/tasks/current.md` into `docs/tasks/completed/<task-id>.md`.
3. Select the next eligible task in `docs/tasks/PLAN.md`.
4. Copy that task file to `docs/tasks/current.md`.
5. Continue automatically.

Do **not** ask the user merely to advance to the next task.

Stop automatic advancement only when:
- current acceptance requires unavailable real hardware/application/manual evidence;
- a required signing credential/private key or privileged external service is unavailable;
- current HEAD contradicts the task/spec in a way that requires a product decision;
- the next task is gated on an unfinished prerequisite;
- a safety/policy constraint prevents the requested implementation;
- all queued tasks are complete.

For an `EXTERNAL_EVIDENCE` task, perform every reachable automated preparation/check, record exactly what remains manual in `status.md`, mark it `MANUAL-PENDING`, and continue to later code-only tasks **only if PLAN.md says the manual evidence is not a prerequisite for them**. Never mark unrun real-host evidence as passed.

## Rust migration rule

For each R1/R2 component:

C++ semantics fixed
→ contract/golden/fuzz corpus frozen
→ Rust side-by-side
→ differential tests
→ security/artifact smoke
→ performance comparison
→ cutover
→ delete old authoritative implementation

Do not change the semantic contract and migrate language in one opaque step. Do not keep a permanent C++/Rust runtime selector.

## Final report format

At the end of a batch/session report only:
- tasks completed;
- HEAD(s) used;
- files changed;
- tests/checks and results;
- tasks marked MANUAL-PENDING/BLOCKED and exact reason;
- next eligible task, if any.
