# Engineering requirement register

Baseline: Frozen specification v1.4, SHA-256 `A89C5822479BBB8771416B214D024DE15CD8D7F161DFCCE86AA3F316BCD9AA9F`  
Last reviewed: 2026-08-17

This file assigns one stable owner to each engineering constraint. A requirement becomes enforceable when its boundary exists; it is not implemented early through placeholder frameworks.

## Security and privacy

| ID | Requirement | First proving phase |
|---|---|---|
| SR-001 | The TSF DLL must not initiate network communication. | 1B import/capability test |
| SR-002 | Production logs must not contain raw keys, preedit, candidates, commits, or clipboard data. | 1B logging review; regression as logging appears |
| SR-003 | Engine failure must not crash or indefinitely block a host. | 1B engine-absent E2E |
| SR-004 | Malformed or version-mismatched IPC must not crash either peer. | 2 contract/fuzz tests |
| SR-005 | Untrusted native addons must not be silently installed or enabled. | 7 package tests |
| SR-006 | User data must remain independent from program replacement and reinstall. | 6 installer layout; 7 transaction tests |
| SR-007 | Package activation must be verified, staged, and atomic; failure leaves active install untouched. | 7 transaction tests |
| SR-008 | Invalid config/theme data must fall back safely without a crash loop. | 4 invalid corpus |
| SR-009 | Sensitive contexts must disable learning, logging, and external-data features. | 5 host/security regression |
| SR-010 | Advertised Win7 support requires PE import checks and smoke tests. | Import constraint from 1B; full VM in 5 |
| SR-011 | Game compatibility must not use injection, memory access, input/graphics hooks, kernel drivers, hiding, obfuscation, or bypasses. | Architecture review from 1B; smoke in 5 |
| SR-012 | Protection conflicts fail safely; the product does not evade protection. | 5 game matrix |
| SR-013 | TSF IPC has bounded deadlines and fails open on timeout/unavailability. | 1B absent-engine E2E; deterministic timeout suite in 2 |
| SR-014 | TSF authenticates the local engine with per-user/session namespace and ACL; signed stable builds also verify installed path and trust. | 2 security integration |
| SR-015 | SYSTEM, LogonUI, and secure desktop contexts must not start or bind a normal user engine. | 2 launcher integration |
| SR-016 | Candidate UI failure must not block input; UILess semantics remain available on request. | 4 UI crash/UILess tests |
| SR-017 | Native DLL resolution must not use an untrusted current directory or arbitrary PATH fallback. | 1B PE/import test; full audit in 5 |
| SR-018 | Untrusted PR/fork workflows receive no production signing or release credentials. | 1A workflow inspection |
| SR-019 | Stable artifacts trace to one commit, pinned inputs, and one verified build lineage without a silent rebuild. | 8 release gate |
| SR-020 | Third-party CI actions/tools in privileged paths use immutable pins and least token permissions. | 1A workflow check; release audit in 8 |

## Reliability

| ID | Requirement | First proving phase |
|---|---|---|
| REL-001 | Each input context, composition, and engine generation has one authoritative owner. | 2 contract/model tests |
| REL-002 | Stale responses and candidate updates cannot mutate a newer context or composition. | 2 late-response tests |
| REL-003 | Engine launch failure and crash loops converge through bounded backoff and Safe Mode. | 2 virtual-clock state-machine tests |
| REL-004 | UI, theme, config, addon, and update failures do not take down the host input path. | 4–7 affected fault tests |
| REL-005 | Atomic writes and activation either publish the complete new state or preserve the previous state. | 4 config and 7 package crash-consistency tests |

## Performance

| ID | Requirement | First proving phase |
|---|---|---|
| NFR-P001 | Host input waits are bounded; 25 ms is an initial measurement budget, not an unverified permanent constant. | 1B bound; percentiles in 2 |
| NFR-P002 | Queues, frames, payloads, candidate counts, allocations, and retries have explicit limits. | 2 protocol contract |
| NFR-P003 | Startup, idle CPU, private bytes, handles, and 60 Hz key repeat have recorded baselines before dogfood. | 3 benchmark |
| NFR-P004 | A hidden candidate UI has no continuous render loop; device loss is recoverable. | 4 UI benchmark |
| NFR-P005 | Production logging performs no synchronous per-key disk I/O and stores no input content. | 1B review and perf test when logging exists |

## Human factors and accessibility

| ID | Requirement | First proving phase |
|---|---|---|
| NFR-HF001 | Candidate UI never steals focus and avoids avoidable layout movement. | 4 host/UI regression |
| NFR-HF002 | Mode and key behavior are predictable and preserve established muscle memory. | 3 input behavior; dogfood |
| NFR-HF003 | Failures are cheap to recover from and do not interrupt ordinary typing with modal UI. | 4–6 affected E2E |
| NFR-HF004 | User-facing settings describe user tasks, not TSF/IPC/ABI internals. | 6 cognitive-load review |
| NFR-A11Y001 | Candidate state is available through TSF UILess/UIElement semantics. | 4 contract/E2E |
| NFR-A11Y002 | High Contrast, keyboard-only operation, and screen-reader semantics do not rely on color/icon alone. | 4 accessibility tests |

## Configuration and visual correctness

| ID | Requirement | First proving phase |
|---|---|---|
| NFR-CFG001 | Windows shell/theme user configuration is strict TOML 1.0 with one typed semantic owner. | 4 parser contract |
| NFR-CFG002 | Machine manifests, locks, and i18n data are strict JSON; unknown/duplicate/invalid fields fail explicitly. | First affected phase |
| NFR-CFG003 | Runtime performs no implicit migration or legacy-format fallback. | First parser contract |
| NFR-CFG004 | Paths remain under their declared roots and writes are permission-safe and atomic. | 4 config and 7 package tests |
| NFR-VIS001 | System Light/Dark/DPI/High Contrast wins by default, with few explicit user overrides. | 4 renderer tests |
| NFR-VIS002 | DirectWrite/system fallback preserves glyph visibility across candidate and annotation surfaces. | 4 golden/render smoke |
| NFR-VIS003 | Horizontal/vertical layouts and 125/150/200% multi-monitor placement remain deterministic. | 4 layout tests |

## Supply chain

| ID | Requirement | First proving phase |
|---|---|---|
| SC-001 | Every dependency has a pinned source/version, known license, maintenance and platform assessment, and a current need. | 1A inventory gate |
| SC-002 | Core PR builds use warnings as errors, affected tests, static analysis, secret scan, license inventory, and SCA. | 1A CI |
| SC-003 | CI permissions are least-privilege; privileged workflows do not execute untrusted PR code. | 1A CI; 8 release audit |
| SC-004 | Release inputs and tools are pinned and verified; caches are never authoritative. | 8 release gate |
| SC-005 | Stable releases include final-byte hashes, signatures, SBOM, source commit, and provenance for the exact tested artifacts. | 8 release gate |

## Data classification

- **S0 Secret/Input:** raw key, preedit, commit, candidate content, clipboard/current selection. No persistence, network, production logs, or telemetry.
- **S1 Sensitive User Data:** dictionary, history, personalization, private configuration. User ACL, explicit owner/retention/delete semantics, local by default.
- **S2 Operational:** version, component state, opaque identities, latency, error category, crash metadata. Allowed for bounded diagnostics without S0/S1.
- **S3 Public:** package metadata, public version, licenses, source commit.

New protocol, API, log, and diagnostic fields must identify their class during review.
