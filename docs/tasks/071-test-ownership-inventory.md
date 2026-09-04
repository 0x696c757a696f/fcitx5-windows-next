# Task 071 Test Ownership Inventory

Task: `RUST-TEST-AUTHORITY-CUTOVER-001`

This is the frozen repository inventory for the 071 cutover. The source inventory
covers every C/C++ file under `tests/` that is a test, support, fixture, fuzz
harness, or performance harness. The target inventory covers every related
CMake target and CTest registration in the top-level and `native-engine`
CMake files. The formal class column uses exactly `KEEP`, `MIGRATE`, or
`DELETE`. `MIGRATE` means that Rust is the intended authority and the C++
source/registration may be removed only after the replacement named here is
green. `KEEP` means the C++ boundary is permitted by Task 071. `KEEP` rows
marked `KEEP-TEMP` are temporary, non-authoritative resource/source-structure
supplements; they do not replace Rust public behavior and are not unfinished
Rust migrations. `DELETE` records an obsolete source or registration removed
after its Rust replacement was verified. There are no unfinished `MIGRATE` or
`DELETE` rows: each such row below includes replacement and deletion evidence.

## Ownership rules

Long-term C++ tests are limited to these boundaries:

1. Direct Fcitx adapter behavior (`fcitx::Instance`, `InputContext`, Fcitx
   key conversion, or the thin Engine adapter).
2. Necessary Win32/COM/ABI or release-artifact boundary behavior.
3. Final mixed C++/Rust integration or E2E behavior.

Rust-owned behavior is tested through the owning crate's public API. C++ source
contracts are structural supplements only and are not accepted as replacements
for Rust behavior, property, fault, fuzz, or performance coverage.

## C++ source inventory

