# Phase 5 implementation and acceptance record

Date: 2026-08-17
Status: automated acceptance passed; external compatibility matrix pending

## Delivered contract

- TSF reads `GUID_PROP_INPUTSCOPE` in the same bounded synchronous edit session as caret state.
  Password, private, PIN and numeric-password scopes fail open to the host before any engine call;
  they therefore cannot enter engine learning, prediction or logging paths. `IS_PASSWORD` is a
  signal rather than a security boundary, consistent with the Microsoft InputScope contract.
- Duplicate `OnTestKeyDown`, orphan Test/KeyUp and synchronous composition termination sequences
  have deterministic regressions. A physical key is sent to the engine only by `OnKeyDown`, and
  normal synchronous `EndComposition` is not treated as an external abort.
- Engine readiness is published from its running event loop. Repeated immediate engine failures
  use the launcher state machine's bounded backoff and enter Safe Mode instead of respawning in a
  tight loop. Safe Mode disables all optional Fcitx addons except pinyin and punctuation, ignores
  user UI configuration/theme, and relaunches the isolated UI in the matching mode.
- Native engine DLL resolution uses dynamically resolved `SetDefaultDllDirectories` and
  `AddDllDirectory`, restricted to application, explicitly added, user and system directories.
  Their absence fails safely without adding post-Win7 hard imports. Windows 7 requires the
  platform update that supplies these loader APIs (KB2533623); no public Win7 support claim is made
  until the VM gate below passes.
- The runtime security gate audits imports for x64/x86 TSF, launcher and UI plus the native engine
  when staged. It rejects network libraries in the host TSF and source capability paths for hooks,
  input replay, remote-thread injection and game-memory access.
- Runtime endpoints remain scoped by user SID and Windows session. Session 0, service accounts and
  secure desktops cannot launch the user engine; named-pipe DACLs are protected and non-inheriting.

## Automated evidence

| Requirement | Evidence | Status |
|---|---|---|
| REG-TSF-001/002 | duplicate TestKeyDown and non-typical Test/KeyUp sequences in real TSF COM E2E | Pass |
| REG-COMP-001 | synchronous `OnCompositionTerminated` during `EndComposition` | Pass |
| REG-IPC-001/002/003 | correlated timeout/late-response, multi-client and bounded cold-start tests retained from Phase 2 | Pass |
| REG-CRASH-001 | real launcher + engine fixture exiting with code 23 reaches Safe Mode without host termination | Pass |
| REG-CTX-001 | 10,000 context switches reject stale snapshots by epoch/context/composition/revision | Pass |
| REG-SEC-001 | InputScope unit matrix and COM property E2E; sensitive Test/KeyDown bypasses engine | Pass |
| REG-DLL-001 automated portion | dual-architecture PE import audit and prohibited-capability source gate | Pass |
| REG-UI-001 automated portion | device-loss and missing/exiting UI isolation plus Safe Mode UI smoke | Pass |
| Real Fcitx normal/Safe Mode | x64 and x86 startup, context, commit, 120-key repeat and settled 1 s idle CPU probe | Pass |
| Dual architecture | Debug x64 and x86: 21/21 CTest plus secret/license/SCA/text gates | Pass |
| Text policy | 114 repository files: UTF-8 without BOM and LF | Pass |

## Release performance evidence

Measured on the reference machine with Release builds. These are trend baselines, not universal
latency promises.

| Benchmark | x64 | x86 | Result |
|---|---:|---:|---|
| key roundtrip p50 / p95 / p99 | 49.5 / 66.7 / 89.2 us | 54.0 / 71.8 / 93.8 us | Pass |
| focus/context churn, 10,000 switches | 523.73 ns/switch | 373.69 ns/switch | Pass |
| TSF lifecycle soak, 10,000 iterations | handle/GDI/USER delta 0/0/0 | handle/GDI/USER delta 0/0/0 | Pass |

## Mature-source decisions

- Weasel callback and composition sequences became replayable TSF regressions rather than
  application-name workarounds.
- Rabbit supplied caret/focus/password scenarios. The implementation uses the official TSF
  InputScope property and does not inherit Rabbit's hook-based input path.
- Moqi and WindInput recovery scenarios are covered by correlated request IDs, stale-response
  rejection, bounded overlapped I/O and the launcher's circuit-breaker state model.
- ImeStudy and ImeModePersistence are pinned as legacy-host scenario sources only. TSF remains the
  sole authoritative frontend.

## Pending environmental evidence

The current machine cannot honestly close the complete Phase 5 compatibility gate. The following
remain required before a public compatibility or Phase 5 release claim:

- Windows 7 SP1 VM with KB2533623: install/register, x86/x64 host smoke and runtime import trace;
- RDP or multiple interactive sessions: simultaneous SID/session endpoint isolation;
- old x86 IMM32/CUAS application and DirectDraw/D3D8/D3D9 full-screen/windowed/Alt+Enter cases;
- Windows Terminal and Microsoft Office host matrix;
- anti-cheat-friendly smoke confirming safe failure with no injection or memory access;
- ProcMon DLL-resolution trace proving no CurrentDir, Temp or Downloads dependency resolution;
- physical focus/caret churn and the Phase 4 Word/Chrome/VS Code, DPI and multi-monitor rows.

Automated substitutes are recorded above, but they do not convert these environment-dependent rows
to Pass. The project must not advertise Windows 7, old-game or full host-matrix support until the
corresponding evidence is attached.
