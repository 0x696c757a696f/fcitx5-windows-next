# Fcitx5 for Windows Next

This repository implements the native Windows frontend and distribution layer for Fcitx5. The architecture keeps the in-process TSF DLL small and moves Fcitx, addons, candidate rendering, configuration, and package management into explicit out-of-process boundaries.

The project follows the frozen engineering specification v1.4. Work advances by accepted vertical phases; the current implementation baseline is Phase 1A.

## Build

Requirements:

- Windows 10 or newer development host
- Visual Studio 2022 with the MSVC x86/x64 C++ workload and Windows SDK
- CMake 3.28 or newer
- PowerShell 7 or Windows PowerShell 5.1

The core build does not require aardio or network access.

```powershell
./tools/build.ps1 bootstrap
./tools/build.ps1 dev
./tools/build.ps1 test
./tools/build.ps1 clean
```

`dev` and `test` build both x64 and x86 unless `-Architecture x64` or `-Architecture x86` is supplied. `test` enables MSVC static analysis, runs CTest, and executes the secret, license-inventory, and dependency checks.

Reference pins, source-audit findings, and the reason for the process boundaries are recorded in [docs/reference-baseline.md](docs/reference-baseline.md).

## Current scope

Phase 1A intentionally contains only the durable common version target needed to prove clean x86/x64 builds. TSF, IPC, mock engine, and commit behavior enter together as the Phase 1B vertical slice. Candidate UI, configuration, packaging, and updating remain out of scope until their specified phases.
