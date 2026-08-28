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
- The default interactive Config GUI must consume the vendored `huanfeng/wind-ui-rust` code path,
  not merely copy its palette or mention it as a reference. Its first shipped Rust shell must follow
  the `settings-input.png` settings layout family: titlebar, left search/navigation, right
  settings pages, grouped cards, native windui controls, and a bottom status/action bar. The old
  Win32/WTL/D2D Settings host remains only a QA/regression/native-gap adapter until replaced.
- Config default accent is WeChat IME style green `#07C160`: light mode is green/white and dark
  mode is green/black. Keep vendored/upstream Qingfeng and windui themes available; product default
  palette overrides must not delete or rewrite third-party theme sources.
- Candidate label/ordinal layout is now a frozen UX requirement: labels are configurable
  presentation only, every row/cell reserves a stable label slot, label text aligns inside that
  slot, candidate text starts after a fixed gap, and selected-row/column/item reveal modes must not
  shift neighboring rows or columns. Candidate drawing work defaults to Rust; remaining native
  renderer/window code is adapter-only until equivalent visual/DPI/a11y/perf evidence is green.
- Candidate visual quality must use WindInput/Qingfeng's actual candidate-window/theme code path
  when the local renderer cannot match it. Directly adopt or port the MIT-licensed `wind-ui`
  candidate window/view/theme implementation with license/source evidence; do not satisfy this
  requirement by saying the code is merely a reference.
- Already-cut-over Rust components must not be reverted to C++ because of historical task wording.
- `R3-03` is the current task only for remaining TSF real-host/manual evidence and focused
  package-candidate usability regressions.
- `047-CONFIG-UX-009` is the next eligible code/product task once the currently reachable
  R3-03 automated checks are recorded; `R3-03` real-host evidence may remain `MANUAL-PENDING`.
