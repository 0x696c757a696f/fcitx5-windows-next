# Task 077 production C++ inventory

Baseline HEAD: `c6a804edcd093b98b8940730853f7301df58a21a`.
Latest integrated cutover HEAD: `3cf297bc857c335c2ef2ce02e3310bfc133e2b18`.

This is the live cutover ledger. `KEEP-CANDIDATE` still requires symbol-level reduction and a final
unconditional classification. There are no accepted conditional rows at 077 completion.

| Current file(s) | Initial class | Required final state |
|---|---|---|
| `protocol/protocol.cpp`, `protocol/protocol.h` | MIGRATE | Delete C++ DTO/bridge after consumers use Rust protocol ABI |
| `protocol/protocol_ffi.h`, `resources/windows/resource.h` | KEEP-CANDIDATE ABI | Retain declarations/resource IDs only where required |
| `src/config/app_main.cpp` | DELETED at `3cf297b` | Rust Settings is the sole shipping shell; legacy WTL target and differential test are deleted |
| `src/config/config_model.h`, `src/config/config_parser.cpp` | DELETE-AFTER-CUTOVER | Delete remaining C++ Config parser/model authority after native consumers use Config Core |
| `src/control/control_main.cpp` | MIGRATE | Replace shipping Control process with Rust and delete |
| `src/package/package_core.cpp`, `package_core.h` | DELETE-AFTER-CUTOVER | Delete C++ package bridge after Rust consumers cut over |
| `src/package/fcitx5_mldsa65_config.h` | KEEP-CANDIDATE native ABI | Retain only if the Rust package build's audited native verifier needs it |
| `src/ui/ui_main.cpp` | MIGRATE/MIXED | Move all product state/protocol/config/orchestration to Rust; retain only necessary HWND/D2D/DWrite seam |
| `src/launcher/launcher_main.cpp`, `state_machine.h`, `state_store.h` | MIGRATE | Replace launcher product process/state/store with Rust and delete |
| `src/launcher/tray_icon.cpp`, `tray_icon.h`, `launcher_rust_abi.h` | KEEP-CANDIDATE ABI | Retain only a required tray/flat ABI seam, otherwise delete |
| `src/engine/mock_engine_main.cpp` | MIGRATE | Replace with Rust fixture and delete |
| `src/engine/presentation_publisher.cpp`, `presentation_publisher.h` | MIGRATE | Move publication state/policy to Rust and delete or retain a mechanics-only seam |
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

Current facts: shipping TSF and Settings are Rust-owned; Candidate semantics are Rust-owned; Rust
protocol, Engine, package, Control, launcher, Windows-common, process-execution, and Config owners
already exist. The first Release package attempt at this baseline stopped in old `ui_main.cpp` under
`/WX`, so no current-HEAD stage exists yet.
