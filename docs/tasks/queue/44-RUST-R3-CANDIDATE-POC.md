# RUST-R3-CANDIDATE-POC Rust Candidate UI differential PoC

**State:** FUTURE-GATED

## Gate

Start only after Candidate UX/layout/UILess contracts are frozen and green for the current C++ UI.

## Scope

- Build an isolated Rust Candidate UI PoC as an out-of-process executable.
- Reuse the versioned IPC/model contract; do not use C++ FFI.
- Model context, composition, revision, epoch, candidate selection, paging, UILess policy, DPI, locale, emoji/color-font, and layout stability with Rust strong types.
- Compare C++ UI and Rust UI against the same mock-engine snapshots and golden layout/interaction corpus.

## Must not do

- Do not replace the shipping C++ Candidate UI during the PoC.
- Do not change the Candidate IPC/model semantics while migrating language.
- Do not add hooks, `SendInput`, injection, anti-cheat bypass, or external attack behavior.

## Done when

- Rust Candidate PoC has differential, accessibility, DPI, layout, and host evidence.
- A later cutover task can decide whether to replace the C++ implementation and delete the old authoritative path.