- After an automatable task is green, archive it, update `status.md`, and copy the next eligible task into `current.md`.
- Continue automatically only when the next task is tightly coupled to the same subsystem and materially reuses the context already loaded.
- At a subsystem or phase boundary, record a minimal handoff in `status.md` and stop the session so the next task starts with fresh context; do not ask the user merely to advance the queue.
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
| 016 | REG-CONFIG-VISUAL-001 | COMPLETED / EMBEDDED-EMOJI-PREVIEW-GREEN / SUPERSEDED-BY-WINDUI-SCREENSHOT-GREEN | Core stabilization 003–014 should be green; 015 may be MANUAL-PENDING per PLAN; later 052-058 provide current Rust Settings screenshot/visual evidence | completed/016-REG-CONFIG-VISUAL-001.md |
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
| 048 | CONFIG-RUST-CUTOVER-001 | MANUAL-PENDING / STAGE2-RUST-CONFIG-BACKEND-SHIPPED-GREEN / SHIPPING-RUST-CONFIG-EXE-GREEN / PACKAGE-CONFIG-SMOKE-GREEN / SIDE-BY-SIDE-DIFFERENTIAL-GREEN / RUST-SETTINGS-UI-PREVIEW-QA-GREEN / RUST-CANDIDATE-PREVIEW-FIDELITY-QA-GREEN / RUST-NUMERIC-APPEARANCE-QA-GREEN / RUST-SYSTEM-FONT-PICKER-QA-GREEN / RUST-FONT-PERSISTENCE-RESTART-QA-GREEN / RUST-ADVANCED-NUMERIC-APPEARANCE-QA-GREEN / RUST-PAGE-VISIBILITY-INPUTMETHODS-QA-GREEN / RUST-LANGUAGE-SELECTOR-QA-GREEN / RUST-PACKAGE-PLUGIN-PAGE-QA-GREEN / RUST-KEYBOARD-TABSTOP-QA-GREEN / STAGE4-REAL-HOST-EVIDENCE-PENDING / PACKAGE-GATE-ENGINE-IDLE-FIXED-BY-050 | 047 theme/preview/operation contract green; user authorized non-Engine Rust cutover; Stage 2 shipping is green, legacy C++ Config is non-authoritative baseline only, 058 makes windui the default code shell, and full Stage 4/release evidence remains external/manual | completed/048-CONFIG-RUST-CUTOVER-001.md |
| 049 | PLUGIN-LIFECYCLE-STABILITY-001 | MANUAL-PENDING / AUTOMATED-LIFECYCLE-GREEN / CONFIG-PACKAGE-ACTIONS-QA-GREEN / PACKAGE-CONFIG-SMOKE-GREEN / ONLINE-RELEASE-ASSETS-PENDING / PACKAGE-GATE-ENGINE-IDLE-FIXED-BY-050 | 048 Stage 2 Rust Config Backend Shipped green unless plugin lifecycle blocks Config itself; production online endpoint/key evidence remains MANUAL-PENDING | completed/049-PLUGIN-LIFECYCLE-STABILITY-001.md |
| 050 | ENGINE-IDLE-PACKAGE-GATE-050 | COMPLETED / PACKAGE-GATE-GREEN / REAL-ENGINE-X64-X86-GREEN | 049 automated lifecycle evidence green; fixes the package/release gate blocker before `REL-01` | completed/050-ENGINE-IDLE-PACKAGE-GATE-050.md |
| 051 | ENGINE-E4-TRANSPORT-FRAMING-001 | COMPLETED / E4-SERVER-PIPE-TRANSPORT-RUST-GREEN | `REL-01` is parked on external/manual evidence; PLAN permits later code-only Engine Rust-migration work to continue without claiming release readiness | completed/051-ENGINE-E4-TRANSPORT-FRAMING-001.md |
| 052 | CONFIG-STAGE4-VISUAL-REDRAW-001 | COMPLETED / VISUAL-REDRAW-QA-GREEN | `REL-01` is parked on external/manual evidence; fixed real Rust Settings visual ghosting/重影 class, owner-drawn navigation, and embedded preview clipping evidence | completed/052-CONFIG-STAGE4-VISUAL-REDRAW-001.md |
| 053 | CONFIG-UIKIT-DESIGN-TOKENS-001 | COMPLETED / RUST-SETTINGS-DESIGN-TOKENS-GREEN | `REL-01` is parked on external/manual evidence; established Rust-owned design tokens before further Win32/D2D visual slices so Config does not keep growing by scattered raw constants | completed/053-CONFIG-UIKIT-DESIGN-TOKENS-001.md |
| 054 | CONFIG-WINDOW-EFFECTS-ADAPTER-001 | COMPLETED / RUST-WINDOW-EFFECTS-ADAPTER-GREEN | 053; added Rust-owned Win7/Win10/Win11 progressive-enhancement adapter contract before using dark titlebar, rounded corners, Mica/system backdrop, or version-specific DWM attributes | completed/054-CONFIG-WINDOW-EFFECTS-ADAPTER-001.md |
| 055 | CONFIG-D2D-SETTINGS-SURFACE-001 | COMPLETED / RUST-SETTINGS-SURFACE-QA-GREEN | 053/054; introduced bounded Settings Surface paint-plan components while retaining native HWND controls where behavior/a11y/IME semantics matter | completed/055-CONFIG-D2D-SETTINGS-SURFACE-001.md |
| 056 | CONFIG-STAGE4-A11Y-DPI-QA-001 | MANUAL-PENDING / AUTOMATED-STAGE4-QA-GREEN | 053/055; automated keyboard/focus/page/no-overlap/DPI/high-contrast-marker/candidate-preview gates are frozen, but Narrator/NVDA and real Win7/Win10/Win11 host evidence remain unavailable locally | completed/056-CONFIG-STAGE4-A11Y-DPI-QA-001.md |
| 057 | CONFIG-WINDUI-ADOPTION-001 | COMPLETED / WINDUI-RUST-CODE-CONSUMED-GREEN / DEFAULT-SHELL-CLOSED-BY-058 | User explicitly required actual `huanfeng/wind-ui-rust` code adoption for ugly Config GUI/UX while 056 real-host evidence remains parked; vendored windui is now a Rust path dependency and self-check consumes actual windui element construction; 058 closes the no-arg default GUI cutover | completed/057-CONFIG-WINDUI-ADOPTION-001.md |
| 058 | CONFIG-WINDUI-SETTINGS-SHELL-001 | COMPLETED / WINDUI-SETTINGS-SHELL-DEFAULT-GREEN | 057; 056 real-host evidence may remain MANUAL-PENDING because this is a code-only GUI cutover slice | completed/058-CONFIG-WINDUI-SETTINGS-SHELL-001.md |
| 059 | CANDIDATE-LABEL-SLOT-RUST-DRAWING-001 | COMPLETED / CANDIDATE-LABEL-SLOT-RUST-DRAWING-GREEN / WINDINPUT-QINGFENG-VISUAL-FOLLOWUP-QUEUED | `REL-01` is parked on external/manual evidence; user explicitly required configurable candidate numbers, reserved row/column label space, stable alignment, Rust drawing, keeping scroll mode visible in Settings, and replacing PoC-level visuals with WindInput/Qingfeng-derived candidate rendering | completed/059-CANDIDATE-LABEL-SLOT-RUST-DRAWING-001.md |
| 060 | CANDIDATE-WINDINPUT-QINGFENG-GREEN-VISUAL-001 | COMPLETED / WINDINPUT-QINGFENG-WECHAT-GREEN-VISUAL-GREEN | 059 label-slot contract green; product default must be WeChat-green light/dark while preserving Qingfeng/upstream theme sources and using actual WindInput candidate visual code where local quality falls short | completed/060-CANDIDATE-WINDINPUT-QINGFENG-GREEN-VISUAL-001.md |
| 061 | CANDIDATE-MICROSOFT-YAHEI-RUST-TEXT-RENDERER-001 | COMPLETED / WINDUI-DWRITE-YAHEI-UI-150PCT-GREEN | 060 visual defaults green; Rust Candidate screenshots now use the Qingfeng windui DirectWrite path, per-monitor DPI awareness, DPI-scaled text, and 150% visual goldens | completed/061-CANDIDATE-MICROSOFT-YAHEI-RUST-TEXT-RENDERER-001.md |
| 062 | CANDIDATE-PRODUCTION-VERTICAL-TYPOGRAPHY-001 | COMPLETED / FULL-CJK-GLYPH-VISIBILITY-GREEN | Rejected the old non-overlap-only proof; shared width budgeting now includes the exact label/text gap, x64/x86 reports assert `typography_text_fits=true`, and fresh matching screenshots show complete rows 4/5 beside comments | completed/062-CANDIDATE-PRODUCTION-VERTICAL-TYPOGRAPHY-001.md |
| 063 | CONFIG-WINDUI-PLUGIN-MANAGER-001 | COMPLETED / REAL-CONTROL-OPERATIONS-GREEN | Default windui Config shows the pinned 21-entry catalog, reads authoritative package state, and runs bounded refresh/install/update/state/remove/repair operations off the UI thread | completed/063-CONFIG-WINDUI-PLUGIN-MANAGER-001.md |
| 064 | CONFIG-CANDIDATE-PLUGIN-USABILITY-CORRECTION-001 | MANUAL-PENDING / AUTOMATED-CORRECTION-GREEN / PRODUCTION-PUBLISH-INPUTS-PREPARED | 062 + 063; Candidate modes and production-input signed Rime repository lifecycle are implemented; protected signing/publication and real-host lifecycle evidence remain external | completed/064-CONFIG-CANDIDATE-PLUGIN-USABILITY-CORRECTION-001.md |
| 065 | CONFIG-CORE-TRANSACTION-CONTRACT-001 | COMPLETED / AUTOMATED-CONTRACT-GREEN / MANUAL-PENDING | Shared Rust Config Current/Draft/Defaults transaction and recovery contract is green; real-host Settings accessibility evidence remains external | completed/065-CONFIG-CORE-TRANSACTION-CONTRACT-001.md |
| 066 | CANDIDATE-SEMANTIC-A11Y-001 | COMPLETED / AUTOMATED-SEMANTIC-CONTRACT-GREEN / MANUAL-PENDING | Rust CandidateModel now owns the renderer/UIA/notification semantic projection, revision-aware notification coalescing, composable capabilities, and sensitive-context privacy policy; real assistive-technology hosts remain external | completed/066-CANDIDATE-SEMANTIC-A11Y-001.md |
| 067 | PLUGIN-PROVENANCE-DATA-BOUNDARY-001 | COMPLETED / AUTOMATED-X64-X86-GREEN / MANUAL-PENDING | Signed v2 manifests now carry bounded runtime ABI/build, typed source provenance, and explicit versioned-program/durable-user-data policy; shared Rust roots enforce canonical, absolute, non-reparse, non-overlapping boundaries and lifecycle preserves external user data | completed/067-PLUGIN-PROVENANCE-DATA-BOUNDARY-001.md |
| 068 | REPOSITORY-FRESHNESS-MIRROR-IDENTITY-001 | SELECTED / CODE-ONLY | 067 package metadata boundary green; extend current v2 repository verification with freshness, freeze/mix-and-match and mirror-identity corpus without claiming TUF | 068-REPOSITORY-FRESHNESS-MIRROR-IDENTITY-001.md |
| 069 | PLUGIN-ECOSYSTEM-BUILD-MATRIX-001 | TODO / CODE-ONLY / EXTERNAL-EVIDENCE-PARTIAL | 067 provenance and 068 freshness contracts green; add Lua and one non-Chinese upstream addon as real pinned build/package/sign-input/install/load slices after the existing Rime production-input path | 069-PLUGIN-ECOSYSTEM-BUILD-MATRIX-001.md |
| 070 | LOW-RESOURCE-SLO-CALIBRATION-001 | TODO / CODE-ONLY / EXTERNAL-EVIDENCE-PARTIAL | 066 semantics, 068 repository path, and 069 ecosystem matrix green; add repeatable low-resource measurement harness and initial SLO evidence, leaving real 2-core/4-GB calibration manual-pending | 070-LOW-RESOURCE-SLO-CALIBRATION-001.md |
| 071 | RUST-TEST-AUTHORITY-CUTOVER-001 | TODO / CODE-ONLY / MIGRATION | 065-070 automated acceptance green; inventory and migrate C++ tests by ownership after the Rust product-plane queue, while preserving required adapter and final mixed-binary evidence | 071-RUST-TEST-AUTHORITY-CUTOVER-001.md |
| 072 | CANDIDATE-WIN32-RENDERER-RUST-CUTOVER-001 | TODO / CODE-ONLY / MIGRATION | 071 ownership ledger green plus 061/062/066 Candidate evidence; move ui_main.cpp product state and renderer host toward Rust, retain only necessary native adapter seams, then delete obsolete C++ authority | 072-CANDIDATE-WIN32-RENDERER-RUST-CUTOVER-001.md |
| REL-01 | RELEASE-01 | RELEASE-GATED / EXTERNAL-EVIDENCE-PENDING | 050 + required external evidence + intended Rust cutovers; code-only queue may continue while this remains parked | release/REL-01-RELEASE-GATE.md |

