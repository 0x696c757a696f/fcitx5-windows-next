# Host compatibility matrix — 2026-08-20

Task: `STAB-HOST-MATRIX-015`  
Build identity: `b43890f6a173c75aeaec826e25e5f29735ba8bd1` plus current working tree queue changes  
Evidence class: `EXTERNAL_EVIDENCE`

This record does not claim unrun real-host evidence passed. It records only
reachable checks in this Codex desktop environment and marks unavailable rows as
`MANUAL-PENDING`.

## Current machine inventory

Collected with PowerShell/CIM on 2026-08-20.

| Field | Value |
|---|---|
| OS | Microsoft Windows 10 IoT 企业版 LTSC |
| Version / build | 10.0.19044 / 19044 |
| OS architecture | 64-bit |
| Session | Console |
| Machine | LENOVO 82DN |
| CPU | Intel(R) Core(TM) i7-10710U CPU @ 1.10GHz |
| Memory | 17,008,873,472 bytes |
| User | `DESKTOP-IQ4M4HU\Benedict` |

## Reachable automated evidence

| Case | Result | Evidence |
|---|---|---|
| x64 TSF/mock-engine COM key path | PASS | `tsf-key-commit-e2e` passed in `out/build/windows-x64-dev` |
| x86 TSF/mock-engine COM key path | PASS | `tsf-key-commit-e2e` passed in `out/build/windows-x86-dev` |
| Candidate UI device/config/UILess/locale smoke | PASS | x64/x86 `candidate-ui-device-smoke`, `candidate-ui-safe-mode-smoke`, `candidate-ui-uiless-presentation-contract`, `candidate-ui-scroll-expansion-contract`, `candidate-ui-locale-contract`, `candidate-ui-live-config-reflow`; x64 also `candidate-ui-live-presentation-contract` |
| Package and source hardening adjacent to host gate | PASS | x64/x86 package core/fuzz/source-contract checks passed in this batch |
| Prohibited special compatibility behavior | PASS for source gate | `source-contract` passed; no new game-specific hook, injection, SendInput emulation, process memory access, or anti-cheat evasion code was added for this task |

The existing `fcitx5_tsf_notepad_e2e_test` is an interactive harness that drives
Notepad with `SendInput`. Because the current task/user boundary forbids adding
or relying on SendInput-style emulation as compatibility behavior, this run did
not use that harness as new 015 evidence. A maintainer can still run a manual
Notepad smoke with real keyboard input and attach its result here.

## Host/application availability observed

| Host/application | Availability in this environment | 015 result |
|---|---|---|
| Notepad | `C:\Windows\system32\notepad.exe` exists | MANUAL-PENDING: real keyboard/TSF smoke not run |
| Windows Terminal | `C:\Users\Benedict\AppData\Local\Microsoft\WindowsApps\wt.exe` exists | MANUAL-PENDING: terminal IME scenario not run |
| Word | `winword.exe` not resolved | MANUAL-PENDING |
| Excel | `excel.exe` not resolved | MANUAL-PENDING |
| Chrome | `chrome.exe` not resolved | MANUAL-PENDING |
| Edge | `msedge.exe` not resolved | MANUAL-PENDING |
| VS Code | `Code.exe` not resolved | MANUAL-PENDING |
| League of Legends client | `LeagueClientUx.exe` not resolved | MANUAL-PENDING |
| Vanguard tray/service UI | `vgtray.exe` not resolved | MANUAL-PENDING |

## Required external matrix still pending

| Required row | Status | Exact missing evidence |
|---|---|---|
| Windows 11 primary | MANUAL-PENDING | OS build, artifact identity, Notepad/Word/Excel/Edge or Chrome/VS Code/Terminal, High Contrast, keyboard-only, Narrator/NVDA where available |
| Windows 10 baseline real desktop | MANUAL-PENDING | Same-lineage install/register/input/candidate/uninstall and RDP or second interactive session; this Codex run did not perform real desktop typing |
| DPI and monitors | MANUAL-PENDING | 100/125/150/200% DPI, multi-monitor/cross-DPI placement, fullscreen/borderless behavior |
| Windows 7 SP1 Legacy VM | MANUAL-PENDING | KB2533623/import check, install→register→input→candidate→uninstall, x64 OS with x86+x64 host/TIP smoke |
| Old x86 / legacy host | MANUAL-PENDING | 32-bit host on x64 Windows, IMM32/CUAS/legacy fullscreen/windowed/Alt+Enter cases |
| Office/browser/editor/terminal | MANUAL-PENDING | Word/Excel, Chrome/Edge, VS Code and Windows Terminal versioned host smokes |
| League of Legends + Vanguard | MANUAL-PENDING | Chat composition/candidate/commit, non-text control passthrough, Alt-Tab, window modes, Vanguard normal operation with no bypass code |
| RDP / multi-session | MANUAL-PENDING | SID/session IPC isolation with simultaneous or remote session |

No compatibility code was changed for 015. PLAN allows later code-only tasks to
continue while this row remains `MANUAL-PENDING`; release/stabilization must not
be declared complete until the required real-host evidence above is attached.
