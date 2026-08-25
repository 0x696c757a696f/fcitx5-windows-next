# Task Plan — v1.8

This queue exists so Codex can proceed without asking the user for a new prompt after every completed item.

## Current rebaseline

The queue below is historical v1.8 ordering. Current execution must first consult
`docs/tasks/rebaseline.md`, which classifies every old queue item against the
current `HEAD` as `TODO`, `ALREADY-GREEN`, `PARTIAL`, `MANUAL-PENDING`, or
`BLOCKED`.

Do not mechanically restart or re-implement a historical task whose current
state is `ALREADY-GREEN` or whose old scope is now superseded by the
2026-08-24 Engine Rust/Fcitx upstream boundary guidance.

## Advancement policy

- `001` is archived as completed.
- `001` through `036`, `RUST-R1-*`, `RUST-R2-*`, and the current automated parts of
  `RUST-R3-*` have been rebaselined against the 2026-08-24 Engine/Fcitx upstream guidance.
- New product-owned Windows code defaults to Rust. Already-cut-over Rust code must not be changed
  back to C++ because an older task file called the C++ implementation a baseline. C++ is reserved
  for the direct Fcitx-facing Engine adapter island; any remaining Win32/COM/D2D/WTL code is a
  temporary native adapter/host that must delegate product semantics to Rust and be queued for
  removal or replacement once equivalent evidence exists.
- Config migration uses explicit stages: Stage 2 `Rust Config Backend Shipped` may ship
  non-interactive Rust-owned Config paths (`fcitx5-config.exe` headless/test CLI,
  package/install/update/remove, import/export, config read/write, schema validation, migration,
  diagnostics, CI automation) with no fallback to the old GUI implementation; Stage 4
  `Rust Config Cutover Complete` is reserved for the real interactive Settings GUI, controls,
  embedded candidate preview, plugin pages, DPI/dark-mode/keyboard/accessibility, persistence, and
  real Windows QA. Stage 2 release notes must not claim full Config migration.
- Already-cut-over Rust components must not be reverted to C++ because of historical task wording.
- `R3-03` is the current task only for remaining TSF real-host/manual evidence and focused
  package-candidate usability regressions.
- `047-CONFIG-UX-009` is the next eligible code/product task once the currently reachable
  R3-03 automated checks are recorded; `R3-03` real-host evidence may remain `MANUAL-PENDING`.
- After an automatable task is green, archive it, update `status.md`, copy the next eligible task into `current.md`, and continue.
- Task `015` contains real-host evidence. Run everything reachable; unavailable cases become `MANUAL-PENDING`. Tasks `016–020` may continue because they are code/product work, but **final stabilization/release cannot be declared complete** until required real-host evidence is actually green.
- Rust R1 starts only after its named C++ semantic/corpus prerequisites are green.
- Rust R2 starts only after launcher/process semantics are frozen in C++.
- Release begins only after required manual evidence and intended migrations are complete.

