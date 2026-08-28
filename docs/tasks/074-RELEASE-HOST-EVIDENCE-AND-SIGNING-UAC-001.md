# Task 074 - Release host, accessibility, signing, and UAC evidence

**Task ID:** `RELEASE-HOST-EVIDENCE-AND-SIGNING-UAC-001`
**Mode:** EXTERNAL-EVIDENCE / PREPARATION + REAL-HOST VALIDATION
**Prerequisites:** `071`, `072`, `073` as applicable; the tested package stage must be identified.
**Authorization:** The user authorized the Accessibility and production signing/UAC evidence work.

## Goal

Close the release evidence gaps without converting fixture results into production claims. Use the
same Build Once stage for every check and record the exact source commit, stage manifest, host,
architecture, privilege, and credential class used.

## Workstreams and delegation

1. **Accessibility/UIA:** inspect the Rust Candidate/Settings semantic projections through the
   shipping native host. Verify keyboard-only traversal, focus/selection state, accessible names,
   bounds, notifications, High Contrast, privacy suppression, and real Narrator plus NVDA output.
   A mock UIA contract or rectangle non-overlap result is not real assistive-technology evidence.
2. **Signing/UAC:** prepare a disposable test certificate and test ML-DSA material only under
   ignored `out/secure`; prove that test credentials are rejected by the production promotion
   contract. With an externally controlled production certificate/key, sign the exact staged PE
   and repository artifacts, verify Authenticode/timestamp/CMS/ML-DSA, and run install, repair,
   update, rollback, and uninstall through the actual UAC boundary. Never commit private keys,
   passwords, or test trust roots; never call a self-signed artifact production-signed.
3. **Low-resource/offline:** execute the Rust measurement harness and package lifecycle on a real
   2-core/4-GB machine or VM, low free-storage boundary, offline mode, and constrained network.
   Capture bounded latency/memory, fail-soft behavior, cache use, cancellation, and recovery.
4. **Host matrix:** run exact final artifacts on declared Windows 7 SP1 Legacy and Windows 10/11
   Modern hosts, x64 and supported x86 host lanes, DPI/multi-monitor, Notepad/Office/browser/
   Terminal/VS Code, RDP where declared, and in-use TSF upgrade. Unsupported rows must be named,
   not silently omitted.

## Hard rules

- `out/secure` credentials are disposable test inputs and stay outside Git; the official ML-DSA
  keyring remains public-key-only in the repository.
- Production Authenticode requires an externally provisioned certificate/private-key boundary and
  timestamping service. Missing SDK, certificate, protected key, or host is `MANUAL-PENDING`.
- UAC evidence must show the elevated token, correct machine/user ownership, cross-account cleanup,
  repair, and restoration after the test. A non-elevated local smoke is insufficient.
- Accessibility evidence must include the actual UI Automation client/assistive technology and
  must not be inferred from Rust unit tests alone.
- Do not claim Windows 7, 2-core/4-GB, offline, production signing, or release readiness from
  local fixtures, generated keys, or an incompatible host.

## Acceptance and evidence

- Every workstream has a machine-readable evidence record under `out/evidence` with commit, stage
  manifest hash, host/build, architecture, result, and remaining limitations.
- Test signing proves the credential separation and all reachable Authenticode/UAC preparation;
  production signing remains pending until the protected external credentials and timestamp are
  actually used.
- Real Narrator/NVDA, Windows 7, low-resource, low-storage, offline/constrained-network, and
  cross-account UAC rows are individually `PASSED` only when actually run, otherwise
  `MANUAL-PENDING` with the exact missing prerequisite.
- No release gate is promoted and no artifact is published by this task.

On completion, update `docs/tasks/status.md`; keep `REL-01` release-gated until every required row
is green and the final signed bytes have matching manifest, SBOM, provenance, and signatures.
