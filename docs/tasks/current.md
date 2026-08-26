# ENGINE-IDLE-PACKAGE-GATE-050 Real Engine idle package-gate fix

**State:** TODO

## Context

`CONFIG-RUST-CUTOVER-001` and `PLUGIN-LIFECYCLE-STABILITY-001` both reached their automated
Config/package evidence, but the full package gate is still blocked by the real Engine acceptance
suite. `tools/build.ps1 package -Architecture all -Configuration Release` reaches the Release
build/runtime-security phase and then fails through `tools/test-fcitx.ps1` because
`fcitx5_engine_integration_test.exe` reports:

```text
engine did not reach a steady idle state within 15 seconds
```

This is now the next code-only release/package blocker. Do not treat plugin lifecycle or release
readiness as complete until this gate passes.

## Specification references

- `docs/spec-v1.8.md` sections 4.1, 4.3, 4.5, 4.6, 16, and the testing policy
- `docs/engine-boundary.md` Engine E4/E5 current boundary notes

## Scope

- Reproduce the failing real Engine acceptance path from `tools/test-fcitx.ps1`.
- Diagnose whether the idle failure comes from the test harness threshold, process/test isolation,
  stale background processes, startup warm-up, addon initialization, or a real Engine busy loop.
- Implement the smallest fix that preserves the bounded-input and fail-open guarantees.
- Keep new product-owned logic Rust-owned where applicable; C++ changes are allowed only in the
  existing integration harness or direct Fcitx-facing Engine adapter seam.
- Preserve x64/x86 Release package acceptance; do not remove real-engine checks just to make the
  package gate green.

## Must not do

- Do not fake the idle condition by deleting the acceptance check.
- Do not hide failed Engine stderr or suppress failing subprocess exit codes.
- Do not introduce input simulation through hooks, `SendInput`, process injection, anti-cheat
  bypass, credential access, or external exploitation.
- Do not broaden this task into unrelated Config, Candidate, package repository, or release work.

## Required validation

- Focused build of `fcitx5_engine_integration_test` and any touched Engine/UI target.
- Focused real-engine acceptance through `tools/test-fcitx.ps1 -Configuration Release` when local
  prerequisites are available.
- At minimum, direct failing mode reproduction plus a focused regression/diagnostic test if the full
  package gate cannot complete locally for an external toolchain reason.
- `tools/check-text-format.ps1`.
- `git diff --check`.

## Done when

- The real Engine idle acceptance no longer fails from the known steady-idle blocker, or the exact
  remaining non-code/toolchain blocker is recorded as `MANUAL-PENDING`/`BLOCKED` in
  `docs/tasks/status.md`.
- Full package gate can continue past `tools/test-fcitx.ps1`, or a narrower local limitation is
  documented with CI/host evidence requirements.
- `docs/tasks/status.md` records HEAD, changed files, commands, results, and any remaining blocker.
