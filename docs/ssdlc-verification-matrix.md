# SSDLC and DevSecOps verification matrix

This document is the executable quality contract for the frozen v1.6 specification. A phase
acceptance note is evidence only when its listed gate has passed for the same source and artifact
lineage. Missing evidence is a release blocker, not an implied pass.

## Gate model

| Gate | Trigger and environment | Required result |
|---|---|---|
| PR | Every pull request and push to `main`, GitHub-hosted Windows 2022, PowerShell 7 | x64/x86 clean configure, warning-as-error build, `/analyze`, CTest contracts/integration/fuzz smoke, PE/import hardening, secrets, dependency, license, locale and UTF-8/LF policies |
| Package | Release candidate build job, clean checkout, no signing secrets | PR gate plus real Fcitx/Rime/Lua acceptance, immutable x64+x86 stage, installer/portable construction and portable relocation test |
| Desktop | Interactive Windows release-verification session using the exact package stage | registered tray icon, launcher health, Config UI behavior contract, real Notepad `ni → 你`, engine restart, and engine-absent `abc` fail-open; writes `out/evidence/desktop-verification.json` |
| Compatibility | Self-hosted VMs and named host applications | Win7 Legacy and Windows 10/11 Modern lanes; x86/x64 Notepad, Word, Chrome, VS Code, terminal, DPI/multi-monitor, RDP and selected fullscreen/legacy applications |
| Release | Protected, tag-bound environment with signing authority | Promote the already tested stage only; Authenticode, hashes, detached manifest, SBOM, provenance, installer/portable smoke, rollback and distribution metadata all verify |

Entry points are deliberately small:

```powershell
./tools/verify-product.ps1 pr -Architecture all -Configuration Release
./tools/verify-product.ps1 package
./tools/verify-product.ps1 desktop
./tools/verify-product.ps1 release
```

## Phase traceability

| Phase / risk | Prevention and implementation control | Verification | Gate |
|---|---|---|---|
| 0 reference drift | pinned reference matrix and explicit keep/refactor/reject decisions | `docs/reference-baseline.md` review and hash/source audit | PR |
| 1A build drift | pinned presets/dependencies, D-drive scratch, UTF-8/LF, no undeclared GUI build step | x64/x86 warning-as-error build plus policy scripts | PR |
| 1B TSF correctness | thin COM DLL, bounded engine call, TSF edit-session commit | TSF activation and key-commit E2E; real Notepad commit | PR + Desktop |
| 2 IPC spoofing/hangs | SID/session namespace, DACL, peer path check, request deadlines, isolated test namespaces | protocol, multi-client, late-response, launcher state and protocol fuzz tests | PR |
| 3 engine regressions | one engine dispatcher, connection-scoped contexts, explicit epoch/revision | real Fcitx Pinyin/Rime/Lua tests, typing fuzz, startup/resource baselines | Package |
| 4 UI stalls/incorrect layout | renderer-neutral CandidateModel, independent D2D/DWrite UI, revision snapshots | model/layout contracts, device-loss/safe-mode smoke, render benchmarks, visual/DPI evidence | PR + Compatibility |
| 5 recovery/security | fail-open TSF, crash-loop Safe Mode, bounded restart, no hook/injection | crash-loop, engine-absent Notepad, runtime imports/hijack checks, host compatibility matrix | PR + Desktop + Compatibility |
| 6 settings/install usability | typed Control API, atomic TOML writes, shared real preview renderer, declared Live/Deferred/Restart-required behavior | config round-trip, UI behavior contract, i18n/resources, portable move and installer/repair/uninstall tests | PR + Package + Desktop |
| 7 untrusted package input | downloader/deployer split, strict manifest/path/signature validation, staging + atomic activation | package transaction tests, archive/path fuzz, signature/hash failure, Rime/Lua/addon functional tests | PR + Package |
| 8 supply chain/release | build once, promote exact artifacts, protected signing, owner-aware updates, previous-known-good | release artifact re-verification, SBOM/provenance, rollback tests and release checklist | Release |

The control-by-control Config cases and layer assignment are normative in
`docs/config-ui-test-cases.md`; adding a visible control requires adding or updating its case in
the same change. The product-wide positive, negative, recovery, delivery and compatibility case
catalog is normative in `docs/product-test-plan.md`.

## Required abuse and failure cases

The following cases are maintained as regression tests or explicit compatibility evidence:

