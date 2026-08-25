# Rust TSF PoC decision — 2026-08-22

> 2026-08-25 status: superseded by the user-approved Rust TSF shipping cutover recorded in
> `docs/tasks/current.md`, `docs/tasks/PLAN.md`, `docs/tasks/rebaseline.md`, and `docs/current.md`.
> This file remains historical decision evidence only. It must not be read as a current instruction
> to keep the shipping TSF implementation in C++.

## Historical decision

At the time of this decision, the project continued the isolated Rust TSF PoC and deferred shipping
cutover. That decision has since been superseded by the 2026-08-23 user-approved Rust TSF shipping
cutover.

## Evidence available

- Rust PoC DLL is activatable through `DllGetClassObject` and exposes `ITfTextInputProcessorEx` plus key/thread/focus sink interfaces.
- Rust PoC COM/ABI exports are panic-contained and length-delimited.
- Shipping C++ TSF baseline E2E consumes the shared `tests/fixtures/tsf_behavior_corpus.json`.
- Rust PoC behavior report covers the shared 10-case corpus.
- Rust PoC artifact audit validates PE shape, min-OS/subsystem, ASLR/NX, binary-size budget, and absence of network/product-engine/config/package/control imports on runnable x64/x86 lanes.
- Rust PoC profile identity report matches the stable release identity: product `Fcitx5 for Windows Next`, profile `Fcitx5`, text-service CLSID `3a21b9e2-4f47-4c36-8bfa-91d7d3b3e901`, language profile GUID `6c2ac726-7703-4b65-89af-a77e9e0da102`.
- Rust PoC IPC boundary report models bounded fail-open behavior for timeout, malformed replies, and generation mismatch.
- Rust PoC composition transcript report records ordered single-edit-session commit/preedit behavior.
- Rust PoC differential summary report records green automated evidence and explicitly keeps ARM64 artifact, real host matrix, and product decision/cutover as pending.

## Cutover blockers

- ARM64 artifact evidence is not green on the current machine because the Visual Studio ARM64 C++ toolchain component is not installed.
- Real host matrix evidence is still manual/unrun for application hosts and scenarios tracked by `STAB-HOST-MATRIX-015`.
- Host-level C++↔Rust differential is still missing for real TSF activation, key, composition, commit, IPC, sink, profile, unload, DPI, UILess, and long-running host behavior.
- No shipping installer/registration path points to the Rust TSF PoC.

## Historical next decision point

At the time of writing, the next decision point was a separate cutover task after ARM64 artifact
evidence, real host matrix evidence, and full host-level differential evidence. Current task files
record the later cutover override and the remaining real-host/manual evidence blocker.
