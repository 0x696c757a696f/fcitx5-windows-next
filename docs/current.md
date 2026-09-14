# Current Repository State

Status: current. Refresh this file from source and same-lineage evidence; do not promote a historical result to a current fact.

## Architecture and ownership

- Windows exposes one product TSF profile. Text commits use TSF EditSession; the input hot path is bounded and fail-open.
- Rust owns new product semantics. The durable C++ boundary is direct Fcitx5 core/addon object adaptation; narrowly necessary Win32/COM/ABI/renderer host code is an adapter, not product authority.
- TSF is a Rust shipping component. Its real Windows-host matrix remains external evidence.
- Candidate semantics, model and layout are Rust-owned; Fcitx remains the semantic input authority. Candidate presentation must not add a second, divergent preview renderer.
- Settings use typed Current/Draft/Defaults state, explicit Apply/Cancel, atomic persistence and real visible controls. A native host can exist only as a thin adapter while the Rust-owned contract is preserved.
- Package and plugin operations keep repository verification, network and signature handling outside Settings. Activation is transactional and rollback-capable.

## Current work

The only active task is [tasks/current.md](tasks/current.md). It defines a bounded product task; do not infer a release, host, accessibility, visual, signing, or online-package result from an automated check alone.

## Known evidence boundaries

- Release readiness requires the same final artifact lineage, protected signing/publishing evidence, and the declared host/compatibility matrix.
- Real host, screen-reader, DPI, online package lifecycle, UAC and signing results remain `MANUAL-PENDING` until recorded with environment and artifact identity.
- `tasks/status.md` and `tasks/completed/` preserve prior evidence. They may describe replaced files or earlier architecture and are not an implementation authority.

## Do not infer

- A screenshot, source-contract check, or passing unit test does not prove Settings, Candidate, or release completion.
- A historical C++/WTL/Win32/HWND test inventory does not define the current Settings implementation authority.
- Green local checks never substitute for unrun external/manual evidence.
