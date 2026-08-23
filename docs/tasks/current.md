# RUST-R3-TSF-POC Rust TSF differential PoC

**State:** MANUAL-PENDING / USER-GATE-OVERRIDE / USER-SHIPPING-CUTOVER-OVERRIDE / SHIPPING-RUST-TSF-X64-X86-GREEN / CXX-SHIPPING-TSF-DELETED / CXX-UILESS-CANDIDATE-DELETED / CXX-INPUT-SCOPE-GUIDS-DELETED / TSF-SUPPORT-ACTIVATION-GUARD-RUST-GREEN / ACTIVATABLE-EMPTY-TIP-GREEN / BEHAVIOR-CORPUS-GREEN / CPP-BASELINE-CORPUS-GREEN / RUST-BEHAVIOR-ABI-REPORT-GREEN / ARTIFACT-AUDIT-GREEN / COM-FAIL-OPEN-GREEN / REFERENCE-REFRESH-GREEN / REFERENCE-CORPUS-GREEN / SERVICE-LIFECYCLE-STATE-GREEN / PROFILE-IDENTITY-ABI-GREEN / IPC-BOUNDARY-GREEN / COMPOSITION-TRANSCRIPT-GREEN / ARM64-BUILD-ENV-PREFLIGHT-GREEN / ARM64-CI-ARTIFACT-GREEN / DIFFERENTIAL-SUMMARY-GREEN / PRODUCT-DECISION-RECORDED / REAL-HOST-MATRIX-PENDING

## Gate override

The original queued task required frozen C++ TSF behavior corpus and real host matrix evidence before implementation. On 2026-08-22 the user explicitly allowed opening this gate.

On 2026-08-23 the user explicitly clarified that replacing and deleting the shipping C++ TSF is authorized. This task is therefore allowed to cut the shipping `fcitx5-tsf.dll` target over to Rust and remove the old shipping C++ TSF implementation.

This override does **not** mark the unrun real-host matrix as passed and does **not** unblock release by itself.

## Scope

- Build the shipping Rust `fcitx5-tsf.dll` using `windows-rs`/COM/TSF bindings directly, without C++ FFI.
- Remove the old shipping C++ TSF implementation instead of keeping a permanent old/new runtime selector.
- Implement only the minimal path needed for `ITfTextInputProcessorEx`, key event sink, composition/commit, activation guard fail-open, candidate UI metadata, single `Fcitx5` profile registration model, and clean deactivate/unadvise/unload behavior.
- Contain every COM callback behind a panic boundary that returns `HRESULT`, resets local TSF state where needed, and fails open instead of aborting the host.
- Keep `unsafe` limited to small COM/Win32 adapter modules; domain state should use typed Rust models.

## Must not do

- Do not add global hooks, `SendInput` emulation, process injection, anti-cheat bypass, credential access, or external exploitation.
- Do not link Fcitx/libime or package/update/config dependencies into the TSF DLL.
- Do not create a permanent runtime selector.

## Required validation

- C++ TSF vs Rust TSF differential against the same mock engine and behavior corpus.
- x86/x64 build and export checks for TSF DLL shape.
- Panic-containment regression for forced internal failure, malformed IPC, unexpected COM state, and engine timeout.
- Real host matrix comparison before any cutover decision.
- Dependency, PE import, min-OS, binary-size, COM refcount/sink cleanup, and unload checks.

## Done when

- The Rust TSF shipping DLL has objective automated differential/export/artifact/security evidence recorded for x64/x86.
- The old shipping C++ TSF implementation is deleted and no permanent runtime selector remains.
- Real-host matrix evidence is either recorded green or this task is archived as `MANUAL-PENDING` with exact missing evidence.
