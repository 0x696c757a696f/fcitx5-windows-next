#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>

namespace {

std::string read_text(const std::filesystem::path& path) {
    std::ifstream input(path, std::ios::binary);
    if (!input)
        throw std::runtime_error("could not open source file");
    return {std::istreambuf_iterator<char>(input), {}};
}

int fail(const char* message) {
    std::cerr << message << '\n';
    return 1;
}

} // namespace

int main(int argc, char** argv) {
    if (argc != 2)
        return fail("expected repository source root");
    const std::filesystem::path sourceRoot = argv[1];
    const auto cmakeSource = read_text(sourceRoot / "CMakeLists.txt");
    const auto runtimeSource = read_text(sourceRoot / "src/engine/fcitx_runtime.cpp");
    const auto warmupMarker = runtimeSource.find("warmupIds");
    if (warmupMarker == std::string::npos)
        return fail("warmup section marker disappeared");
    const auto processKeyMarker = runtimeSource.find("RuntimeResult FcitxRuntime::processKey",
                                                     warmupMarker);
    if (processKeyMarker == std::string::npos)
        return fail("processKey marker disappeared");
    const auto warmupSection =
        runtimeSource.substr(warmupMarker, processKeyMarker - warmupMarker);
    if (warmupSection.find("keyEvent(") != std::string::npos ||
        warmupSection.find("FcitxKey_") != std::string::npos) {
        return fail("REG-WARMUP-001: generic warmup must not synthesize text key events");
    }
    if (warmupSection.find("warmupHasNoUserState") == std::string::npos ||
        warmupSection.find("takeCommit();") != std::string::npos) {
        return fail("REG-WARMUP-001: warmup must fail closed on user-state output");
    }
    const auto uiSource = read_text(sourceRoot / "src/ui/ui_main.cpp");
    if (uiSource.find("L\"zh-CN\", &format") != std::string::npos) {
        return fail("REG-PROFILE-001: candidate DWrite locale must not be hardcoded to zh-CN");
    }
    if (uiSource.find("last_write_time") != std::string::npos ||
        uiSource.find("configWriteTime_") != std::string::npos) {
        return fail("STAB-CAND-LOCALE-013: Candidate UI must not poll config file metadata per snapshot");
    }
    if (uiSource.find("contentLocaleUtf8") == std::string::npos ||
        uiSource.find("dwriteLocale_.c_str()") == std::string::npos ||
        uiSource.find("--locale-self-test") == std::string::npos) {
        return fail("STAB-CAND-LOCALE-013: Candidate UI must drive DWrite locale from candidate content metadata");
    }
    if (uiSource.find("--candidate-ux-self-test") == std::string::npos ||
        uiSource.find("resolveAutomaticPresentation") == std::string::npos ||
        uiSource.find("compositionAutoOrientation_") == std::string::npos ||
        uiSource.find("compositionStableWidth_") == std::string::npos ||
        uiSource.find("REG-CAND-STABLE-001") == std::string::npos ||
        uiSource.find("REG-CAND-AUTO-001") == std::string::npos) {
        return fail("REG-CAND-UX: Candidate UI must have auto layout and composition-scoped width contract");
    }
    const auto launcherSource = read_text(sourceRoot / "src/launcher/launcher_main.cpp");
    const auto trayIconSource = read_text(sourceRoot / "src/launcher/tray_icon.cpp");
    const auto trayIconHeader = read_text(sourceRoot / "src/launcher/tray_icon.h");
    const auto rustLauncherCoreSource = read_text(sourceRoot / "rust/launcher-core/src/lib.rs");
    const auto jobMarker = launcherSource.find("HANDLE job = CreateJobObjectW");
    const auto jobLimitMarker = launcherSource.find("SetInformationJobObject", jobMarker);
    const auto firstUiLaunch = launcherSource.find("launchUi(uiPath", jobMarker);
    const auto firstEngineLaunch = launcherSource.find("launchEngine(enginePath", jobMarker);
    if (jobMarker == std::string::npos || jobLimitMarker == std::string::npos ||
        firstUiLaunch == std::string::npos || firstEngineLaunch == std::string::npos ||
        jobLimitMarker > firstUiLaunch || jobLimitMarker > firstEngineLaunch) {
        return fail("REG-LAUNCHER-LEDGER-001: configure job limits before launching children");
    }
    if (launcherSource.find("if (installedDefaults)\n        (void)tray.create") !=
            std::string::npos ||
        trayIconSource.find("if (!window_)\n        return;") == std::string::npos) {
        return fail("REG-BRAND-001: launcher must not create a default tray/taskbar surface");
    }
    if (std::filesystem::exists(sourceRoot / "src/launcher/state_machine.cpp") ||
        std::filesystem::exists(sourceRoot / "src/launcher/state_store.cpp") ||
        cmakeSource.find("src/launcher/state_machine.cpp") != std::string::npos ||
        cmakeSource.find("src/launcher/state_store.cpp") != std::string::npos ||
        cmakeSource.find("add_library(fcitx5_launcher_state INTERFACE)") ==
            std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_state_store_load_utf16") ==
            std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_state_store_save_utf16") ==
            std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_state_request_start") ==
            std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_default_state_store_path_utf16") ==
            std::string::npos ||
        rustLauncherCoreSource.find("state_store_parser_matches_frozen_cpp_ledger_contract") ==
            std::string::npos ||
        rustLauncherCoreSource.find("state_store_save_load_and_publish_match_frozen_cpp_contract") ==
            std::string::npos ||
        launcherSource.find("#include <filesystem>") != std::string::npos ||
        trayIconHeader.find("#include <filesystem>") != std::string::npos ||
        trayIconHeader.find("std::filesystem::path") != std::string::npos ||
        launcherSource.find("std::filesystem") != std::string::npos ||
        launcherSource.find("generationBin") != std::string::npos ||
        launcherSource.find("installedRoot") != std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_resolve_default_process_paths_utf16") ==
            std::string::npos ||
        rustLauncherCoreSource.find("launcher_default_process_paths_match_cpp_generation_contract") ==
            std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_absolute_windows_path_utf16") ==
            std::string::npos ||
        trayIconSource.find("MultiByteToWideChar") != std::string::npos ||
        trayIconSource.find("const wchar_t* statusText") != std::string::npos ||
        trayIconSource.find("Safe mode") != std::string::npos ||
        trayIconSource.find("服务未运行") != std::string::npos ||
        trayIconSource.find("L\" — \"") != std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_tray_status_text_utf16") ==
            std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_tray_input_method_display_utf16") ==
            std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_tray_tooltip_utf16") ==
            std::string::npos ||
        rustLauncherCoreSource.find("tray_text_and_tooltip_match_cpp_display_contract") ==
            std::string::npos ||
        launcherSource.find("std::wstring quote") != std::string::npos ||
        launcherSource.find("quote(enginePath)") != std::string::npos ||
        launcherSource.find("quote(uiPath)") != std::string::npos ||
        launcherSource.find("std::to_wstring(GetCurrentProcessId())") !=
            std::string::npos ||
        trayIconSource.find("L\"\\\"\" + configPath_") != std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_engine_command_utf16") ==
            std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_ui_command_utf16") == std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_config_command_utf16") ==
            std::string::npos ||
        rustLauncherCoreSource.find("launcher_child_command_lines_match_cpp_contract") ==
            std::string::npos ||
        launcherSource.find("protocol::decodeHeader(header") != std::string::npos ||
        rustLauncherCoreSource.find("fcitx5_launcher_frame_body_size") == std::string::npos ||
        rustLauncherCoreSource.find("launcher_pipe_frame_header_matches_cpp_protocol_contract") ==
            std::string::npos ||
        cmakeSource.find("FCITX_RELEASE_DATA_DIRECTORY=${FCITX_RELEASE_DATA_DIRECTORY}") ==
            std::string::npos) {
        return fail("LAUNCHER-RUST: launcher state/path/tray/command/frame policy must be Rust-owned");
    }
    const auto controlSource = read_text(sourceRoot / "src/control/control_main.cpp");
    const auto configAppSource = read_text(sourceRoot / "src/config/app_main.cpp");
    const auto rustProcessExecutionSource =
        read_text(sourceRoot / "rust/process-execution-core/src/lib.rs");
    if (controlSource.find("CreateProcessW(") != std::string::npos ||
        controlSource.find("WaitForSingleObject(process.hProcess") != std::string::npos) {
        return fail("REG-PROC-PIPE-001: Control must use the shared process executor");
    }
    if (std::filesystem::exists(sourceRoot / "src/config/process_execution.h") ||
        std::filesystem::exists(sourceRoot / "src/config/process_execution.cpp") ||
        std::filesystem::exists(sourceRoot / "tests/unit/process_execution_test.cpp") ||
        controlSource.find("#include \"process_execution.h\"") != std::string::npos ||
        configAppSource.find("#include \"process_execution.h\"") != std::string::npos ||
        cmakeSource.find("src/config/process_execution.cpp") != std::string::npos ||
        cmakeSource.find("tests/unit/process_execution_test.cpp") != std::string::npos ||
        cmakeSource.find("fcitx5_process_execution_core_rust") == std::string::npos ||
        rustProcessExecutionSource.find("process_output_is_drained_bounded_and_failure_visible") ==
            std::string::npos ||
        rustProcessExecutionSource.find("timeout_kills_child_process_tree") == std::string::npos) {
        return fail("PROCESS-EXECUTION-RUST: process execution must be Rust-owned and the old C++ adapter/test must stay deleted");
    }
    const auto peerSource = read_text(sourceRoot / "src/ipc/peer_verification.cpp");
    if (peerSource.find("pathsReferToSameFile") != std::string::npos ||
        peerSource.find("queryExecutableFileIdentity") == std::string::npos ||
        peerSource.find("executableFilesMatch") == std::string::npos) {
        return fail("REG-PEER-ID-001: peer exact mode must use strict executable identity");
    }
    const auto installerSource = read_text(sourceRoot / "installer/fcitx5-windows.iss");
    if (installerSource.find("--set-startup") != std::string::npos ||
        installerSource.find("--background") != std::string::npos ||
        installerSource.find("per_user_startup\":\"user-plane\"") == std::string::npos) {
        return fail("REG-INSTALL-UAC-001: installer must not own per-user startup/session state");
    }
    const auto registerDevScript = read_text(sourceRoot / "tools/register-dev.ps1");
    const auto rustRegisterCore = read_text(sourceRoot / "rust/register-core/src/lib.rs");
    const auto rustRegisterCli = read_text(sourceRoot / "rust/register-core/src/main.rs");
    if (std::filesystem::exists(sourceRoot / "src/register/register_main.cpp") ||
        rustRegisterCli.find("--validate-artifact") == std::string::npos ||
        rustRegisterCli.find("artifact_valid") == std::string::npos ||
        rustRegisterCli.find("paired architecture TSF DLL is missing") == std::string::npos ||
        rustRegisterCli.find("fcitx5-register") == std::string::npos ||
        rustRegisterCli.find("validate_product_artifact") == std::string::npos ||
        rustRegisterCli.find("operation_requires_admin") == std::string::npos ||
        rustRegisterCli.find("operation_export") == std::string::npos ||
        rustRegisterCli.find("registration_status_for_dll") == std::string::npos ||
        rustRegisterCli.find("is_elevated") == std::string::npos ||
        rustRegisterCli.find("invoke_registration_export") == std::string::npos ||
        rustRegisterCli.find("LoadLibraryExW(") != std::string::npos ||
        rustRegisterCli.find("GetProcAddress(") != std::string::npos ||
        rustRegisterCli.find("AllocateAndInitializeSid(") != std::string::npos ||
        rustRegisterCore.find("validate_product_artifact") == std::string::npos ||
        rustRegisterCore.find("REGISTER_OPERATION_VALIDATE_ARTIFACT") == std::string::npos ||
        rustRegisterCore.find("REGISTER_DLL_ARGUMENT_INVALID") == std::string::npos ||
        rustRegisterCore.find("REGISTER_EXPORT_UNREGISTER_SERVER") == std::string::npos ||
        rustRegisterCore.find("REGISTER_STATUS_PATH_MISMATCH") == std::string::npos ||
        rustRegisterCore.find("fcitx5_register_is_elevated") == std::string::npos ||
        rustRegisterCore.find("fcitx5_register_invoke_registration_export") ==
            std::string::npos ||
        rustRegisterCore.find("AllocateAndInitializeSid") == std::string::npos ||
        rustRegisterCore.find("LoadLibraryExW") == std::string::npos ||
        rustRegisterCore.find("DllRegisterServer") == std::string::npos ||
        rustRegisterCore.find("DllUnregisterServer") == std::string::npos ||
        rustRegisterCore.find("REGISTER_ARTIFACT_PAIRED_DLL_MISSING") ==
            std::string::npos) {
        return fail("STAB-REGISTER-BOOTSTRAP-012: register helper must be the Rust CLI and validate product artifacts");
    }
    if (registerDevScript.find("Remove-StaleComRegistration") == std::string::npos ||
        registerDevScript.find("out/package") == std::string::npos ||
        registerDevScript.find("registered DLL is missing; removing stale COM registration") ==
            std::string::npos) {
        return fail("REG-UPDATE-TSF: dev TSF unregister must clean stale package-stage COM registrations");
    }
    const auto rustBootstrapSource =
        read_text(sourceRoot / "rust/package-core/src/bootstrap_main.rs");
    if (std::filesystem::exists(sourceRoot / "src/bootstrap/bootstrap_main.cpp") ||
        rustBootstrapSource.find("#![windows_subsystem = \"windows\"]") ==
            std::string::npos ||
        rustBootstrapSource.find("TerminateProcess(process.process, ERROR_TIMEOUT)") ==
            std::string::npos ||
        rustBootstrapSource.find("TerminateProcess(info.process, ERROR_TIMEOUT)") ==
            std::string::npos ||
        rustBootstrapSource.find("WaitForSingleObject(process.process, 5000)") ==
            std::string::npos ||
        rustBootstrapSource.find("WaitForSingleObject(info.process, 5000)") ==
            std::string::npos) {
        return fail("STAB-REGISTER-BOOTSTRAP-012: bootstrap timeouts must confirm child termination");
    }
    const auto packageSource = read_text(sourceRoot / "src/package/package_core.cpp");
    const auto packageArchiveSource = read_text(sourceRoot / "src/package/package_archive.cpp");
    const auto packageCorpus =
        read_text(sourceRoot / "tests/fixtures/package_path_corpus.json");
    if (packageSource.find("OrdinalIgnoreCaseLess") == std::string::npos ||
        packageArchiveSource.find("is_unix_symlink") == std::string::npos ||
        packageArchiveSource.find("manifest paths collide on Windows") == std::string::npos ||
        packageCorpus.find("\"case_collision_sets\"") == std::string::npos ||
        packageCorpus.find("COM9.log") == std::string::npos) {
        return fail("REG-PKG-WINPATH-001: package path policy must have shared hostile Windows corpus and collision/link guards");
    }
    const auto rustPackageCliBuild = read_text(sourceRoot / "tools/build-rust-package-cli.ps1");
    const auto rustToolchain = read_text(sourceRoot / "rust-toolchain.toml");
    const auto cargoManifest = read_text(sourceRoot / "Cargo.toml");
    const auto rustPackageCoreManifest =
        read_text(sourceRoot / "rust/package-core/Cargo.toml");
    const auto cargoLock = read_text(sourceRoot / "Cargo.lock");
    const auto rustPackageCore =
        read_text(sourceRoot / "rust/package-core/src/lib.rs");
    const auto rustPackageCoreBuild =
        read_text(sourceRoot / "rust/package-core/build.rs");
    const auto rustPackageCoreMiniz =
        read_text(sourceRoot / "rust/package-core/c/miniz_archive.c");
    const auto rustPackageCoreBinary =
        read_text(sourceRoot / "rust/package-core/src/main.rs");
    const auto rustDownloaderBinary =
        read_text(sourceRoot / "rust/package-core/src/downloader_main.rs");
    const auto rustDownloaderBuild =
        read_text(sourceRoot / "tools/build-rust-downloader-cli.ps1");
    const auto rustUpdaterBinary =
        read_text(sourceRoot / "rust/package-core/src/updater_main.rs");
    const auto rustUpdaterBuild =
        read_text(sourceRoot / "tools/build-rust-updater-cli.ps1");
    const auto rustDeployerBinary =
        read_text(sourceRoot / "rust/package-core/src/deployer_main.rs");
    const auto rustDeployerBuild =
        read_text(sourceRoot / "tools/build-rust-deployer-cli.ps1");
    const auto deploymentCoreSource = read_text(sourceRoot / "src/updater/deployment_core.cpp");
    const auto dependencyCheck = read_text(sourceRoot / "tools/check-dependencies.ps1");
    const auto dependencyInventory = read_text(sourceRoot / "third_party/dependencies.json");
    const auto rustPackageCoreArtifactSmoke =
        read_text(sourceRoot / "tools/test-rust-package-core-artifact.ps1");
    const auto versionHeaderTemplate = read_text(sourceRoot / "cmake/version.h.in");
    const auto rustWindowsCommonCore =
        read_text(sourceRoot / "rust/windows-common-core/src/lib.rs");
    const auto rustWindowsCommonManifest =
        read_text(sourceRoot / "rust/windows-common-core/Cargo.toml");
    if (std::filesystem::exists(sourceRoot / "src/common/version.cpp") ||
        cargoManifest.find("\"rust/windows-common-core\"") == std::string::npos ||
        cargoLock.find("name = \"fcitx5-windows-common-core\"") == std::string::npos ||
        rustWindowsCommonManifest.find("crate-type = [\"staticlib\", \"rlib\"]") ==
            std::string::npos ||
        rustWindowsCommonCore.find("FCITX_WINDOWS_VERSION") == std::string::npos ||
        rustWindowsCommonCore.find("FCITX_RELEASE_CHANNEL_NAME") == std::string::npos ||
        rustWindowsCommonCore.find("fcitx5_windows_common_version") == std::string::npos ||
        rustWindowsCommonCore.find("fcitx5_windows_common_architecture") ==
            std::string::npos ||
        cmakeSource.find("fcitx5-windows-common-core") == std::string::npos ||
        cmakeSource.find("FCITX_RUST_WINDOWS_COMMON_STATICLIB") == std::string::npos ||
        cmakeSource.find("FCITX_WINDOWS_VERSION=${PROJECT_VERSION}") == std::string::npos ||
        versionHeaderTemplate.find("fcitx5_windows_common_version") == std::string::npos ||
        versionHeaderTemplate.find("fcitx5_windows_common_release_channel") ==
            std::string::npos) {
        return fail("COMMON-RUST: product version/channel/architecture must be Rust-owned and the old C++ version source must stay deleted");
    }
    if (rustToolchain.find("channel = \"1.98.0\"") == std::string::npos ||
        rustToolchain.find("aarch64-pc-windows-msvc") == std::string::npos ||
        cargoManifest.find("\"rust/package-core\"") == std::string::npos ||
        cargoLock.find("name = \"fcitx5-package-core\"") == std::string::npos ||
        rustPackageCore.find("#![deny(unsafe_code)]") == std::string::npos ||
        rustPackageCore.find("mod mldsa_verify_adapter") == std::string::npos ||
        rustPackageCore.find("unsafe extern \"C\"") == std::string::npos ||
        rustPackageCore.find("include_str!(\"../../../tests/fixtures/package_path_corpus.json\")") ==
            std::string::npos ||
        rustPackageCore.find("SafeRelativePackagePath") == std::string::npos ||
        rustPackageCore.find("PackageId") == std::string::npos ||
        rustPackageCore.find("parse_manifest") == std::string::npos ||
        rustPackageCore.find("parse_trusted_keys") == std::string::npos ||
        rustPackageCore.find("parse_signature_envelope") == std::string::npos ||
        rustPackageCore.find("verify_signature_envelope") == std::string::npos ||
        rustPackageCore.find("verify_mldsa65_signature") == std::string::npos ||
        rustPackageCore.find("resolve_exact_dependencies") == std::string::npos ||
        rustPackageCore.find("verify_payload_inventory") == std::string::npos ||
        rustPackageCore.find("verify_payload_digests") == std::string::npos ||
        rustPackageCore.find("sha256_digest") == std::string::npos ||
        rustPackageCore.find("blake3_digest") == std::string::npos ||
        rustPackageCore.find("verify_payload_bytes") == std::string::npos ||
        rustPackageCore.find("verify_payload_root") == std::string::npos ||
        rustPackageCore.find("stage_verified_payload_tree") == std::string::npos ||
        rustPackageCore.find("path_contains_reparse_component") == std::string::npos ||
        rustPackageCore.find("parse_lockfile") == std::string::npos ||
        rustPackageCore.find("read_installed_lockfile") == std::string::npos ||
        rustPackageCore.find("write_installed_lockfile_atomic") == std::string::npos ||
        rustPackageCore.find("activate_staged_payload_tree") == std::string::npos ||
        rustPackageCore.find("set_installed_package_state") == std::string::npos ||
        rustPackageCore.find("mark_installed_package_for_removal") == std::string::npos ||
        rustPackageCore.find("finalize_installed_package_removal") == std::string::npos ||
        rustPackageCore.find("validate_archive_inventory") == std::string::npos ||
        rustPackageCore.find("stage_validated_archive_zip") == std::string::npos ||
        rustPackageCore.find("VerifiedArtifact") == std::string::npos ||
        rustPackageCoreBuild.find("mldsa_native.c") == std::string::npos ||
        rustPackageCoreBuild.find("fcitx5_mldsa65_config.h") == std::string::npos ||
        rustPackageCoreBuild.find("miniz.c") == std::string::npos ||
        rustPackageCoreMiniz.find("mz_zip_reader_extract_to_mem") == std::string::npos ||
        rustPackageCoreBinary.find("--self-check") == std::string::npos ||
        rustPackageCoreBinary.find("--validate-manifest") == std::string::npos ||
        rustPackageCoreBinary.find("--validate-keyring") == std::string::npos ||
        rustPackageCoreBinary.find("--verify-manifest-v2") == std::string::npos ||
        rustPackageCoreBinary.find("--mark-remove") == std::string::npos ||
        rustPackageCoreBinary.find("--finalize-remove") == std::string::npos ||
        rustPackageCoreBinary.find("--audit-self-pe") == std::string::npos ||
        rustPackageCoreBinary.find("winhttp.dll") == std::string::npos ||
        rustPackageCoreBinary.find("parse_trusted_keys") == std::string::npos ||
        rustPackageCoreBinary.find("verify_signature_envelope") == std::string::npos ||
        std::filesystem::exists(sourceRoot / "src/package/downloader_main.cpp") ||
        rustPackageCoreManifest.find("name = \"fcitx5-downloader\"") ==
            std::string::npos ||
        rustDownloaderBinary.find("WinHttpOpen") == std::string::npos ||
        rustDownloaderBinary.find("WinHttpCrackUrl") == std::string::npos ||
        rustDownloaderBinary.find("WINHTTP_OPTION_REDIRECT_POLICY_NEVER") ==
            std::string::npos ||
        rustDownloaderBinary.find("downloader refuses to run elevated") ==
            std::string::npos ||
        rustDownloaderBinary.find("only credential-free HTTPS is allowed") ==
            std::string::npos ||
        rustDownloaderBinary.find("sha256_digest") == std::string::npos ||
        rustDownloaderBinary.find("MoveFileExW") == std::string::npos ||
        rustDownloaderBuild.find("fcitx5-downloader.exe") == std::string::npos ||
        rustDownloaderBuild.find("--bin fcitx5-downloader") == std::string::npos ||
        std::filesystem::exists(sourceRoot / "src/updater/updater_main.cpp") ||
        rustPackageCoreManifest.find("name = \"fcitx5-updater\"") ==
            std::string::npos ||
        rustUpdaterBinary.find("#![forbid(unsafe_code)]") == std::string::npos ||
        rustUpdaterBinary.find("--activate-runtime-generation") == std::string::npos ||
        rustUpdaterBinary.find("cleanup_previous_known_good") == std::string::npos ||
        rustUpdaterBinary.find("activate_staged_payload_tree") == std::string::npos ||
        rustUpdaterBinary.find("activate_installed_version_for_rollback") ==
            std::string::npos ||
        rustUpdaterBinary.find("update_failed:") == std::string::npos ||
        rustUpdaterBuild.find("fcitx5-updater.exe") == std::string::npos ||
        rustUpdaterBuild.find("--bin fcitx5-updater") == std::string::npos ||
        cmakeSource.find("Building Rust fcitx5-updater CLI") == std::string::npos ||
        std::filesystem::exists(sourceRoot / "src/package/deployer_main.cpp") ||
        rustPackageCoreManifest.find("name = \"fcitx5-deployer\"") ==
            std::string::npos ||
        rustDeployerBinary.find("mod win32") == std::string::npos ||
        rustDeployerBinary.find("copy_exclusive_artifact") == std::string::npos ||
        rustDeployerBinary.find("protected_install_root") == std::string::npos ||
        rustDeployerBinary.find("stage_validated_archive_zip") == std::string::npos ||
        rustDeployerBinary.find("activate_staged_payload_tree") == std::string::npos ||
        rustDeployerBinary.find("deployer request is invalid") == std::string::npos ||
        rustDeployerBuild.find("fcitx5-deployer.exe") == std::string::npos ||
        rustDeployerBuild.find("--bin fcitx5-deployer") == std::string::npos ||
        cmakeSource.find("Building Rust fcitx5-deployer CLI") == std::string::npos ||
        deploymentCoreSource.find("fcitx5_update_cleanup_previous_known_good_utf16") ==
            std::string::npos ||
        deploymentCoreSource.find("fcitx5_update_install_tsf_dll_generation_utf16") ==
            std::string::npos ||
        deploymentCoreSource.find("fcitx5_update_cleanup_old_tsf_dlls_utf16") ==
            std::string::npos ||
        deploymentCoreSource.find("fcitx5_update_runtime_generation_directory_utf16") ==
            std::string::npos ||
        deploymentCoreSource.find("fcitx5_update_install_runtime_generation_utf16") ==
            std::string::npos ||
        deploymentCoreSource.find("CopyFileW(") != std::string::npos ||
        deploymentCoreSource.find("DeleteFileW(") != std::string::npos ||
        deploymentCoreSource.find("copy_directory_tree") != std::string::npos ||
        deploymentCoreSource.find("stage_runtime_payload") != std::string::npos ||
        deploymentCoreSource.find("publish_runtime_directory") != std::string::npos ||
        rustPackageCore.find("cleanup_previous_known_good") == std::string::npos ||
        rustPackageCore.find("install_tsf_dll_generation") == std::string::npos ||
        rustPackageCore.find("cleanup_old_tsf_dlls") == std::string::npos ||
        rustPackageCore.find("install_runtime_generation") == std::string::npos ||
        rustPackageCore.find("publish_runtime_directory") == std::string::npos ||
        rustPackageCore.find("stage_runtime_payload") == std::string::npos ||
        rustPackageCore.find("fcitx5_update_install_tsf_dll_generation_utf16") ==
            std::string::npos ||
        rustPackageCore.find("fcitx5_update_cleanup_old_tsf_dlls_utf16") ==
            std::string::npos ||
        rustPackageCore.find("fcitx5_update_install_runtime_generation_utf16") ==
            std::string::npos ||
        rustPackageCore.find("fcitx5_update_runtime_generation_directory_utf16") ==
            std::string::npos ||
        rustPackageCore.find("MOVEFILE_DELAY_UNTIL_REBOOT") == std::string::npos ||
        rustPackageCore.find("invalid core package id for --cleanup-previous") ==
            std::string::npos ||
        rustPackageCore.find("cleanup target escapes the versions directory") ==
            std::string::npos ||
        rustPackageCliBuild.find("fcitx5-package.exe") == std::string::npos ||
        rustPackageCliBuild.find("fcitx5-package-core.exe") == std::string::npos ||
        rustPackageCliBuild.find("--bin fcitx5-package-core") == std::string::npos ||
        cmakeSource.find("rust-package-core-artifact-smoke") == std::string::npos ||
        cmakeSource.find("rust-package-core-packaged-artifact-smoke") == std::string::npos ||
        cmakeSource.find("Building Rust fcitx5-package CLI") == std::string::npos ||
        cmakeSource.find("CARGO_TARGET_DIR") == std::string::npos ||
        cmakeSource.find("FCITX_RUST_TARGET") == std::string::npos ||
        cmakeSource.find("FCITX_MINIZ_SOURCE_DIR") == std::string::npos ||
        rustPackageCoreArtifactSmoke.find("fcitx5-package-core-smoke.zip") ==
            std::string::npos ||
        rustPackageCoreArtifactSmoke.find("CargoTarget") == std::string::npos ||
        rustPackageCoreArtifactSmoke.find("--audit-self-pe") == std::string::npos ||
        dependencyCheck.find("Cargo.lock contains untracked third-party crate sources") ==
            std::string::npos ||
        dependencyCheck.find("arrayref' -and $version -eq '0.3.10'") ==
            std::string::npos ||
        dependencyInventory.find("\"rust-crate-blake3\"") ==
            std::string::npos) {
        return fail("RUST-R1-01: Rust package-core workspace must be pinned, locked, safe, and consume the frozen package path corpus");
    }
    const auto rustProviderBuild = read_text(sourceRoot / "tools/build-rust-provider-cli.ps1");
    const auto rustProviderBinary =
        read_text(sourceRoot / "rust/package-core/src/provider_main.rs");
    if (rustPackageCoreManifest.find("name = \"fcitx5-provider\"") == std::string::npos ||
        rustPackageCore.find("pub struct PlumPlan") == std::string::npos ||
        rustPackageCore.find("pub enum ProviderTrust") == std::string::npos ||
        rustPackageCore.find("make_plum_plan") == std::string::npos ||
        rustPackageCore.find("run_plum_provider") == std::string::npos ||
        rustPackageCore.find("JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE") == std::string::npos ||
        rustPackageCore.find("CREATE_SUSPENDED") == std::string::npos ||
        rustPackageCore.find("provider_runner_propagates_nonzero_and_times_out_without_live_input_data") ==
            std::string::npos ||
        rustProviderBinary.find("#![forbid(unsafe_code)]") == std::string::npos ||
        rustProviderBinary.find("--allow-unverified") == std::string::npos ||
        rustProviderBinary.find("--plum") == std::string::npos ||
        rustProviderBinary.find("--audit-self-pe") == std::string::npos ||
        rustProviderBuild.find("--bin fcitx5-provider") == std::string::npos ||
        rustProviderBuild.find("fcitx5-provider.exe") == std::string::npos ||
        cmakeSource.find("Building Rust fcitx5-provider CLI") == std::string::npos ||
        cmakeSource.find("provider-boundary-smoke") == std::string::npos ||
        cmakeSource.find("src/package/provider_main.cpp") != std::string::npos ||
        cmakeSource.find("src/package/provider_policy.cpp") != std::string::npos) {
        return fail("RUST-R1-04: provider policy/runner must be Rust authoritative, bounded, and covered by artifact smoke");
    }
    const auto configSource = read_text(sourceRoot / "src/config/app_main.cpp");
    const auto candidateLayoutSource = read_text(sourceRoot / "rust/candidate-core/src/lib.rs");
    const auto candidatePocManifest =
        read_text(sourceRoot / "rust/candidate-core/Cargo.toml");
    const auto candidatePocBinary =
        read_text(sourceRoot / "rust/candidate-core/src/bin/candidate_poc.rs");
    const auto candidateUiSource = read_text(sourceRoot / "src/ui/ui_main.cpp");
    const auto candidateIntegrationSource =
        read_text(sourceRoot / "tests/integration/candidate_ui_config_integration_test.cpp");
    const auto configQaSource = read_text(sourceRoot / "rust/config-qa/src/main.rs");
    const auto configPocSource = read_text(sourceRoot / "rust/config-poc/src/main.rs");
    const auto configPocManifest = read_text(sourceRoot / "rust/config-poc/Cargo.toml");
    const auto englishLocale = read_text(sourceRoot / "locales/en-US.json");
    if (configSource.find("struct DesignTokens") == std::string::npos ||
        configSource.find("designTokens()") == std::string::npos ||
        configSource.find("--ui-visual-contract-test") == std::string::npos ||
        configSource.find("--ui-live-preview-contract-test") == std::string::npos ||
        configSource.find("ensureProductionPreview()") == std::string::npos ||
        configSource.find("fcitx5-ui.exe") == std::string::npos ||
        configSource.find("#include \"candidate_layout.h\"") != std::string::npos ||
        configSource.find("#include \"config_model.h\"") == std::string::npos ||
        configSource.find("fcitx::windows::ui::layout(input)") == std::string::npos ||
        configSource.find("fcitx::windows::ui::renderSegments") == std::string::npos ||
        configSource.find("shūrùfǎ") == std::string::npos ||
        configSource.find("zhōngwén") == std::string::npos ||
        configSource.find("previewLabel(L\"3\")") ==
            std::string::npos ||
        configSource.find("previewComment(L\"emoji\")") ==
            std::string::npos ||
        configSource.find("PreviewCandidate{previewLabel(L\"2\"), L\"🎉\", previewComment(L\"emoji\")}") ==
            std::string::npos ||
        configSource.find("currentPreviewVisualConfig") == std::string::npos ||
        configSource.find("parseTheme") == std::string::npos ||
        configSource.find("resolveTheme") == std::string::npos ||
        configSource.find("parseD2DColor") == std::string::npos ||
        configSource.find("resources\" / L\"themes\" / L\"default\"") == std::string::npos ||
        cmakeSource.find("fcitx5::config") == std::string::npos ||
        candidateUiSource.find("fcitx::windows::ui::renderSegments") == std::string::npos ||
        candidateUiSource.find("kDrawTextOptions") == std::string::npos ||
        candidateUiSource.find("0x4U") == std::string::npos ||
        candidateLayoutSource.find("fcitx5_candidate_render_segments") == std::string::npos ||
        candidateLayoutSource.find("run_candidate_poc_self_check") == std::string::npos ||
        candidateLayoutSource.find("cpp_ffi") == std::string::npos ||
        candidateLayoutSource.find("send_input") == std::string::npos ||
        candidateLayoutSource.find("global_hooks") == std::string::npos ||
        candidateLayoutSource.find("process_injection") == std::string::npos ||
        candidatePocManifest.find("name = \"fcitx5-candidate-poc\"") == std::string::npos ||
        candidatePocBinary.find("--self-check") == std::string::npos ||
        candidatePocBinary.find("--window-smoke") == std::string::npos ||
        candidatePocBinary.find("CreateWindowExW") == std::string::npos ||
        candidatePocBinary.find("WS_EX_NOACTIVATE") == std::string::npos ||
        candidatePocBinary.find("AccessibleObjectFromWindow") == std::string::npos ||
        candidatePocBinary.find("get_acc_name") == std::string::npos ||
        candidatePocBinary.find("msaa_accessible_name_readable") == std::string::npos ||
        candidatePocBinary.find("IUIAutomation") == std::string::npos ||
        candidatePocBinary.find("uia_name_readable") == std::string::npos ||
        candidatePocBinary.find("uia_control_type") == std::string::npos ||
        candidatePocBinary.find("--screenshot") == std::string::npos ||
        candidatePocBinary.find("--demo-snapshot") == std::string::npos ||
        candidatePocBinary.find("--scroll-demo-snapshot") == std::string::npos ||
        candidatePocBinary.find("--host-snapshot") == std::string::npos ||
        candidatePocBinary.find("host_snapshot_scenario") == std::string::npos ||
        candidatePocBinary.find("--dpi-scale") == std::string::npos ||
        candidatePocBinary.find("snapshot_name") == std::string::npos ||
        candidatePocBinary.find("host_name") == std::string::npos ||
        candidatePocBinary.find("locale_name") == std::string::npos ||
        candidatePocBinary.find("candidate_count") == std::string::npos ||
        candidatePocBinary.find("visible_candidate_rects") == std::string::npos ||
        candidatePocBinary.find("painted_candidate_rects") == std::string::npos ||
        candidatePocBinary.find("layout_driven_paint") == std::string::npos ||
        candidatePocBinary.find("layout_rects_inside_window") == std::string::npos ||
        candidatePocBinary.find("layout_rects_non_overlapping") == std::string::npos ||
        candidatePocBinary.find("inspect_layout_rectangles") == std::string::npos ||
        candidatePocBinary.find("WINDOW_LAYOUT_RECTS") == std::string::npos ||
        candidatePocBinary.find("WINDOW_SELECTED_VISIBLE") == std::string::npos ||
        candidatePocBinary.find("visible_window_rects") == std::string::npos ||
        candidatePocBinary.find("visible_text_lines") == std::string::npos ||
        candidatePocBinary.find("dpi_scale") == std::string::npos ||
        candidatePocBinary.find("scroll_mode") == std::string::npos ||
        candidatePocBinary.find("GetDIBits") == std::string::npos ||
        candidatePocBinary.find("visual_non_background_pixels") == std::string::npos ||
        cmakeSource.find("rust-candidate-poc-contract") == std::string::npos ||
        cmakeSource.find("rust-candidate-poc-window-smoke") == std::string::npos ||
        cmakeSource.find("rust-candidate-poc-demo-snapshot") == std::string::npos ||
        cmakeSource.find("rust-candidate-poc-scroll-demo-snapshot") == std::string::npos ||
        cmakeSource.find("rust-candidate-poc-host-snapshot") == std::string::npos ||
        cmakeSource.find("mock-word") == std::string::npos ||
        cmakeSource.find("candidate-poc-host-${FCITX_CANDIDATE_HOST_SNAPSHOT}.bmp") ==
            std::string::npos ||
        cmakeSource.find("rust-candidate-poc-dpi-smoke") == std::string::npos ||
        cmakeSource.find("fcitx5_candidate_poc_rustbin") == std::string::npos ||
        cmakeSource.find("candidate-poc-window-smoke.bmp") == std::string::npos ||
        cmakeSource.find("config-ui-preview-fidelity-qa") == std::string::npos ||
        candidateIntegrationSource.find("cpp-candidate-demo.bmp") == std::string::npos ||
        candidateIntegrationSource.find("cpp-candidate-scroll-demo.bmp") == std::string::npos ||
        candidateIntegrationSource.find("capture_window") == std::string::npos ||
        candidateIntegrationSource.find("visual_non_background_pixels") == std::string::npos ||
        candidateIntegrationSource.find("rust-candidate-demo.json") == std::string::npos ||
        candidateIntegrationSource.find("rust-candidate-scroll-demo.json") == std::string::npos ||
        candidateIntegrationSource.find("expect_rust_layout_paint_evidence") ==
            std::string::npos ||
        candidateIntegrationSource.find("layout_driven_paint") == std::string::npos ||
        candidateIntegrationSource.find("layout_rects_inside_window") == std::string::npos ||
        candidateIntegrationSource.find("layout_rects_non_overlapping") == std::string::npos ||
        candidateIntegrationSource.find("painted_candidate_rects") == std::string::npos ||
        candidateIntegrationSource.find("demo-snapshot") == std::string::npos ||
        candidateIntegrationSource.find("scroll-demo-snapshot") == std::string::npos ||
        candidateIntegrationSource.find("Rust/C++ candidate demo width diverged") ==
            std::string::npos ||
        candidateIntegrationSource.find("Rust/C++ candidate scroll-demo width diverged") ==
            std::string::npos ||
        configQaSource.find("--candidate-ui-exe") == std::string::npos ||
        configQaSource.find("candidate-ui-demo-reference.bmp") == std::string::npos ||
        configQaSource.find("config-appearance-candidate-preview.bmp") == std::string::npos ||
        configQaSource.find("selected_green_bbox") == std::string::npos ||
        configQaSource.find("assert_preview_matches_candidate_theme") == std::string::npos ||
        configQaSource.find("shared_theme_pixels") == std::string::npos ||
        configPocSource.find("fcitx5-config-poc") == std::string::npos ||
        configPocSource.find("--window-smoke") == std::string::npos ||
        configPocSource.find("CreateWindowExW") == std::string::npos ||
        configPocSource.find("rust-config-poc-window-smoke") == std::string::npos ||
        cmakeSource.find("rust-config-poc-window-smoke") == std::string::npos ||
        cmakeSource.find("config-poc-window-smoke.json") == std::string::npos ||
        configPocSource.find("Fcitx5 for Windows Next") == std::string::npos ||
        configPocSource.find("candidate_preview_embedded") == std::string::npos ||
        configPocSource.find("candidate_preview_current_theme") == std::string::npos ||
        configPocSource.find("candidate_preview_not_external_window") == std::string::npos ||
        configPocSource.find("candidate_preview_rect") == std::string::npos ||
        configPocSource.find("candidate_preview_embedded_in_config_content") ==
            std::string::npos ||
        configPocSource.find("candidate_preview_uses_real_theme_contract") ==
            std::string::npos ||
        configPocSource.find("candidate_preview_renderer_contract") == std::string::npos ||
        configPocSource.find("checked_dpi_scale_percents") == std::string::npos ||
        configPocSource.find("layout_rects_inside_window") == std::string::npos ||
        configPocSource.find("layout_rects_non_overlapping") == std::string::npos ||
        configPocSource.find("addon_action_row_rects") == std::string::npos ||
        configPocSource.find("settings_operation_state_machine") == std::string::npos ||
        configPocManifest.find("fcitx5-control-core") == std::string::npos ||
        configPocManifest.find("fcitx5-package-core") == std::string::npos ||
        configPocSource.find("control_schema_json") == std::string::npos ||
        configPocSource.find("parse_manifest") == std::string::npos ||
        configPocSource.find("parse_repository_index") == std::string::npos ||
        configPocSource.find("parse_trusted_keys") == std::string::npos ||
        configPocSource.find("parse_lockfile") == std::string::npos ||
        configPocSource.find("set_package_state_entries") == std::string::npos ||
        configPocSource.find("mark_package_for_removal_entries") == std::string::npos ||
        configPocSource.find("finalize_package_removal_entries") == std::string::npos ||
        configPocSource.find("typed_control_schema_consumed") == std::string::npos ||
        configPocSource.find("package_core_lifecycle_remove_checked") == std::string::npos ||
        configPocSource.find("package_action_state_machine") == std::string::npos ||
        configPocSource.find("signed_repository_required_for_install") == std::string::npos ||
        configPocSource.find("unconfigured_repository_install_blocked") == std::string::npos ||
        configPocSource.find("RepositoryTrustState::TrustedSignedMetadata") ==
            std::string::npos ||
        configPocSource.find("no_unsafe_commands_for_package_actions") == std::string::npos ||
        configPocSource.find("localized_operation_errors") == std::string::npos ||
        configPocSource.find("SelectCandidateFont") == std::string::npos ||
        configPocSource.find("ToggleAdvancedAppearance") == std::string::npos ||
        configPocSource.find("InstallAddon") == std::string::npos ||
        configPocSource.find("UpdateAddon") == std::string::npos ||
        configPocSource.find("UninstallAddon") == std::string::npos ||
        configPocSource.find("EnableAddon") == std::string::npos ||
        configPocSource.find("DisableAddon") == std::string::npos ||
        cmakeSource.find("rust-config-poc-contract") == std::string::npos ||
        configSource.find("--set-presentation") == std::string::npos ||
        configSource.find("--reset-presentation") == std::string::npos ||
        configSource.find("candidate.automatic") == std::string::npos ||
        englishLocale.find("\"nav.theme\": \"Theme\"") != std::string::npos ||
        englishLocale.find("\"nav.repair\": \"Repair\"") != std::string::npos ||
        englishLocale.find("\"appearance.more\"") == std::string::npos ||
        englishLocale.find("\"candidate.automatic\"") == std::string::npos ||
        englishLocale.find("\"nav.packages\": \"Add-ons & Extensions\"") ==
            std::string::npos ||
        englishLocale.find("\"updates.title\": \"Updates\"") == std::string::npos) {
        return fail("REG-CONFIG-VISUAL-001: Config must use task navigation and shared design tokens");
    }
    if (configSource.find("--demo --parent-pid") != std::string::npos ||
        configSource.find("previewProcess_") != std::string::npos ||
        configSource.find("previewProcessRunning") != std::string::npos ||
        configSource.find("drawPreviewPill") != std::string::npos ||
        configSource.find("drawText(brush, L\"ni hao") != std::string::npos) {
        return fail("REG-CONFIG-LIVE-001: Config candidate preview must be embedded, not an external demo window");
    }
    if (std::filesystem::exists(sourceRoot / "src/candidate/candidate_model.h") ||
        std::filesystem::exists(sourceRoot / "src/candidate/candidate_model.cpp") ||
        std::filesystem::exists(sourceRoot / "tests/unit/candidate_model_test.cpp") ||
        std::filesystem::exists(sourceRoot / "tests/perf/focus_context_churn.cpp") ||
        std::filesystem::exists(sourceRoot / "tests/perf/candidate_render_bench.cpp") ||
        std::filesystem::exists(sourceRoot / "tests/perf/candidate_model_bench.cpp") ||
        cmakeSource.find("fcitx5_candidate_model") != std::string::npos ||
        cmakeSource.find("src/candidate/candidate_model.cpp") != std::string::npos ||
        cmakeSource.find("tests/unit/candidate_model_test.cpp") != std::string::npos ||
        cmakeSource.find("tests/perf/focus_context_churn.cpp") != std::string::npos ||
        cmakeSource.find("tests/perf/candidate_render_bench.cpp") != std::string::npos ||
        cmakeSource.find("tests/perf/candidate_model_bench.cpp") != std::string::npos ||
        cmakeSource.find("fcitx5_candidate_core_rust") == std::string::npos ||
        candidateLayoutSource.find("fcitx5_candidate_model_apply") == std::string::npos ||
        candidateLayoutSource.find("candidate_model_matches_frozen_cpp_contract") ==
            std::string::npos) {
        return fail("CANDIDATE-MODEL-RUST: candidate model semantics must be Rust-owned and the old C++ header/source/tests must stay deleted");
    }
    if (std::filesystem::exists(sourceRoot / "src/ui/candidate_interaction.h") ||
        std::filesystem::exists(sourceRoot / "src/ui/candidate_interaction.cpp") ||
        std::filesystem::exists(sourceRoot / "tests/unit/candidate_interaction_test.cpp") ||
        cmakeSource.find("fcitx5_candidate_interaction") != std::string::npos ||
        cmakeSource.find("src/ui/candidate_interaction.cpp") != std::string::npos ||
        cmakeSource.find("tests/unit/candidate_interaction_test.cpp") != std::string::npos ||
        candidateLayoutSource.find("fcitx5_candidate_hit_test") == std::string::npos ||
        candidateLayoutSource.find("fcitx5_candidate_selection_intent") ==
            std::string::npos ||
        candidateLayoutSource.find("interaction_helpers_match_cpp_contract") ==
            std::string::npos) {
        return fail("CANDIDATE-INTERACTION-RUST: candidate hit-test and selection intent must be Rust-owned and the old C++ source must stay deleted");
    }
    if (std::filesystem::exists(sourceRoot / "src/ui/candidate_layout.h") ||
        std::filesystem::exists(sourceRoot / "src/ui/candidate_layout.cpp") ||
        std::filesystem::exists(sourceRoot / "tests/unit/candidate_layout_test.cpp") ||
        cmakeSource.find("add_library(fcitx5_candidate_layout INTERFACE)") ==
            std::string::npos ||
        cmakeSource.find("src/ui/candidate_layout.cpp") != std::string::npos ||
        cmakeSource.find("tests/unit/candidate_layout_test.cpp") != std::string::npos ||
        candidateLayoutSource.find("fcitx5_candidate_layout_run") == std::string::npos ||
        candidateLayoutSource.find("fcitx5_candidate_render_segments") ==
            std::string::npos ||
        candidateLayoutSource.find("layout_matches_frozen_cpp_contract") ==
            std::string::npos ||
        candidateLayoutSource.find("render_segments_match_label_column_and_comment_contract") ==
            std::string::npos) {
        return fail("CANDIDATE-LAYOUT-RUST: candidate layout/render segments must be Rust-owned and the old C++ header/source/test must stay deleted");
    }
    if (configSource.find("confirmDialog(") == std::string::npos ||
        configSource.find("MessageBoxW") == std::string::npos ||
        configSource.find("std::wstring(get(\"app.title\")) + L\" — \"") ==
            std::string::npos ||
        configSource.find("dialog.reset_appearance.body") == std::string::npos ||
        configSource.find("dialog.remove_package.body") == std::string::npos ||
        configSource.find("dialog.repair.body") == std::string::npos ||
        configSource.find("dialog.trust_failure.body") == std::string::npos ||
        englishLocale.find("\"dialog.language_restart.body\"") == std::string::npos ||
        englishLocale.find("\"dialog.button.cancel\"") == std::string::npos) {
        return fail("CONFIG-UX-007: Settings destructive/trust dialogs must use localized title/body/button keys");
    }
    const auto portableSmoke = read_text(sourceRoot / "tools/test-portable.ps1");
    const auto releaseSmoke = read_text(sourceRoot / "tools/test-release-artifacts.ps1");
    const auto trustedKeyTemplate = read_text(sourceRoot / "security/trusted-keys.template.json");
    if (portableSmoke.find("function Stop-PortableSmokeProcesses") == std::string::npos ||
        portableSmoke.find("Get-CimInstance Win32_Process") == std::string::npos ||
        portableSmoke.find("Test-PackageOutputWritable") == std::string::npos ||
        portableSmoke.find("Remove-Item -LiteralPath $resolved -Recurse -Force") ==
            std::string::npos ||
        releaseSmoke.find("function Stop-PortableSmokeProcesses") == std::string::npos ||
        releaseSmoke.find("Test-ArtifactDirectoryWritable") == std::string::npos ||
        releaseSmoke.find("Test-NoPrivateKeyMaterial") == std::string::npos ||
        releaseSmoke.find("private_key_base64") == std::string::npos ||
        trustedKeyTemplate.find("\"official_required_signatures\": [\"mldsa65\"]") ==
            std::string::npos ||
        trustedKeyTemplate.find("\"key_id\": \"official-2026-mldsa65\"") ==
            std::string::npos ||
        trustedKeyTemplate.find("\"public_key_base64\"") == std::string::npos ||
        trustedKeyTemplate.find("private_key") != std::string::npos ||
        trustedKeyTemplate.find("secret_key") != std::string::npos ||
        trustedKeyTemplate.find("seed_base64") != std::string::npos) {
        return fail("CONFIG-UX-008: portable/package smoke must clean started processes and prove output remains writable");
    }
    const auto resourceHeader = read_text(sourceRoot / "resources/windows/resource.h");
    const auto appRc = read_text(sourceRoot / "resources/windows/app.rc");
    const auto tsfRc = read_text(sourceRoot / "resources/windows/tsf.rc");
    const auto brandDocs = read_text(sourceRoot / "docs/brand-assets.md");
    if (cmakeSource.find("resources/windows/tsf.rc") == std::string::npos ||
        resourceHeader.find("IDI_FCITX5_TSF") == std::string::npos ||
        appRc.find("fcitx5.ico") == std::string::npos ||
        tsfRc.find("fcitx5-tsf.ico") == std::string::npos ||
        configSource.find("SetCurrentProcessExplicitAppUserModelID") == std::string::npos ||
        configSource.find("settings_app_user_model_id") == std::string::npos ||
        brandDocs.find("original geometric artwork") == std::string::npos ||
        brandDocs.find("micro-penguin glyph") == std::string::npos) {
        return fail("REG-BRAND-001: product/TSF penguin icons and Settings AppUserModelID must be wired");
    }
    const auto configParserSource = read_text(sourceRoot / "src/config/config_parser.cpp");
    const auto rustControlCoreSource = read_text(sourceRoot / "rust/control-core/src/lib.rs");
    if (rustControlCoreSource.find("diagnostics_plan_json") == std::string::npos ||
        rustControlCoreSource.find("\"sensitive_input\":false") == std::string::npos ||
        rustControlCoreSource.find("--diagnostics-plan") == std::string::npos ||
        controlSource.find("diagnosticsPlanJson") == std::string::npos ||
        controlSource.find("kRootActionDiagnosticsPlan") == std::string::npos ||
        configSource.find("runControl({L\"--diagnostics-plan\"}") == std::string::npos) {
        return fail("RUST-R2-03: Diagnostics must use Rust typed dry-run plan through Control and Config");
    }
    if (controlSource.find("--reset-presentation") == std::string::npos ||
        rustControlCoreSource.find("reset_presentation") == std::string::npos ||
        configParserSource.find("resetPresentationToml") == std::string::npos ||
        configParserSource.find("root.erase(\"appearance\")") == std::string::npos ||
        configParserSource.find("orientation = \"automatic\"") == std::string::npos) {
        return fail("REG-CONFIG-LIVE-001: Appearance reset must use typed sparse override removal");
    }
    const auto rustTsfPocManifest = read_text(sourceRoot / "rust/tsf-poc/Cargo.toml");
    const auto rustTsfPocSource = read_text(sourceRoot / "rust/tsf-poc/src/lib.rs");
    const auto rustTsfSupportManifest =
        read_text(sourceRoot / "rust/tsf-support-core/Cargo.toml");
    const auto rustTsfSupportSource =
        read_text(sourceRoot / "rust/tsf-support-core/src/lib.rs");
    const auto rustTsfPocSmoke =
        read_text(sourceRoot / "tests/unit/rust_tsf_poc_export_smoke.cpp");
    const auto rustTsfPocArtifactAudit =
        read_text(sourceRoot / "tests/unit/rust_tsf_poc_artifact_audit.cpp");
    const auto buildScript = read_text(sourceRoot / "tools/build.ps1");
    const auto ciCacheScript = read_text(sourceRoot / "tools/configure-ci-cache.ps1");
    const auto cargoConfig = read_text(sourceRoot / ".cargo/config.toml");
    const auto compilerOptions = read_text(sourceRoot / "cmake/CompilerOptions.cmake");
    const auto cmakePresets = read_text(sourceRoot / "CMakePresets.json");
    const auto coreWorkflow = read_text(sourceRoot / ".github/workflows/core.yml");
    const auto releaseWorkflow = read_text(sourceRoot / ".github/workflows/release.yml");
    const auto fastToolchainScript =
        read_text(sourceRoot / "tools/prepare-fast-toolchain.ps1");
    const auto tsfKeyCommitTest =
        read_text(sourceRoot / "tests/integration/tsf_key_commit_test.cpp");
    const auto rustTsfPocCorpus =
        read_text(sourceRoot / "tests/fixtures/tsf_behavior_corpus.json");
    if (cmakeSource.find("fcitx5_tsf_poc_rustdll") == std::string::npos ||
        cmakeSource.find("rust-tsf-poc-unit") == std::string::npos ||
        cmakeSource.find("rust-tsf-poc-export-smoke") == std::string::npos ||
        cmakeSource.find("rust-tsf-poc-artifact-audit") == std::string::npos ||
        rustTsfPocManifest.find("windows = { version = \"0.62.2\"") == std::string::npos ||
        rustTsfPocManifest.find("\"Win32_UI_TextServices\"") == std::string::npos ||
        rustTsfPocSource.find("ITfTextInputProcessorEx") == std::string::npos ||
        rustTsfPocSource.find("IClassFactory") == std::string::npos ||
        rustTsfPocSource.find("activatable_empty_tip:true") == std::string::npos ||
        rustTsfPocSource.find("Fcitx5TsfService") == std::string::npos ||
        rustTsfPocSource.find("ITfKeyEventSink_Impl") == std::string::npos ||
        rustTsfPocSource.find("catch_unwind") == std::string::npos ||
        rustTsfPocSource.find("panic_to_hresult") == std::string::npos ||
        rustTsfPocSource.find("DllGetClassObject") == std::string::npos ||
        rustTsfPocSource.find("DllCanUnloadNow") == std::string::npos ||
        rustTsfPocSource.find("TsfPocBehaviorState") == std::string::npos ||
        rustTsfPocSource.find("state: RefCell<TsfPocBehaviorState>") ==
            std::string::npos ||
        rustTsfPocSource.find("service_lifecycle_callbacks_mutate_and_cleanup_domain_state") ==
            std::string::npos ||
        rustTsfPocSource.find("fail_open_key_down_for_test") == std::string::npos ||
        rustTsfPocSource.find("tsf_behavior_corpus_report") == std::string::npos ||
        rustTsfPocSource.find("tsf_behavior_differential_report") == std::string::npos ||
        rustTsfPocSource.find("Fcitx5TsfPocBehaviorReport") == std::string::npos ||
        rustTsfPocSource.find("Fcitx5TsfPocProfileIdentityReport") == std::string::npos ||
        rustTsfPocSource.find("tsf_profile_identity_report") == std::string::npos ||
        rustTsfPocSource.find("Fcitx5TsfPocIpcBoundaryReport") == std::string::npos ||
        rustTsfPocSource.find("tsf_ipc_boundary_report") == std::string::npos ||
        rustTsfPocSource.find("BoundedIpcClient") == std::string::npos ||
        rustTsfPocSource.find("GenerationMismatch") == std::string::npos ||
        rustTsfPocSource.find("generation_mismatch_fails_open") == std::string::npos ||
        rustTsfPocSource.find("host_blocking_call") == std::string::npos ||
        rustTsfPocSource.find("Fcitx5TsfPocCompositionTranscriptReport") ==
            std::string::npos ||
        rustTsfPocSource.find("tsf_composition_transcript_report") == std::string::npos ||
        rustTsfPocSource.find("EditSessionTranscript") == std::string::npos ||
        rustTsfPocSource.find("begin_edit_session") == std::string::npos ||
        rustTsfPocSource.find("update_preedit_start_composition") == std::string::npos ||
        rustTsfPocSource.find("Fcitx5TsfPocDifferentialSummaryReport") ==
            std::string::npos ||
        rustTsfPocSource.find("tsf_differential_summary_report") == std::string::npos ||
        rustTsfPocSource.find("arm64_ci_artifact_green") == std::string::npos ||
        rustTsfPocSource.find("product_decision\\\":\\\"shipping_rust_cutover") ==
            std::string::npos ||
        rustTsfPocSource.find("product_display_name") == std::string::npos ||
        rustTsfPocSource.find("Fcitx5 for Windows Next") == std::string::npos ||
        rustTsfPocSource.find("profile_display_name") == std::string::npos ||
        rustTsfPocSource.find("Fcitx5") == std::string::npos ||
        rustTsfPocSource.find("text_service_clsid") == std::string::npos ||
        rustTsfPocSource.find("3a21b9e2-4f47-4c36-8bfa-91d7d3b3e901") ==
            std::string::npos ||
        rustTsfPocSource.find("language_profile_guid") == std::string::npos ||
        rustTsfPocSource.find("6c2ac726-7703-4b65-89af-a77e9e0da102") ==
            std::string::npos ||
        rustTsfPocSource.find("dynamic_profile_registration") == std::string::npos ||
        rustTsfPocSource.find("rust_poc_registers_profile") == std::string::npos ||
        rustTsfPocSource.find("Fcitx5TsfPocForcedFailureForTest") == std::string::npos ||
        rustTsfPocSource.find("\"rust_case_passes\"") == std::string::npos ||
        rustTsfPocSource.find("\"cpp_baseline_ctest\":\"tsf-key-commit-e2e\"") ==
            std::string::npos ||
        rustTsfPocSource.find("\"full_host_differential_pending\":true") ==
            std::string::npos ||
        rustTsfPocSource.find("engine_timeout_fails_open") == std::string::npos ||
        rustTsfPocSource.find("malformed_ipc_fails_open") == std::string::npos ||
        rustTsfPocSource.find("uiless_candidate_show_false_preserves_metadata") ==
            std::string::npos ||
        rustTsfPocSource.find("key_busy_focus_change_does_not_clear_composition") ==
            std::string::npos ||
        rustTsfPocSource.find("single_edit_session_commit_preedit_update") ==
            std::string::npos ||
        rustTsfPocSource.find("cxx_tsf_remains_authoritative:false") == std::string::npos ||
        rustTsfPocSource.find("bounded_ipc_client:not-linked") == std::string::npos ||
        rustTsfPocSource.find("send_input:false") == std::string::npos ||
        rustTsfPocSource.find("global_hooks:false") == std::string::npos ||
        rustTsfPocSource.find("process_injection:false") == std::string::npos ||
        rustTsfPocSmoke.find("CLASS_E_CLASSNOTAVAILABLE") == std::string::npos ||
        rustTsfPocSmoke.find("factory should create an empty ITfTextInputProcessorEx") ==
            std::string::npos ||
        rustTsfPocSmoke.find("Fcitx5TsfPocBehaviorReport") == std::string::npos ||
        rustTsfPocSmoke.find("Fcitx5TsfPocProfileIdentityReport") == std::string::npos ||
        rustTsfPocSmoke.find("profile identity report should match stable release identity") ==
            std::string::npos ||
        rustTsfPocSmoke.find("Fcitx5TsfPocIpcBoundaryReport") == std::string::npos ||
        rustTsfPocSmoke.find("generation_mismatch_fails_open") == std::string::npos ||
        rustTsfPocSmoke.find(
            "IPC boundary report should fail open for slow or invalid engine replies") ==
            std::string::npos ||
        rustTsfPocSmoke.find("Fcitx5TsfPocCompositionTranscriptReport") ==
            std::string::npos ||
        rustTsfPocSmoke.find(
            "composition transcript should preserve single edit-session operation order") ==
            std::string::npos ||
        rustTsfPocSmoke.find("Fcitx5TsfPocDifferentialSummaryReport") ==
            std::string::npos ||
        rustTsfPocSmoke.find(
            "differential summary should list green and pending evidence") ==
            std::string::npos ||
        rustTsfPocSmoke.find("Fcitx5TsfPocForcedFailureForTest") == std::string::npos ||
        rustTsfPocSmoke.find("forced internal failure should convert panic to HRESULT") ==
            std::string::npos ||
        rustTsfPocSmoke.find("key callbacks should fail open on unexpected COM state") ==
            std::string::npos ||
        rustTsfPocSmoke.find("\\\"rust_case_passes\\\":10") == std::string::npos ||
        rustTsfPocSmoke.find("\\\"uiless_candidate_show_false_preserves_metadata\\\"") ==
            std::string::npos ||
        rustTsfPocSmoke.find("\\\"key_busy_focus_change_does_not_clear_composition\\\"") ==
            std::string::npos ||
        rustTsfPocSmoke.find("\\\"single_edit_session_commit_preedit_update\\\"") ==
            std::string::npos ||
        rustTsfPocSmoke.find("\\\"cpp_baseline_consumes_same_corpus\\\":true") ==
            std::string::npos ||
        rustTsfPocArtifactAudit.find("parsePe") == std::string::npos ||
        rustTsfPocArtifactAudit.find("winhttp.dll") == std::string::npos ||
        rustTsfPocArtifactAudit.find("ws2_32.dll") == std::string::npos ||
        rustTsfPocArtifactAudit.find("2 * 1024 * 1024") == std::string::npos ||
        rustTsfPocArtifactAudit.find("ASLR and NX") == std::string::npos ||
        rustTsfPocArtifactAudit.find("product engine/config/package/control") ==
            std::string::npos ||
        rustTsfPocCorpus.find("\"format_version\": 1") == std::string::npos ||
        rustTsfPocCorpus.find("\"activate_advises_sinks\"") == std::string::npos ||
        rustTsfPocCorpus.find("\"key_down_commit_applies_text\"") == std::string::npos ||
        rustTsfPocCorpus.find("\"key_down_preedit_starts_composition\"") ==
            std::string::npos ||
        rustTsfPocCorpus.find("\"key_up_routes_release_without_eating\"") ==
            std::string::npos ||
        rustTsfPocCorpus.find("\"engine_timeout_fails_open\"") == std::string::npos ||
        rustTsfPocCorpus.find("\"malformed_ipc_fails_open\"") == std::string::npos ||
        rustTsfPocCorpus.find("\"deactivate_unadvises_sinks_and_clears_composition\"") ==
            std::string::npos ||
        rustTsfPocCorpus.find("\"uiless_candidate_show_false_preserves_metadata\"") ==
            std::string::npos ||
        rustTsfPocCorpus.find("\"key_busy_focus_change_does_not_clear_composition\"") ==
            std::string::npos ||
        rustTsfPocCorpus.find("\"single_edit_session_commit_preedit_update\"") ==
            std::string::npos ||
        buildScript.find("Import-MsvcEnvironment") == std::string::npos ||
        buildScript.find("Microsoft.VisualStudio.Component.VC.Tools.ARM64") ==
            std::string::npos ||
        buildScript.find("amd64_arm64") == std::string::npos ||
        buildScript.find("Assert-FastWindowsToolchain") == std::string::npos ||
        buildScript.find("Default build requires $tool") == std::string::npos ||
        buildScript.find("clang-cl") == std::string::npos ||
        buildScript.find("lld-link") == std::string::npos ||
        buildScript.find("ninja") == std::string::npos ||
        buildScript.find("sccachePath = [System.IO.Path]::GetFullPath") ==
            std::string::npos ||
        buildScript.find("CMAKE_C_COMPILER_LAUNCHER=$sccachePath") ==
            std::string::npos ||
        ciCacheScript.find("RUSTC_WRAPPER = 'sccache'") == std::string::npos ||
        ciCacheScript.find("CARGO_INCREMENTAL = '0'") != std::string::npos ||
        ciCacheScript.find("SCCACHE_CACHE_SIZE = '30G'") == std::string::npos ||
        ciCacheScript.find("SCCACHE_IGNORE_SERVER_IO_ERROR = '1'") == std::string::npos ||
        ciCacheScript.find("ACTIONS_RUNTIME_TOKEN") == std::string::npos ||
        ciCacheScript.find("GITHUB_ACTIONS") == std::string::npos ||
        cargoManifest.find("debug = \"line-tables-only\"") == std::string::npos ||
        cargoManifest.find("[profile.dev.package.\"*\"]") == std::string::npos ||
        cargoConfig.find("rustc-wrapper = \"sccache\"") == std::string::npos ||
        cargoConfig.find("linker = \"rust-lld\"") == std::string::npos ||
        cargoConfig.find("aarch64-pc-windows-msvc") == std::string::npos ||
        cmakePresets.find("\"generator\": \"Ninja Multi-Config\"") == std::string::npos ||
        cmakePresets.find("\"CMAKE_C_COMPILER\": \"clang-cl\"") == std::string::npos ||
        cmakePresets.find("\"CMAKE_CXX_COMPILER\": \"clang-cl\"") == std::string::npos ||
        cmakePresets.find("\"CMAKE_LINKER_TYPE\": \"LLD\"") == std::string::npos ||
        cmakePresets.find("--target=aarch64-pc-windows-msvc") == std::string::npos ||
        cmakePresets.find("\"FCITX_TARGET_ARCH\": \"arm64\"") == std::string::npos ||
        cmakeSource.find("FCITX_COMPILER_IS_CLANG_CL") == std::string::npos ||
        cmakeSource.find("CMAKE_MSVC_DEBUG_INFORMATION_FORMAT") == std::string::npos ||
        cmakeSource.find("CMAKE_CXX_SCAN_FOR_MODULES") == std::string::npos ||
        cmakeSource.find("FCITX_PROJECT_PCH_HEADER") == std::string::npos ||
        compilerOptions.find("FCITX_TARGET_USES_PCH") == std::string::npos ||
        compilerOptions.find("target_precompile_headers") == std::string::npos ||
        cmakeSource.find("src/pch/fcitx_windows_pch.h") == std::string::npos ||
        cmakeSource.find("FCITX_RUST_BUILD_ENV") == std::string::npos ||
        cmakeSource.find("FCITX_SCCACHE_EXECUTABLE") == std::string::npos ||
        cmakeSource.find("RUSTC_WRAPPER=${FCITX_SCCACHE_EXECUTABLE}") == std::string::npos ||
        cmakeSource.find("SCCACHE_DIR=$ENV{SCCACHE_DIR}") == std::string::npos ||
        cmakeSource.find("FCITX_EFFECTIVE_TARGET_ARCH") == std::string::npos ||
        coreWorkflow.find("Restore fast Windows toolchain cache") == std::string::npos ||
        coreWorkflow.find("Prepare fast Windows toolchain") == std::string::npos ||
        releaseWorkflow.find("Restore fast Windows toolchain cache") == std::string::npos ||
        releaseWorkflow.find("Prepare fast Windows toolchain") == std::string::npos ||
        fastToolchainScript.find("out/toolchains/fast") == std::string::npos ||
        fastToolchainScript.find("7zr.exe") == std::string::npos ||
        fastToolchainScript.find("clang+llvm-$llvmVersion-x86_64-pc-windows-msvc.tar.xz") ==
            std::string::npos ||
        fastToolchainScript.find("cmake-$cmakeVersion-windows-x86_64.zip") ==
            std::string::npos ||
        fastToolchainScript.find("tar.exe") == std::string::npos ||
        fastToolchainScript.find("curl.exe") == std::string::npos ||
        fastToolchainScript.find("ctest") == std::string::npos ||
        fastToolchainScript.find("ninja") == std::string::npos ||
        fastToolchainScript.find("sccache") == std::string::npos ||
        fastToolchainScript.find("clang-cl") == std::string::npos ||
        fastToolchainScript.find("lld-link") == std::string::npos ||
        buildScript.find("vcvarsall.bat") ==
            std::string::npos) {
        return fail("RUST-R3-TSF-POC: Rust TSF PoC must stay isolated, panic-contained, and non-authoritative");
    }
    if (cmakeSource.find("tsf_behavior_corpus.json") == std::string::npos ||
        tsfKeyCommitTest.find("verifyBehaviorCorpus") == std::string::npos ||
        tsfKeyCommitTest.find("TSF behavior corpus missing marker") == std::string::npos ||
        tsfKeyCommitTest.find("\\\"key_down_commit_applies_text\\\"") ==
            std::string::npos ||
        tsfKeyCommitTest.find("\\\"deactivate_unadvises_sinks_and_clears_composition\\\"") ==
            std::string::npos ||
        tsfKeyCommitTest.find("\\\"uiless_candidate_show_false_preserves_metadata\\\"") ==
            std::string::npos ||
        tsfKeyCommitTest.find("\\\"key_busy_focus_change_does_not_clear_composition\\\"") ==
            std::string::npos ||
        tsfKeyCommitTest.find("\\\"single_edit_session_commit_preedit_update\\\"") ==
            std::string::npos) {
        return fail("RUST-R3-TSF-POC: C++ TSF baseline must consume the shared behavior corpus before differential cutover");
    }
    if (rustTsfPocManifest.find("fcitx5-package-core") != std::string::npos ||
        rustTsfPocManifest.find("fcitx5-control-core") != std::string::npos ||
        rustTsfPocManifest.find("fcitx5-candidate-core") != std::string::npos ||
        rustTsfPocSource.find("SendInput") != std::string::npos ||
        rustTsfPocSource.find("SetWindowsHookEx") != std::string::npos ||
        rustTsfPocSource.find("CreateRemoteThread") != std::string::npos ||
        rustTsfPocSource.find("WriteProcessMemory") != std::string::npos) {
        return fail("RUST-R3-TSF-POC: Rust TSF PoC must not link product control/package/candidate or prohibited host APIs");
    }
    if (std::filesystem::exists(sourceRoot / "src/tsf/activation_guard.h") ||
        std::filesystem::exists(sourceRoot / "src/tsf/activation_guard.cpp") ||
        std::filesystem::exists(sourceRoot / "tests/unit/tsf_activation_guard_test.cpp") ||
        cargoManifest.find("\"rust/tsf-support-core\"") == std::string::npos ||
        cargoLock.find("name = \"fcitx5-tsf-support-core\"") == std::string::npos ||
        rustTsfSupportManifest.find("crate-type = [\"staticlib\", \"rlib\"]") ==
            std::string::npos ||
        rustTsfSupportManifest.find("Win32_Storage_FileSystem") == std::string::npos ||
        controlSource.find("#include \"activation_guard.h\"") != std::string::npos ||
        cmakeSource.find("tests/unit/tsf_activation_guard_test.cpp") != std::string::npos ||
        rustTsfSupportSource.find("fcitx5_tsf_activation_guard_status") ==
            std::string::npos ||
        rustTsfSupportSource.find("fcitx5_tsf_activation_attempt_begin") ==
            std::string::npos ||
        rustTsfSupportSource.find("MOVEFILE_REPLACE_EXISTING") == std::string::npos ||
        rustTsfSupportSource.find("SendInput") != std::string::npos ||
        rustTsfSupportSource.find("SetWindowsHookEx") != std::string::npos ||
        rustTsfSupportSource.find("CreateRemoteThread") != std::string::npos ||
        rustTsfSupportSource.find("WriteProcessMemory") != std::string::npos ||
        cmakeSource.find("FCITX_RUST_TSF_SUPPORT_CORE_STATICLIB") ==
            std::string::npos ||
        cmakeSource.find("fcitx5-tsf-support-core") == std::string::npos ||
        cmakeSource.find("FCITX_RELEASE_DATA_DIRECTORY=") == std::string::npos) {
        return fail("TSF-SUPPORT-RUST: activation guard policy must be Rust-owned and the old C++ header/source/test must stay deleted");
    }
    if (std::filesystem::exists(sourceRoot / "src/tsf/input_scope_policy.h") ||
        std::filesystem::exists(sourceRoot / "tests/unit/input_scope_policy_test.cpp") ||
        cmakeSource.find("tests/unit/input_scope_policy_test.cpp") != std::string::npos ||
        cmakeSource.find("fcitx5_input_scope_policy_test") != std::string::npos ||
        cmakeSource.find("tsf-sensitive-input-policy") != std::string::npos ||
        rustTsfPocSource.find("fn is_sensitive_input_scope") == std::string::npos ||
        rustTsfPocSource.find("sensitive_input_scope_policy_matches_frozen_cpp_contract") ==
            std::string::npos ||
        rustTsfPocSource.find("IS_PASSWORD") == std::string::npos ||
        rustTsfPocSource.find("IS_ALPHANUMERIC_PIN_SET") == std::string::npos) {
        return fail("TSF-SUPPORT-RUST: input-scope sensitivity policy must be Rust-owned and the old C++ header/test must stay deleted");
    }
    std::cout << "source-contract ok\n";
    return 0;
}
