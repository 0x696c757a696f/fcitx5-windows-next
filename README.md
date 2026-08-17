# Fcitx5 for Windows Next

This repository implements the native Windows frontend and distribution layer for Fcitx5. The architecture keeps the in-process TSF DLL small and moves Fcitx, addons, candidate rendering, configuration, and package management into explicit out-of-process boundaries.

The project follows the frozen engineering specification v1.5. Phases 0 through 8 are implemented;
compatibility evidence is tracked separately from automated gates.

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
# After `package`, protected CI supplies the three FCITX_RELEASE_* secrets:
./tools/build.ps1 release
./tools/build.ps1 clean
```

`dev` and `test` build both x64 and x86 unless `-Architecture x64` or `-Architecture x86` is supplied. `test` enables MSVC static analysis, runs CTest, and executes the secret, license-inventory, and dependency checks.

Reference pins, source-audit findings, and the reason for the process boundaries are recorded in [docs/reference-baseline.md](docs/reference-baseline.md).

`package` builds and tests the exact x64/x86 Release artifacts, verifies the real Fcitx engine, creates the portable ZIP and Inno installer, then runs a portable move smoke. `tools/test-installer.ps1` performs the elevated install/repair/uninstall smoke and restores the development TSF registration.

## Current scope

The native C++/WTL Config includes the online signed repository plugin manager. Package/addon/data/
theme/translation updates use exact dependencies, bounded signed archives, staging, atomic activation,
restart-safe enable/disable/removal, and isolated downloader/deployer/provider processes. Candidate
scroll mode uses the production C++ D2D/DWrite renderer. Stable/Beta/Nightly identities, update-owner
arbitration, previous-known-good rollback, Authenticode, SPDX SBOM, provenance, WinGet and Chocolatey
metadata are part of the protected release path.

`fcitx5-rime` is distributed as an addon package, not built into the host or TSF DLL. A repository
entry must declare exact compatible runtime dependencies. Rime Lua works only when the signed package
set includes the matching `librime-lua` module and its exact `librime` dependency; an ABI, architecture,
missing-dependency, signature, or hash mismatch is rejected instead of producing a partially enabled
addon.

Acceptance evidence is recorded per phase under `docs/phase-*-acceptance.md`.
