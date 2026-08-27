# Task 066 - Candidate semantic accessibility contract

**Mode:** CHANGE / CODE-ONLY
**Task ID:** `CANDIDATE-SEMANTIC-A11Y-001`
**Prerequisite:** `065` automated contract green.
**Evidence class:** automated x64/x86 semantic contract; real Narrator/NVDA hosts remain manual.

## Goal

Make Rust `CandidateModel` the explicit single semantic source for renderer, UIA, and notification,
with revision-aware stale suppression and sensitive-context privacy enforcement.

## Constraints and acceptance

- Read `ponytail`, `rust-skills`, and `tdd`; use worktree-local `CARGO_TARGET_DIR`. Keep new domain
  logic Rust-owned; native renderer/UIA adapters only consume the Rust semantic DTO.
- Notifications carry snapshot identity, coalesce compatible changes, cancel/drop stale revisions, and
  do not derive a separate candidate order or selection state.
- Capabilities compose: keyboard, UIA, Narrator/NVDA compatibility, High Contrast, large text,
  reduced motion, reduced candidates, and stable layout. Do not add a disability mode.
- Password/PIN/sensitive contexts suppress speech, text logging, learning, and network access.
- Add deterministic x64/x86 tests for stale/cancelled notifications, semantic parity, capabilities,
  and sensitive-context suppression. Record real Narrator/NVDA/Win7/10/11 results as `MANUAL-PENDING`.
