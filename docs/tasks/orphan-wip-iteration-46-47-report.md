# Orphan WIP rescue and responsibility split — Iteration 46 + 47 report

## 1. Scope and starting state

Codex's orphaned in-work-tree changes were rescued byte-for-byte, the group that already had a
stronger durable copy was de-duplicated, and the remainder was split by responsibility and put
through the landing gate.

Starting state (read-only record, Iteration 46 Phase 1):

- `HEAD = c159be0bc3198a295e934c6aa6d57c47672d5971`, equal to `origin/main`.
- `git status --porcelain` = **29 entries** (18 modified + 11 untracked), grouped as:
  - **A (12)**: vendored WindUI — 8 modified + 4 untracked (`Cargo.lock`,
    `examples/uia_smoke.rs`, `src/accessibility.rs`, `src/platform/win32/accessibility.rs`).
  - **B (7)**: localization — 6 new `locales/*.json` + `rust/config-core/tests/locale_contract.rs`.
  - **C (7 modified)**: `rust/candidate-core/src/{axis_layout.rs, frame_update.rs, bin/fcitx5_ui.rs,
    bin/candidate_poc.rs}`, `rust/config-core/src/config_core.rs`, `rust/config-poc/src/main.rs`,
    `rust/control-core/src/lib.rs`.
  - **D (3 modified)**: `CMakeLists.txt`, `tests/integration/tsf_notepad_e2e_test.cpp`,
    `docs/tasks/status.md`.
- No `TODO`/`FIXME`/`unimplemented!` markers; the tree had never been built or tested.

## 2. Iteration 46 — byte rescue plus group A de-duplication

### 2.1 Byte checkpoint (17 paths)

- Per-file SHA-256 manifest of B/C/D recorded before any mutation:
  `%TEMP%/c2c-iter46/bcd-manifest-before.txt`.
- Isolated worktree `D:\Documents\GitHub\fcitx5-windows-next-wip-bcd` on branch
  `wip/orphan-bcd-20260918`, created from `c159be0`.
- Exactly the 17 B/C/D files copied (explicit paths, **not** group A), staged explicitly, committed:

  | item | value |
  | --- | --- |
  | checkpoint commit | `c09aa5abcb51b253fa25c0f53b4e9f6156fa899f` |
  | message | `wip: preserve orphan config candidate and qa changes` |
  | paths | 17 |
  | manifest equality | **PASS** (17 vs 17 byte-identical) |

### 2.2 Group A de-duplication (three proofs, then removal)

| proof | command / method | result |
| --- | --- | --- |
| 1. durability | `git rev-parse 4ead534b…` + `git ls-remote origin refs/heads/c2c/windui-replayability-iter44` | both resolve to `4ead534bb5634598bd09f63da62d56400380fea7` |
| 2. replay guard | `pwsh tools/verify-windui-vendor-coverage.ps1` (run in the iter44 worktree, from pin + 3 registered patches) | `GUARD_EXIT=0` — 11 covered paths, 168 upstream, 128 vendored checked, 0 unaccounted |
| 3. byte equivalence | `git ls-tree -r` blob SHAs vs `git hash-object` over the worktree's vendored tree | 128 blobs, **0 mismatches, 0 extras** |

Only after all three passed, the 8 modified vendored paths were restored with explicit-path
`git checkout --` and the 4 A-only untracked paths (including the vendored `Cargo.lock`) were
removed. No `checkout .`, no `restore .`, no `clean` was used anywhere. Porcelain went **29 → 17**,
exactly B/C/D.

### 2.3 Durability and clean end state

- `wip/orphan-bcd-20260918` pushed; remote ref confirmed `=` `c09aa5a…`.
- `c2c/windui-replayability-iter44` re-confirmed durable at `4ead534…`.
- B/C/D removed from the original worktree with explicit paths only. Porcelain **17 → 0**,
  `HEAD = c159be0 = origin/main`.
- Reversibility probe: `git cat-file -e` exit 0 for sampled paths on the checkpoint branch.
- Byte-equivalence evidence: `bcd-manifest-before.txt` and `bcd-manifest-after.txt`, both 17 lines,
  identical SHA-256 `F94F729E5C2CBFF07FED0144273456AA33695AC36A28AF4B99FE3F9E32EFA725`.

