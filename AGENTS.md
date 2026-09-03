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

- Use the installed `pi-caveman` extension (`/caveman`) to cut output tokens. When enabled (or as a
  standing style), write terse like a smart caveman: drop articles/filler/pleasantries/hedging,
  fragments OK, keep technical terms and code blocks exact, use `[thing] [action] [reason]. [next]`.
  Auto-clarity: expand for security warnings, irreversible-action confirmations, or confused users.

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

Subagents are implemented with `deepseek-v4-flash`. The root agent keeps orchestration,
decomposition, integration, ambiguous root-cause analysis, architecture review, and decisions where
a wrong assumption could cause substantial rework; well-specified execution is delegated to
`deepseek-v4-flash` subagents.

Delegation rules:

- Route models automatically; do not ask the user to choose or switch models for ordinary task execution.
- Delegate only bounded work with explicit files/symbols, constraints, acceptance criteria, and tests.
- Give subagents the minimum relevant context; do not make them rediscover the repository.
- Parallelize only genuinely independent workstreams.
- The root agent owns decomposition, integration, conflict resolution, and final validation.
- Do not spawn a subagent when describing and supervising the work would cost more context than doing it directly.
- After two materially equivalent failures, stop repeating the same approach and reconsider the root-cause model.

Execution environment for implementation agents (give every subagent these absolute paths):

- Use PowerShell 7 at `D:\Program Files\PowerShell\7\pwsh.exe`.
- Use the repository toolchain root `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains`.
- Prefer the fast tools under `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast`.
- Use the pinned repository Cargo at
  `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home\bin\cargo.exe`
  (or the exact pinned toolchain path selected by the task), and keep `CARGO_TARGET_DIR`
  inside the agent worktree.
- Set `RUSTUP_HOME=D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\rustup-home`,
  `CARGO_HOME=D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\cargo-home`, and
  `RUSTUP_TOOLCHAIN=1.98.0-x86_64-pc-windows-msvc`.
- The repository `.cargo/config.toml` already points `rustc-wrapper` at the pinned sccache; also pass
  the absolute sccache path explicitly:
  `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast\sccache-0.17.0\sccache-v0.17.0-x86_64-pc-windows-msvc\sccache.exe`.
- CMake/CTest:
  `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast\cmake-3.31.8\cmake-3.31.8-windows-x86_64\bin`.
- Ninja: `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast\ninja-1.13.2\ninja.exe`.
- LLVM bin:
  `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast\llvm-22.1.8\clang+llvm-22.1.8-x86_64-pc-windows-msvc\bin`.
- Python: `D:\Dev\pixi\envs\python\python.exe`.
- Use PowerShell syntax only on this Windows task; never use Bash quoting/heredocs. Wrap statement
  blocks (`foreach`, `if`) in `$()`/`@()` before piping, expand wildcard directories with
  `Get-ChildItem -Filter` before passing real paths to `rg`, and prefer single quotes around complex
  regexes. Multi-line Python uses a PowerShell here-string piped to `python -`, never a Bash heredoc.
- Default every pure Safe Rust crate/entry to `#![forbid(unsafe_code)]`; only FFI, Win32/COM,
  allocator, or low-level ABI files may be named exceptions, and they must still use
  `#![deny(unsafe_op_in_unsafe_fn)]`, narrow `unsafe` blocks, and `SAFETY` comments.

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

Test ownership follows product ownership. All new test code defaults to Rust. Rust-owned product
semantics (including Config, Candidate, package/update, control, launcher, TSF product state, and
protocols) require Rust-authoritative unit, contract, property/model, fault, fuzz, performance, and
source-structure coverage. C++ tests may remain long-term only when they directly operate on
`fcitx::Instance`, `InputContext`, `Addon`, `InputPanel`, `CandidateList`, or another direct Fcitx
adapter boundary; verify a necessary Win32/COM/ABI adapter; or exercise the final mixed C++/Rust
binary through an integration/E2E test. Migration-only C++/Rust differential or golden tests may
coexist temporarily while a corpus is frozen, but the cutover task must delete the old C++ authority
and tests that served only that authority. C++ source-string/source-contract tests never replace Rust
public-behavior tests; classify each existing C++ test as `KEEP`, `MIGRATE`, or `DELETE` before
changing it.

## Testing

Run affected tests first. Expand only when the changed boundary requires it.

Use deterministic barriers/fake clocks/fixtures where possible instead of arbitrary sleeps. A reproducible bug fix should leave a regression test.

For every migration task, record test ownership and corpus continuity before editing tests. Move
Rust-owned unit, contract, property/model, fault, fuzz, performance, and source-structure checks to
Rust in bounded slices. Preserve only the three permitted long-term C++ categories above, and keep
the CMake/CTest route for tests that remain or for final mixed-binary integration/E2E. Run and record
both x64 and x86 lanes when the affected target supports them. A temporary differential or golden
test must name the Rust cutover task that removes it; no permanent C++/Rust test authority or
duplicate product contract is allowed.

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