| File | Class | Owner / reason | Rust replacement or keep boundary | Deletion / registration evidence |
|---|---|---|---|---|
| `tests/fixtures/crashing_engine_main.cpp` | KEEP | Launcher mixed-binary crash fixture | Final launcher/engine integration fixture; no product authority | `CMakeLists.txt:1543-1545`; retained by `launcher-crash-loop-safe-mode` |
| `tests/fuzz/package_fuzz.cpp` | MIGRATE | Package path/archive parser fuzz authority | `rust/package-core` public parser plus deterministic malformed/archive fuzz smoke and shared path corpus | Deleted; `rust-package-core-fuzz-smoke` and `rust-package-core-path-corpus-fuzz-smoke` replace the C++ registrations in `CMakeLists.txt` |
| `tests/fuzz/protocol_fuzz.cpp` | MIGRATE | Protocol codec fuzz authority | `rust/protocol-core` public codec plus deterministic malformed-frame fuzz smoke | Deleted; `rust-protocol-fuzz-smoke` replaces `protocol-fuzz-smoke` in `CMakeLists.txt` |
| `tests/integration/candidate_ui_config_integration_test.cpp` | KEEP | Candidate UI/Control mixed integration and screenshot boundary | Final mixed C++ UI + Rust Candidate/Control integration | `CMakeLists.txt:1468-1481`; retained as `candidate-ui-live-presentation-contract` |
| `tests/integration/control_package_integration_test.cpp` | KEEP | Control/downloader/package mixed integration | Final mixed executable/package lifecycle boundary with fixture signer | `CMakeLists.txt:977-995`; retained as `control-package-stopped-service-contract` |
| `tests/integration/fcitx_engine_integration_test.cpp` | MIGRATE | Real-Fcitx engine mixed integration | `rust/ipc-client/src/bin/engine_e2e.rs` (safe Rust driver over the real `fcitx5-engine.exe` + `fcitx5-ui.exe`; same six scenario flags and assertions) | Deleted in 078 engine-E2E slice; `tools/test-fcitx.ps1` builds the `fcitx5_engine_e2e` CMake target and runs it with the real staged engine exactly as before |
| `tests/integration/ipc_generation_routing_test.cpp` | MIGRATE | Named-pipe generation routing integration authority | `rust/ipc-client/tests/ipc_generation_routing.rs` (safe Rust client over the real mock engine) | Deleted in 078 Stage 3/4; `rust-ipc-client-wire` runs the Rust replacement through Cargo |
| `tests/integration/ipc_idle_client_test.cpp` | MIGRATE | Named-pipe idle-client integration authority | `rust/ipc-client/tests/ipc_idle_client.rs` (safe Rust client over the real mock engine) | Deleted in 078 Stage 3/4; `rust-ipc-client-wire` runs the Rust replacement through Cargo |
| `tests/integration/ipc_late_response_test.cpp` | MIGRATE | Reconnect/late-response integration authority | `rust/ipc-client/tests/ipc_late_response.rs` (safe Rust client; hand-rolled protocol server) | Deleted in 078 Stage 3/4; `rust-ipc-client-wire` runs the Rust replacement through Cargo |
| `tests/integration/ipc_multi_client_test.cpp` | MIGRATE | Multi-client IPC integration authority | `rust/ipc-client/tests/ipc_multi_client.rs` (safe Rust client over the real mock engine) | Deleted in 078 Stage 3/4; `rust-ipc-client-wire` runs the Rust replacement through Cargo |
| `tests/integration/ipc_roundtrip_test.cpp` | MIGRATE | Pipe client/engine roundtrip integration authority | `rust/ipc-client/tests/ipc_roundtrip.rs` (safe Rust client over the real mock engine) | Deleted in 078 Stage 3/4; `rust-ipc-client-wire` runs the Rust replacement through Cargo |
| `tests/integration/launcher_crash_loop_test.cpp` | KEEP | Launcher/engine crash-loop integration | Final mixed launcher shell + Rust launcher state + crash fixture | `CMakeLists.txt:1548-1556`; retained as `launcher-crash-loop-safe-mode` |
| `tests/integration/launcher_integration_test.cpp` | KEEP | Launcher/engine lifecycle integration | Final mixed launcher shell + Rust launcher state + IPC | `CMakeLists.txt:1533-1540`; retained as `launcher-engine-lifecycle` |
| `tests/integration/tsf_key_commit_test.cpp` | KEEP | Shipping TSF COM and composition E2E | Final Rust TSF DLL COM/IPC mixed E2E and frozen behavior corpus | `CMakeLists.txt:1592-1605`; retained as `tsf-key-commit-e2e` |
| `tests/integration/tsf_notepad_e2e_test.cpp` | KEEP | Interactive desktop TSF E2E | Final Win32/COM host E2E; intentionally not CTest-registered | `CMakeLists.txt:1607-1615`; built but manual-host only |
| `tests/perf/handle_leak_soak.cpp` | KEEP | Repeated TSF COM activation/resource boundary | Necessary COM/ABI performance/soak boundary | `CMakeLists.txt:1625-1630`; retained under `FCITX_BUILD_BENCHMARKS` |
| `tests/perf/ipc_codec_bench.cpp` | MIGRATE | Protocol codec performance authority | Rust standard-library benchmark route in `rust/protocol-core` | Deleted; `rust-ipc-codec-bench` runs `fcitx5-protocol-bench` through Cargo in `CMakeLists.txt` and `tools/benchmark.ps1` |
| `tests/perf/key_roundtrip_bench.cpp` | KEEP | Engine pipe roundtrip mixed performance | Final mixed IPC/engine performance boundary; it launches the mock Engine and measures the native IPC adapter | `CMakeLists.txt:1653-1656`; retained under `FCITX_BUILD_BENCHMARKS` |
| `tests/support/deployment_test_core.h` | DELETE | Helper existed only for superseded C++ deployment authority | Rust `fcitx5-package-core` deployment/recovery public tests | Deleted with the C++ deployment/updater authority; no remaining consumer or CMake registration |
| `tests/support/fcitx5_mldsa65_test_config.h` | KEEP | Native ML-DSA fixture signer support | Test-only cryptographic fixture support for mixed package integration | Consumed by `fcitx5_mldsa65_test_sign`/fixture signer at `CMakeLists.txt:948-975` |
| `tests/support/pqc_fixture_signer.cpp` | KEEP | Disposable signed-package fixture generator | Final mixed package integration support; not product authority | `CMakeLists.txt:966-975`; retained for `control-package-stopped-service-contract` |
| `tests/support/tsf_test_identity.h` | KEEP | Shared TSF COM identity/host support | Necessary Win32/COM identity boundary for TSF tests | Included by `tsf_*` tests and `handle_leak_soak`; retained with those boundaries |
| `tests/unit/brand_resource_test.cpp` | KEEP | KEEP-TEMP release-resource structure supplement; no product behavior authority | Necessary release-resource/artifact boundary; no Rust public-behavior replacement is required by this task | Retained as non-authoritative `fcitx5_brand_resource_test` / `brand-resource-contract` at `CMakeLists.txt:936-941`; future Rust structural coverage may replace it |
| `tests/unit/config_parser_test.cpp` | DELETE | Legacy C++ Config parser behavior | Rust `config-core` transaction/validation contract and `config-poc` CLI contract | Deleted; `rust-config-core-transaction-contract` and `rust-config-core-cli-contract` remain registered |
| `tests/unit/control_repository_rollback_test.cpp` | KEEP | Final mixed Control executable/package repository integration | Directly exercises the shipping Control binary, Rust package core, disposable signing fixture, and rollback boundary; it does not duplicate package-core behavior authority | `CMakeLists.txt:1035-1050`; retained as `control-repository-rollback` |
| `tests/unit/deployment_core_test.cpp` | DELETE | C++ deployment state/update authority | Rust `package-core` deployment state, TSF generation, and rollback tests | Deleted; no C++ target or CTest registration remains |
| `tests/unit/engine_core_contract_test.cpp` | KEEP | Rust Engine C ABI contract | Necessary C ABI adapter test for the Rust Engine core exports; Rust owns the Engine semantics and this test only verifies the ABI boundary | `CMakeLists.txt:1185-1191`; retained as `engine-core-contract` |
| `tests/unit/key_event_test.cpp` | KEEP | Fcitx key conversion adapter | Direct Fcitx key semantics and native Engine adapter boundary | `native-engine/CMakeLists.txt:177-190`; retained as `key-event-contract` |
| `tests/unit/launcher_state_test.cpp` | DELETE | C++ launcher state-model authority | Rust `launcher-core` public state-machine/model tests | Deleted; no C++ target or CTest registration remains |
| `tests/unit/package_core_test.cpp` | DELETE | C++ package/path/signature/archive authority | Rust `package-core` manifest, path, signature, archive, repository, and lifecycle tests | Deleted; no C++ target or CTest registration remains |
| `tests/unit/protocol_differential_test.cpp` | DELETE | Temporary C++/Rust protocol differential and golden runner | Rust protocol fixed corpus, golden-byte, property, and rejection tests; shared golden remains consumed by Rust | Deleted after Rust golden parity; corpus remains consumed by Rust |
| `tests/unit/protocol_test.cpp` | DELETE | C++ protocol public behavior authority | Rust `protocol-core/src/tests.rs` public codec contract and property tests | Deleted; no C++ target or CTest registration remains |
| `tests/unit/register_artifact_test.cpp` | KEEP | Rust register executable artifact/Win32 boundary | Necessary release executable artifact smoke | `CMakeLists.txt:1005-1010`; retained as `register-artifact-validation` |
| `tests/unit/release_identity_test.cpp` | KEEP | Release GUID/pipe/COM identity ABI boundary | Necessary Win32/COM/ABI identity uniqueness check | `CMakeLists.txt:929-934`; retained as `release-identity-contract` |
| `tests/unit/runtime_identity_test.cpp` | KEEP | Runtime identity and named-object ABI boundary | Necessary Win32 IPC identity adapter check | `CMakeLists.txt:1485-1489`; retained as `runtime-identity-contract` |
| `tests/unit/rust_tsf_poc_artifact_audit.cpp` | KEEP | Rust TSF DLL PE/export artifact boundary | Necessary COM/ABI/artifact smoke for shipping Rust TSF | `CMakeLists.txt:1584-1589`; retained as `rust-tsf-poc-artifact-audit` |
| `tests/unit/rust_tsf_poc_export_smoke.cpp` | KEEP | Rust TSF exported ABI smoke | Necessary COM/ABI adapter boundary | `CMakeLists.txt:1568-1581`; retained as `rust-tsf-poc-export-smoke` |
| `tests/unit/source_contract_test.cpp` | KEEP | KEEP-TEMP source-structure supplement; never a behavior authority | Necessary repository architecture/source gate while Rust structural coverage is not yet available; never substitutes for Rust public-behavior tests | Retained as secondary `source-contract` at `CMakeLists.txt:1193-1196`; future Rust structural coverage may replace it |
| `tests/unit/tsf_module_test.cpp` | KEEP | Shipping TSF COM class factory/activation | Necessary Win32/COM adapter boundary | `CMakeLists.txt:1559-1566`; retained as `tsf-module-activation` |
| `tests/unit/updater_cleanup_test.cpp` | DELETE | C++ test authority for Rust updater cleanup semantics | Rust `package-core` update/cleanup public tests and updater CLI route | Deleted; no C++ target or CTest registration remains |
| `tests/unit/version_test.cpp` | KEEP | Windows version/architecture release ABI identity | Necessary Win32/release-artifact boundary; checks the compiled ABI rather than product policy | `CMakeLists.txt:923-927`; retained as `version-contract` |

