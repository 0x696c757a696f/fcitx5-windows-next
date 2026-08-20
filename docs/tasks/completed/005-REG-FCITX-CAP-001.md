# Current Task — REG-FCITX-CAP-001 Complete Fcitx InputContext capabilities

**Mode:** CHANGE
**Task ID:** `REG-FCITX-CAP-001`
**Prerequisite:** 003 protocol normalization complete where key forwarding depends on it

## Goal

Implement/advertise surrounding-text, delete-surrounding, forward-key and related capabilities truthfully so real Fcitx engines/addons never hit silent empty implementations.

## Specification references

- §0.5 item 4
- Engine/InputContext sections
- Phase 3
- `REG-FCITX-CAP-001`

## Required behavior / implementation contract

- Audit current `EngineInputContext` capability flags against implemented callbacks.
- Implement only capabilities the Windows/TSF bridge can support correctly; otherwise do not advertise them.
- Define safe failure semantics for unsupported host capability.
- Keep all Fcitx API interaction on the Engine execution context.

## Out of scope

- Rust FFI around Fcitx
- Rewriting Fcitx
- Profile UI

## Required validation

- Contract tests for surrounding text read/update.
- Delete-surrounding behavior and unsupported-host behavior.
- Forward-key behavior with no duplicate injection/passthrough.
- At least one real engine/addon path using each supported capability when feasible.

## Done when

- Advertised capability equals actual behavior.
- No empty callback silently discards requested behavior.
- Host remains stable when capability is unavailable.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
