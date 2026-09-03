# Task 077 production C++ inventory

Baseline HEAD: `c6a804edcd093b98b8940730853f7301df58a21a`.
Latest integrated cutover HEAD: `1781b2d` (protocol DTO bridge deleted at `ac6d200`; AGENTS/
queue docs at `b282dfd`; x86 protocol cutover verification at `1781b2d`).

This ledger is now closed: every row below has an unconditional `KEEP` (with reason) or
`DELETED at <commit>` state. No `KEEP-CANDIDATE`, `MIGRATE`, or conditional row remains.

| Current file(s) | Initial class | Required final state |
|---|---|---|
| `protocol/protocol.cpp`, `protocol/protocol.h` | DELETED at `ac6d200` | C++ DTO/bridge deleted; all consumers now use the Rust `protocol_ffi.h` flat ABI |
| `protocol/protocol_ffi.h`, `resources/windows/resource.h`, `src/config/config_snapshot_ffi.h` | KEEP (ABI declarations) | Flat C ABI declarations and resource IDs only; no C++ product logic. `config_snapshot_ffi.h` is consumed only by the Fcitx adapter and Candidate renderer to borrow the Rust Config snapshot |
| `src/config/app_main.cpp` | DELETED at `3cf297b` | Rust Settings is the sole shipping shell; legacy WTL target and differential test are deleted |
| `src/config/config_model.h`, `src/config/config_parser.cpp` | DELETED at `2788831` | Config Core owns typed parsing, validation, recovery, persistence, and resolved Candidate visuals |
| `src/control/control_main.cpp` | DELETED at `955137e` | Rust is the sole shipping Control executable and owns Engine/Launcher commands plus signed repository/package lifecycle |
| `src/package/package_core.cpp`, `package_core.h` | DELETED at `2788831` | Rust package-core and the shipping Rust lifecycle binaries are the only package authority |
| `src/package/fcitx5_mldsa65_config.h` | KEEP (native ABI config) | Verify-only ML-DSA-65 configuration consumed by `rust/package-core/build.rs` to compile the audited native verifier; ships no private-key/sign API |
| `src/ui/ui_main.cpp` | KEEP (HWND/D2D/DWrite renderer adapter) | Config/theme/default/Safe Mode/label semantics come from Config Core via `config_snapshot_ffi.h`; protocol decode is the Rust `protocol_ffi.h` codec; model/layout/presentation/hit-test/selection are the Rust candidate-core ABI; C++ retains only the native window/renderer and the self-test/demo harness required by the final mixed-binary `candidate-ui-*` CTest group |
| `src/launcher/launcher_main.cpp`, `state_machine.h`, `state_store.h` | DELETED at `240496e` | Rust is the sole Launcher supervisor, command, state-store, process, IPC-server, and shipping executable owner |
| `src/launcher/tray_icon.cpp`, `tray_icon.h`, `launcher_rust_abi.h` | DELETED at `240496e` | No default tray is required; the temporary tray and flat C++ ABI seams were removed |
| `src/engine/mock_engine_main.cpp` | DELETED at `601a4d0` | Safe Rust fixture preserves the final mixed IPC/Launcher/TSF corpus and exact verified pipe-peer handshake |
| `src/engine/presentation_publisher.cpp`, `presentation_publisher.h` | DELETED at `7d1aeaa` | Rust Engine owns validation, coalescing, exact peer transport, bounded I/O, retry/reconnect, stop, and destruction; only the opaque ABI RAII call site remains inside the approved Fcitx process adapter |
| `src/ipc/pipe_client.cpp`, `pipe_client.h`, `launcher_client.cpp`, `launcher_client.h` | KEEP (thin native IPC transport, final mixed-binary consumer) - **pending deletion at Stage 3 of 078** | `ui_main.cpp` production use migrated to the Rust opaque candidate-select client (`fcitx5_windows_common_candidate_select_client_*`) at 078 Stage 1/2; the four C++ files remain only because the integration tests still link `fcitx5::ipc_client` until the Stage 3 Rust-client migration. Marshal pipe transport + peer verification against the Rust windows-common + protocol-core C ABI; frame validation, accept/reject, and response scalars are Rust-owned (`fcitx5_windows_common_pipe_transact(_with_error)`, `_open/close_pipe_client_utf16`, `_deadline_*`, `_next_pipe_client_request_id`, `_next_launcher_request_id`, `_utf8_to_wide_utf16`, `_utf8_offset_to_wide`, `_apply_hello/key/launcher/engine_status_response_scalars`, `_accept_candidate_select_request/response`, `_ipc_status_ok`, `_set_last_error`; `fcitx5_protocol_core_encode_hello/key/candidate_select/launcher_request`, `decode_hello/key/candidate_select/launcher/engine_status_response`). Local `KeyResult`/`CaretRect`/`EngineStatusResult`/`LauncherResponse` mirrors and per-context `ContextState` (composition/revision) are transport request bookkeeping, not product authority. Consumers remaining: `tests/integration/ipc_*`, `launcher_*` groups (Stage 3 migrates them to Rust clients). |
| `src/ipc/peer_verification.cpp`, `peer_verification.h` | KEEP (native peer-identity ABI seam, verified policy-free) | All peer policy is delegated to Rust `fcitx5_windows_common_verify_pipe_server_peer_utf16` / `verify_pipe_client_peer_utf16`; C++ contains only `extern "C"` declarations + two-pass query/fill marshalling (bool→u8 flags, SID/executable-path copy) plus a compile-time macro read (`developmentPeerExceptionEnabled`). Consumers: `fcitx_engine_main.cpp` pipe-host accept path (line 457) and `ui_main.cpp` `servePresentation` (line 2871) |
| `src/platform/runtime_identity.*`, `pipe_security.*` | KEEP (native Win32/C ABI seam, verified policy-free) | Identity/generation/executable-match and pipe-security policy live in Rust `fcitx5_windows_common_local_name_utf16`, `_local_test_namespace_utf16`, `_current_generation_utf16`, `_current_generation_for_module_utf16`, `_current_generation_from_install_root_utf16`, `_installation_root_for_module_utf16`, `_portable_data_root_for_module_utf16`, `_default_data_root_for_module_utf16`, `_may_launch_user_engine_utf16`, `_executable_files_match_utf16`, `_paths_refer_to_same_file_utf16`, `_executable_paths_match_utf16`, `_process_identity_with_executable_file_utf16`, `_current_identity_with_executable_file_utf16`, `_executable_file_identity_utf16`, `_pipe_security_create_utf16`/`_attributes`/`_destroy`. C++ only runs the `rustWide` two-pass query/fill wrapper or the `PipeSecurity` RAII holder; no product decision. Consumers: engine host, IPC clients, and the Candidate renderer host |
| `src/engine/fcitx_dispatcher.*`, `fcitx_runtime.*`, `key_event.*`, `windows_keyboard.cpp` | KEEP (direct Fcitx adapter) | Only direct `fcitx::Instance`/`InputContext`/`EventDispatcher`/key/addon object access, the Rust engine-core ledger/snapshot FFI, and the Rust protocol key DTO; product ledger/state/snapshot policy is Rust-owned. The snapshot-store blob marshalling helpers (`serializeSnapshot`/`deserializeSnapshot`) mirror the Rust `snapshot.rs` canonical format, which Rust `snapshot_store_put` decodes/validates fail-closed on every store |
| `src/engine/fcitx_engine_main.cpp` | KEEP (native process host) | Windows pipe host that starts the Fcitx adapter, routes decoded Rust-protocol requests, and owns no product protocol/routing policy beyond the native host/pipe loop |
| `src/engine/engine_core_ffi.h` | KEEP (ABI declarations) | Flat declarations of the Rust engine-core C ABI only; no implementation |
| `src/pch/fcitx_windows_pch.h` | DELETED at `23fb50e` | Obsolete project PCH option/header removed; remaining approved adapters compile directly |