## 3. Iteration 47 — split by responsibility and landing gate

Split performed on a separate verification branch
`c2c/orphan-wip-triage` in worktree `D:\Documents\GitHub\fcitx5-windows-next-triage`, based on
`c159be0`. Nothing was cherry-picked into `main`.

No file mixed responsibilities, so the split is file-level; hunk splitting was not required.
Basis for classification (each candidate diff read in full):

- `rust/config-core/src/config_core.rs` (+38): `UI_LANGUAGE_*` constants and `validate_snapshot`
  switched to the 8-locale `UI_LANGUAGE_VALUES` → prerequisite.
- `rust/control-core/src/lib.rs` (+220/−46): 8-locale Windows LANGID → locale-file mapping,
  `ascii_wide` helper, `config_locale_file_for_override` rewrite, updated tests → prerequisite.
- `rust/config-poc/src/main.rs` (all 7 hunks): candidate-count row no longer guarded by
  `CandidateLayoutMode::Scroll`, `frozen_settings_model`/`require_languages`/`apply_language`
  widened to the 9-language policy, two new tests, adapter test widened to 5 layouts × 9 page sizes
  → prerequisite.
- `CMakeLists.txt` (+10): `--comparison-screenshot` arguments for label-slot snapshot tests. The
  flag is implemented in `candidate_poc.rs` (3 occurrences) → bound to the Candidate commit, not the
  prerequisite.

### 3.1 Candidate commits

| # | commit | message | paths |
| --- | --- | --- | --- |
| 1 | `e5a1d69f5f67691d363ecbe9d513175ac3f4d495` | `settings: restore Iteration 40 locale policy and candidate page-size prerequisite` | 10 — 6 × `locales/*.json`, `rust/config-core/tests/locale_contract.rs`, `rust/config-core/src/config_core.rs`, `rust/config-poc/src/main.rs`, `rust/control-core/src/lib.rs` |
| 2 | `240d76224e4269e7be0e90bc58cc1e394b980a8e` | `candidate: preserve Candidate runtime and UI layout work` | 5 — `rust/candidate-core/src/axis_layout.rs`, `rust/candidate-core/src/frame_update.rs`, `rust/candidate-core/src/bin/fcitx5_ui.rs`, `rust/candidate-core/src/bin/candidate_poc.rs`, `CMakeLists.txt` |

Path accounting: union of the two commits = 15 paths vs the 17-path checkpoint. The two omissions
are exactly the intended ones; extras = 0.

### 3.2 Landing gate results

| gate | command | result |
| --- | --- | --- |
| whitespace | `git diff --check` | PASS |
| root lock | `git status --porcelain -- Cargo.lock` | unchanged |
| new dependencies | `git diff c159be0 HEAD -- "**/Cargo.toml"` | 0 changes |
| Rust tests x64 | `cargo test --locked -p <crate> --target x86_64-pc-windows-msvc` | config-core 28, control-core 68, config-poc 66, candidate-core 116 — all 0 failed, exit 0 |
| Rust tests x86 | same, `--target i686-pc-windows-msvc` | 28 / 68 / 66 / 116 — all 0 failed, exit 0 |
| locale contract | included in the `fcitx5-config-core` suite (`tests/locale_contract.rs`) | 28 tests, all green |
| CTest lanes x64 | `ctest -C Debug -R "<8 lanes>"` on `out/build/windows-x64-dev` | **8/8 passed**, 0 failed |
| CTest lanes x86 | same on `out/build/windows-x86-dev` | **8/8 passed**, 0 failed |
| product gate x64 | `pwsh tools/verify-product.ps1 pr -Architecture x64` | **EXIT=0**, `100% tests passed, 0 tests failed out of 81`, `78.00 sec` |

Lane set (both architectures): `release-plugin-source-contract`, `config-ui-behavior-contract`,
`config-ui-visual-contract`, `config-ui-interaction-coverage`, `rust-config-ui-preview-qa`,
`rust-config-poc-contract`, `candidate-ui-ux-contract`, `candidate-ui-live-presentation-contract`.

Naming deviation (recorded, not treated as a pass): the contract's `source-contract` does not exist
in this configuration (`ctest -N -R "^source-contract$"` → `Total Tests: 0`). The nearest existing
equivalents, `release-plugin-source-contract` and `brand-resource-contract`, were run and passed.

