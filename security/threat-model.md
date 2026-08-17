# Threat model

Baseline date: 2026-08-17  
Scope: planned trust boundaries through the First Usable/Dogfood milestone

The input data plane is `host → TSF → local IPC → engine/Fcitx → CandidateModel → UI or commit`. It is sensitive, low latency, and has no network capability. The later management plane may fetch packages but cannot subscribe to keys, preedit, candidates, or commit history.

## Assets and boundaries

| Asset/boundary | Primary threats | Required mitigation and evidence |
|---|---|---|
| Host ↔ `fcitx5-tsf.dll` | Host crash, arbitrary DLL load, key disclosure, denial of input | Thin noexcept COM boundary; Win7-safe imports; restricted DLL resolution; absent-engine fail-open E2E |
| TSF ↔ engine pipe | Peer spoofing, cross-session access, malformed frames, replay/stale state, indefinite block | SID/session namespace, explicit DACL, peer PID/path/trust, strict v2 contract, request/state identities, overlapped deadline, fuzz/fault tests |
| Engine ↔ Fcitx/addons | Native addon reads input or crashes engine, ABI mismatch | Load only in engine; source/signature/ABI classification; Safe Mode; crash/backoff tests |
| Engine ↔ candidate UI | Candidate disclosure, stale-window update, UI crash or focus theft | Revisioned CandidateModel projection; no input business logic; no network; crash/no-focus E2E |
| Config/control ↔ engine | Management client obtains S0 or mutates live state unsafely | Separate typed control API with no input/history endpoints; validate and atomically publish config |
| Package/updater ↔ internet | Manifest/artifact tampering, downgrade, key compromise, privilege escalation | Signed strict manifest, SHA-256, staging/verify/atomic activation, minimal elevated deployer, revocation/rollback evidence |
| Program versions ↔ user data | Upgrade destroys or rolls back irreplaceable data | Separate roots and transactions; at most one previous-known-good program artifact |
| CI/release | Malicious PR steals secrets or substitutes artifacts | Read-only PR token, immutable Actions, protected signing, build once/promote same bytes, final hashes/SBOM/provenance |

## STRIDE baseline

| Threat | Current design response | Verification owner |
|---|---|---|
| Spoofing | Local peers are identified by SID, logon session, PID, expected installed path, and release trust where available. | Phase 2 IPC security integration |
| Tampering | Strict bounded wire/config/package contracts; signed package metadata and artifacts; atomic publication. | Phase 2/4/7 contract and crash-consistency tests |
| Repudiation | S2-only local diagnostics record version, opaque identities, state transition, error category, and artifact hashes—not typed content. | Affected logging tests and release manifest |
| Information disclosure | S0 stays within input-plane memory, is never logged, and cannot reach network-capable components. | Imports/capability checks and API/log review |
| Denial of service | Bounded IPC, queues, retry/backoff, fail-open TSF, independent UI, Safe Mode. | Phase 1B absent peer; Phase 2 fault/virtual-time; Phase 4 UI crash |
| Elevation of privilege | TSF does not broker arbitrary launch; package service validates before a minimal deployer performs an allowlisted activation. | Phase 2 launcher and Phase 7 deployer tests |

## Explicitly forbidden mitigations

The project does not solve compatibility or reliability through global keyboard hooks, `SendInput` replay, process injection, game-memory access, kernel drivers, process hiding, obfuscation, unrestricted shell execution, broad pipe ACLs, runtime legacy parsers, or a network-capable input process.

## Change rule

Update this file only when a change creates or alters a trust boundary, permission, network capability, native plugin execution path, or signing/update boundary. Ordinary visual and text changes do not require a new threat-model ceremony.
