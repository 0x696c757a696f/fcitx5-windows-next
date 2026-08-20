# Current Task — STAB-REGISTER-BOOTSTRAP-012 Register/bootstrap validated artifact + side-effect containment

**Mode:** CHANGE
**Task ID:** `STAB-REGISTER-BOOTSTRAP-012`

## Goal

Harden production register/bootstrap helpers so they only operate on validated product artifacts and cannot report failure while side-effecting children keep modifying system state in the background.

## Specification references

- §0.5 item 12
- Register/bootstrap helper sections
- Installer/package side-effect timeout rules
- `STAB-REGISTER-BOOTSTRAP-012`

## Required behavior / implementation contract

- Register helper validates DLL path/root belongs to the current product artifact before COM/TSF registration side effects.
- Bootstrap helper validates the helper artifact it elevates/launches and keeps child side effects bounded.
- Timeouts must confirm child termination/completion before UI reports a failed/complete state.
- Installer/repair flows use the hardened helpers instead of ad-hoc side-effect children.

## Out of scope

- Redesigning TSF registration identity.
- Adding a Windows service.
- Authenticode policy redesign beyond current local artifact validation.

## Required validation

- Valid product artifact register/repair path succeeds in test fixture.
- Wrong path / outside-root / missing paired architecture artifact is rejected before side effects.
- Hung or slow child registration/repair is terminated or proven finished before reporting.
- Installer/repair E2E or closest automated fixture covers the helper path.

## Done when

- Production register/bootstrap side effects are path/artifact validated.
- No helper path reports final status while an unbounded child continues mutating registration.
- Relevant automated installer/repair checks are green.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
