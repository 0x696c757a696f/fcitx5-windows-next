# Task 067 - Plugin provenance and data boundary

**Mode:** CHANGE / CODE-ONLY
**Task ID:** `PLUGIN-PROVENANCE-DATA-BOUNDARY-001`
**Prerequisite:** `065` automated Config contract green.
**Evidence class:** automated x64/x86 package contract; production publication remains manual.

## Goal

Extend the current Rust package path with verified plugin `runtime_abi`, `runtime_build`, source
provenance, and explicit separation of versioned program packages from user data.

## Constraints and acceptance

- Read `ponytail`, `rust-skills`, and `tdd`; use worktree-local `CARGO_TARGET_DIR`. Preserve the
  current ML-DSA-65 v2 verifier and upstream standard Fcitx addon/build-farm semantics.
- Reject missing/mismatched ABI/build/provenance metadata before activation. Metadata permissions are
  declarations and audit inputs, not a sandbox; addons remain outside the TSF host.
- Version install/remove/rollback never deletes user dictionary, Rime user data, or configuration.
  Cover Chinese main path, Rime/Lua, and one non-Chinese addon in the metadata corpus.
- Add deterministic x64/x86 verifier/path/lifecycle tests. Protected signing keys, release publication,
  real online lifecycle, Authenticode, and UAC remain precisely `MANUAL-PENDING`.
