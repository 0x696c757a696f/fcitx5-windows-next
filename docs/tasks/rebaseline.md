# Task Queue Rebaseline

Date: 2026-08-27

HEAD at rebaseline refresh: `6ed75672c95e9b4ec5e82b764d4a0d251faff3dd`

Working tree at rebaseline refresh: only the user's untracked local helper
`fcitx5_context_efficiency_v2.ps1`; it is excluded from repository tasks and commits.

Purpose: interpret the old v1.8 queue against the current repository truth. The old task files remain historical evidence. This file is the current queue interpretation until a later rebaseline replaces it.

Allowed states:

- `ALREADY-GREEN`: current `HEAD` already satisfies the old task and has current regression/evidence.
- `PARTIAL`: meaningful current implementation/evidence exists, but the old task cannot be treated as fully closed for the current product direction.
- `MANUAL-PENDING`: automated preparation/evidence exists where possible, but required external or real-host evidence is missing.
- `TODO`: still needs code/design work and is not blocked by missing external evidence.
- `BLOCKED`: cannot proceed until a named prerequisite or external decision/evidence exists.

## Current Guidance Overlay

The 2026-08-24 guidance changes how this queue is interpreted:

- Existing Windows ports are not architecture authority.
- Fcitx core and upstream addon semantics remain upstream-owned.
- Engine is not permanently all C++; it converges to Rust Engine Product Core plus a thin C++ Fcitx adapter.
- Product-owned non-Engine surfaces continue moving toward Rust authority through gated cutovers;
  Config is no longer treated as a durable WTL/C++ exception.
- New product-owned Windows code defaults to Rust. Already Rust-owned components must not regress
  to C++. C++ is allowed only for direct Fcitx-facing Engine object manipulation, plus temporary
  native adapter/host seams that delegate product semantics to Rust and are tracked until removal or
  replacement.
- Already-cut-over Rust components must not be reverted to C++ because an older task/spec paragraph
  described a former C++ baseline.
- Plugins/addons must support static/built-in and dynamic/package-loaded models, not `Addon == DLL`.
- Candidate actions and surrounding-text/text-edit operations must stay capability-aware instead of freezing today's Fcitx API shape.
- `fcitx-contrib/fcitx5-windows` is completely excluded from architecture authority. Use
  `fcitx/fcitx5` core and each addon upstream for Fcitx semantics; use Windows ports only as
  compatibility case studies when the current task requires them.
- Candidate label/ordinal UX is now frozen as a Rust-owned presentation/layout requirement:
  configurable labels, reserved per-row/per-cell label slots, right-aligned labels inside the slot,
  stable candidate text columns, and selected row/column/item reveal without layout shift.
- Candidate/Config default visual direction is WeChat IME style green: `#07C160` accent, light
  green/white, dark green/black. Qingfeng/WindInput upstream themes and variants remain available;
  project defaults are overrides, not deletion of third-party theme sources.
- Candidate visual quality must use WindInput/Qingfeng's actual candidate-window/view/theme code
  path when local renderer output cannot match it. Do not satisfy this by only documenting
  WindInput as a reference.
- Real-host evidence is never inferred from CTest.

Local execution overlay:

- PowerShell commands use `D:\Program Files\PowerShell\7\pwsh.exe`.
- Python commands use `D:\Dev\pixi\envs\python\python.exe`.
- Rust commands use repo-local Cargo from
  `D:\Documents\GitHub\fcitx5-windows-next\out\toolchains\rust\rustup-home\toolchains\1.98.0-x86_64-pc-windows-msvc\bin\cargo.exe`.
- Prefer repo-local `out\toolchains\fast\sccache-0.17.0\sccache-v0.17.0-x86_64-pc-windows-msvc\sccache.exe`
  on `PATH` where build/test scripts support it.

2026-08-27 refresh:

- The old R3 FUTURE-GATED queue files and old Config UX source tasks have been superseded by
  completed task files and are not executable queue entries.
- `047` through `058` have completed the reachable code-only Settings path through a vendored
  `huanfeng/wind-ui-rust` default Settings shell.
- `059` through `061` are green for label-slot geometry, WindInput/Qingfeng visual adoption, and the
  windui DirectWrite/YaHei text path. The latest visual audit shows that these checks did not prove
  production-like five-candidate vertical density or typed candidate/label/comment typography.
  `062-CANDIDATE-PRODUCTION-VERTICAL-TYPOGRAPHY-001` is therefore the current code-only task while
  `RELEASE-01` remains parked on external/manual evidence.

