# Fcitx5 for Windows Next

This repository implements the native Windows frontend and distribution layer for Fcitx5. The architecture keeps the in-process TSF DLL small and moves Fcitx, addons, candidate rendering, configuration, and package management into explicit out-of-process boundaries.

The project follows the frozen engineering specification v1.5. Work advances by accepted vertical phases; Phases 0 through 6 are implemented, with compatibility evidence tracked separately from automated gates.

## Build

Requirements:

- Windows 10 or newer development host
- Visual Studio 2022 with the MSVC x86/x64 C++ workload and Windows SDK
- Visual Studio ATL component (`Microsoft.VisualStudio.Component.VC.ATL`)
- CMake 3.28 or newer
- PowerShell 7 or Windows PowerShell 5.1

The runtime and Config build use the pinned C++/WTL CMake toolchain and require no IDE UI.

```powershell
./tools/build.ps1 bootstrap
./tools/build.ps1 dev
./tools/build.ps1 test
./tools/build.ps1 package -Architecture all -Configuration Release
./tools/build.ps1 clean
```

`dev` and `test` build both x64 and x86 unless `-Architecture x64` or `-Architecture x86` is supplied. `test` enables MSVC static analysis, runs CTest, and executes the secret, license-inventory, and dependency checks.

Reference pins, source-audit findings, and the reason for the process boundaries are recorded in [docs/reference-baseline.md](docs/reference-baseline.md).

`package` builds and tests the exact x64/x86 Release artifacts, verifies the real Fcitx engine, creates the portable ZIP and Inno installer, then runs a portable move smoke. `tools/test-installer.ps1` performs the elevated install/repair/uninstall smoke and restores the development TSF registration.

## Current scope

Phase 6 adds the C++/WTL Config application, typed Control API, external English/Chinese locales, production-renderer candidate preview, TSF registration/repair utility, portable layout, and Inno installer. Package/addon/update transactions remain Phase 7 work.

Acceptance evidence is recorded per phase under `docs/phase-*-acceptance.md`.
