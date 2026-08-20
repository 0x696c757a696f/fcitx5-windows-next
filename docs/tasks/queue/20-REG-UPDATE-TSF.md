# Current Task — REG-UPDATE-TSF In-use TSF DLL generation draining

**Mode:** CHANGE
**Task ID:** `REG-UPDATE-TSF`
**Prerequisite:** 009 repository state + 012 installer/registration semantics + 014 package path corpus should be stable

## Goal

Allow updates while old `fcitx5-tsf.dll` images remain loaded in Word/Chrome/etc. by temporarily running complete isolated generations, without killing hosts or teaching the new Engine to decode the old protocol.

## Specification references

- §7.3.1 Generation Side-by-Side / Drain
- §0.5 item 16
- Phase 7
- `REG-UPDATE-TSF-001` / `REG-UPDATE-TSF-002`

## Required behavior / implementation contract

- Stage/verify complete N+1 before touching the canonical TSF path.
- Use safe same-volume rename/versioned retention of the old in-use DLL and atomically activate the new canonical DLL.
- Embed/associate generation + protocol/build identity so N TSF connects only to N runtime and N+1 only to N+1.
- Launcher may supervise two complete generations during drain, but never mixed-generation components.
- Clean old generation when clients are gone; use bounded pending cleanup/reboot deletion as final fallback.
- Keep previous-known-good rollback distinct from a draining generation.

## Out of scope

- Rust implementation language unless selected in subsequent R1 tasks

## Required validation

- `REG-UPDATE-TSF-001`: old host remains open through N→N+1; old host keeps N, new host gets N+1.
- `REG-UPDATE-TSF-002`: N+1 health failure rolls back safely while N host remains usable.
- No host-kill/default Restart Manager shutdown required.
- No old-protocol decoder/shim in N+1.

## Done when

- In-use TSF update no longer depends on overwriting a loaded DLL.
- Generation isolation is machine-testable.
- Cleanup is bounded and crash-recoverable.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
