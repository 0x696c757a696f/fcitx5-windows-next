# Task Plan — v1.8

This queue exists so Codex can proceed without asking the user for a new prompt after every completed item.

## Advancement policy

- `001` is archived as completed.
- `002` is the current task.
- After an automatable task is green, archive it, update `status.md`, copy the next eligible task into `current.md`, and continue.
- Task `015` contains real-host evidence. Run everything reachable; unavailable cases become `MANUAL-PENDING`. Tasks `016–020` may continue because they are code/product work, but **final stabilization/release cannot be declared complete** until required real-host evidence is actually green.
- Rust R1 starts only after its named C++ semantic/corpus prerequisites are green.
- Rust R2 starts only after launcher/process semantics are frozen in C++.
- Release begins only after required manual evidence and intended migrations are complete.

| Order | Task | State | Prerequisite | Task file |
|---|---|---|---|---|
| 001 | REG-UILESS-001 | COMPLETED | — | completed/001-REG-UILESS-001.md |
| 002 | REG-CTX-002 | CURRENT | 001 | current.md |
| 003 | REG-KEY-INTL-001 | QUEUED | 002 / CandidateModel task completed or independently verified not to conflict | queue/03-REG-KEY-INTL-001.md |
| 004 | REG-PROFILE-001 | QUEUED | 003 KeyEvent contract complete if profile metadata shares the breaking IPC update | queue/04-REG-PROFILE-001.md |
| 005 | REG-FCITX-CAP-001 | QUEUED | 003 protocol normalization complete where key forwarding depends on it | queue/05-REG-FCITX-CAP-001.md |
| 006 | REG-WARMUP-001 | QUEUED | 005 InputContext semantics fixed first | queue/06-REG-WARMUP-001.md |
| 007 | REG-LAUNCHER-LEDGER-001 | QUEUED | C++ semantics must be correct before R2 | queue/07-REG-LAUNCHER-LEDGER-001.md |
| 008 | REG-PROC-PIPE-001 | QUEUED | C++ behavior frozen before R2 | queue/08-REG-PROC-PIPE-001.md |
| 009 | REG-REPO-STATE-001 | QUEUED | previous eligible task | queue/09-REG-REPO-STATE-001.md |
| 010 | REG-PEER-ID-001 | QUEUED | previous eligible task | queue/10-REG-PEER-ID-001.md |
| 011 | REG-INSTALL-UAC-001 | QUEUED | previous eligible task | queue/11-REG-INSTALL-UAC-001.md |
| 012 | STAB-REGISTER-BOOTSTRAP-012 | QUEUED | 011 ownership model should be known before final installer E2E | queue/12-STAB-REGISTER-BOOTSTRAP-012.md |
| 013 | STAB-CAND-LOCALE-013 | QUEUED | 004 single profile metadata contract should be available | queue/13-STAB-CAND-LOCALE-013.md |
| 014 | REG-PKG-WINPATH-001 | QUEUED | previous eligible task | queue/14-REG-PKG-WINPATH-001.md |
| 015 | STAB-HOST-MATRIX-015 | QUEUED | previous eligible task | queue/15-STAB-HOST-MATRIX-015.md |
| 016 | REG-CONFIG-VISUAL-001 | QUEUED | Core stabilization 003–014 should be green; 015 may be MANUAL-PENDING per PLAN | queue/16-REG-CONFIG-VISUAL-001.md |
| 017 | REG-CONFIG-LIVE-001 | QUEUED | 016 visual component system complete | queue/17-REG-CONFIG-LIVE-001.md |
| 018 | REG-CAND-UX | QUEUED | 013 locale metadata available; 017 can consume the new Auto setting | queue/18-REG-CAND-UX.md |
| 019 | REG-BRAND-001 | QUEUED | 004 single profile identity complete; 016 resource/visual system available | queue/19-REG-BRAND-001.md |
| 020 | REG-UPDATE-TSF | QUEUED | 009 repository state + 012 installer/registration semantics + 014 package path corpus should be stable | queue/20-REG-UPDATE-TSF.md |
| R1-01 | RUST-R1-01 | FUTURE-GATED | 014 corpus green; 009 repository-state semantics available where shared | rust/R1-01-PACKAGE-CORE.md |
| R1-02 | RUST-R1-02 | FUTURE-GATED | RUST-R1-01 + 009 complete | rust/R1-02-REPOSITORY.md |
| R1-03 | RUST-R1-03 | FUTURE-GATED | 020 generation contract + R1-01/02 | rust/R1-03-UPDATER-DOWNLOADER.md |
| R1-04 | RUST-R1-04 | FUTURE-GATED | 008 process execution semantics + R1-01 | rust/R1-04-PROVIDER.md |
| R1-05 | RUST-R1-05 | FUTURE-GATED | 011/012 installer semantics + R1-03 | rust/R1-05-DEPLOYER-CONDITIONAL.md |
| R2-01 | RUST-R2-01 | FUTURE-GATED | 007 launcher C++ contract green; 020 generation model if launcher supervises drains | rust/R2-01-LAUNCHER.md |
| R2-02 | RUST-R2-02 | FUTURE-GATED | 008 green | rust/R2-02-CONTROL-PROCESS.md |
| R2-03 | RUST-R2-03 | FUTURE-GATED | R2-02; 012 register/bootstrap contract | rust/R2-03-DIAGNOSTICS.md |
| REL-01 | RELEASE-01 | FUTURE-GATED | All stabilization tasks + required external evidence + intended R1/R2 cutovers | release/REL-01-RELEASE-GATE.md |
| REL-01 | RELEASE-01 | RELEASE-GATED | All stabilization tasks + required external evidence + intended R1/R2 cutovers | release/REL-01-RELEASE-GATE.md |

## Important dependency notes

- `004` and `019` are intentionally separate: `004` establishes the **single Windows TSF profile identity/metadata contract**; `019` finalizes the **penguin brand assets and shell presentation**.
- `014` freezes Windows path semantics before Rust R1.
- `020` freezes TSF generation-draining semantics before the Rust updater/downloader cutover.
- A task may be skipped only when current HEAD already satisfies it **and** the required regression/evidence is present; record `ALREADY-GREEN` with evidence in `status.md`.
