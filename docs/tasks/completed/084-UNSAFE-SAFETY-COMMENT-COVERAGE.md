# 084 — UNSAFE-SAFETY-COMMENT-COVERAGE

## Task

Enforce the frozen unsafe-boundary rule at per-site granularity: every
`unsafe` block, fn, and impl carries a SAFETY comment stating the actual
invariant (pointer provenance, validity window, aliasing, lifetime) — no
boilerplate. `clippy::undocumented_unsafe_blocks` is adopted as a standing
`warn` lint at every FFI crate root so the coverage cannot regress.

## Baseline (clippy, workspace)

480 undocumented sites: package-core 137, control-core 109, engine-core 93,
tsf-poc 44, protocol-core 43, windows-common-core 33, register-core 15,
ipc-client 2, tsf-support-core 2, config-core 1, release-pqc-signer 1.
Candidate's FFI/Win32 modules also require explicit unsafe-policy exceptions;
they are covered by the repository policy test.

## Acceptance

- Workspace clippy `-W clippy::undocumented_unsafe_blocks`: 0 sites.
- No product-logic changes: comments, lint attributes, policy inventory, and
  formatting only.
- All workspace Rust tests, rustfmt, and x64 CTest lane green.

## Status

COMPLETED / UNSAFE-SAFETY-COVERAGE-GREEN.

Evidence: workspace strict Clippy reports zero undocumented unsafe blocks;
all workspace Rust tests and x64 CTest (79/79) pass; workspace rustfmt passes.
The only non-comment change is the required `deny(unsafe_op_in_unsafe_fn)`
exception declaration and its policy-test inventory entry for Candidate FFI/Win32
modules. No product logic changed.