## CMake target inventory

The following rows enumerate every C++ test/support/fixture/fuzz/performance
target and every CTest registration associated with this inventory. Rust-only
CTest routes are included so the cutover ledger proves the replacement remains
registered. `KEEP` on a Rust-only row means the Rust route is the retained
authority, not a C++ exception.

| CMake / CTest item | Class | Owner / reason | Replacement or keep boundary | Registration / deletion evidence |
|---|---|---|---|---|
| `fcitx5_version_test` / `version-contract` | KEEP | Win32 version/architecture ABI | `tests/unit/version_test.cpp` boundary | `CMakeLists.txt:923-927` |
| `fcitx5_release_identity_test` / `release-identity-contract` | KEEP | GUID/COM identity ABI | `tests/unit/release_identity_test.cpp` boundary | `CMakeLists.txt:929-934` |
| `fcitx5_brand_resource_test` / `brand-resource-contract` | KEEP | KEEP-TEMP release-resource structure supplement | Necessary release-resource/artifact boundary; non-authoritative and not a Rust behavior replacement | Retained at `CMakeLists.txt:936-941`; future Rust structural coverage may replace it |
| `release-plugin-source-contract` | KEEP | Release artifact source gate | PowerShell release artifact gate; no C++ product authority | `CMakeLists.txt:942-946` |
| `fcitx5_mldsa65_test_sign` | KEEP | Test-only PQC fixture library | Shared by retained mixed package fixture tests | `CMakeLists.txt:948-961` |
| `fcitx5_package_core_test` / `package-core-contract` | DELETE | C++ package authority | Rust package-core public contract/corpus | Deleted; Rust `rust-package-core-differential` plus package-core Cargo tests are the retained authority |
| `fcitx5_pqc_fixture_signer` | KEEP | Disposable package-signing fixture | Retained mixed package integration support | `CMakeLists.txt:966-975` |
| `fcitx5_control_package_integration_test` / `control-package-stopped-service-contract` | KEEP | Final mixed Control/package E2E | Retained mixed binary boundary | `CMakeLists.txt:977-995` |
| `launcher-version`, `mock-engine-version`, `control-version`, `control-schema`, `control-startup-query`, `register-version`, `package-version`, `updater-version`, `downloader-version`, `downloader-boundary-smoke`, `deployer-version`, `deployer-boundary-smoke`, `provider-version`, `provider-boundary-smoke` | KEEP | Shipping executable CLI/artifact smoke | Rust shipping binaries through supported CTest route | `CMakeLists.txt:997-1031` |
| `fcitx5_register_artifact_test` / `register-artifact-validation` | KEEP | Rust register artifact boundary | Retained Win32 executable artifact smoke | `CMakeLists.txt:1005-1010` |
| `fcitx5_deployment_core_test` / `deployment-previous-known-good` | DELETE | C++ deployment authority | Rust package-core deployment tests | Deleted; Rust package-core unit contract covers activation, rollback, owner, and generation state |
| `fcitx5_updater_cleanup_test` / `updater-cleanup-previous` | DELETE | C++ updater cleanup authority | Rust package-core updater CLI contract | Deleted; `rust-package-core-updater-cli-contract` replaces the CTest route |
| `fcitx5_control_repository_rollback_test` / `control-repository-rollback` | KEEP | Final mixed Control/package integration | Retained C++ integration boundary over the shipping Control executable, Rust package core, and fixture signer | `CMakeLists.txt:1035-1050` |
| `control-config-roundtrip` | KEEP | Control + config/package mixed CLI integration | Retained final mixed CLI route | `CMakeLists.txt:1052-1055` |
| `config-ui-headless-smoke`, `config-ui-i18n-check`, `config-ui-resource-check`, `config-ui-behavior-contract`, `config-ui-visual-contract`, `config-ui-live-preview-contract`, `config-ui-interaction-coverage` | KEEP | Final shipping Config UI adapter/E2E routes | Native shell/renderer adapter and Rust Config frontend | `CMakeLists.txt:1056-1065` |
| `config-ui-preview-fidelity-qa`, `rust-config-ui-preview-qa`, `rust-config-poc-contract`, `rust-config-core-transaction-contract`, `rust-config-core-cli-contract` | KEEP | Rust Config public/visual contract routes | Retained Rust authority | `CMakeLists.txt:1066-1114` |
| `rust-config-side-by-side-contract`, `config-rust-legacy-baseline-build`, `config-rust-side-by-side-differential`, `config-rust-shipping-lineage`, `config-rust-legacy-headless-cli` | KEEP | Migration corpus/lineage gates | Temporary named 048 migration gates; not a permanent runtime selector | `CMakeLists.txt:1115-1172`; deletion owner is Config cutover lineage, not 071 |
| `rust-config-poc-window-smoke` | KEEP | Rust Config window adapter smoke | Retained Rust route | `CMakeLists.txt:1174-1182` |
| `fcitx5_protocol_test` / `protocol-contract` | DELETE | C++ protocol authority | Rust protocol-core public tests | Deleted; `rust-protocol-core-contract` consumes the public codec corpus |
| `fcitx5_protocol_differential_test` / `protocol-differential-contract` | DELETE | Temporary C++/Rust differential authority | Rust golden/property/rejection contract; retain corpus only | Deleted after Rust golden parity; `tests/unit/protocol_wire_golden.inc` remains consumed by Rust |
| `fcitx5_engine_core_contract_test` / `engine-core-contract` | KEEP | Rust Engine ABI exports | Retained necessary C ABI adapter test; Rust owns the underlying Engine state semantics | `CMakeLists.txt:1185-1191` |
| `fcitx5_config_parser_test` / `config-toml-contract` | DELETE | C++ Config parser authority | Rust config-core/config-poc contracts | Deleted; `rust-config-core-transaction-contract` and `rust-config-core-cli-contract` remain registered |
| `fcitx5_source_contract_test` / `source-contract` | KEEP | KEEP-TEMP source-structure supplement | Secondary architecture gate only; never counted as Rust public behavior or product authority | Retained at `CMakeLists.txt:1193-1196`; future Rust structural coverage may replace it |
| `fcitx5_rust_package_core_test`, `fcitx5_rust_package_core_artifact_smoke`, `rust-package-core-differential`, `rust-package-core-artifact-smoke`, `fcitx5_rust_package_core_packaged_artifact_smoke`, `rust-package-core-packaged-artifact-smoke` | KEEP | Rust package authority and artifact routes | Retained Rust package-core routes | `CMakeLists.txt:1201-1281`, `1398-1420` |
| `fcitx5_rust_candidate_poc_contract`, `rust-candidate-poc-contract`, `rust-candidate-poc-window-smoke`, `rust-candidate-poc-demo-snapshot`, `rust-candidate-poc-scroll-demo-snapshot`, label-slot snapshots, `rust-candidate-poc-typography-vertical`, host snapshots, `rust-candidate-poc-dpi-smoke` | KEEP | Rust Candidate authority/visual routes | Retained Rust Candidate routes; visual cutover remains task 072 | `CMakeLists.txt:1282-1440` |
| `candidate-ui-device-smoke`, `candidate-ui-safe-mode-smoke`, `candidate-ui-interaction-contract`, `candidate-ui-uiless-presentation-contract`, `candidate-ui-scroll-expansion-contract`, `candidate-ui-locale-contract`, `candidate-ui-ux-contract`, `candidate-ui-live-config-reflow` | KEEP | Final Candidate native adapter/UI routes | Retained UI adapter/E2E routes | `CMakeLists.txt:1447-1465` |
| `fcitx5_candidate_ui_config_integration_test` / `candidate-ui-live-presentation-contract` | KEEP | Final mixed Candidate/Config integration | Retained C++ integration boundary | `CMakeLists.txt:1468-1481` |
| `fcitx5_runtime_identity_test` / `runtime-identity-contract` | KEEP | Win32 identity adapter | Retained ABI boundary | `CMakeLists.txt:1485-1489` |
| `fcitx5_launcher_state_test` / `launcher-state-model` | DELETE | C++ launcher model authority | Rust launcher-core public state-machine/model tests | Deleted; x64/x86 Rust launcher-core tests replace `launcher-state-model` |
| `fcitx5_ipc_roundtrip_test` / `ipc-key-commit-roundtrip` | MIGRATE | C++ wire-test target for roundtrip authority | `rust/ipc-client` tests replaced the C++ client authority | Deleted in 078 Stage 3/4; `rust-ipc-client-wire` runs the Rust replacement under Cargo |
| `fcitx5_ipc_multi_client_test` / `ipc-multi-client`, `ipc-multi-client-32` | MIGRATE | C++ wire-test target for multi-client authority | `rust/ipc-client` tests replaced the C++ client authority | Deleted in 078 Stage 3/4; `rust-ipc-client-wire` runs the Rust replacement under Cargo |
| `fcitx5_ipc_idle_client_test` / `ipc-idle-client` | MIGRATE | C++ wire-test target for idle-client authority | `rust/ipc-client` tests replaced the C++ client authority | Deleted in 078 Stage 3/4; `rust-ipc-client-wire` runs the Rust replacement under Cargo |
| `fcitx5_ipc_generation_routing_test` / `ipc-generation-routing` | MIGRATE | C++ wire-test target for generation-routing authority | `rust/ipc-client` tests replaced the C++ client authority | Deleted in 078 Stage 3/4; `rust-ipc-client-wire` runs the Rust replacement under Cargo |
| `fcitx5_ipc_late_response_test` / `ipc-late-response-reconnect` | MIGRATE | C++ wire-test target for reconnect authority | `rust/ipc-client` tests replaced the C++ client authority | Deleted in 078 Stage 3/4; `rust-ipc-client-wire` runs the Rust replacement under Cargo |
| `fcitx5_launcher_integration_test` / `launcher-engine-lifecycle` | KEEP | Final mixed launcher/engine integration | Retained boundary | `CMakeLists.txt:1533-1540` |
| `fcitx5_crash_engine_fixture` | KEEP | Crash fixture for final mixed launcher E2E | Retained fixture | `CMakeLists.txt:1543-1545` |
| `fcitx5_launcher_crash_loop_test` / `launcher-crash-loop-safe-mode` | KEEP | Final mixed launcher recovery integration | Retained boundary | `CMakeLists.txt:1548-1556` |
| `fcitx5_tsf_module_test` / `tsf-module-activation` | KEEP | Shipping Rust TSF COM activation | Retained COM boundary | `CMakeLists.txt:1559-1566` |
| `fcitx5_rust_tsf_poc_export_smoke` / `rust-tsf-poc-unit`, `rust-tsf-poc-export-smoke` | KEEP | Rust TSF library/export ABI | Retained Rust unit plus ABI smoke | `CMakeLists.txt:1568-1581` |
| `fcitx5_rust_tsf_poc_artifact_audit` / `rust-tsf-poc-artifact-audit` | KEEP | Rust TSF PE/export artifact | Retained ABI/artifact boundary | `CMakeLists.txt:1584-1589` |
| `fcitx5_tsf_key_commit_test` / `tsf-key-commit-e2e` | KEEP | Final shipping TSF COM/IPC E2E | Retained mixed boundary and corpus consumer | `CMakeLists.txt:1592-1605` |
| `fcitx5_tsf_notepad_e2e_test` | KEEP | Manual desktop TSF E2E | Retained but not CTest-registered | `CMakeLists.txt:1607-1615` |
| `fcitx5_engine_integration_test` | MIGRATE | Final Engine integration executable | `fcitx5_engine_e2e` Rust harness (CMake custom target building `rust/ipc-client/src/bin/engine_e2e.rs`) | Deleted in 078 engine-E2E slice; `tools/test-fcitx.ps1` now builds and runs `engine_e2e.exe` |
| `fcitx5_protocol_fuzz` / `protocol-fuzz-smoke` | DELETE | C++ protocol fuzz authority | Rust deterministic fuzz smoke | Deleted; `rust-protocol-fuzz-smoke` is registered under the Cargo route |
| `fcitx5_package_fuzz` / `package-manifest-path-fuzz-smoke`, `package-path-corpus-fuzz-smoke` | DELETE | C++ package fuzz authority | Rust deterministic fuzz/corpus smoke | Deleted; `rust-package-core-fuzz-smoke` and `rust-package-core-path-corpus-fuzz-smoke` are registered under Cargo |
| `fcitx5_handle_leak_soak` | KEEP | COM/ABI resource soak | Retained under benchmark option | `CMakeLists.txt:1625-1630` |
| `fcitx5_ipc_codec_bench` | DELETE | C++ codec benchmark authority | Rust standard-library benchmark executable | Deleted; `rust-ipc-codec-bench` and `tools/benchmark.ps1` run the Rust benchmark |
| `fcitx5_key_roundtrip_bench` | KEEP | Final mixed IPC/Engine benchmark | Retained under benchmark option | `CMakeLists.txt:1653-1656` |
| `fcitx5_key_event_test` / `key-event-contract` | KEEP | Direct Fcitx key adapter | Retained native Engine adapter test | `native-engine/CMakeLists.txt:177-190` |

## Corpus and deletion gates

- `tests/unit/protocol_wire_golden.inc` is a shared frozen corpus, not an
  independent C++ authority. The Rust protocol replacement must consume the
  exact bytes before the two C++ protocol sources are removed. Its deletion is
  therefore not part of the protocol test-source deletion.
- `tests/fixtures/package_path_corpus.json` remains shared by Rust package-core
  tests and retained package integration. It must not be deleted with the C++
  package test.
- `tests/fixtures/tsf_behavior_corpus.json` remains consumed by the retained
  Rust TSF shipping route and the final mixed `tsf-key-commit-e2e` route.
- For every completed `MIGRATE` row, final deletion evidence is: the named Rust Cargo
  test/bin route passes for x64 and x86 where supported; `CMakeLists.txt` no
  longer contains the C++ source/target/CTest registration; `rg` finds no
  remaining registration; and `ctest -N` lists the Rust replacement instead.
- Brand resource and source-contract rows are `KEEP-TEMP` structural
  supplements: they are retained because this task does not introduce a Rust
  source/resource inspection API, are not Rust behavior authority, and do not
  count as unfinished `MIGRATE` rows. Completion evidence for all migrated rows
  is recorded in `docs/tasks/status.md` after validation.