Baseline counts: 41 project-owned production `.cpp/.h`; 30 project-owned test/support `.cpp/.h`.
Test classification starts from `docs/tasks/071-test-ownership-inventory.md` and must be rechecked
after each product process cutover. `source_contract_test.cpp` is only a structure supplement.

After `3cf297b`: 40 production `.cpp/.h`; 29 test/support `.cpp/.h`.
`tests/unit/source_contract_test.cpp` and its CMake target were deleted at the cutover boundary;
Rust public-behavior tests remain authoritative and a Rust-authored unsafe/source policy gate replaces
the temporary C++ string-matching supplement.

After `1ec0921`, Engine request decoding accepts every current request type through Rust
`protocol-core` and rejects malformed or response frames. After `ce5eafa`, Launcher Engine
supervision owns deterministic launch/readiness/stop/reap/forced-termination error handling in Rust.
After `639a1c9`, Engine presentation publication policy is Safe Rust; only native transport and peer
mechanics remained. At `7d1aeaa`, verified pipe transport and lifetime moved to Rust, the two C++
publisher files were deleted, x64/x86 each passed 141 Rust tests, and the GNU/native
`fcitx5-engine.exe` target built successfully. At `240496e`, the Rust Launcher became the shipping
x64/x86 Debug and Release executable, kept an always-available next named-pipe listener, passed the
final mixed lifecycle/crash-to-Safe-Mode E2E tests, and deleted all six remaining Launcher C++/header
files. These were the final Launcher rows; the still-`MIGRATE` rows at that point (Candidate,
Config/package/Control, protocol/IPC) were closed by the later commits recorded above.