| Order | Task | State | Prerequisite | Task file |
|---|---|---|---|---|
| 001 | REG-UILESS-001 | COMPLETED | — | completed/001-REG-UILESS-001.md |
| 002 | REG-CTX-002 | COMPLETED | 001 | completed/002-REG-CTX-002.md |
| 003 | REG-KEY-INTL-001 | COMPLETED | 002 / CandidateModel task completed or independently verified not to conflict | completed/003-REG-KEY-INTL-001.md |
| 004 | REG-PROFILE-001 | COMPLETED | 003 KeyEvent contract complete if profile metadata shares the breaking IPC update | completed/004-REG-PROFILE-001.md |
| 005 | REG-FCITX-CAP-001 | COMPLETED | 003 protocol normalization complete where key forwarding depends on it | completed/005-REG-FCITX-CAP-001.md |
| 006 | REG-WARMUP-001 | COMPLETED | 005 InputContext semantics fixed first | completed/006-REG-WARMUP-001.md |
| 007 | REG-LAUNCHER-LEDGER-001 | COMPLETED | C++ semantics must be correct before R2 | completed/007-REG-LAUNCHER-LEDGER-001.md |
| 008 | REG-PROC-PIPE-001 | COMPLETED | C++ behavior frozen before R2 | completed/008-REG-PROC-PIPE-001.md |
| 009 | REG-REPO-STATE-001 | COMPLETED | previous eligible task | completed/009-REG-REPO-STATE-001.md |
| 010 | REG-PEER-ID-001 | COMPLETED | previous eligible task | completed/010-REG-PEER-ID-001.md |
| 011 | REG-INSTALL-UAC-001 | MANUAL-PENDING | previous eligible task | completed/011-REG-INSTALL-UAC-001.md |
| 012 | STAB-REGISTER-BOOTSTRAP-012 | COMPLETED | 011 ownership model should be known before final installer E2E | completed/012-STAB-REGISTER-BOOTSTRAP-012.md |
| 013 | STAB-CAND-LOCALE-013 | COMPLETED | 004 single profile metadata contract should be available | completed/013-STAB-CAND-LOCALE-013.md |
| 014 | REG-PKG-WINPATH-001 | COMPLETED | previous eligible task | completed/014-REG-PKG-WINPATH-001.md |
| 015 | STAB-HOST-MATRIX-015 | MANUAL-PENDING | previous eligible task | completed/015-STAB-HOST-MATRIX-015.md |
| 016 | REG-CONFIG-VISUAL-001 | COMPLETED / EMBEDDED-EMOJI-PREVIEW-GREEN / AFTER-SCREENSHOT-PENDING | Core stabilization 003–014 should be green; 015 may be MANUAL-PENDING per PLAN | completed/016-REG-CONFIG-VISUAL-001.md |
| 017 | REG-CONFIG-LIVE-001 | COMPLETED / REVERIFIED-AFTER-016 | 016 visual component system complete | completed/017-REG-CONFIG-LIVE-001.md |
| 018 | REG-CAND-UX | COMPLETED | 013 locale metadata available; 017 can consume the new Auto setting | completed/018-REG-CAND-UX.md |
| 019 | REG-BRAND-001 | MANUAL-PENDING | 004 single profile identity complete; 016 resource/visual system available | completed/019-REG-BRAND-001.md |
| 020 | REG-UPDATE-TSF | COMPLETED | 009 repository state + 012 installer/registration semantics + 014 package path corpus should be stable | completed/020-REG-UPDATE-TSF.md |
| 021 | CONFIG-UX-001 | COMPLETED | 016/017/018 green; user explicitly queued Settings UX follow-up | completed/021-CONFIG-UX-001.md |
| 022 | CONFIG-UX-002 | COMPLETED | 021 | completed/022-CONFIG-UX-002.md |
| 023 | CONFIG-UX-003 | COMPLETED | 021 | completed/023-CONFIG-UX-003.md |
| 024 | CONFIG-UX-004 | COMPLETED | 023 | completed/024-CONFIG-UX-004.md |
| 025 | CONFIG-UX-005 | COMPLETED | 023/024 | completed/025-CONFIG-UX-005.md |
| 026 | CONFIG-UX-006 | COMPLETED | 021; TRUST tasks may be required for official online repository enablement | completed/026-CONFIG-UX-006.md |
| 027 | CONFIG-UX-007 | COMPLETED | 021 | completed/027-CONFIG-UX-007.md |
| 028 | CONFIG-UX-008 | COMPLETED | package smoke follow-up | completed/028-CONFIG-UX-008.md |
| 036 | CONFIG-QA-001 | COMPLETED | user requested software-test style Settings verification before Rust Config PoC | completed/036-CONFIG-QA-001.md |
| 029 | TRUST-001 | COMPLETED | user selected PQC-first repository trust design | completed/029-TRUST-001.md |
| 030 | TRUST-002 | COMPLETED | 029 | completed/030-TRUST-002.md |
| 031 | TRUST-003 | COMPLETED | 029/030 + verifier implementation decision | completed/031-TRUST-003.md |
| 032 | TRUST-004 | COMPLETED | 030/031 | completed/032-TRUST-004.md |
| 033 | TRUST-005 | COMPLETED | 031/032 | completed/033-TRUST-005.md |
| 034 | TRUST-006 | COMPLETED | 026 + 031 | completed/034-TRUST-006.md |
| 035 | PLUGIN-LIFECYCLE-001 | MANUAL-PENDING | 026 + 031/033 where real online install is exercised | completed/035-PLUGIN-LIFECYCLE-001.md |
| R1-01 | RUST-R1-01 | COMPLETED | 014 corpus green; 009 repository-state semantics available where shared | completed/RUST-R1-01.md |
| R1-02 | RUST-R1-02 | COMPLETED | RUST-R1-01 + 009 complete | completed/RUST-R1-02.md |
| R1-03 | RUST-R1-03 | COMPLETED | 020 generation contract + R1-01/02 | completed/RUST-R1-03.md |
| R1-04 | RUST-R1-04 | COMPLETED | 008 process execution semantics + R1-01 | completed/RUST-R1-04.md |
| R1-05 | RUST-R1-05 | COMPLETED / RUST-DEPLOYER-CUTOVER-GREEN | 011/012 installer semantics + R1-03 | completed/RUST-R1-05.md |
| R2-01 | RUST-R2-01 | COMPLETED | 007 launcher C++ contract green; 020 generation model if launcher supervises drains | completed/RUST-R2-01.md |
| R2-02 | RUST-R2-02 | COMPLETED / ADR-SPLIT-ADAPTERS | 008 green | completed/RUST-R2-02.md |
| R2-03 | RUST-R2-03 | COMPLETED | R2-02; 012 register/bootstrap contract | completed/RUST-R2-03.md |
| R3-01 | RUST-R3-CANDIDATE-POC | MANUAL-PENDING / AUTOMATED-POC-GREEN / CONFIG-EMBEDDED-EMOJI-PREVIEW-GREEN / CANDIDATE-MODEL-RUST-CUTOVER-GREEN / CANDIDATE-MODEL-HEADER-DELETED / CANDIDATE-INTERACTION-RUST-CUTOVER-GREEN / CANDIDATE-INTERACTION-HEADER-DELETED / CANDIDATE-LAYOUT-RUST-CUTOVER-GREEN / CANDIDATE-LAYOUT-HEADER-DELETED | Candidate UX/layout/UILess contracts frozen; R1/R2 not blocked; user explicitly opened R3 gate; candidate model, interaction, and layout/render-segment semantics are Rust-owned and old C++ sources/obsolete headers/tests/benches are deleted where their Rust corpus is authoritative | completed/RUST-R3-CANDIDATE-POC.md |
| R3-02 | RUST-R3-CONFIG-POC | COMPLETED / AUTOMATED-POC-GREEN | Settings operation model and typed Control/config/package boundaries frozen; R1/R2 not blocked | completed/RUST-R3-CONFIG-POC.md |
| R3-03 | RUST-R3-TSF-POC | MANUAL-PENDING / USER-GATE-OVERRIDE / USER-SHIPPING-CUTOVER-OVERRIDE / SHIPPING-RUST-TSF-X64-X86-GREEN / CXX-SHIPPING-TSF-DELETED / TSF-SUPPORT-ACTIVATION-GUARD-RUST-GREEN / REAL-HOST-MATRIX-PENDING | User explicitly allowed opening this gate and later explicitly authorized deleting/replacing shipping C++ TSF; automated x64/x86 Rust shipping TSF gates and TSF activation-guard Rust support cutover are green, but real host matrix evidence remains MANUAL-PENDING | completed/RUST-R3-TSF-POC.md |
| 047 | CONFIG-UX-009 | COMPLETED / FONT-PERSISTENCE-PACKAGE-GATE-GREEN / RUST-SYSTEM-FONT-PICKER-GREEN / STAGED-APP-QA-GREEN | R3-03 automated package/candidate smoke green; real-host matrix may remain MANUAL-PENDING because this is Settings/Candidate UX product work, not release certification | completed/047-CONFIG-UX-009.md |
| 048 | CONFIG-RUST-CUTOVER-001 | IN-PROGRESS / SIDE-BY-SIDE-DIFFERENTIAL-GREEN / REAL-CANDIDATE-PREVIEW-HOST-REQUIRED | 047 theme/preview/operation contract green; user authorized non-Engine Rust cutover; shipping Config C++ shell is temporary only | current.md |
| 049 | PLUGIN-LIFECYCLE-STABILITY-001 | TODO | 048 Stage 2 Rust Config Backend Shipped green unless plugin lifecycle blocks Config itself; full Stage 4 interactive GUI cutover/candidate-preview QA may remain in 048; production online endpoint/key evidence may remain MANUAL-PENDING | queue/049-PLUGIN-LIFECYCLE-STABILITY-001.md |
| REL-01 | RELEASE-01 | RELEASE-GATED | All stabilization tasks + required external evidence + intended Rust cutovers | release/REL-01-RELEASE-GATE.md |

