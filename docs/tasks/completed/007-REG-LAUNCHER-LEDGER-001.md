# Current Task — REG-LAUNCHER-LEDGER-001 Persistent launcher crash ledger

**Mode:** CHANGE
**Task ID:** `REG-LAUNCHER-LEDGER-001`
**Prerequisite:** C++ semantics must be correct before R2

## Goal

Persist only the minimal recovery ledger needed so launcher restart cannot erase a crash storm, SafeMode latch, or backoff window.

## Specification references

- §0.5 item 7
- Launcher/recovery sections
- Phase 2/5
- `REG-LAUNCHER-LEDGER-001`

## Required behavior / implementation contract

- Persist a minimal typed ledger such as safe-mode latch, crash count/window start, and last healthy state/time as actually required by current state machine.
- Use atomic state publication; distinguish absent first-run state from corrupt established state.
- Define deterministic healthy-window reset semantics.
- Configure Job Object limits before launching children; if current HEAD still launches before job configuration, fix/test in this task.

## Out of scope

- Rust R2 migration (later)
- Package/update state

## Required validation

- Crash threshold → SafeMode → launcher process restart → still SafeMode.
- Healthy window/reset policy clears ledger exactly when specified.
- Corrupt/truncated ledger fails safely.
- Job configuration failure leaves zero launched children if that defect remains in HEAD.

## Done when

- Launcher restart no longer bypasses crash accounting.
- Recovery state is minimal and atomically persisted.
- No whole internal state machine snapshot is serialized.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
