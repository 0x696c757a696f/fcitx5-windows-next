# Current Task — STAB-REGISTER-BOOTSTRAP-012 Register/bootstrap validated-artifact and child lifecycle hardening

**Mode:** CHANGE
**Task ID:** `STAB-REGISTER-BOOTSTRAP-012`
**Prerequisite:** 011 ownership model should be known before final installer E2E

## Goal

Restrict registration/bootstrap helpers to validated product artifacts and ensure timed-out mutating child operations cannot continue changing registration after the parent reports failure.

## Specification references

- §0.5 item 12
- Installer/register/bootstrap sections
- Phase 5/6

## Required behavior / implementation contract

- Registration helper accepts only an artifact rooted in the validated install/staging transaction, with architecture/product identity checks required by current package contract.
- Do not treat an arbitrary absolute path named `fcitx5-tsf.dll` as sufficient production authorization.
- Mutating child timeout must establish termination/completion before returning final failure.
- Keep helper minimal and C++; do not turn it into a general privileged broker.

## Out of scope

- Rust rewrite
- General privilege service

## Required validation

- Valid staged x86/x64 TSF registration succeeds.
- Arbitrary external DLL path is rejected by production path.
- Hung registration child timeout leaves no late background mutation.
- Repair/install rollback E2E around registration failure.

## Done when

- Helper consumes validated product identity only.
- No side-effect child outlives a reported terminal timeout.
- Installer/repair can recover deterministically.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
