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
