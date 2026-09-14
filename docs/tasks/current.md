# STAB-003 — Candidate Preview Production Path

Status: current

This is the sole active task. Governing product rules are in
[`../product-contract.md`](../product-contract.md), current repository facts are in
[`../current.md`](../current.md), and prior evidence is searchable in `status.md`.

Goal:
Replace the Settings candidate preview's fake/demo drawing with the shipping Rust
candidate layout and rendering path, while preserving draft → Apply/Cancel behavior.

Scope:
- `rust/config-poc/src/main.rs`
- `rust/candidate-core` only when a narrow existing shipping render input is missing
- `rust/config-core` snapshot fields only as necessary

Constraints:
- Rust-only product work; pure crates forbid unsafe code.
- Do not modify `third_party/wind-ui-rust`; use its public API through a product-local adapter.
- Do not add C++, qingfeng, a second layout engine, or a second candidate renderer.
- No global hooks, injected keystrokes, or simulated host evidence.

Acceptance:
- The preview directly consumes the production CandidateModel/layout/resolved-theme/render path.
- Its corpus includes CJK, Latin, punctuation, emoji, comment text, selection, and every supported layout mode.
- Draft changes to layout, axis direction, page size, font, and theme affect the preview from the same typed snapshot used for Apply/Cancel.
- Invalid or incomplete draft configuration fails soft without stale or clipped pixels.
- Rust unit/contract coverage includes every mode, invalid configuration, and high-contrast marker behavior.
- The STAB-002 inventory reclassifies `candidate.preview` from Fake/Demo to FullyBound only after verification.
- Real Narrator/UIA and true host visual evidence remain explicitly MANUAL-PENDING.

Done when:
The visible preview has one production Rust rendering authority, tests are green,
and its truth inventory state is updated with evidence.
