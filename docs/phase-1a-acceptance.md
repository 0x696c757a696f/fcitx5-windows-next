# Phase 1A acceptance

Date: 2026-08-17  
Result: passed

## Evidence

- Clean build entry: `./tools/build.ps1 clean` followed by `./tools/build.ps1 test`.
- Toolchain: CMake 4.4.2, Visual Studio Build Tools 2022 17.14, MSVC 19.44.35228, Windows SDK 10.0.19041.0.
- x64 artifact: `fcitx5_version_test.exe`, PE machine `0x8664`.
- x86 artifact: `fcitx5_version_test.exe`, PE machine `0x014c`.
- MSVC `/W4 /WX /sdl /analyze`, CFG, DEP/ASLR, and CET-compatible link options completed for project targets.
- CTest `version-contract`: 1/1 passed on x64 and 1/1 passed on x86.
- `dev` entry completed on both architectures without aardio or network access.
- Paired secret scanner self-test and repository scan passed.
- Paired license inventory self-test passed; declared third-party dependencies: 0.
- SCA baseline passed; no third-party build/runtime dependency directives are present.
- GitHub Core workflow uses a read-only token and an immutable checkout Action SHA.

## Gate impact

This phase adds build and CI configuration but no TSF/IPC boundary, network capability, runtime dependency, installer, signing operation, user configuration, or input hot-path work. Phase 1B is now allowed.
