# Task 073 - Fcitx5 official rebaseline build promotion

**Result:** COMPLETED / OFFICIAL-CLANG64-X64-BUILD-GREEN / MANUAL-PENDING

Official `fcitx/fcitx5@cdd0b9d900770d1ad1229d759213215d5dc23a90` was verified with
the fail-closed patch queue and built through the native-engine target using the
pinned MSYS2 CLANG64 toolchain. Fcitx5 core, libime, chinese-addons, librime,
fcitx5-rime, fcitx5-lua, and fcitx5-unikey configured, built, and installed.

The build used isolated roots under `out/official-rebaseline-*`, preserving the
old rollback source checkout. Stage checks passed for the Fcitx data directory,
Rime data, engine executable, and keyboard/Unikey/Lua modules. Bootstrap now
supports isolated roots, pinned MSYS2 GPG verification, optional toolchain sync,
Windows executable compatibility names, and the Unikey Windows build patches.

Remaining manual evidence: x86 native-engine lane, real Windows 7/10/11 hosts,
Accessibility/Narrator/NVDA, production signing/UAC, low-resource/offline host,
and release publication. These are not inferred from this local build.
