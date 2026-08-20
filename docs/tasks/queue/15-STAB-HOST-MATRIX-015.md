# Current Task — STAB-HOST-MATRIX-015 Real Windows host / Win7 / game compatibility evidence

**Mode:** REVIEW
**Task ID:** `STAB-HOST-MATRIX-015`
**Evidence class:** `EXTERNAL_EVIDENCE` — never claim unrun real-host evidence passed.

## Goal

Collect the reachable real-host evidence required by v1.8 without adding special bypass paths: Win7 Legacy VM, Office/browser/editor/terminal/RDP/DPI, old x86 host, and League of Legends + Vanguard smoke.

## Specification references

- §12 compatibility matrix
- Phase 5
- `REG-GAME-001` / `REG-LOL-001`
- Win7 support matrix

## Required behavior / implementation contract

- Create/update an exact test matrix and result record with OS build, architecture, host version, build identity, and pass/fail evidence.
- Use ordinary TSF/Windows integration only. Do not add game-specific input injection or circumvention behavior.
- Run all environments available to the current machine/CI.
- For unavailable hosts, mark `MANUAL-PENDING` with exact unrun cases; never infer pass.

## Constraints / notes

- PLAN permits later code-only UI tasks to continue while this item is MANUAL-PENDING; final stabilization/release cannot be declared complete until required real-host evidence is actually green.

## Required validation

- Notepad/Word/Chrome/Edge/VS Code/Terminal smoke where available.
- DPI 100/125/150/200%, multi-monitor, Alt-Tab, fullscreen/borderless where relevant.
- Win7 x64 VM: install→register→input→candidate→uninstall.
- LoL + Vanguard: chat composition/candidate/commit, passthrough of non-text controls, Alt-Tab, window modes.
- RDP/old x86 host where available.

## Done when

- Automatable/reachable evidence is recorded.
- Unavailable real-host checks are explicitly `MANUAL-PENDING`, not passed.
- No special compatibility code was added without a failing reproducible host case.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
