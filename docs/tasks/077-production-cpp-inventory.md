# Task 077 production C++ inventory

Baseline HEAD: `c6a804edcd093b98b8940730853f7301df58a21a`.
Latest integrated cutover HEAD: `2788831`.

This is the live cutover ledger. `KEEP-CANDIDATE` still requires symbol-level reduction and a final
unconditional classification. There are no accepted conditional rows at 077 completion.

| Current file(s) | Initial class | Required final state |
|---|---|---|
| `protocol/protocol.cpp`, `protocol/protocol.h` | MIGRATE | Delete C++ DTO/bridge after consumers use Rust protocol ABI |
| `protocol/protocol_ffi.h`, `resources/windows/resource.h` | KEEP-CANDIDATE ABI | Retain declarations/resource IDs only where required |
| `src/config/app_main.cpp` | DELETED at `3cf297b` | Rust Settings is the sole shipping shell; legacy WTL target and differential test are deleted |
| `src/config/config_model.h`, `src/config/config_parser.cpp` | DELETED at `2788831` | Config Core owns typed parsing, validation, recovery, persistence, and resolved Candidate visuals |
| `src/control/control_main.cpp` | DELETED at `955137e` | Rust is the sole shipping Control executable and owns Engine/Launcher commands plus signed repository/package lifecycle |
| `src/package/package_core.cpp`, `package_core.h` | DELETED at `2788831` | Rust package-core and the shipping Rust lifecycle binaries are the only package authority |
| `src/package/fcitx5_mldsa65_config.h` | KEEP-CANDIDATE native ABI | Retain only if the Rust package build's audited native verifier needs it |
| `src/ui/ui_main.cpp` | MIGRATE/MIXED; Rust visual snapshot landed at `596a31b` | Config/theme/default/Safe Mode/label semantics now come from Config Core; move remaining product state/protocol/orchestration to Rust and retain only necessary HWND/D2D/DWrite seam |
| `src/launcher/launcher_main.cpp`, `state_machine.h`, `state_store.h` | DELETED at `240496e` | Rust is the sole Launcher supervisor, command, state-store, process, IPC-server, and shipping executable owner |
| `src/launcher/tray_icon.cpp`, `tray_icon.h`, `launcher_rust_abi.h` | DELETED at `240496e` | No default tray is required; the temporary tray and flat C++ ABI seams were removed |
| `src/engine/mock_engine_main.cpp` | DELETED at `601a4d0` | Safe Rust fixture preserves the final mixed IPC/Launcher/TSF corpus and exact verified pipe-peer handshake |
| `src/engine/presentation_publisher.cpp`, `presentation_publisher.h` | DELETED at `7d1aeaa` | Rust Engine owns validation, coalescing, exact peer transport, bounded I/O, retry/reconnect, stop, and destruction; only the opaque ABI RAII call site remains inside the approved Fcitx process adapter |
| `src/ipc/pipe_client.cpp`, `pipe_client.h`, `launcher_client.cpp`, `launcher_client.h` | MIGRATE | Delete after Rust-owned process consumers use Rust IPC APIs |
| `src/ipc/peer_verification.cpp`, `peer_verification.h` | KEEP-CANDIDATE native ABI | Remove duplicate policy; retain only necessary peer identity mechanics |
| `src/platform/runtime_identity.*`, `pipe_security.*` | KEEP-CANDIDATE native ABI | Remove duplicate policy; retain only necessary Win32/C ABI mechanics |
| `src/engine/fcitx_dispatcher.*`, `fcitx_runtime.*`, `key_event.*`, `windows_keyboard.cpp` | KEEP-CANDIDATE Fcitx | Retain only direct Fcitx event/object/key/addon adapter symbols |
| `src/engine/fcitx_engine_main.cpp` | KEEP-CANDIDATE mixed | Retain Fcitx lifecycle/native pipe host; migrate product protocol/routing/policy |
| `src/engine/engine_core_ffi.h` | KEEP-CANDIDATE ABI | Narrow Rust Engine declarations only |
| `src/pch/fcitx_windows_pch.h` | DELETE-CANDIDATE | Delete unless remaining approved C++ adapters still benefit |

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
files. These facts do not complete the other still-`MIGRATE` rows.

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
lifecycle behavior and is registered as `rust-control-package-lifecycle` on x64/x86. This does not
delete the old C++ Config/package bridges while other consumers and retained mixed tests still use
them. At `2788831`, the remaining consumers moved to Rust public contracts, both bridges and their
C++-authoritative tests were deleted, and the retained final Candidate mixed E2E applies typed TOML
through the public Rust Control/Config transaction. Rust Windows-common restores the required live
Candidate notification behind one named low-level Win32 exception while the Rust Control binary
remains `forbid(unsafe_code)`.

Current facts: shipping TSF, Settings, Launcher, Control, and the mock Engine fixture are Rust-owned;
Candidate semantics and resolved visual configuration are Rust-owned; Rust protocol, Engine,
package, Windows-common, process-execution, and Config owners already exist. The remaining migration
rows are Candidate adapter reduction, the protocol/IPC bridges, and residual platform/Engine symbol
classifications. No current-HEAD release stage exists yet.