2026-08-27 requirements integration:

- Fcitx5 Core and upstream addon semantics remain the authority. No addon enters the TSF host;
  native in-process permission/capability metadata is not a sandbox. Core input stays offline.
- Freeze one Rust Config Core contract for `Current`/`Draft`/`Defaults`, validate/diff/transaction,
  read-only Draft preview, atomic commit, last-known-good and safe recovery. GUI, CLI and tests share
  this contract; Rust backend shipping is not Config Cutover Complete.
- Freeze CandidateModel as the single source for renderer/UIA/notification semantics, including stale
  notification coalescing/cancellation and sensitive-context suppression. Accessibility is compositional,
  and low-resource performance is a release gate.
- Add follow-up work for plugin `runtime_abi`/`runtime_build`/provenance and separated user data,
  repository freshness/freeze/mix-and-match/mirror identity, a real Lua/non-Chinese upstream addon
  build matrix, and measured 2-core/4-GB SLO calibration. `runtime_build` is signed provenance/
  diagnostic data, not an ABI equality gate. Current ML-DSA-65 v2 signing remains unchanged;
  ARM64/TUF/RemoteAddon/AppContainer remain future scope.
- Add `071-RUST-TEST-AUTHORITY-CUTOVER-001` after the 065-070 product queue. All new tests default
  to Rust, and Rust-owned product semantics require Rust-authoritative behavior/contract/property/
  fault/fuzz/performance/source-structure coverage. Long-term C++ tests are limited to direct Fcitx
  adapter tests, necessary Win32/COM/ABI adapter tests, and final mixed-binary integration/E2E;
  migration-only differential/golden tests must have a cutover deletion condition.

## P0 Guidance Tasks

| Current-guidance task | Rebaseline state | Evidence | Next action |
|---|---|---|---|
| P0-0 Engine Rust/C++ boundary freeze | `ALREADY-GREEN` | `docs/engine-boundary.md` records Fcitx owners, Rust product-state owners, current call graph, current IPC schema, current lack of Engine Rust ABI, and Fcitx/addon patch inventory; source-contract guards representative markers. | Start Engine E1 only after choosing the Rust protocol/core crate boundary. |
| P0-1 Current HEAD truth | `ALREADY-GREEN` | `docs/current.md` records current shipping architecture, language map, PoC states, red lights, windui Config shell status, and next tasks. | Update only when the real state changes materially. |
| P0-2 Old queue rebaseline | `ALREADY-GREEN` | This file classifies the old queue against current `HEAD`. | Keep PLAN pointed here before automatic advancement decisions. |
| P0-3 Rust dependency/license/SBOM lockstep | `ALREADY-GREEN` | `tools/cargo-inventory.ps1`, `tools/check-dependencies.ps1`, and `tools/generate-sbom.ps1` verify Cargo registry packages against `third_party/dependencies.json`; status records SBOM evidence. | External advisory review remains outside automation. |
| P0-4 Runtime-security scans Rust | `ALREADY-GREEN` | `tools/check-runtime-security.ps1` scans C++ and Rust source and has explicit `Win10`/`Win7` min-OS lanes. | Keep `Win7` lane visibly red until the Rust/MSVC hard import strategy is solved. |
| P0-5 Single TSF profile code convergence | `ALREADY-GREEN` | `docs/tsf-profile-boundary.md` freezes the one-profile contract; Rust TSF registers only `Fcitx5`, unregisters obsolete profile GUIDs as cleanup, and reports `windows_profile_count:1` plus `dynamic_profile_registration:false`. `rust/register-core` stays a DLL/CLSID registration helper and the installer only invokes x64/x86 register helpers, with no per-engine profile list. Source-contract guards these boundaries. | Keep legacy dynamic profile data as cleanup input only; do not reintroduce per-engine Windows profiles without a new explicit product decision. |
| P0-6 Generation draining real E2E | `MANUAL-PENDING` | Automated generation/update contracts exist, but real Word/Chrome old-generation host survival and cleanup evidence is not recorded. | Prepare/run the real-host generation-drain matrix. |

## Old v1.8 Queue Rebaseline

