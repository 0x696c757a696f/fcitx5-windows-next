# Task Plan — v1.8

This queue exists so Codex can proceed without asking the user for a new prompt after every completed item.

## Advancement policy

- `001` is archived as completed.
- `R1-01` is the current task.
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
| 015 | STAB-HOST-MATRIX-015 | QUEUED | previous eligible task | queue/15-STAB-HOST-MATRIX-015.md |
| 016 | REG-CONFIG-VISUAL-001 | COMPLETED / AFTER-SCREENSHOT-PENDING | Core stabilization 003–014 should be green; 015 may be MANUAL-PENDING per PLAN | completed/016-REG-CONFIG-VISUAL-001.md |
| 017 | REG-CONFIG-LIVE-001 | COMPLETED / REVERIFIED-AFTER-016 | 016 visual component system complete | completed/017-REG-CONFIG-LIVE-001.md |
| 018 | REG-CAND-UX | COMPLETED | 013 locale metadata available; 017 can consume the new Auto setting | completed/018-REG-CAND-UX.md |
| 019 | REG-BRAND-001 | MANUAL-PENDING | 004 single profile identity complete; 016 resource/visual system available | queue/19-REG-BRAND-001.md |
| 020 | REG-UPDATE-TSF | COMPLETED | 009 repository state + 012 installer/registration semantics + 014 package path corpus should be stable | completed/020-REG-UPDATE-TSF.md |
| 021 | CONFIG-UX-001 | COMPLETED | 016/017/018 green; user explicitly queued Settings UX follow-up | completed/021-CONFIG-UX-001.md |
| 022 | CONFIG-UX-002 | COMPLETED | 021 | completed/022-CONFIG-UX-002.md |
| 023 | CONFIG-UX-003 | COMPLETED | 021 | completed/023-CONFIG-UX-003.md |
| 024 | CONFIG-UX-004 | COMPLETED | 023 | completed/024-CONFIG-UX-004.md |
| 025 | CONFIG-UX-005 | COMPLETED | 023/024 | completed/025-CONFIG-UX-005.md |
| 026 | CONFIG-UX-006 | COMPLETED | 021; TRUST tasks may be required for official online repository enablement | completed/026-CONFIG-UX-006.md |
| 027 | CONFIG-UX-007 | COMPLETED | 021 | completed/027-CONFIG-UX-007.md |
| 028 | CONFIG-UX-008 | COMPLETED | package smoke follow-up | completed/028-CONFIG-UX-008.md |
| 029 | TRUST-001 | COMPLETED | user selected PQC-first repository trust design | completed/029-TRUST-001.md |
| 030 | TRUST-002 | COMPLETED | 029 | completed/030-TRUST-002.md |
| 031 | TRUST-003 | COMPLETED | 029/030 + verifier implementation decision | completed/031-TRUST-003.md |
| 032 | TRUST-004 | COMPLETED | 030/031 | completed/032-TRUST-004.md |
| 033 | TRUST-005 | COMPLETED | 031/032 | completed/033-TRUST-005.md |
| 034 | TRUST-006 | COMPLETED | 026 + 031 | completed/034-TRUST-006.md |
| 035 | PLUGIN-LIFECYCLE-001 | MANUAL-PENDING | 026 + 031/033 where real online install is exercised | completed/035-PLUGIN-LIFECYCLE-001.md |
| R1-01 | RUST-R1-01 | CURRENT | 014 corpus green; 009 repository-state semantics available where shared | current.md |
| R1-02 | RUST-R1-02 | FUTURE-GATED | RUST-R1-01 + 009 complete | queue/37-RUST-R1-02.md |
| R1-03 | RUST-R1-03 | FUTURE-GATED | 020 generation contract + R1-01/02 | queue/38-RUST-R1-03.md |
| R1-04 | RUST-R1-04 | FUTURE-GATED | 008 process execution semantics + R1-01 | queue/39-RUST-R1-04.md |
| R1-05 | RUST-R1-05 | FUTURE-GATED | 011/012 installer semantics + R1-03 | queue/40-RUST-R1-05.md |
| R2-01 | RUST-R2-01 | FUTURE-GATED | 007 launcher C++ contract green; 020 generation model if launcher supervises drains | queue/41-RUST-R2-01.md |
| R2-02 | RUST-R2-02 | FUTURE-GATED | 008 green | queue/42-RUST-R2-02.md |
| R2-03 | RUST-R2-03 | FUTURE-GATED | R2-02; 012 register/bootstrap contract | queue/43-RUST-R2-03.md |
| R3-01 | RUST-R3-CANDIDATE-POC | FUTURE-GATED | Candidate UX/layout/UILess contracts frozen; R1/R2 not blocked | queue/44-RUST-R3-CANDIDATE-POC.md |
| R3-02 | RUST-R3-CONFIG-POC | FUTURE-GATED | Settings operation model and typed Control/config/package boundaries frozen; R1/R2 not blocked | queue/45-RUST-R3-CONFIG-POC.md |
| R3-03 | RUST-R3-TSF-POC | FUTURE-GATED | TSF C++ behavior corpus frozen; host matrix evidence available; Candidate/Config Rust decision not blocking | queue/46-RUST-R3-TSF-POC.md |
| REL-01 | RELEASE-01 | FUTURE-GATED | All stabilization tasks + required external evidence + intended R1/R2 cutovers | release/REL-01-RELEASE-GATE.md |
| REL-01 | RELEASE-01 | RELEASE-GATED | All stabilization tasks + required external evidence + intended R1/R2 cutovers | release/REL-01-RELEASE-GATE.md |

## Important dependency notes

- `004` and `019` are intentionally separate: `004` establishes the **single Windows TSF profile identity/metadata contract**; `019` finalizes the **penguin brand assets and shell presentation**.
- `014` freezes Windows path semantics before Rust R1.
- `020` freezes TSF generation-draining semantics before the Rust updater/downloader cutover.
- Long-term language direction: Engine remains the C++ Fcitx5 island; TSF, Candidate, Config, package/update/launcher/control/provider/diagnostics and other product-owned layers should move toward Rust only through explicit gated migration tasks with differential evidence.
- `R3-01`/`R3-02`/`R3-03` keep Candidate, Config, and TSF Rust paths open without destabilizing the current C++ baselines. They are PoC/differential gates, not authorization to rewrite those components during R1/R2.
- A task may be skipped only when current HEAD already satisfies it **and** the required regression/evidence is present; record `ALREADY-GREEN` with evidence in `status.md`.
