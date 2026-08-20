# Current Task — REG-CAND-UX Candidate Auto layout and composition-scoped width stability

**Mode:** CHANGE
**Task ID:** `REG-CAND-UX`
**Prerequisite:** 013 locale metadata available; 017 can consume the new Auto setting

## Goal

Improve candidate presentation without changing input semantics: add Auto/Horizontal/Vertical resolution and prevent distracting width shrink/grow jitter inside one composition.

## Specification references

- §5 Candidate UI/UX
- Phase 6/4 presentation rules
- `REG-CAND-STABLE-001` / `REG-CAND-AUTO-001`

## Required behavior / implementation contract

- Add `automatic` layout mode with deterministic resolution based on active engine/content characteristics and available work area; explicit horizontal/vertical override always wins.
- Keep layout choice stable within a composition unless a defined hard constraint requires change.
- Use composition-scoped width hysteresis/stability so short-lived candidate shortening does not constantly shrink the window; reset on composition end/new identity.
- Preserve current scroll viewport, label-column alignment, selection, ellipsis, work-area clamp and DPI behavior.

## Out of scope

- Rust
- New animation system
- WebView

## Required validation

- `REG-CAND-STABLE-001`: long→short→long candidates within one composition.
- `REG-CAND-AUTO-001`: Chinese short candidates, long annotation, non-Chinese engine, edge-of-screen.
- Explicit override precedence.
- DPI/multi-monitor and candidate interaction regressions.

## Done when

- Candidate window no longer jitters width unnecessarily.
- Auto is deterministic and does not randomly flip within one composition.
- No candidate-selection/input-semantic change.

After completion, update `docs/tasks/status.md` and advance according to `docs/tasks/PLAN.md`.
