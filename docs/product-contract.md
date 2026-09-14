# Product Contract

Status: current. This is the compact, long-lived product contract. Task files may add bounded acceptance criteria but may not weaken these rules.

## 1. Product and security invariants

- Windows registers one product TSF profile. Engines and addons remain internal Fcitx state, not additional Windows input profiles.
- Text commits travel only through TSF EditSession. The TSF hot path has a bounded deadline and fails open: a host must not lose ordinary typing when Fcitx is unavailable.
- No Hook, SendInput, injection, UI Automation, coordinate clicking, undocumented window messages, or process-memory access participates in input, commit, build, or recovery behavior.
- The TSF DLL does not load addons, Rime, Lua, downloaders, extractors, or GUI code. It delegates through bounded IPC/runtime boundaries.
- Product diagnostics never contain raw keys, preedit, candidates, commits, or user dictionary contents.

## 2. Ownership and architecture

- New product-owned Windows semantics default to safe Rust. Pure Rust components use `#![forbid(unsafe_code)]`; FFI/Win32/COM/ABI seams keep unsafe narrow, documented, and auditable.
- The durable C++ island is direct Fcitx5 core/addon object adaptation (`fcitx::Instance`, input contexts, addon/config objects, input panels and candidate lists). It is not a general product-state exception.
- Candidate model, layout, interaction, selection presentation and Settings/product policy are Rust-owned. A renderer/window host may be native only as a thin adapter with equivalent regression, accessibility, DPI, localization and visual evidence before it can be removed.
- Fcitx core and each addon upstream define Fcitx semantics. Other Windows ports are compatibility references, never architecture authority.

## 3. Runtime boundaries

- TSF is the host-facing Rust component; real host compatibility is an external evidence gate.
- Engine keeps a thin C++ Fcitx adapter while Rust owns product protocol, state, validation, IPC, deadlines, generations, revisions, snapshots and diagnostics.
- Candidate snapshots are bounded; UI callbacks are intents, never local commit decisions.
- Control, package, provider, downloader, updater, deployer, register and bootstrap policy are Rust-owned or Rust CLI cutovers. Native elevation/registry/process seams must stay narrow.

## 4. Settings and theme contract

- Settings have typed `Current`, `Draft`, and `Defaults` state. Every visible control declares Live, Deferred, or Restart-required behavior; Deferred changes expose Apply/Cancel and persistence is atomic.
- A visible control is fully bound, read-only status, explicitly unavailable, or clearly fake/demo—never silently decorative.
- Preview consumes the same production CandidateModel/layout/theme render contract as the candidate window; no independent preview renderer is permitted.
- Themes use strict `theme.toml` and bounded assets. Third-party themes execute no code. Typed resolution precedes rendering; light/dark token parity, unit-aware geometry, valid numeric input and system-font fallback are required.

## 5. Candidate contract

- Fcitx remains the semantic authority for candidate ordering, selection and commit. Rust CandidateModel/layout/render state owns presentation.
- Supported user-facing modes are `automatic`, `stacked`, `flow`, `scroll`, and `vertical_text`; they map deterministically to the persisted layout model.
- Labels reserve stable slots, candidate text origins do not jump, selected items remain revealed without layout shift, and CJK/emoji/comments/high-contrast/DPI behavior has regression evidence.
- Scroll is an expanded bounded browser, not a third orientation: input stays responsive, candidate transport is capped, and renderer state never creates an unbounded UI tree.

## 6. Plugin and package contract

- Verified repository metadata is authoritative. Settings does not perform networking or signature verification.
- Package activation is staged, transactional and rollback-capable; archives and manifests fail closed before activation. Bundled/protected components cannot be destructively managed.
- Program/version directories are distinct from user data. Upgrade and removal touch only declared owned files; in-use TSF upgrades drain generations without forcibly closing hosts.

## 7. Evidence and release gates

- A test result applies only to the recorded source, configuration, architecture and artifact lineage. Historical tests, screenshots and staged artifacts do not prove current completion.
- External evidence is fail-closed: real host matrix, accessibility, online package lifecycle, UAC, protected signing/publishing, final artifact smoke and compatibility results must be recorded as such.
- Build once, test once, then promote exactly those bytes. Signing/publish jobs do not recompile. Final artifacts require hashes, signed manifest, SBOM, provenance and rollback evidence.

## 8. Explicit non-goals

- No default WebView2, Tauri, Qt, heavy GUI runtime, theme script, arbitrary grid, second renderer, persistent C++/Rust authority split, or TSF-loaded addon runtime.
- No claim of release readiness from automated checks alone.
