# Task 070 - Low-resource SLO calibration

**Mode:** CHANGE / CODE-ONLY / EXTERNAL-EVIDENCE-PARTIAL
**Task ID:** `LOW-RESOURCE-SLO-CALIBRATION-001`
**Prerequisite:** `066`, `068`, and `069` automated contracts green.
**Evidence class:** repeatable local harness plus real 2-core/4-GB calibration manual evidence.

## Goal

Add the smallest repeatable measurement path for Core, TSF shim, Candidate UI, and heavy-plugin
activation so initial low-resource SLOs can be calibrated without treating estimates as facts.

## Constraints and acceptance

- Read `rust-skills`; use worktree-local `CARGO_TARGET_DIR`. New measurement
  and product logic is Rust-first; no benchmark framework or telemetry service unless current code
  cannot provide a bounded script/harness.
- Separate offline Core measurements from Rime/Mozc/Lua activation. Capture latency/memory inputs
  reproducibly and retain only privacy-safe aggregate metrics.
- x64/x86 automated harnesses must produce bounded, comparable output and not use sleep/retry as a
  correctness barrier. Mark all numerical values as initial SLOs pending real calibration.
- Real 2-core/4-GB host, low-storage, offline/constrained-network, accessibility, Win7, and production
  signing/UAC results remain `MANUAL-PENDING`; accessibility and low-resource remain release gates.
