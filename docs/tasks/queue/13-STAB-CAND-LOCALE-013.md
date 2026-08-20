# Current Task — STAB-CAND-LOCALE-013 Candidate locale metadata and config-generation reload

**Mode:** CHANGE
**Task ID:** `STAB-CAND-LOCALE-013`
**Prerequisite:** 004 single profile metadata contract should be available

## Goal

Remove hard-coded `zh-CN` presentation assumptions and input-frequency filesystem metadata polling from the Candidate UI.

## Specification references

- §0.5 item 13
- Phase 4
- Candidate locale/config polling sections

## Required behavior / implementation contract

- DWrite locale comes from active Fcitx input method/content locale/text-language policy, not the fixed Windows profile LANGID.
- Config/theme reload is driven by generation/broadcast/explicit notification.
- Filesystem timestamp checks, if retained as fallback, run at low frequency and never once per candidate snapshot/key.
- Preserve safe fallback when locale metadata is absent.

## Required validation

- Non-zh-CN real engine/content locale visual/golden test.
- Locale changes after internal engine switch without creating a second Windows profile.
- Instrumentation/test proving no per-snapshot filesystem metadata query.
- Config generation change updates renderer.

## Done when

- No hard-coded `zh-CN` in product rendering path except explicit test fixture/fallback where documented.
- No input-frequency config file metadata polling.
- Candidate text remains correct across locale changes.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
