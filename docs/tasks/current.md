# Task 074 - Release host, accessibility, signing, and UAC evidence

**Task ID:** `RELEASE-HOST-EVIDENCE-AND-SIGNING-UAC-001`

**Prerequisites:** `071`, `072`, and completed official x64 `073` stage.
**Mode:** `EXTERNAL-EVIDENCE / PREPARATION + REAL-HOST VALIDATION`
**Authorization:** Accessibility and production signing/UAC evidence work is authorized.

## Goal

Close the release evidence gaps without converting fixture results into production claims. Use the
same Build Once stage for every check and record the exact source commit, stage manifest, host,
architecture, privilege, and credential class used.

## Reachable work

- Validate the official Fcitx5 stage and its manifest identity.
- Run available Rust measurement, package, artifact, desktop, and UAC preparation checks.
- Generate machine-readable evidence under `out/evidence`.
- Verify disposable test credentials remain outside Git and cannot satisfy the production promotion
  contract.
- Inspect CI workflows and repair only a reproduced failure.

## External evidence still required

- Real Narrator/NVDA/UI Automation client evidence, including keyboard traversal, names, bounds,
  notifications, High Contrast, and privacy suppression.
- Production Authenticode certificate/private-key and timestamp service, ML-DSA release key,
  exact staged-byte signing, UAC install/repair/update/rollback/uninstall, and cross-account cleanup.
- Real 2-core/4-GB and low-storage machine/VM, offline/constrained-network lifecycle evidence.
- Windows 7 SP1 Legacy and Windows 10/11 Modern host matrix, supported architectures, DPI,
  multi-monitor, Notepad/Office/browser/Terminal/VS Code, RDP where declared, and in-use TSF update.

## Hard rules

- `out/secure` contains disposable test inputs only and is never committed.
- A self-signed or locally generated artifact is never called production-signed.
- Local desktop/semantic/rectangle tests are not real accessibility evidence.
- Do not claim Windows 7, low-resource, offline, production signing, UAC, or release readiness
  without the corresponding host and credential evidence.
- Keep `REL-01` release-gated until every required row is green and signed bytes match the manifest,
  SBOM, provenance, and signatures.

## Acceptance

- Each workstream has a machine-readable record under `out/evidence` containing commit, stage
  manifest hash, host/build, architecture, result, and limitations.
- Reachable automated preparation passes are recorded; unavailable real-host rows are individually
  `MANUAL-PENDING` with the exact missing prerequisite.
- No release gate is promoted and no artifact is published by this task.

On completion, update `docs/tasks/status.md`; if all automatable checks pass, archive this task and
select the next eligible task from `docs/tasks/PLAN.md`, leaving external rows `MANUAL-PENDING`.