## Important dependency notes

- `004` and `019` are intentionally separate: `004` establishes the **single Windows TSF profile identity/metadata contract**; `019` finalizes the **penguin brand assets and shell presentation**.
- `014` freezes Windows path semantics before Rust R1.
- `020` freezes TSF generation-draining semantics before the Rust updater/downloader cutover.
- Long-term language direction: direct Fcitx-facing Engine object manipulation remains the C++ Fcitx5 adapter island; Engine product protocol/state/validation/revision/generation/policy/IPC/diagnostics moves toward Rust. TSF, Candidate, Config, package/update/launcher/control/provider/diagnostics and other product-owned layers default to Rust ownership, with gated differential evidence before replacing any remaining shipping adapter. Config is no longer a durable WTL/C++ exception; it now has an explicit Rust cutover queue item.
- Completed queue source files are removed from `docs/tasks/queue` after archiving; only active or
  not-yet-selected queue files should remain there.
- Historical docs removed during cleanup were mined first: retained Hi-DPI, package lifecycle,
  threat-model, performance, and release requirements now live in current queue files, release gate,
  `docs/current.md`, or `docs/tasks/status.md`.
- `R3-01`/`R3-02`/`R3-03` are interpreted against current reality, not their old FUTURE-GATED
  wording: TSF shipping has cut over to Rust; Candidate domain model/layout/interaction are
  Rust-owned; Config remains a WTL/Win32 shell only as a temporary migration host with Rust product
  logic/PoC evidence. Historical C++ baselines are regression corpora, not a reason to reintroduce
  C++ ownership.
