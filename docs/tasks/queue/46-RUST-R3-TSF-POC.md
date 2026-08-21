# RUST-R3-TSF-POC Rust TSF differential PoC

**State:** FUTURE-GATED

## Gate

Start only after:

- Current C++ TSF behavior is frozen with a reusable corpus for activation/deactivation, key down/up, composition, commit, IPC timeout/fail-open, sink lifecycle, profile registration, and DLL unload behavior.
- Required host matrix evidence exists for the C++ baseline, including Notepad, Word/Office, Chrome/Edge, VS Code, Terminal, RDP, x86 host coverage, UILess, DPI, and game/anti-cheat smoke on the normal TSF path.
- R1/R2 package/update/launcher/process migration work is not blocked by this PoC.

## Scope

- Build an isolated Rust `fcitx5-tsf` PoC using `windows-rs`/COM/TSF bindings directly, without C++ FFI.
- Keep the C++ TSF as the shipping authoritative implementation until differential and real-host evidence prove the Rust PoC is not worse.
- Implement only the minimal path needed for `ITfTextInputProcessorEx`, key event sink, composition/commit, bounded IPC client, single `Fcitx5` profile registration model, and clean deactivate/unadvise/unload behavior.
- Contain every COM callback behind a panic boundary that returns `HRESULT`, resets local TSF state where needed, and fails open instead of aborting the host.
- Keep `unsafe` limited to small COM/Win32 adapter modules; domain state should use typed Rust models.

## Must not do

- Do not replace or delete the C++ TSF during the PoC.
- Do not add global hooks, `SendInput` emulation, process injection, anti-cheat bypass, credential access, or external exploitation.
- Do not link Fcitx/libime or package/update/config dependencies into the TSF DLL.
- Do not create a permanent runtime selector. If a future cutover is approved, it must have a separate task that removes the old authoritative implementation.

## Required validation

- C++ TSF vs Rust TSF differential against the same mock engine and behavior corpus.
- x86/x64 build and export checks for TSF DLL shape.
- Panic-containment regression for forced internal failure, malformed IPC, unexpected COM state, and engine timeout.
- Real host matrix comparison before any cutover decision.
- Dependency, PE import, min-OS, binary-size, COM refcount/sink cleanup, and unload checks.

## Done when

- The Rust TSF PoC has objective differential and host evidence recorded.
- A product decision is recorded: continue PoC, abandon, or create a separate cutover task.
- Shipping C++ TSF remains unchanged unless a later cutover task explicitly authorizes replacement.