## Important dependency notes

- `004` and `019` are intentionally separate: `004` establishes the **single Windows TSF profile identity/metadata contract**; `019` finalizes the **penguin brand assets and shell presentation**.
- `014` freezes Windows path semantics before Rust R1.
- `020` freezes TSF generation-draining semantics before the Rust updater/downloader cutover.
- Long-term language direction: direct Fcitx-facing Engine object manipulation remains the C++ Fcitx5 adapter island; Engine product protocol/state/validation/revision/generation/policy/IPC/diagnostics moves toward Rust. TSF, Candidate, Config, package/update/launcher/control/provider/diagnostics and other product-owned layers default to Rust ownership, with gated differential evidence before replacing any remaining shipping adapter. Config is no longer a durable WTL/C++ exception; it now has an explicit Rust cutover queue item.
- Historical docs removed during cleanup were mined first: retained Hi-DPI, package lifecycle,
  threat-model, performance, and release requirements now live in current queue files, release gate,
  `docs/current.md`, or `docs/tasks/status.md`.
- `R3-01`/`R3-02`/`R3-03` are interpreted against current reality, not their old FUTURE-GATED
  wording: TSF shipping has cut over to Rust; Candidate domain model/layout/interaction are
  Rust-owned; Config remains a WTL/Win32 shell only as a temporary migration host with Rust product
  logic/PoC evidence. Historical C++ baselines are regression corpora, not a reason to reintroduce
  C++ ownership.
- A task may be skipped only when current HEAD already satisfies it **and** the required regression/evidence is present; record `ALREADY-GREEN` with evidence in `status.md`.
