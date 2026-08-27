# Fcitx5 for Windows Next — Codex Queue Rules

This repository is an authorized open-source Windows input-method project. Work only in this repository and in public documentation/reference code needed by the currently selected task.

## Sources of truth

1. `docs/spec-v1.8.md` — long-term engineering specification.
2. `docs/tasks/PLAN.md` — ordered task queue and gate dependencies.
3. `docs/tasks/current.md` — the one task authorized for implementation right now.
4. `docs/tasks/status.md` — execution evidence and pending external verification. Search it by current task ID, subsystem, or evidence key; never read it wholesale.

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

## Context efficiency

Preserve context for reasoning and code, not bulk input.

- Search before reading. Use `rg`/symbol search to locate relevant definitions, references, specification sections, and evidence.
- Never read `docs/tasks/status.md`, the full specification, completed-task archives, or other large documents wholesale. Read only relevant ranges.
- For large source files, locate the relevant symbol or call site first and read only the necessary surrounding ranges.
- Do not inspect `third_party/`, historical audits, or unrelated reference implementations unless the current task or affected code path requires them.
- Do not reread unchanged files when the relevant content is already known.
- Keep command output bounded. Prefer quiet/targeted commands; on failure inspect the relevant error and nearby context instead of dumping full build or test logs.
- Validate the smallest affected boundary first. Expand to workspace/full integration/release checks only when the changed boundary or current acceptance criteria require it.
- If two repair attempts fail without materially changing the symptom, stop tweaking and re-check the root-cause assumption.
- Keep intermediate explanations short. Durable evidence belongs in task/status files; the final report follows the format below.

## Model delegation and reasoning effort

The root agent is the orchestrator. Route work automatically instead of asking the user to switch models.

Optimize for successful work per unit of context and compute. Spend strong-model reasoning on uncertainty and delegate well-specified execution downward.

Preferred routing:

- Luna medium: delegate deterministic, well-specified implementation, tests, fixtures, mechanical edits, straightforward refactors, and regressions whose root cause and acceptance criteria are already known.
- Terra high: delegate bounded debugging or implementation that requires judgment but has limited architectural uncertainty.
- Terra xhigh: delegate bounded investigations that need materially deeper reasoning while cost efficiency still matters.
- Sol medium: keep orchestration, decomposition, integration, ambiguous root-cause analysis, architecture review, and decisions where a wrong assumption could cause substantial rework.
- Sol xhigh: use for TSF/COM/ABI/FFI, concurrency/lifetime/unsafe boundaries, security-sensitive design, protocol redesign, difficult cross-component regressions, or when a focused lower-cost investigation fails to establish a trustworthy root cause.
- GPT-5.5 high/xhigh: optional independent second opinion only, not part of the normal escalation ladder.

Do not use Sol high as an automatic intermediate step. Do not use Sol max by default.

After Sol or Terra removes uncertainty, delegate the now-bounded implementation and regression work to Luna whenever practical.

Delegation rules:

- Route models automatically; do not ask the user to choose or switch models for ordinary task execution.
- Delegate only bounded work with explicit files/symbols, constraints, acceptance criteria, and tests.
- Give subagents the minimum relevant context; do not make them rediscover the repository.
- Parallelize only genuinely independent workstreams.
- The root agent owns decomposition, integration, conflict resolution, and final validation.
- Do not spawn a subagent when describing and supervising the work would cost more context than doing it directly.
- After two materially equivalent failures, stop repeating the same approach and reconsider the root-cause model.

## Implementation rule

Implement the smallest correct vertical slice that satisfies the current task. Necessary producer/consumer changes, protocol changes, fixtures, and regressions are in scope only when required for correctness.

Do not perform unrelated cleanup, framework migration, dependency churn, naming changes, or future tasks.

Architecture defaults:
- The only durable C++ island is direct Engine integration with Fcitx5 core/addon objects:
  `fcitx::Instance`, `InputContext`, addon/config objects, `InputPanel`, `CandidateList`,
  and the thin conversion/adapter code required to consume upstream Fcitx semantics.
- New product-owned Windows code defaults to Rust. Do not add new C++ product-state, validation,
  operation, parsing, package/update, settings, candidate-domain, TSF, or UI-domain logic. Use C++
  only for the direct Fcitx-facing Engine adapter island, or for a tiny native adapter seam that
  delegates its product semantics to Rust and is recorded as temporary evidence.
- Do not regress a component that has already cut over to Rust back to C++ merely because older task
  text called the C++ implementation the baseline. Historical C++ behavior remains a corpus/reference,
  not the target language.
- Current state: the shipping TSF DLL is Rust; the old shipping C++ TSF implementation has been
  deleted. Remaining TSF work is real-host/manual evidence and focused bug fixes unless a new task
  explicitly opens more TSF scope.
- Current state: Candidate model/layout/interaction are Rust-owned; Win32/D2D/DWrite drawing code is
  only a renderer/window adapter until a renderer migration task has equivalent visual/DPI evidence.
- Current state: Config still has a WTL/Win32 shipping shell only as a temporary migration adapter.
  It is not a durable C++ exception. New Settings state, validation, preview contracts,
  package/update/control orchestration, operation models, and UI-domain code must be Rust-owned.
  The task queue must keep shrinking and then replacing the C++ Config shell once behavior,
  accessibility, DPI, localization, and visual-regression evidence are frozen.
- Rust migration still needs contract/golden/fuzz or equivalent regression evidence before replacing
  behavior. Do not change semantics and language in one opaque step; split large GUI migrations into
  executable cutover slices that preserve user-visible behavior while moving ownership to Rust.
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
5. Continue automatically only when the next task is tightly coupled to the same subsystem and materially reuses the context already loaded.
6. At a subsystem or phase boundary, leave the next task selected in `current.md`, record the minimal handoff in `status.md`, and stop the session so the next task starts with fresh context.

Do **not** ask the user merely to advance to the next task.

Stop automatic advancement when:
- the next eligible task crosses a subsystem or phase boundary and would benefit from fresh context;
- current acceptance requires unavailable real hardware/application/manual evidence;
- a required signing credential/private key or privileged external service is unavailable;
- current HEAD contradicts the task/spec in a way that requires a product decision;
- the next task is gated on an unfinished prerequisite;
- a safety/policy constraint prevents the requested implementation;
- all queued tasks are complete.

For an `EXTERNAL_EVIDENCE` task, perform every reachable automated preparation/check, record exactly what remains manual in `status.md`, mark it `MANUAL-PENDING`, and continue to later code-only tasks **only if PLAN.md says the manual evidence is not a prerequisite for them** and the next task remains within the same useful context boundary. Never mark unrun real-host evidence as passed.

## Rust migration rule

For each R1/R2/R3 or later Rust migration component:

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