x86 product lane: `verify-product.ps1` accepts `-Architecture x86`, but only the contract-required
x64 product gate was run. The x86 side is covered by the same 8 lanes plus all crate full tests
under an independent Debug configuration. The x64 result is therefore **not** described as
dual-architecture product verification.

Environment note: the first x64 Release attempt failed at `[14/19] Building Rust fcitx5-bootstrap
CLI` with `Copy-Item: tools/build-rust-bootstrap-cli.ps1:60 Access to the path
'…\out\cargo-target\x86_64-pc-windows-msvc\release\fcitx5-bootstrap.exe' is denied.`, accompanied
by `Blocking waiting for file lock on build directory`. The cause was Kaspersky quarantining that
executable (confirmed by the user). After the file was restored, an unchanged re-run returned
EXIT=0 with 81/81 green. This is an environment (antivirus) artifact, not a candidate-content
defect.

### 3.3 Verdicts

| candidate | verdict | reason |
| --- | --- | --- |
| Settings prerequisite `e5a1d69` | **landable** | every gate above green |
| Candidate runtime/UI `240d762` | **landable** (independent of P3) | every gate above green; not landed by P3 |
| `tests/integration/tsf_notepad_e2e_test.cpp` (+1) | **CHECKPOINTED / not landed** | no independent task basis for a +1-line Notepad E2E change; remains on the checkpoint branch |
| `docs/tasks/status.md` (+1) | **not carried** | WIP status text must not enter product commits |
| group A vendored WindUI (12 paths) | **not carried in the worktree** | byte-equivalent to the durable `4ead534` + replay patch; authoritative copy is the recovery branch |

No test was deleted, no unrelated WIP was repaired, and no frozen acceptance criterion was
rewritten.

## 4. Delivered artifacts

| artifact | identity |
| --- | --- |
| checkpoint branch | `wip/orphan-bcd-20260918` = `c09aa5abcb51b253fa25c0f53b4e9f6156fa899f` (pushed) |
| verification branch | `c2c/orphan-wip-triage` = `240d76224e4269e7be0e90bc58cc1e394b980a8e` (pushed) |
| prerequisite commit | `e5a1d69f5f67691d363ecbe9d513175ac3f4d495` |
| candidate commit | `240d76224e4269e7be0e90bc58cc1e394b980a8e` |
| group A durable copy | `4ead534bb5634598bd09f63da62d56400380fea7` on `c2c/windui-replayability-iter44` (pushed) |
| A replay patch | `third_party/patches/wind-ui-rust/windui-local-product-delta.patch`, SHA-256 `26FF6699BCD73145AE9B021820C304DA5348727BD2288D5AC43CEC155B0BA918` |
| base | `c159be0` |
| manifests | `%TEMP%/c2c-iter46/bcd-manifest-{before,after}.txt`, SHA-256 `F94F729E5C2CBFF07FED0144273456AA33695AC36A28AF4B99FE3F9E32EFA725` |

Original worktree end state: porcelain = 0, `HEAD = c159be0`, zero bytes lost.

## 5. P3 stacked base

P3 must start from the following stack, in this order:

1. `c159be0` — current `main` (also carries the Iteration 43/44 reports).
2. integrate `4ead534bb5634598bd09f63da62d56400380fea7` — WindUI replayable UIA recovery commit on
   `c2c/windui-replayability-iter44` (14 whitelist paths; verified byte-exact replay).
3. integrate the **verified Settings prerequisite**
   `e5a1d69f5f67691d363ecbe9d513175ac3f4d495` from `c2c/orphan-wip-triage`.
4. only then begin P3 content.

`240d762` (Candidate runtime/UI) is deliberately **not** part of the P3 base; it is an independent
candidate whose integration belongs to the Candidate workstream.

## 6. Deferred / still open

- Not executed in this goal: P3 Iteration 45 content and the P4 preview-owner read-only audit.
- MANUAL-PENDING (unchanged): real-host Narrator/NVDA, real-host visual parity for 081/082/083,
  REL-01 signing/UAC/Win7 evidence, TSF R3-03 cutover gating.
- 084 UNSAFE-SAFETY-COMMENT-COVERAGE: 480 undocumented unsafe sites still pending; delegation was
  unavailable because the shared weekly model quota was exhausted.
