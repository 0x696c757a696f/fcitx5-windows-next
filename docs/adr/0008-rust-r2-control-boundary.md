# ADR 0008: Rust R2 Control/process boundary

- Status: accepted
- Date: 2026-08-22
- Task: RUST-R2-02

## Context

RUST-R2-02 requires the Control CLI/shared process execution boundary to move to Rust after the C++ drain/timeout/cancel semantics are frozen. The v1.8 specification also keeps several Windows-facing or domain-owned surfaces in C++ for this stabilization gate:

- WTL/Config remains C++ and consumes typed Control/config/package boundaries.
- TSF activation guard helpers remain C++ because they are linked into the TSF DLL boundary.
- Launcher IPC and existing Win32 process hosting remain small C++ adapters around typed state machines/protocols.
- Package/update trust, lifecycle, repository sequence, and repair semantics are Rust/package-core or Rust/control-core boundaries, but Control still composes them with downloader and launcher adapters.

Earlier R2-02 slices moved the authoritative process executor, Control schema/catalog, command classification, JSON formatting, startup registry primitive, launcher action sequencing, package lifecycle transactions, installed-package repair verification, and repository sequence repair/reset into Rust. The remaining `fcitx5-control.exe` C++ code is now adapter/composition code for Win32, TSF, launcher IPC, Config TOML, and data collection.

## Decision

Treat RUST-R2-02 as complete when the following boundaries are true:

1. `fcitx5-process-execution-core` is the only shared child-process primitive used by Config/Control.
2. `fcitx5-control-core` owns the Control command catalog/schema, action/arity classification, usage text, and stable JSON response assembly.
3. Rust owns package repair/lifecycle/repository sequence operations that are part of Control repair/update state.
4. C++ `fcitx5-control.exe` may remain as a thin adapter for:
   - Win32 registry/filesystem path discovery;
   - TSF activation guard access;
   - launcher IPC stop/resume/status calls;
   - Config TOML parse/merge/write while Config remains C++;
   - package/repository row collection and downloader orchestration while calling Rust package-core/control-core/process-execution boundaries.

Do not create a runtime C++/Rust selector. C++ adapter code must not reimplement process drain/timeout/cancel semantics, Control command catalog decisions, or Rust-owned package repair/lifecycle state transitions.

## Consequences

- RUST-R2-02 can be archived after the focused R2 validation set is green.
- RUST-R2-03 remains justification-gated: diagnostics/repair modeling should start only if it adds concrete safety/testability value beyond the typed Control/package boundaries already created here.
- Future TSF/Candidate/Config Rust work remains gated by the explicit R3 PoC tasks and must not be smuggled into R2 completion.
