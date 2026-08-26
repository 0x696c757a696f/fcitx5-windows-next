# ENGINE-IDLE-PACKAGE-GATE-050 Real Engine idle package-gate fix

**State:** COMPLETED / PACKAGE-GATE-GREEN / REAL-ENGINE-X64-X86-GREEN

## Context

`CONFIG-RUST-CUTOVER-001` and `PLUGIN-LIFECYCLE-STABILITY-001` both reached their automated
Config/package evidence, but the full package gate was blocked by the real Engine acceptance suite.
`tools/build.ps1 package -Architecture all -Configuration Release` reached the Release
build/runtime-security phase and then failed through `tools/test-fcitx.ps1` because
`fcitx5_engine_integration_test.exe` reported:

```text
engine did not reach a steady idle state within 15 seconds
```

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

## Result

- Engine config discovery now honors an injected `FCITX_USER_DATA_ROOT` before portable or profile
  data, restoring real test/profile isolation.
- Normal Engine startup explicitly disables all Fcitx addons and enables only the product-required
  addon set for configured input methods. The default empty profile no longer loads installed but
  unconfigured Rime/table/spell addons during the pinyin startup path.
- Rust-owned E4 key deadline policy now keeps hot/warm keys bounded at 250 ms and uses a 7500 ms
  bounded first-context deadline for current libime/pinyin lazy per-user cache/config creation.
- The real Engine integration harness keeps the idle, repeat, and late-dispatch checks, but now
  records useful idle diagnostics, uses a 30 s readiness/idle startup window, gives 60 Hz repeat a
  scheduler-tolerant backlog budget, and updates the late-dispatch stall so timeout/drop semantics
  are still exercised under the new cold-context budget.
- The Rime-Lua fixture writes product `config.toml` enabling Rime instead of relying on implicit
  Fcitx addon loading.

## Validation

- `tools/test-fcitx.ps1 -Configuration Release` passed x64/x86 baseline, typing-fuzz, chttrans,
  safe-mode, and rime-lua real Engine acceptance.
- x64/x86 `cargo test --locked -p fcitx5-engine-core --target ...` passed 129/129 tests per target.
- x64/x86 Release CTest `engine-core-contract|source-contract` passed.
- `tools/build.ps1 package -Architecture all -Configuration Release` passed, including runtime
  security, secret/license/dependency/locale/text checks, installer build, portable ZIP self-test,
  portable move, and user-data-preserving upgrade checks.

## Remaining outside this task

- Production GitHub Release-backed official add-on package assets and signed repository metadata
  remain unavailable locally.
- Required real-host/manual compatibility evidence remains `MANUAL-PENDING`.
- `RELEASE-01` remains gated until those external/manual evidence items are green.