| Task | Rebaseline state | Current evidence | Current interpretation |
|---|---|---|---|
| 001 `REG-UILESS-001` | `ALREADY-GREEN` | Completed task file and source-contract TSF UILess/candidate markers. | Historical stabilization remains green. |
| 002 `REG-CTX-002` | `ALREADY-GREEN` | Rust Candidate model owns per-context freshness and source-contract guards Candidate Rust ownership. | Do not rerun as a C++ CandidateModel task. |
| 003 `REG-KEY-INTL-001` | `ALREADY-GREEN` | Completed key-event task plus source-contract/key tests covering logical text, scan code, AltGr, release, and dead-key markers. | Future work is corpus expansion, not a from-scratch KeyEvent redesign. |
| 004 `REG-PROFILE-001` | `ALREADY-GREEN` | Single product profile identity is Rust/release-identity owned; old `src/tsf/input_profiles.*` and `src/tsf/guids.h` are guarded deleted. | Keep only `Fcitx5` as product profile; audit legacy cleanup separately under P0-5. |
| 005 `REG-FCITX-CAP-001` | `ALREADY-GREEN` | Completed task and current Engine boundary capability vocabulary. | Fcitx capability handling now feeds the Engine capability model. |
| 006 `REG-WARMUP-001` | `ALREADY-GREEN` | Source-contract blocks synthetic warmup key events and requires `warmupHasNoUserState`. | Do not reintroduce user-state warmup. |
| 007 `REG-LAUNCHER-LEDGER-001` | `ALREADY-GREEN` | Launcher state/store policy moved to Rust and old C++ state sources are deleted. | Remaining launcher C++ shell shrink is new Rust migration work, not this old ledger task. |
| 008 `REG-PROC-PIPE-001` | `ALREADY-GREEN` | Rust process-execution core owns bounded drain/timeout behavior; old C++ process-execution files/tests are guarded deleted. | Control/config shell command use must stay on the shared Rust executor. |
| 009 `REG-REPO-STATE-001` | `ALREADY-GREEN` | Package/repository trust state is covered by completed trust/package tasks and Rust package-core. | Later repository UX/workflow changes should be separate tasks. |
| 010 `REG-PEER-ID-001` | `ALREADY-GREEN` | Peer same-principal/session/executable verification policy is Rust-owned in `windows-common-core`; C++ peer wrapper is a Win32 adapter. | No new C++ peer policy owner. |
| 011 `REG-INSTALL-UAC-001` | `MANUAL-PENDING` | Completed task records reachable automation; UAC/admin installer evidence remains external. | Requires real elevated installer/uninstaller evidence. |
| 012 `STAB-REGISTER-BOOTSTRAP-012` | `ALREADY-GREEN` | Rust register/bootstrap policy and current profile identity gates are present. | Keep registry/elevation adapter thin. |
| 013 `STAB-CAND-LOCALE-013` | `ALREADY-GREEN` | Candidate UI drives DWrite locale from snapshot content metadata; source-contract blocks metadata polling and hardcoded `zh-CN`. | Current green. |
| 014 `REG-PKG-WINPATH-001` | `ALREADY-GREEN` | Rust package-core/path corpus and dependency checks are current. | New package path work must extend the Rust corpus. |
| 015 `STAB-HOST-MATRIX-015` | `MANUAL-PENDING` | Historical task records missing real-host matrix. | Final stabilization/release remains gated. |
| 016 `REG-CONFIG-VISUAL-001` | `ALREADY-GREEN` | Automated Config visual/resource/live preview evidence exists, and later `052` through `058` added redraw, design tokens, windui code adoption, default windui Settings shell, and screenshot/source-contract evidence. | Release readiness still requires manual/real-host evidence, but this old visual task is no longer the active Config technology blocker. |
| 017 `REG-CONFIG-LIVE-001` | `ALREADY-GREEN` | Config live preview tests and Candidate preview integration are recorded. | Preserve while deciding Config implementation route. |
| 018 `REG-CAND-UX` | `ALREADY-GREEN` | Candidate UX/orientation/scroll/presentation contracts are green and Rust candidate core owns model/layout/interaction. | Future work is boundary consolidation and candidate-action upstream alignment. |
| 019 `REG-BRAND-001` | `MANUAL-PENDING` | Assets exist, but public brand/manual presentation evidence remains pending. | Product polish remains P2. |
| 020 `REG-UPDATE-TSF` | `MANUAL-PENDING` | Automated generation model exists, but current guidance requires real old/new TSF generation host E2E. | Covered by P0-6. |
| 021 `CONFIG-UX-001` | `ALREADY-GREEN` | Old Config UX slices are completed and later 047/052-058 replaced the engineering-oriented default with a windui Settings shell. | Future UX changes are new tasks, not a rerun of this old item. |
| 022 `CONFIG-UX-002` | `ALREADY-GREEN` | Same evidence class as 021 plus windui default shell evidence. | Current green for old queue interpretation. |
| 023 `CONFIG-UX-003` | `ALREADY-GREEN` | Same evidence class as 021 plus windui default shell evidence. | Current green for old queue interpretation. |
| 024 `CONFIG-UX-004` | `ALREADY-GREEN` | Same evidence class as 021 plus windui default shell evidence. | Current green for old queue interpretation. |
| 025 `CONFIG-UX-005` | `ALREADY-GREEN` | Same evidence class as 021 plus windui default shell evidence. | Current green for old queue interpretation. |
| 026 `CONFIG-UX-006` | `ALREADY-GREEN` | Package/update surface automation is covered by 049; shell polish is covered by 058. | Production online package evidence remains manual under plugin/release gates. |
| 027 `CONFIG-UX-007` | `ALREADY-GREEN` | Same evidence class as 021 plus windui default shell evidence. | Current green for old queue interpretation. |
| 028 `CONFIG-UX-008` | `ALREADY-GREEN` | Same evidence class as 021 plus package smoke/plugin lifecycle automation. | Production online evidence remains manual under plugin/release gates. |
| 029 `TRUST-001` | `ALREADY-GREEN` | Trust design and package-core trust implementation are recorded. | Current green. |
| 030 `TRUST-002` | `ALREADY-GREEN` | Trust follow-up recorded. | Current green. |
| 031 `TRUST-003` | `ALREADY-GREEN` | Verifier decision and implementation evidence recorded. | Current green. |
| 032 `TRUST-004` | `ALREADY-GREEN` | Repository/package trust follow-up recorded. | Current green. |
| 033 `TRUST-005` | `ALREADY-GREEN` | Repository/package trust follow-up recorded. | Current green. |
| 034 `TRUST-006` | `ALREADY-GREEN` | Repository/package trust follow-up recorded. | Current green. |
| 035 `PLUGIN-LIFECYCLE-001` | `MANUAL-PENDING` | Real online install/update lifecycle evidence remains external. | Keep plugin lifecycle as manual pending until exercised. |
| 036 `CONFIG-QA-001` | `ALREADY-GREEN` | Rust black-box Config QA harness exists and completed evidence is recorded. | Use as evidence for Config spike comparisons, not as a product-ready Config claim. |
| R1-01 `RUST-R1-01` | `ALREADY-GREEN` | Package core Rust authority and later Cargo/SBOM lockstep evidence. | Current green. |
| R1-02 `RUST-R1-02` | `ALREADY-GREEN` | Repository/package Rust migration evidence recorded. | Current green. |
| R1-03 `RUST-R1-03` | `ALREADY-GREEN` | Updater/downloader/deployer Rust CLI wiring and runtime-security lane evidence. | Keep downloader as only source-network owner. |
| R1-04 `RUST-R1-04` | `ALREADY-GREEN` | Rust provider is authoritative. | Current green. |
| R1-05 `RUST-R1-05` | `ALREADY-GREEN` | Deployer is Rust/default-built and the old C++ deployer shell was deleted. | Current green for old gate; no C++ deployer resurrection. |
| R2-01 `RUST-R2-01` | `PARTIAL` | Launcher state/path/tray/command/frame policy is Rust-owned, but process/job/tray/window/pipe serving shell remains C++. | Continue launcher shell cutover where behavior corpus is frozen. |
| R2-02 `RUST-R2-02` | `PARTIAL` | Control/process execution has many Rust-owned slices, but remaining Control/config/package command shell surfaces are still C++. | Continue non-Engine C++ shrink through focused slices. |
| R2-03 `RUST-R2-03` | `PARTIAL` | Diagnostics/status JSON and shared common-core slices exist, but diagnostics product surface is not fully Rust-owned. | Continue as product-owned Rust migration. |
| R3-01 `RUST-R3-CANDIDATE-POC` | `PARTIAL` | Candidate model/layout/interaction are Rust-owned and old C++ domain headers/tests are deleted; C++ D2D/DWrite renderer/window remains as adapter. | New candidate domain code defaults to Rust; renderer/window C++ is tolerated only as a visual adapter until equivalent renderer migration evidence exists. |
| R3-02 `RUST-R3-CONFIG-POC` | `ALREADY-GREEN` | Shipping Config uses Rust for the product executable and default interactive GUI shell; the vendored windui Settings shell is the no-argument route, while legacy C++ WTL and Rust Win32/D2D hosts are regression/QA-only. | Full release claims still depend on Stage 4 manual/real-host evidence. |
| R3-03 `RUST-R3-TSF-POC` | `MANUAL-PENDING` | Shipping Rust TSF automated gates are green and old C++ TSF sources are deleted; real-host matrix remains missing. | Do not declare release-ready until real-host evidence is recorded. |
| 059 `CANDIDATE-LABEL-SLOT-RUST-DRAWING-001` | `ALREADY-GREEN / VISUAL-FOLLOWUP-QUEUED` | Rust `candidate-core` now owns candidate label formatting, label-slot planning, reserved hidden labels, selected item/row/column reveal, render segments, and Rust PoC drawing evidence. The Settings Appearance page keeps scroll mode visible outside Advanced and the embedded preview passes the scroll-mode flag into Rust layout. | Preserve the Rust-owned label-slot/scroll evidence; 060 owns WindInput/Qingfeng-derived green visual adoption. |
| 060 `CANDIDATE-WINDINPUT-QINGFENG-GREEN-VISUAL-001` | `ALREADY-GREEN` | WindInput/Qingfeng-derived Rust visual plan, WeChat-green light/dark defaults, and six layout/theme screenshots are recorded. | Preserve the third-party source/license and typed palette evidence. |
| 061 `CANDIDATE-MICROSOFT-YAHEI-RUST-TEXT-RENDERER-001` | `ALREADY-GREEN` | Candidate PoC bitmap text uses the vendored windui DirectWrite path with Microsoft YaHei UI, DPI awareness, and 150% screenshot evidence. | Do not interpret renderer/font selection alone as proof of production typography quality. |
| 062 `CANDIDATE-PRODUCTION-VERTICAL-TYPOGRAPHY-001` | `COMPLETED / FULL-CJK-GLYPH-VISIBILITY-GREEN` | Rust `QingfengCandidateTheme` owns typography and the shared visual plan no longer adds an unbudgeted text offset. x64/x86 reports assert `typography_text_fits=true`; matching screenshots visibly contain complete `水` and `收` glyphs beside `~b`/`~d`. | Preserve the full-glyph fail-fast contract; rectangle non-overlap alone is not acceptable evidence. |
| 063 `CONFIG-WINDUI-PLUGIN-MANAGER-001` | `COMPLETED / REAL-CONTROL-OPERATIONS-GREEN` | Default Rust windui Config shows the pinned 21-entry catalog in a scrollable left pane, fixed details/actions on the right, and real package state/errors from bounded typed Control operations. | Production online install/update evidence remains manual until signed repository artifacts exist. |
| REL-01 `RELEASE-01` | `BLOCKED` | Real-host, installer/UAC, production plugin lifecycle assets, generation-drain, Narrator/NVDA, and release signing/provenance evidence remain incomplete. Config code-only shell polish is green through 058. | Release gate cannot advance from local-only evidence. |

## Next Eligible Work

`RELEASE-01` stays parked. The first code-only task is `065-CONFIG-CORE-TRANSACTION-CONTRACT-001`,
then `066-CANDIDATE-SEMANTIC-A11Y-001`, `067-PLUGIN-PROVENANCE-DATA-BOUNDARY-001`,
`068-REPOSITORY-FRESHNESS-MIRROR-IDENTITY-001`, `069-PLUGIN-ECOSYSTEM-BUILD-MATRIX-001`, and
`070-LOW-RESOURCE-SLO-CALIBRATION-001`. After those automated slices, the next eligible task is
`071-RUST-TEST-AUTHORITY-CUTOVER-001`, subject to its explicit inventory and migration prerequisites.
These are Rust-first and must keep C++ limited to the direct Fcitx adapter, upstream native addon, or
thin Windows ABI/renderer adapter.

1. Prepare P0-6 real generation-drain E2E, installer/UAC, plugin lifecycle, Narrator/NVDA, and Rust
   TSF host matrix evidence before any release-readiness claim.
2. Continue non-Engine C++ shrink only when PLAN adds an explicit Rust migration task with a frozen
   behavior corpus and regression evidence.
3. Future Config work should build on the vendored windui shell, not re-open the old Win32 default
   host.