- `REL-01` is parked as release-gated until production release assets, signing/key evidence, and
  required real-host/manual compatibility evidence exist. Code-only migration tasks may continue
  while preserving that release gate; do not claim release readiness from local CTest/package smoke.
- `053` through `058` are the current code-only Settings modernization queue opened from the
  Rust-owned Config GUI direction. `057` vendors and consumes `wind-ui-rust`; `058` makes its
  settings-shell code path the default interactive GUI and keeps the old Win32 host as QA/regression
  scaffolding only. They do not make `REL-01` releasable; release remains gated on external/manual
  evidence and production signed artifacts.
- `059` carries the newly frozen Candidate label/ordinal UX into an executable Rust drawing slice:
  configurable ordinal style/display/scope, stable reserved label slots in horizontal/vertical/grid
  layouts, selected row/column reveal without text-column shift, screenshots across layouts, and no
  new C++ candidate-domain or drawing-state ownership.
- `060` upgrades the reachable Candidate/Config visual defaults: Config and Candidate default to
  WeChat-style green (`#07C160`) with light green/white and dark green/black, while Qingfeng and
  upstream theme sources remain available. Candidate defaults must use a CJK-first font chain
  (`Microsoft YaHei`, `Microsoft YaHei UI`, `system`). Candidate screenshots must be driven by
  WindInput/Qingfeng candidate visual code/tokens, not a plain engineering proof window.
