# Task Queue Rebaseline

Date: 2026-08-24

HEAD at rebaseline start: `9d5e4d38e180a7d84f8e3986ce24766399ad9c2e`

Working tree at rebaseline start: clean.

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
- Product-owned non-Engine surfaces continue moving toward Rust authority through gated cutovers.
- Plugins/addons must support static/built-in and dynamic/package-loaded models, not `Addon == DLL`.
- Candidate actions and surrounding-text/text-edit operations must stay capability-aware instead of freezing today's Fcitx API shape.
- Real-host evidence is never inferred from CTest.

## P0 Guidance Tasks

| Current-guidance task | Rebaseline state | Evidence | Next action |
|---|---|---|---|
| P0-0 Engine Rust/C++ boundary freeze | `ALREADY-GREEN` | `docs/engine-boundary.md` records Fcitx owners, Rust product-state owners, current call graph, current IPC schema, current lack of Engine Rust ABI, and Fcitx/addon patch inventory; source-contract guards representative markers. | Start Engine E1 only after choosing the Rust protocol/core crate boundary. |
| P0-1 Current HEAD truth | `ALREADY-GREEN` | `docs/current.md` records current shipping architecture, language map, PoC states, red lights, and next tasks. | Update only when the real state changes materially. |
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
| 016 `REG-CONFIG-VISUAL-001` | `PARTIAL` | Automated Config visual/resource/live preview evidence exists, but current guidance says Config UX still needs product-level technology choice and polish. | Do not treat old green as product-ready Config. |
| 017 `REG-CONFIG-LIVE-001` | `ALREADY-GREEN` | Config live preview tests and Candidate preview integration are recorded. | Preserve while deciding Config implementation route. |
| 018 `REG-CAND-UX` | `ALREADY-GREEN` | Candidate UX/orientation/scroll/presentation contracts are green and Rust candidate core owns model/layout/interaction. | Future work is boundary consolidation and candidate-action upstream alignment. |
| 019 `REG-BRAND-001` | `MANUAL-PENDING` | Assets exist, but public brand/manual presentation evidence remains pending. | Product polish remains P2. |
| 020 `REG-UPDATE-TSF` | `MANUAL-PENDING` | Automated generation model exists, but current guidance requires real old/new TSF generation host E2E. | Covered by P0-6. |
| 021 `CONFIG-UX-001` | `PARTIAL` | Old Config UX slices completed, but current guidance says current Config remains too engineering-oriented. | Fold into Config technology/product spike. |
| 022 `CONFIG-UX-002` | `PARTIAL` | Same evidence class as 021. | Fold into Config technology/product spike. |
| 023 `CONFIG-UX-003` | `PARTIAL` | Same evidence class as 021. | Fold into Config technology/product spike. |
| 024 `CONFIG-UX-004` | `PARTIAL` | Same evidence class as 021. | Fold into Config technology/product spike. |
| 025 `CONFIG-UX-005` | `PARTIAL` | Same evidence class as 021. | Fold into Config technology/product spike. |
| 026 `CONFIG-UX-006` | `PARTIAL` | Same evidence class as 021, with package/update surface dependencies. | Fold into Config technology/product spike and plugin/update UX. |
| 027 `CONFIG-UX-007` | `PARTIAL` | Same evidence class as 021. | Fold into Config technology/product spike. |
| 028 `CONFIG-UX-008` | `PARTIAL` | Same evidence class as 021. | Fold into Config technology/product spike. |
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
| R1-05 `RUST-R1-05` | `ALREADY-GREEN` | Deployer is Rust/default-built; ADR kept thin C++ where appropriate historically, but current CMake builds Rust deployer. | Current green for old gate; no C++ deployer resurrection. |
| R2-01 `RUST-R2-01` | `PARTIAL` | Launcher state/path/tray/command/frame policy is Rust-owned, but process/job/tray/window/pipe serving shell remains C++. | Continue launcher shell cutover where behavior corpus is frozen. |
| R2-02 `RUST-R2-02` | `PARTIAL` | Control/process execution has many Rust-owned slices, but remaining Control/config/package command shell surfaces are still C++. | Continue non-Engine C++ shrink through focused slices. |
| R2-03 `RUST-R2-03` | `PARTIAL` | Diagnostics/status JSON and shared common-core slices exist, but diagnostics product surface is not fully Rust-owned. | Continue as product-owned Rust migration. |
| R3-01 `RUST-R3-CANDIDATE-POC` | `PARTIAL` | Candidate model/layout/interaction are Rust-owned and old C++ domain headers/tests are deleted; C++ D2D/DWrite renderer/window remains. | Current guidance supports keeping renderer C++ for now while shrinking duplicate state. |
| R3-02 `RUST-R3-CONFIG-POC` | `PARTIAL` | Rust Config PoC and QA are green, but shipping Config is still C++/WTL and guidance requires a formal technology spike/ADR. | Create Config spike/ADR task before cutover. |
| R3-03 `RUST-R3-TSF-POC` | `MANUAL-PENDING` | Shipping Rust TSF automated gates are green and old C++ TSF sources are deleted; real-host matrix remains missing. | Do not declare release-ready until real-host evidence is recorded. |
| REL-01 `RELEASE-01` | `BLOCKED` | Real-host, installer/UAC, plugin lifecycle, generation-drain, Config product polish, and release signing/provenance evidence remain incomplete. | Release gate cannot advance. |

## Next Eligible Work

1. Continue non-Engine C++ shrink only where Rust owner and regression evidence already exist.
2. Prepare P0-6 real generation-drain E2E and Rust TSF host matrix evidence.
3. Start Engine E1 planning only from the frozen call graph/schema in `docs/engine-boundary.md`.
4. Prepare Config technology/product spike before adding more Config UI controls.
