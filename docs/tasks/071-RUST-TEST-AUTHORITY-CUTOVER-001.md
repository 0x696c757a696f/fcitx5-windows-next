# Task 071 - Rust test authority cutover

**Mode:** CHANGE / CODE-ONLY / MIGRATION
**Task ID:** `RUST-TEST-AUTHORITY-CUTOVER-001`
**Prerequisite:** `065` through `070` automated acceptance is green. Any external evidence left by
those tasks remains explicitly `MANUAL-PENDING` and must not be silently promoted.
**Evidence class:** repository-wide C++ test inventory, Rust migration, corpus continuity, and x64/x86
automated routing evidence; real-host evidence remains owned by its existing release/host tasks.

## Goal

Make Rust the default and authoritative test language for Rust-owned product semantics without
mechanically deleting the small C++ boundary tests that still prove Fcitx integration, Windows ABI
behavior, or the final mixed binary. The task is an ordered migration program, not permission to
rewrite every test in one opaque change.

## Scope and ownership rules

- Inventory every existing C++ test source and test target in the repository, including CTest-linked
  targets and source-contract/source-string tests. Classify each item exactly once as `KEEP`,
  `MIGRATE`, or `DELETE`, with owner, reason, replacement, and evidence location.
- `KEEP` is allowed only for tests that directly operate on `fcitx::Instance`, `InputContext`,
  `Addon`, `InputPanel`, `CandidateList` or a thin direct Fcitx adapter; verify a necessary
  Win32/COM/ABI adapter; or exercise the final C++/Rust mixed binary through integration/E2E.
- `MIGRATE` applies to Rust-owned Config, Candidate, package/update, control, launcher, TSF product
  state, protocol, validation, parsing, policy, and other product semantics. Migrate unit, contract,
  property/model, fault-injection, fuzz, performance, and source-structure coverage to Rust in
  bounded vertical slices.
- `DELETE` applies to obsolete tests, tests whose only authority is deleted, and duplicate C++ tests
  made redundant by the Rust public-behavior contract. Do not delete a test merely because it is
  written in C++.
- C++ source-string/source-contract checks are structural supplements only. They cannot replace Rust
  public behavior, contract, property, fault, fuzz, or performance tests.
- C++↔Rust differential/golden tests may remain only while a frozen corpus is needed for migration.
  Each such test must name its owning cutover and deletion condition; the cutover removes the old
  C++ authority and tests serving only that authority. No permanent runtime or test dual stack.

## Constraints

- Read `AGENTS.md`, this task, and only the referenced specification ranges before implementation.
- Fully read and follow `rust-skills`; use a worktree-local `CARGO_TARGET_DIR`.
- Use PowerShell 7 at `D:\Program Files\PowerShell\7\pwsh.exe`.
- Use `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains`, with fast tools under
  `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\fast`; use the pinned repository Cargo
  documented in `AGENTS.md`.
- Preserve the top-level CMake/CTest route for tests that remain C++ or validate the final mixed
  binary. Do not introduce a generic test framework, duplicate protocol, or test-only product logic.
- Keep the 065-070 order and do not expand this task into new product behavior. New test code is Rust
  unless it falls within the three permitted C++ categories.

## Required work

1. Freeze an inventory covering all C++ test files, targets, CMake/CTest registration, owner,
   classification, Rust replacement or keep rationale, and deletion evidence.
2. Establish Rust public-behavior/contract coverage before removing any C++ Rust-owned authority test.
   Preserve the existing golden/differential corpus byte-for-byte or document a reviewed translation
   with parity checks; include invalid, boundary, failure, and privacy cases where applicable.
3. Migrate every C++ test source/target marked `MIGRATE` in the inventory to its owning Rust crate,
   covering the applicable unit, property/model, fault, fuzz, performance, and source-structure
   checks. This may be delivered as bounded vertical slices across multiple commits within task
   071, but the task cannot complete while any inventory `MIGRATE` item lacks its Rust replacement
   and deletion evidence. Source-structure checks must remain secondary to public behavior and must
   not be the only proof of a contract.
4. Keep or add only the justified C++ adapter and final mixed-binary tests, and make their boundary
   explicit in names and documentation.
5. Wire every replacement and retained test through the supported Cargo or CMake/CTest entry point,
   including x64 and x86 where the target supports both. Prove that removed C++ tests are no longer
   registered and that required corpus fixtures are still consumed.

## Acceptance

- A complete inventory has no unclassified C++ test source or CTest target; every row is `KEEP`,
  `MIGRATE`, or `DELETE` with a defensible reason and file/target evidence.
- Every inventory item marked `MIGRATE` has a merged Rust replacement, passing evidence, and deletion
  evidence for the superseded C++ test; every item marked `DELETE` has deletion/registration evidence.
  The ledger has no unfinished `MIGRATE` or `DELETE` item at task completion.
- Rust-owned product semantics have Rust-authoritative public-behavior coverage for the migrated
  scope, including the applicable contract/property/fault/fuzz/performance checks; C++ source-string
  checks are not counted toward that requirement.
- The final repository's long-term C++ tests are demonstrably limited to exactly the three permitted
  `KEEP` categories: direct Fcitx adapter, necessary Win32/COM/ABI adapter, or final mixed-binary
  integration/E2E. No other C++ test remains authoritative or registered.
- Any temporary differential/golden test has a named removal gate; at cutover, obsolete C++
  authority tests and only-that-authority fixtures are deleted with repository evidence.
- x64 and x86 results are recorded separately where supported; Cargo, CMake, and CTest routes pass
  without sleep/retry barriers, and removed tests are absent from test registration.
- `git diff --check`, focused Rust test/format checks, affected CMake/CTest checks, corpus continuity
  checks, and source-contract checks pass. Unavailable real-host, Narrator/NVDA, Win7/10/11, UAC,
  signing, or production-online evidence remains `MANUAL-PENDING` in the task evidence; it is never
  represented as automated test success.

## Deliverables

- The per-file/per-target test ownership inventory and migration ledger.
- Rust replacement tests and retained C++ boundary tests, limited to the approved scope.
- CMake/CTest and Cargo routing plus x64/x86 evidence.
- Deletion/corpus-continuity evidence and a status entry identifying any remaining `MANUAL-PENDING`
  work.

On completion, update `docs/tasks/status.md`, archive this task, and select the next eligible task
according to `docs/tasks/PLAN.md`. Do not claim that all C++ has been removed when a permitted adapter
or final mixed-binary test remains.
