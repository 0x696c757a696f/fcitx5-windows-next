# Current Task — REG-INSTALL-UAC-001 Installer machine/user ownership under cross-account UAC

**Mode:** CHANGE
**Task ID:** `REG-INSTALL-UAC-001`
**Evidence class:** `EXTERNAL_EVIDENCE` — never claim unrun real-host evidence passed.

## Goal

Separate machine-owned installation/registration from per-user startup/session/config so uninstall using another administrator account does not leave or delete the wrong user's state.

## Specification references

- §0.5 item 11
- Installer ownership sections
- Phase 6
- `REG-INSTALL-UAC-001`

## Required behavior / implementation contract

- Define authoritative owner for Program Files/system registration vs per-user startup/session/config.
- Do not depend on elevated uninstaller's current HKCU to remove the installing user's startup state.
- Persist only the owner identity needed for correct cleanup using a safe machine-owned mechanism.
- Uninstall must not delete unrelated user data by guessing profiles.

## Out of scope

- Per-user MSI/Store packaging
- Multi-user sync service

## Required validation

- Standard user installs using credentials of a different administrator, then uninstalls.
- Machine artifacts removed; original user's startup/session state removed according to policy; admin HKCU untouched unless it actually owns state.
- Same-account admin install/uninstall regression.
- Repair/reinstall ownership regression.

## Done when

- `REG-INSTALL-UAC-001` passes on a real or suitable Windows VM.
- Machine/user ownership is explicit in installer/control code.
- No elevated-current-HKCU assumption remains.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
