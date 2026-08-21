# PLUGIN-LIFECYCLE-001 Install/update/uninstall stability

**State:** MANUAL-PENDING
**Plan source:** `docs/tasks/plugin-install-update-stability-plan.md`

## Goal

Verify add-on install, update, remove-after-update, repair, and normal-use stability after the Settings and trust-chain work is in place.

## Acceptance

- Package lifecycle regression suite passes on x64 and x86.
- Config package actions are covered by interaction tests.
- Update followed by uninstall removes executable payloads.
- User/package-owned data preservation policy is respected.
- Any real online endpoint gap is recorded as `MANUAL-PENDING` with exact missing evidence.

## Manual-pending evidence

Automated package lifecycle and Settings interaction coverage passed. Real official online repository evidence remains unavailable because this checkout does not include a provisioned official trusted public key in `security/trusted-keys.template.json` and no production signed repository/update endpoint evidence has been supplied.
