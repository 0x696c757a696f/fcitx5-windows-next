# ADR 0002: Real-engine input deadline and cold-context budget

- Status: accepted for Phase 3; measure again after Candidate UI integration
- Date: 2026-08-17
- Scope: TSF client to native Fcitx5 engine

## Context

ADR 0001 kept a 25 ms absolute deadline based on the mock engine. A real Fcitx5 5.1.14 engine with
libime 1.1.14 and Chinese Addons 5.1.12 initially needed about 1.24 seconds to construct its first
Pinyin InputContext. Applying the Windows fork's mature lifecycle pattern—load addons and warm one
InputContext before signaling readiness—reduced measured client context startup to 3.7–4.5 ms.
Warm keys measured 1.1–1.5 ms and commits 0.25–0.41 ms on the reference machine.

## Decision

Keep 25 ms as the absolute deadline for established input contexts. Permit 100 ms only for the
first request of a newly observed context; this is a bounded cold-context budget, not a retry or a
general deadline increase. The engine completes addon loading and one warm-up key before its ready
event. Timeout remains fail-open and disconnects the protocol state.

## Consequences

- Normal input retains the host-protection bound established in Phase 2.
- A pathological new-context operation cannot freeze a host indefinitely.
- Candidate UI traffic must not share or extend the key deadline in Phase 4.
- Both budgets require re-measurement if Fcitx, libime, addons, models, or dispatcher ownership
  changes.
