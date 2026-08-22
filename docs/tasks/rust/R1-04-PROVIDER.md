# Current Task — RUST-R1-04 Rust provider policy/runner

**Mode:** CHANGE
**Task ID:** `RUST-R1-04`
**Prerequisite:** 008 process execution semantics + R1-01

## Goal

Migrate the isolated provider-management boundary to Rust only after its command/input/output policy is frozen and bounded.

## Specification references

- Rust R1 provider sections
- Package/provider security policy
- `REG-RUST-DIFF-001`

## Required behavior / implementation contract

- Provider input/output schema is bounded and validated.
- No arbitrary shell proxy or unvalidated command construction.
- Timeout/cancel/process containment follows the authoritative process-exec contract.
- Provider never receives live key/preedit/candidate/commit content unless a separately approved product requirement exists.

## Required validation

- Differential provider fixtures.
- Malformed/oversize/hung provider cases.
- Package/artifact smoke and dependency gates.

## Current frozen boundary

The Rust cutover mirrors the previous provider surfaces and removes the old
C++ authoritative implementation:

- Rust `make_plum_plan`
  - bounded absolute provider/user/cache paths
  - pinned `rime-install.bat`
  - `ProviderTrust` classification
- Rust `fcitx5-provider.exe`
  - `--version`
  - `--allow-unverified`
  - `--plum`
  - elevated-run refusal
  - pinned System32 `cmd.exe` launch / timeout / non-zero exit-code propagation

## Fixture inventory to keep visible

- Path validation:
  - absolute provider/user/cache directories
  - reparse-point rejection
  - command metacharacter rejection
  - missing `rime-install.bat`
- Trust classification:
  - official preset source
  - unverified third-party source
  - explicit `--allow-unverified` override
- Runner failure categories:
  - elevation refusal
  - system directory unavailable
  - process launch failure
  - process timeout
  - non-zero child exit propagation

Current regression coverage lives in:

- Rust `provider_plum_policy_matches_frozen_cpp_boundary`
- Rust `provider_runner_propagates_nonzero_and_times_out_without_live_input_data`
- CTest `provider-version`
- CTest `provider-boundary-smoke`

## Done when

- Rust provider runner is authoritative and bounded.
- No new input-data-plane network path exists.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