- `061` hardens the Candidate screenshot text path itself: bitmap text must be drawn by the
  Rust-owned renderer with a Microsoft YaHei CJK-first path, and source contracts must prevent
  silent regression to rough GDI `DrawTextW` screenshot text.
- `062` closes the remaining production-typography evidence gap: the Rust Qingfeng visual plan owns
  candidate/label/comment font-size and row-height tokens, and a five-candidate vertical screenshot
  proves right-aligned labels, annotations, stable text origins, and normal Windows IME density.
- `065` through `070` are the post-release-parked code-only queue. They freeze and then implement
  Config transaction/recovery, Candidate semantic accessibility, plugin provenance/data boundaries,
  repository freshness/mirror identity, a real upstream-addon ecosystem matrix, and measured
  low-resource gates in dependency order. `runtime_build` is signed provenance/diagnostic data, not
  an ABI equality gate. The queue does not claim ARM64, TUF, RemoteAddon/AppContainer, production
  signing, UAC, or real-host evidence.
- `071` is deliberately after `070` and is not part of the 065-070 product slices. It inventories
  every existing C++ test target/file as `KEEP`, `MIGRATE`, or `DELETE`, migrates Rust-owned unit,
  contract, property, fault, fuzz, performance, and source-structure coverage to Rust in bounded
  slices, and retains only direct Fcitx adapter, necessary Win32/COM/ABI adapter, and final
  mixed-binary integration/E2E C++ tests. Temporary differential/golden tests require an explicit
  deletion condition; CMake/CTest routing and x64/x86 continuity remain required.
- Every future Rust task must use a worktree-local `CARGO_TARGET_DIR`; before implementation its
  execution agent must fully read `ponytail`, `rust-skills`, and `tdd`. New product logic is Rust;
  C++ is limited to the direct Fcitx adapter, upstream native addon, or a thin Windows ABI/renderer
  adapter. Do not create a second Config truth, generic GUI framework, or permanent protocol dual stack.
- A task may be skipped only when current HEAD already satisfies it **and** the required regression/evidence is present; record `ALREADY-GREEN` with evidence in `status.md`.
