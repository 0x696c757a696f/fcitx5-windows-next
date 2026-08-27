# Task 064 - Config Candidate and plugin usability correction

**Mode:** CODE / PRODUCT CORRECTION
**Task ID:** `CONFIG-CANDIDATE-PLUGIN-USABILITY-CORRECTION-001`
**Prerequisite:** 062 and 063 automated contracts green

## Goal

Correct two user-visible regressions in the default Rust WindUI Settings shell:

- restore the complete Candidate layout mode selection, including Scroll, `N x 6`, and `6 x N`,
  where `N` is derived from the authoritative maximum candidate/page-size setting;
- make the plugin page complete one real supported Windows plugin lifecycle through a production
  signed repository path instead of exposing a catalog whose default endpoint cannot serve assets.

## Specification references

- Candidate Appearance basic/advanced settings and authoritative `page_size` ownership
- Candidate horizontal, vertical, grid, and scroll layout contracts
- Package repository v2 signatures, immutable assets, trust, install/update/remove, and rollback
- Rust Config Stage 4 accessibility, DPI, persistence, and authoritative reread requirements

## Required behavior / implementation contract

- The default WindUI Appearance page exposes all supported Candidate layout modes. Mode labels and
  preview dimensions derive `N` from authoritative `candidate.page_size`; theme data must not own or
  duplicate page size.
- Layout mode, page size, and scroll-cell settings persist through the existing typed Control/config
  path and survive authoritative reread/restart.
- Keyboard, accessibility names, DPI layout, and embedded preview remain usable for every mode.
- The plugin page preserves strict signature/trust checks and typed bounded operations. It must not
  report fixture-only packages, local ignored keys, or an unreachable placeholder endpoint as a
  production online lifecycle.
- Produce at least one supported Windows plugin package plus v2 signed index/metadata through a
  production-input build/publish path. Never commit a private signing key.
- When external publication credentials or hosting are unavailable, complete all repository-side
  package/sign/publish automation and record the exact publication step as `MANUAL-PENDING`; do not
  claim an online install passed until the public immutable assets are actually reachable.

## Required validation

- Focused x64/x86 Rust tests for mode mapping, `N` derivation, persistence, and preview geometry.
- Fresh x64/x86 Appearance evidence covering Scroll, `N x 6`, and `6 x N`, including no clipping.
- Package-core tests for signed v2 index/package verification and install/enable/disable/remove.
- A real online lifecycle test against published immutable assets, or an exact `MANUAL-PENDING`
  record when publication credentials/hosting are unavailable.
- Source/security/license/dependency/text checks for changed boundaries.

## Done when

- Candidate users can select and persist the requested modes in the default Settings UI.
- At least one supported plugin can be discovered, downloaded, verified, installed, enabled,
  disabled, and removed from the production repository path, or the only remaining gap is precisely
  documented external publication that cannot be performed locally.

After completion, update `docs/tasks/status.md`, archive this task, and return `current.md` to the
release gate according to `docs/tasks/PLAN.md`.
