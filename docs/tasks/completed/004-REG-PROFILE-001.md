# Current Task — REG-PROFILE-001 Single Windows Fcitx5 TSF profile

**Mode:** CHANGE
**Task ID:** `REG-PROFILE-001`
**Prerequisite:** 003 KeyEvent contract complete if profile metadata shares the breaking IPC update

## Goal

Make Windows expose one stable `Fcitx5` TSF profile while Fcitx input method/group and content locale change internally.

## Specification references

- §0.5 Stabilization Gate item 2/14.0 item 4
- §9 single Windows TSF profile
- Phase 3
- `REG-PROFILE-001`

## Required behavior / implementation contract

- Register exactly one stable profile GUID and user-visible profile name `Fcitx5`.
- Do not dynamically register a Windows profile per Rime/Mozc/Pinyin/Hangul/m17n engine.
- Track active Fcitx IM/group and BCP-47/content locale as internal runtime metadata.
- Treat the TSF registration LANGID as shell identity only; never infer current content language from it.
- Switching internal engine must not create/remove Windows profiles.

## Out of scope

- Final penguin artwork polish (task 019)
- Candidate renderer redesign
- Rust

## Required validation

- `REG-PROFILE-001` with a Chinese engine and at least one real non-Chinese engine.
- Install/register/unregister regression ensuring one profile only.
- Profile switch/restart preserves stable GUID/name.

## Done when

- Windows picker exposes one `Fcitx5` profile.
- Internal engine switch updates runtime metadata without shell-profile proliferation.
- No code path assumes fixed `zh-CN` merely because the registered LANGID is Chinese.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