After `601a4d0` and `955137e`, the C++ mock Engine fixture and 2,143-line C++ Control process are
deleted. Rust Control and package lifecycle share one signed `repository/index.json` /
`index.sig.json` cache, preserve verified offline archives, and pass the formerly known-failing
stopped-service and repository rollback mixed tests on x64/x86. The repository-wide Rust source
policy rejects every `allow(unsafe_code)`, requires `forbid(unsafe_code)` by default, and requires
named low-level exceptions to deny unsafe operations inside unsafe functions.

At `596a31b`, Candidate stopped consuming the C++ Config model/parser for visual state. Config Core
now resolves Current/LKG/default recovery, selected local theme light/dark layers, Safe Mode,
validated colors/fonts, and custom label sequences through one narrow snapshot ABI. The Rust
Control/package E2E owns signed repository, package install/update/repair/remove, and theme
lifecycle behavior and is registered as `rust-control-package-lifecycle` on x64/x86. At `2788831`,
the remaining consumers moved to Rust public contracts, both bridges and their C++-authoritative
tests were deleted, and the retained final Candidate mixed E2E applies typed TOML through the
public Rust Control/Config transaction. Rust Windows-common restores the required live Candidate
notification behind one named low-level Win32 exception while the Rust Control binary remains
`forbid(unsafe_code)`.

At `23fb50e`, the obsolete project PCH and its CMake policy were deleted. The C++ version behavior
test and C++ key-roundtrip benchmark were also deleted: Windows-common Rust tests retain version /
channel / architecture authority, while a new Safe Rust measurement-core benchmark owns the final
verified-pipe roundtrip measurement on x64/x86. The direct Fcitx key adapter now consumes the flat
Rust protocol C ABI instead of the C++ protocol DTO. The remaining C++ protocol/IPC bridge was
fully deleted at `ac6d200` (see the closed row above); consumers now call `protocol_ffi.h` directly.

At `9a63682`, the rejected legacy Rust Win32 Settings/QA host was deleted together with its
environment-variable runtime selector and old control-ID automation. The sole interactive Config
path is the existing WindUI shell containing Candidate layout/theme controls and the plugin manager.
Config QA is now a small `forbid(unsafe_code)` WindUI screenshot/report/PNG contract, and the
obsolete duplicate CTest plus two unsafe-policy exceptions were removed. The build entry point also
invalidates an architecture cache whose effective C/C++ target flags no longer match its preset,
preventing an x86 build from reusing x64 clang objects against x86 CRT libraries.

Current facts: shipping TSF, Settings, Launcher, Control, the Candidate UI renderer host, and the
mock Engine fixture are integrated into their shipping C++/Rust boundaries; Candidate semantics,
resolved visual configuration, protocol codec/validation, Engine ledger/product state/snapshot,
package, Windows-common, process-execution, and Config are Rust-owned. All remaining project-owned
production C++ is classified above as an approved `KEEP` (direct Fcitx adapter, necessary native
Win32/C ABI seam, ABI declaration header, or required renderer/process host) or `DELETED`. There are
no open `MIGRATE`/`KEEP-CANDIDATE` rows. No current-HEAD release stage exists yet; real-host
Accessibility/signing/UAC/Win7 evidence remains external and unclaimed.
