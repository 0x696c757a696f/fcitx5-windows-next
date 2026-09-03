# Task 068 - Repository freshness and mirror identity

**Mode:** CHANGE / CODE-ONLY
**Task ID:** `REPOSITORY-FRESHNESS-MIRROR-IDENTITY-001`
**Prerequisite:** `067` automated package metadata boundary green.
**Evidence class:** deterministic repository corpus; no TUF support claim.

## Goal

Harden the existing Rust ML-DSA-65 v2 repository protocol against stale/frozen/mixed metadata and
bind mirrors to the same verified repository identity.

## Constraints and acceptance

- Read `rust-skills`; use worktree-local `CARGO_TARGET_DIR`. Extend the current
  protocol in one versioned path; do not build permanent v2/v3 runtime dual stacks.
- Verify monotonic sequence, expiry/freshness, channel identity, mirror identity, and coherent target
  metadata before package selection. Reject rollback, freeze, and mix-and-match fixtures.
- A mirror supplies bytes for an already verified identity and does not become a trust root. Do not
  claim TUF root/snapshot/timestamp, RemoteAddon, AppContainer, or ARM64 support.
- Add x64/x86 deterministic tests for all rejection cases and update production input generation only
  when it remains ML-DSA-65 v2 compatible. Protected production signing/publication remains manual.
