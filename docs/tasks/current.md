# Task 074 - Release host, accessibility, signing, and UAC evidence

**Task ID:** `RELEASE-HOST-EVIDENCE-AND-SIGNING-UAC-001`
**Mode:** EXTERNAL-EVIDENCE / PREPARATION + REAL-HOST VALIDATION
**Prerequisites:** `071`, `072`, `073` as applicable; the tested package stage must be identified.
**Authorization:** The user authorized the Accessibility and production signing/UAC evidence work.

## Goal

Close the release evidence gaps without converting fixture results into production claims. Use the
same Build Once stage for every check and record the exact source commit, stage manifest, host,
architecture, privilege, and credential class used.

## Execution order

0. Freeze one unsigned package stage from the intended source commit and locked toolchains.
1. Modern Windows 10/11 host, TSF, keyboard/UIA, Narrator, and NVDA evidence.
2. Production signing plus real UAC install, repair, update, rollback, uninstall, and ownership.
3. Production online plugin repository lifecycle when online package installation ships.
4. Win7 SP1, 2-core/4-GB, low-storage, offline, and constrained-network evidence.
5. GitHub Actions rehearsal, then hand the exact immutable stage to `REL-01` for promotion.

## Hard rules

- No row is passed from fixtures, generated credentials, or an incompatible host.
- One campaign uses one stage identity; later signing/publishing never recompiles it.
- No requirement is waived without an explicit support-matrix/specification change.
- `REL-01` stays release-gated until every required row is green.

## Done when

- Every batch is recorded against the frozen stage with exact host, privilege, and credential facts.
- Final signed bytes have matching manifest, SBOM, provenance, and signatures.
- `REL-01` can promote without rebuilding.
