# Task 073 - Fcitx5 official rebaseline build promotion

**Task ID:** `FCITX-UPSTREAM-REBASELINE-BUILD-001`

**Prerequisite:** `FCITX-UPSTREAM-REBASELINE-001` patch audit green and the pinned
MSYS2/CLANG64 toolchain available.

## Goal

Configure and build the official `fcitx/fcitx5` commit
`cdd0b9d900770d1ad1229d759213215d5dc23a90` with the explicit Windows patch queue,
then decide whether the official pin can replace the rollback reference in the
shipping lane.

## Required evidence

- Run `tools/bootstrap-fcitx.ps1 -VerifyPatchesOnly` against the prepared source
  checkouts and retain the forward/reverse patch report.
- Configure/build/install the affected native-engine CLANG64 lane for the pinned
  x64 source and run its focused tests; run x86 affected tests where supported.
- Check staged `share/fcitx5`, `FCITX_DATA_DIRS`, user-data-root, and dispatcher
  behavior without changing product semantics.
- Keep real Windows host compatibility, signing, and release publication as
  explicit manual evidence; do not claim them from local build output.

If the build or focused tests fail, leave the official pin in the compatibility
lane, retain `50a3069a2f1bb8647abef713d98ad10d0713b752` as rollback reference, and
record the exact failure before changing the patch queue.

## Completion evidence (2026-08-28)

- Official source: `fcitx/fcitx5@cdd0b9d900770d1ad1229d759213215d5dc23a90`.
- `bootstrap-fcitx.ps1 -VerifyPatchesOnly` passed in an isolated source root;
  all nine pinned patches were `APPLY-CLEAN` or `ALREADY-APPLIED`.
- Pinned MSYS2/CLANG64 configured, built, and installed Fcitx5 core, libime,
  chinese-addons, librime, fcitx5-rime, fcitx5-lua, fcitx5-unikey, and the
  repository native-engine target.
- Stage checks passed for `share/fcitx5`, `share/rime-data/fcitx5.yaml`,
  `FCITX_DATA_DIRS`, `fcitx5-engine.exe`, and the windowskeyboard, unikey, and
  luaaddonloader modules.
- The bootstrap now supports isolated roots under `out/`, uses pinned MSYS2 GPG
  for signature verification, skips already validated toolchain sync on request,
  preserves Unix-style addon tool names on Windows, and applies the two Unikey
  Windows build patches through the same fail-closed queue.
- Not claimed here: x86 native-engine build, real Windows host compatibility,
  Accessibility, production signing/UAC, and release publication.
