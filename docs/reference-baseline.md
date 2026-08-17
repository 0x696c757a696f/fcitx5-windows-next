# Phase 0 reference baseline

Status: accepted for Phase 0  
Baseline date: 2026-08-17  
Task contract: CHANGE, limited in this batch to Phase 0 research artifacts

## Specification authority

The implementation baseline is `Fcitx5_for_Windows_工程规格_现代软件工程_轻量SSDLC_DevSecOps_Codex执行版_v1.4(1).md`, Frozen 1.4. The source copy inspected on 2026-08-17 had SHA-256:

```text
A89C5822479BBB8771416B214D024DE15CD8D7F161DFCCE86AA3F316BCD9AA9F
```

The specification is not copied into this repository. This document records the decisions needed to start implementation without turning reference repositories into product dependencies.

## Pinned sources

Branch heads below were resolved on 2026-08-17. A pin is evidence for this audit, not an instruction to vendor the repository. Before copying any source, inspect that file's copyright and license notice and record it in `THIRD_PARTY_NOTICES.md`.

| Source | Pin | License state at audit | Current use |
|---|---|---|---|
| [gaboolic/fcitx5-windows](https://github.com/gaboolic/fcitx5-windows/tree/72c0b91414dd6e8702209e7cb9c10deb86bb719b) | `72c0b91414dd6e8702209e7cb9c10deb86bb719b` | GPL-3.0 repository | Required subsystem audit; behavior and implementation-pattern reference only |
| [fcitx-contrib/fcitx5-windows](https://github.com/fcitx-contrib/fcitx5-windows/tree/aec96f21f06c30f75c1035242afbccfd29826681) | `aec96f21f06c30f75c1035242afbccfd29826681` | GPL-3.0 repository | Minimal Windows prototype and registration/TSF comparison |
| [fcitx/fcitx5](https://github.com/fcitx/fcitx5/tree/a31cff7968e7864fa6d40ca4764afc7b859bfb88) | `a31cff7968e7864fa6d40ca4764afc7b859bfb88` | REUSE metadata; core is LGPL-2.1-or-later, verify per file | Authoritative Fcitx core API and build behavior |
| [fcitx-contrib/fcitx5-plugins](https://github.com/fcitx-contrib/fcitx5-plugins/tree/46337894e0ab82cb9e1b7948ea31def00ecd3be2) | `46337894e0ab82cb9e1b7948ea31def00ecd3be2` | MIT repository | Cross-platform addon feasibility; not needed before Fcitx integration |
| [fcitx/fcitx5-macos](https://github.com/fcitx/fcitx5-macos/tree/5209c3e77d3334eccde72ded46eb82598a68bd56) | `5209c3e77d3334eccde72ded46eb82598a68bd56` | GPL-3.0 repository; dependencies have their own licenses | Core/frontend and package-management comparison |
| [Microsoft Windows classic samples](https://github.com/microsoft/Windows-classic-samples/tree/d59e5f1dc9c768615e4e1ab1f0f009e6a3ed747c) | `d59e5f1dc9c768615e4e1ab1f0f009e6a3ed747c` | GitHub reports NOASSERTION; inspect SampleIME notices before reuse | Primary TSF semantic reference, especially SampleIME |
| [rime/weasel](https://github.com/rime/weasel/tree/287ce64e202c9c573c7cfb5557048ad3f6a5e95c) | `287ce64e202c9c573c7cfb5557048ad3f6a5e95c` | GPL-3.0 repository | Production TSF lifecycle and host-compatibility patterns |
| [chewing/windows-chewing-tsf](https://github.com/chewing/windows-chewing-tsf/tree/342ead0c0b445ec376fbd6ffb3b105e78c499419) | `342ead0c0b445ec376fbd6ffb3b105e78c499419` | GPL-3.0 repository | Composition, UIElement, IPC ACL, and packaging patterns |
| [gaboolic/moqi-im-windows](https://github.com/gaboolic/moqi-im-windows/tree/13af5bc97eedc4c31bbf57418abe2427c8881aa5) | `13af5bc97eedc4c31bbf57418abe2427c8881aa5` | MIT repository | Request correlation and launcher lifecycle comparison |
| [EasyIME/PIME](https://github.com/EasyIME/PIME/tree/9f6a1e9161b7f609eb1fadf282048c2907da04c9) | `9f6a1e9161b7f609eb1fadf282048c2907da04c9` | GitHub reports NOASSERTION; inspect per-file notices | Launcher/process-model comparison only |
| [rimeinn/rabbit](https://github.com/rimeinn/rabbit/tree/b99f01109008f700eb0241770ccfceff0593e200) | `b99f01109008f700eb0241770ccfceff0593e200` | GPL-3.0 repository | Phase 5 caret/focus/password behavior and compatibility scenarios; no Hook code copied |
| [huanfeng/WindInput](https://github.com/huanfeng/WindInput/tree/e7a0693ef94312ec78e3df7e5b8a1c00490ce27b) | `e7a0693ef94312ec78e3df7e5b8a1c00490ce27b` | Inspect per-file notices before reuse | Phase 5 timeout, stale-response and recovery behavior only |
| [katahiromz/ImeStudy](https://github.com/katahiromz/ImeStudy/tree/8200c2552210d8ad913208679d1aa3aa1de28e7d) | `8200c2552210d8ad913208679d1aa3aa1de28e7d` | MIT repository | Phase 5 IMM32/CUAS host behavior and test-scenario reference |
| [mangokingTW/ImeModePersistence](https://github.com/mangokingTW/ImeModePersistence/tree/533b28d0bd70d609e62ad27e2d816bebd25e3cbd) | `533b28d0bd70d609e62ad27e2d816bebd25e3cbd` | MIT repository | Phase 5 legacy compatibility risk and scenario reference; no injection path inherited |

Microsoft documentation is versionless and is therefore pinned by access date rather than commit:

- [Text Services Framework](https://learn.microsoft.com/windows/win32/tsf/text-services-framework), accessed 2026-08-17.
- [Text Services Framework reference](https://learn.microsoft.com/windows/win32/api/_tsf/), accessed 2026-08-17.
- [Input method editor requirements](https://learn.microsoft.com/windows/apps/develop/input/input-method-editor-requirements), accessed 2026-08-17.
- [Dynamic-link library security](https://learn.microsoft.com/windows/win32/dlls/dynamic-link-library-security), accessed 2026-08-17.
- [Named-pipe security and access rights](https://learn.microsoft.com/windows/win32/ipc/named-pipe-security-and-access-rights), accessed 2026-08-17.

## Reference implementation matrix

The first source decides intended platform semantics. Secondary sources supply proven compatibility patterns; they do not override official semantics or this specification.

| Decision area | Primary reference | Secondary reference | Rule for implementation |
|---|---|---|---|
| TSF activation, sinks, key callbacks, edit sessions | Microsoft TSF + SampleIME | Weasel, Windows Chewing | Follow COM/TSF contracts first; retain host-specific workarounds only with a regression case |
| Composition lifecycle | SampleIME | Windows Chewing, Weasel | A context owns one current composition; distinguish expected termination from external abort |
| Fcitx integration | Fcitx upstream | gaboolic prototype, fcitx-contrib prototype | Keep upstream Fcitx in `fcitx5-engine.exe`; Windows code is a platform adapter |
| Thin host boundary | Project specification | gaboolic prototype as a bad/good-case source | TSF DLL contains COM/TSF plus a small bounded IPC client only |
| IPC framing and correlation | Project specification | Moqi, Windows Chewing | Current protocol only; explicit little-endian fields, size limits, request/response IDs, epoch/context/composition/revision |
| IPC identity and ACL | Microsoft named-pipe security | Windows Chewing, Moqi | SID + logon session namespace, explicit DACL, server/client PID and installed-image verification |
| IPC timeout and recovery | Microsoft overlapped I/O semantics | Moqi; WindInput when first needed | No unbounded synchronous read/write or `Sleep` polling on a host input thread |
| Launcher lifecycle | Project state machine | Moqi, PIME | Per-user/session launcher; warmup, bounded start, backoff, safe mode, no restart storm |
| Candidate state | Project CandidateModel contract | Windows Chewing, win-mcbopomofo when first needed | Engine is sole state owner; renderer receives immutable revisioned snapshots |
| Candidate rendering and UILess | Direct2D/DirectWrite + TSF UIElement docs | Windows Chewing, Weasel | Separate native UI process; UI crash must not stop composition or commit |
| Focus, caret, password fields | Microsoft TSF/InputScope | Weasel; Rabbit when first needed | Drop stale updates by identity; sensitive contexts do not learn, log, or predict |
| Win7 and modern Windows | Microsoft API contracts | Mature TSF implementations | Link only Win7-safe imports in Win7 processes; capability-detect modern paths at runtime |
| DLL loading | Microsoft DLL security | Windows Chewing packaging | Restrict search paths; no CurrentDir/Temp/Downloads dependency resolution |
| Addons and package types | Fcitx upstream | fcitx5-macos, fcitx5-plugins | Addons load only in engine; different package types keep different trust levels |
| Build and release | CMake/MSVC documentation | Windows Chewing | One entry point; build once and promote the same artifacts |

Repositories mentioned by the specification but not required by Phase 0 are deliberately not cloned or made dependencies. Their exact commit must be pinned when their problem becomes reachable (for example, UILess UI in Phase 4 or legacy games in Phase 5).

## Audit of `gaboolic/fcitx5-windows`

The audited commit is `72c0b91414dd6e8702209e7cb9c10deb86bb719b` (2026-04-05, `候选框改成webview (#6)`). The audit covered the TSF, IPC, engine, candidate/WebView, registration/build, and existing test paths. No code was copied.

### TSF frontend

Keep as behavior/reference evidence:

- The implementation has the expected COM/TSF surface: activation, thread-manager events, key-event sink, edit sessions, composition sink, compartments, and UIElement plumbing.
- `OnTestKeyDown` is deliberately a non-mutating peek, while `OnKeyDown` performs the edit session. This is the right response to duplicate Test/KeyDown callback sequences and directly informs `REG-TSF-001`.
- Candidate popup operations use non-activating window behavior (`SWP_NOACTIVATE`, `SW_SHOWNA`), which matches the no-focus-steal invariant.
- Registration and UTF-8/UTF-16 conversion code are useful implementation references, subject to file-level GPL obligations if copied.

Refactor or reimplement:

- TSF currently owns engine-shaped state through a broad `ImeEngine` interface and directly queries candidates, profiles, tray actions, and configuration operations. The new DLL gets only the minimal input data-plane client and a TSF-owned projection of revisioned engine state.
- Synchronous edit sessions are used around processing. Phase 1B may keep the minimal correct edit-session pattern, but IPC inside it must have a small bounded wait and fail open.
- Candidate positioning fallbacks are useful behavior evidence, but caret/focus identity must be tied to `context_id`, `composition_id`, `engine_epoch`, and `revision` before Phase 4.
- The source compiles with `_WIN32_WINNT=0x0A00` and directly calls `GetDpiForWindow`. The Win7 target must instead avoid a Win10 hard import and select modern DPI functions through capability detection.

Do not inherit:

- `makeDefaultImeEngine()` falls back to an in-process Fcitx engine or stub. Runtime fallback and dual architecture paths are forbidden; a missing/mismatched engine must fail open explicitly.
- Explorer-specific lifetime pinning, tray scheduling, profile/config reload, sub-config launching, and engine management make the host adapter too broad. These belong outside the DLL unless a TSF contract strictly requires a minimal bridge.
- No WebView, addon, configuration parser, network stack, or general-purpose Fcitx library may load in a host process.

### IPC

Keep as behavior/reference evidence:

- Frames have magic/version/body-size fields and a maximum packet size.
- Decoder paths check truncation, body length, candidate counts, and exact trailing bytes in several operations.
- The out-of-process option proves the TSF-to-engine split is feasible and provides a concrete first vertical-slice reference.

Refactor or reimplement:

- Protocol v1 has only a native C++ `ImeIpcFrameHeader` plus opcode/status. Protocol v2 needs an explicit wire contract with request correlation and state identities; both endpoints change together and v1 is deleted rather than supported alongside it.
- `memcpy` of a native struct is ABI/padding/endian dependent. Encode and decode every integer explicitly.
- Each response currently returns a large mixed snapshot including candidates, input methods, and tray actions. The initial vertical slice carries only handshake, key, minimal state, commit, and failure fields; management data belongs on a separate control contract later.
- The pipe endpoint uses a user name. Replace it with a SID + logon-session-derived endpoint and explicit security descriptor.

Do not inherit:

- `CreateNamedPipeW(..., nullptr)` uses the default security descriptor, and neither side verifies PID, image path, installation root, signature, SID, or session.
- Reads and writes are synchronous `ReadFile`/`WriteFile` loops with no cancellation or timeout. A stalled peer can block a host thread indefinitely.
- Launch can wait on a mutex for 60 seconds, then poll with `Sleep(50)` up to 100 times, followed by additional retry sleeps. This violates bounded input behavior and deterministic testing.
- Requests have no `request_id`/`response_to`; stale or interleaved responses cannot be rejected safely.
- The server accepts and serves one connection at a time despite `PIPE_UNLIMITED_INSTANCES`; multi-client x86/x64 hosts need independent concurrent sessions.
- The server has no launcher state machine, crash backoff, safe mode, epoch generation, or SYSTEM/LogonUI exclusion.

### Engine and input contexts

Keep as behavior/reference evidence:

- Fcitx `Instance` and addon loading can live in a standalone process.
- Key conversion, Fcitx event delivery, portable data-path handling, and commit/preedit/candidate extraction are valuable mapping references.
- A per-connection session already suggests separating client context state from the shared Fcitx process.

Refactor or reimplement:

- The authoritative owner becomes the engine's `InputContext`, keyed by explicit context and composition identities. The TSF and UI keep non-authoritative projections only.
- Fcitx calls must run through one dispatcher/event-loop owner. A request handler may not pump the event loop ad hoc to make UI state appear synchronous.
- Commit delivery must be correlated with a context/composition/revision; a global or session queue drained by a separate `ServerPopCommit` operation is not sufficient.

Do not inherit:

- Never link Fcitx Core, libime, Rime, Lua, or addons into `fcitx5-tsf.dll`. The prototype's non-IPC build path does so, and even its IPC CMake path links `Fcitx5::Core` headers/libraries into the TSF target and stages Core/Utils DLLs beside the TSF DLL.
- Do not let TSF initiate input-method configuration, Rime reload, sub-config execution, or tray actions over the input hot-path protocol.
- Do not maintain a stub-engine runtime fallback. Tests may use a separately built mock engine with the same current contract.

### Candidate UI and WebView

Keep as behavior/reference evidence:

- `ITfCandidateListUIElement` exposes count, selection, strings, and page data, proving UILess metadata can be supplied independently of visual painting.
- Popup placement, work-area clamping, no-activation display, DPI refresh, and device/resource fallback cases are useful test inputs.

Refactor or reimplement:

- Define one renderer-neutral `CandidateModel` contract owned by engine state. UIElement and the native renderer consume the same revisioned snapshot.
- Render in an independent `fcitx5-ui.exe` using Direct2D/DirectWrite. The TSF process keeps only UIElement semantics and caret-rectangle forwarding.

Do not inherit:

- WebView2 is enabled by default, created by `CandidateWindow` inside the TSF process, receives generated JavaScript/JSON state, loads frontend assets, and stages `WebView2Loader.dll` beside the TSF DLL. This directly violates the thin-host, Win7, dependency, and blast-radius requirements.
- Do not use npm/Parcel/WebView assets for the candidate hot path, and do not create a script bridge for candidate selection or resizing.
- Do not keep parallel GDI and WebView runtime renderers behind a feature switch. Implement the one current D2D/DWrite direction.

### Build, deployment, and tests

Keep as behavior/reference evidence:

- The repository contains useful registration/unregistration, portable staging, dependency-build, and installer experiments.
- The prototype documents real Windows/MSYS2, Unicode-path, addon, Rime, libime model, and runtime-DLL issues. Recheck those against the pinned upstream versions when the corresponding phase is reached.

Refactor or reimplement:

- Phase 1A establishes MSVC CMake presets and x86/x64 core shells first. Cross/MSYS2 dependency work is evaluated only when Fcitx integration becomes reachable.
- Registration becomes an explicit tool/script target. Installation and package activation remain later phases.

Do not inherit:

- The existing automated test is only two assertions for GUID formatting and UTF conversion. It does not prove TSF lifecycle, IPC bounds, failure behavior, composition, or commit.
- Do not copy ad-hoc build scripts, large patch collections, installers, or package-provider work into Phase 1A.
- Do not make network package installation part of an otherwise clean core build.

## Architecture decisions established by Phase 0

1. `fcitx5-tsf.dll` is x86/x64 and contains COM/TSF plus a minimal bounded IPC client; it has no Fcitx, addon, renderer, network, WebView, or user-config dependency.
2. `fcitx5-engine.exe` is the single native-OS-architecture owner of Fcitx and all authoritative input contexts.
3. Phase 1B uses a separate mock engine implementing the one current minimal protocol; it is a test/development peer, not a runtime fallback.
4. Protocol v2 is introduced as the first and only protocol in this repository. Version mismatch fails explicitly; no v1 parser or compatibility shim is implemented.
5. Phase 1B implements only the fields required to handshake, deliver one key event, return minimal preedit/commit state, and identify a request/context. Phase 2 adds the complete identity, authentication, concurrency, launcher, and fault-model requirements after the vertical slice exists.
6. The candidate renderer is a separate native process. Until Phase 4, no in-DLL visual renderer is expanded beyond what is strictly needed for the Phase 1B commit slice.
7. Win7 compatibility is an API-import constraint from the first binary. Modern behavior is capability-detected; it is not disabled globally.
8. Input and management planes remain separate. The Phase 1 protocol contains no package, update, configuration-launch, or network operations.

## Phase 0 acceptance record

| Requirement | Evidence | Result |
|---|---|---|
| Fix reference repositories/commits | Pinned-sources table with resolved branch-head SHAs and license state | Pass |
| Establish Reference Matrix | Decision-area matrix identifies primary and secondary sources plus implementation rule | Pass |
| Audit current gaboolic TSF/IPC/WebView/engine code | Subsystem audit records concrete keep/refactor/do-not-inherit findings | Pass |
| Identify the preferred source for each key design | Reference matrix covers TSF, composition, Fcitx, IPC, launcher, candidate state/UI, security, compatibility, build/release | Pass |
| Avoid README/old-memory implementation | Audit used pinned source files and official Microsoft semantics; no product implementation started | Pass |
| Avoid contaminating the product repository with reference trees | Reference clones were kept outside the workspace; only this evidence document is stored here | Pass |

Phase 0 adds no dependency, permission, process, protocol, hot-path cost, or distributable code. The next allowed work is Phase 1A's minimal build baseline.