- malformed, truncated, oversized and late IPC frames; peer mismatch; concurrent clients;
- engine missing, crash during a request, crash loop and user-stopped state;
- candidate revision reordering, device loss, empty/large candidate lists and scroll-mode paging;
- invalid TOML, atomic-write failure semantics and settings restart/round-trip;
- archive traversal, duplicate/case-colliding paths, bad hash/signature, interrupted activation and rollback;
- portable relocation, install/repair/uninstall and stale TSF profile removal;
- sensitive/password scopes, secure desktop, service accounts and cross-session boundaries;
- long typing/focus churn, fuzz seeds and handle/resource soak.

No raw key, preedit, candidate, commit or user dictionary content may be uploaded as CI evidence.
Interactive evidence contains pass/fail metadata and artifact identity only.

## Product-function acceptance inventory

This inventory prevents feature completion from being inferred from compilation or from a single
happy-path screenshot. Every user-visible row requires implementation evidence, an automated
contract where practical, and desktop evidence when Windows/TSF/Shell behavior is involved.

| User capability | Required verification | Blocking gate |
|---|---|---|
| Start without a terminal | `Start Fcitx5.exe` and installed startup launch the GUI-subsystem launcher; no persistent console window; launcher and engine health become reachable | Package + Desktop |
| Understand current state | Windows notification icon exists; the Desktop gate opens the real Shell popup and invokes Settings, Diagnostics, Restart, Pause, Resume and Exit while asserting their resulting process/state transitions | Desktop |
| Recover the service | kill/restart engine, launcher circuit breaker, Safe Mode and manual Restart change the process generation without losing the host application | PR + Desktop |
| Type real text | registered x64/x86 TSF commits `ni -> U+4F60` in real Notepad; engine absence passes `abc` through; long continuous typing and focus churn do not hang | Package + Desktop + Compatibility |
| Use bundled engines/addons | Pinyin, Rime, `librime-lua`, `fcitx5-lua`, `fcitx5-chinese-addons` and `fcitx5-chttrans` each have a functional candidate/commit or transformation assertion, not only a file-existence check | Package |
| Configure every page | every control declares Live/Deferred/Restart-required semantics; Deferred groups expose Apply/Cancel; persistence, reload, invalid TOML, preview fidelity and diagnostics/repair/package states are asserted | PR + Package + Desktop |
| Trust preview fidelity | Config preview and the real candidate window use the same CandidateModel/layout/theme renderer contract; live saved changes reflow an active candidate snapshot | PR + Desktop + Compatibility |
| Use horizontal/vertical/scroll candidates | labels align with candidates, page/navigation semantics remain stable, empty/large lists are bounded, and the Rabbit/macOS scroll-grid behavior has layout and rendering regressions | PR + Compatibility |
| Manage packages safely | installed/bundled state is visible; bundled components cannot be destructively managed; online refresh, install, update, disable/enable, uninstall, rollback, offline and bad-signature states are tested against a signed fixture repository | PR + Package + Release |
| Move/upgrade portable build | entry points remain usable after relocation; configuration and Rime user data survive move and archive-overwrite upgrade; product data never falls back to the system drive unexpectedly | Package |
| Install, repair and remove | installer registers the intended profiles, repair restores files/registration, uninstall removes owned profiles/files, stale profile cleanup works, and user-data retention follows the selected policy | Desktop + Release |
| Resist malformed input | deterministic protocol, package/archive and stateful typing fuzz suites retain seeds; malformed/oversized/late data fails closed at trust boundaries and fails open in the TSF host | PR + Package |

The package-manager row is not satisfied by placeholder URLs or an empty production keyring.
Until a signed production repository is provisioned, local bundled-component reporting may pass but
online download/update/uninstall remains an explicit release blocker.

## Evidence freshness and lineage

- Evidence records the source commit, dirty/clean state, architecture, configuration and immutable
  stage manifest hash. A result from a different stage cannot close a row.
- Headless Config tests inspect behavior and persistence; screenshots are supplemental visual
  evidence only. Real desktop gates exercise the Shell, COM registration and host application.
- A regression in one row reopens that row and all downstream gates. Fixing it requires rerunning
  the smallest affected test plus every downstream gate before release.
- Compatibility results expire when the toolchain baseline, imported Windows APIs, installer,
  TSF registration or renderer capability path changes.

## Definition of done and stop rules

A change is complete only when affected requirements have tests at the lowest practical layer and
all applicable gates pass. A green headless test never substitutes for real TSF desktop input, and
a manual click test never substitutes for a deterministic contract test.

Release stops immediately when any required gate lacks evidence, the stage differs from the tested
lineage, production keys are absent, or a critical/high security finding is unresolved. Known gaps
must remain visibly unchecked in `docs/release-checklist.md`; they may not be converted into prose
that suggests acceptance.
