# Fcitx5 for Windows Next

Fcitx5 for Windows Next is a native Windows frontend and distribution layer for
Fcitx5. It is designed around a small in-process TSF DLL, an out-of-process
Fcitx engine, an on-demand Direct2D/DirectWrite candidate window, a native WTL
configuration app, and a signed package/update system.

The project goal is simple to say and hard to earn: after installation, a user
selects **Fcitx5 for Windows Next** from `Win + Space` and types normally. They
should not need to start helper EXEs by hand, understand internal protocols, or
recover from a frozen host application because an input-method component failed.

## Current engineering baseline

The repository is being developed as a production Windows input-method
distribution with strict automated gates for build, runtime behavior, security,
packaging and release artifacts.

Existing code covers TSF, IPC, launcher, engine, candidate UI, config, packages,
updater and release tooling. Acceptance is based on fresh tests and artifact
checks for the code being changed, not on historical green runs.

## Architecture in one screen

```text
Host app
  ↓
fcitx5-tsf.dll
  small COM/TSF boundary, EditSession commit, UILess projection, bounded IPC
  ↓
fcitx5-launcher.exe
  per-user/per-session lifecycle, tray status, engine/UI supervision
  ↓
fcitx5-engine.exe
  Fcitx5 instance, InputContext authority, CandidateModel, config ownership
  ↓
fcitx5-ui.exe
  on-demand D2D/DWrite candidate rendering from immutable snapshots
```

Important boundaries:

- TSF does not download packages, load addons, run Rime/Lua, render the visual
  candidate window, or own candidate truth.
- Candidate UI does not commit text directly into host applications.
- Engine owns input semantics, candidate identity, context isolation and config.
- Config and package tools are on-demand management surfaces, not permanent
  input-path processes.
- Windows registers one product TSF profile. Pinyin, Rime, Mozc and future
  engines are internal Fcitx input methods, not separate Windows profiles.

## What is implemented

Subject to v1.7 re-acceptance, the repository currently includes:

- x86/x64 native TSF registration and activation paths;
- versioned IPC with deadline/fail-open behavior;
- launcher lifecycle, tray status and crash-loop recovery;
- Fcitx engine integration and candidate model contracts;
- D2D/DWrite candidate UI with horizontal/vertical, scroll-mode behavior,
  configurable opacity, and inline-or-panel preedit display;
- WTL configuration app with startup, presentation, input-method and package
  surfaces;
- signed package repository primitives for addons, input data, themes,
  translations and components;
- update/deployment primitives including generation-aware runtime draining and
  in-use TSF DLL replacement by rename-old → install-new → delayed cleanup;
- release gates for package artifacts, identity, dependency policy, secrets,
  license inventory and text/locale hygiene.

## Build requirements

- Windows development host
- Visual Studio 2022 with MSVC C++ x86/x64 tooling
- Windows SDK
- Visual Studio ATL component (`Microsoft.VisualStudio.Component.VC.ATL`)
- CMake 3.28 or newer
- PowerShell 7 or Windows PowerShell 5.1

The native Config app uses WTL/ATL through the pinned CMake toolchain. No IDE
wizard, designer, GUI automation or manual publish step is part of the build.

## Common build commands

```powershell
./tools/build.ps1 bootstrap
./tools/build.ps1 dev
./tools/build.ps1 test
./tools/build.ps1 package -Architecture all -Configuration Release
```

Useful variants:

```powershell
./tools/build.ps1 dev -Architecture x64
./tools/build.ps1 test -Architecture x86
./tools/build.ps1 clean
```

`dev` and `test` build both x64 and x86 unless an architecture is supplied.
`test` enables the project gates, runs CTest, and executes security, license,
locale and dependency checks.

`package` builds the exact x64/x86 Release artifacts, validates the runtime
surface, creates the portable ZIP and installer, and runs portable smoke checks.
Protected release jobs supply signing and publication secrets; local development
must not fake production signing evidence.

## Tests and validation

The project treats tests as phase evidence, not decoration. Depending on the
change, use the narrowest useful gate first and then widen:

```powershell
ctest --test-dir out/build/windows-x64-dev -C Debug --output-on-failure
./tools/build.ps1 test -Architecture x64
./tools/build.ps1 package -Architecture all -Configuration Release
```

Input-path changes usually require more than unit tests: contract tests,
cross-process integration, fuzz/fault coverage, and when TSF behavior changes,
real-host desktop evidence.

Do not mark a phase complete from old logs after touching related code.

## Installation, UAC and desktop testing

Most development work does not require UAC. Building, packaging, parser tests,
package tests, config tests and candidate renderer tests are user-mode tasks.

UAC is only expected for operations that change machine-level installation or
registration state, such as installer smoke tests, repair/uninstall checks or
system TSF registration. When elevated testing is needed, run the dedicated
installer/desktop scripts and record fresh evidence.

## Package and plugin model

Plugins, input-method data, themes and translations are distributed through the
signed package repository. Package metadata declares dependencies, permissions,
source identity, manifest hash and config surface. The Config app should render
these surfaces through typed APIs instead of maintaining giant Windows-specific
hardcoded maps.

Rime is treated as an internal Fcitx input method/addon package. It is not a
separate TSF profile and is not loaded by the TSF DLL.

## Security posture

The current red lines are deliberate:

- no hooks, SendInput, UI Automation or coordinate-click commit paths;
- no injection or remote `FreeLibrary` into host applications;
- no TSF-side addon loading, networking, package extraction or scripting;
- no unsigned package activation;
- no permanent runtime protocol compatibility shim to hide breaking changes;
- no Restart Manager “kill every host using the input method” default upgrade
  flow.

Runtime updates use deployment-level generation side-by-side draining: each
generation keeps strict internal versioning, while old host processes may
naturally drain on the old generation until they exit.
