# Phase 3 acceptance record

Date: 2026-08-17  
Status: accepted

## Delivered contract

- A single native x64 `fcitx5-engine.exe` owns the Fcitx5 Instance and event loop. All Fcitx calls
  are scheduled onto one `EventDispatcher`; pipe workers never call Fcitx directly.
- Each `(client PID, TSF context ID)` has a distinct Fcitx InputContext. Protocol v3 returns commit,
  preedit, UTF-8 caret, engine epoch, composition ID and monotonic revision.
- The TSF frontend creates, updates and ends a real TSF composition. Navigation/edit keys are
  consumed only while composition is active; Ctrl/Alt shortcuts remain host-owned.
- The engine loads the pinned Windows Fcitx5 fork, libime and Chinese Addons from paths relative to
  its executable. It sets the process DLL directory before addon loading and requires the exact
  same-user/session client identity established in Phase 2.

## Acceptance evidence

| Specification result | Evidence | Result |
|---|---|---|
| Composition and commit | automated TSF mock: `n` preedit, caret, Space, Chinese commit, EndComposition | Pass |
| Real Pinyin | native engine: `ni`, visible preedit, Space, non-empty Chinese commit | Pass |
| Context isolation | two context IDs alternate focus; queued commit/preedit remain context-local | Pass |
| Engine restart | persisted epoch source advances strictly after process restart | Pass |
| One engine for both TSFs | x64 and x86 MSVC clients pass against the same x64 CLANG64 engine | Pass |
| Dispatcher ownership | every runtime operation is scheduled on the Fcitx event-loop thread | Pass |
| 60 Hz input | 120 paced requests complete in 2.01 s without accumulated backlog | Pass |
| Startup/resources | ready in 2.30–2.58 s; idle CPU 0 us/1 s; private bytes 54–55 MiB | Pass |
| Hot-path budget | new context 3.7–4.5 ms; hot key 1.1–1.5 ms; commit 0.25–0.41 ms | Pass |
| Supply chain | archive SHA/signing fingerprint, source commits, patch hashes and package versions pinned | Pass |

The reference-machine measurements are acceptance baselines, not universal latency promises.
`tools/test-fcitx.ps1` rebuilds the native engine and repeats the real x64/x86 test. The general
MSVC gate continues to run protocol, fault, launcher, security, formatting, license and SCA checks.

## Reference-driven deltas

- The frozen gaboolic fork supplies the Windows portability baseline and the mature executable-
  relative DLL/data-path setup.
- Official Fcitx lifecycle semantics commit pending preedit on focus-out; the isolation regression
  therefore verifies separated queued results instead of assuming unfinished preedit survives a
  focus switch.
- The Windows CLANG64 repository has no xkbcommon package. Phase 3's required table path is supplied
  by libime and Chinese Addons dictionaries; the upstream keyboard addon remains disabled rather
  than introducing an unpinned replacement.
- Latest libime 1.1.15 and Chinese Addons 5.1.13 require Fcitx 5.1.20 and are incompatible with the
  frozen Windows fork. The accepted compatible band is libime 1.1.14 plus Chinese Addons 5.1.12.

## Security note

MSYS2's `06-windows-files.post` copies the machine hosts file into a missing portable `/etc/hosts`.
That copy triggered ESET `Win32/Qhost` on this machine; it was not an engine artifact. Bootstrap now
creates a two-entry localhost file before the first MSYS process starts. It never modifies the
Windows hosts file and does not weaken antivirus policy.
