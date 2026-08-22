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
    const auto controlSource = read_text(sourceRoot / "src/control/control_main.cpp");
    if (controlSource.find("CreateProcessW(") != std::string::npos ||
        controlSource.find("WaitForSingleObject(process.hProcess") != std::string::npos) {
        return fail("REG-PROC-PIPE-001: Control must use the shared process executor");
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
    const auto registerSource = read_text(sourceRoot / "src/register/register_main.cpp");
    if (registerSource.find("validateProductArtifact") == std::string::npos ||
        registerSource.find("--validate-artifact") == std::string::npos ||
        registerSource.find("paired architecture TSF DLL is missing") == std::string::npos) {
        return fail("STAB-REGISTER-BOOTSTRAP-012: register helper must validate product artifacts");
    }
    const auto bootstrapSource = read_text(sourceRoot / "src/bootstrap/bootstrap_main.cpp");
    if (bootstrapSource.find("TerminateProcess(process.hProcess, ERROR_TIMEOUT)") ==
            std::string::npos ||
        bootstrapSource.find("TerminateProcess(info.hProcess, ERROR_TIMEOUT)") ==
            std::string::npos ||
        bootstrapSource.find("WaitForSingleObject(process.hProcess, 5000)") ==
            std::string::npos ||
        bootstrapSource.find("WaitForSingleObject(info.hProcess, 5000)") ==
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
    const auto dependencyCheck = read_text(sourceRoot / "tools/check-dependencies.ps1");
    const auto dependencyInventory = read_text(sourceRoot / "third_party/dependencies.json");
    const auto rustPackageCoreArtifactSmoke =
        read_text(sourceRoot / "tools/test-rust-package-core-artifact.ps1");
    const auto cmakeSource = read_text(sourceRoot / "CMakeLists.txt");
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
    const auto englishLocale = read_text(sourceRoot / "locales/en-US.json");
    if (configSource.find("struct DesignTokens") == std::string::npos ||
        configSource.find("designTokens()") == std::string::npos ||
        configSource.find("--ui-visual-contract-test") == std::string::npos ||
        configSource.find("--ui-live-preview-contract-test") == std::string::npos ||
        configSource.find("ensureProductionPreview()") == std::string::npos ||
        configSource.find("fcitx5-ui.exe") == std::string::npos ||
        configSource.find("--demo --parent-pid") == std::string::npos ||
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
    if (controlSource.find("--reset-presentation") == std::string::npos ||
        controlSource.find("reset_presentation") == std::string::npos ||
        configParserSource.find("resetPresentationToml") == std::string::npos ||
        configParserSource.find("root.erase(\"appearance\")") == std::string::npos ||
        configParserSource.find("orientation = \"automatic\"") == std::string::npos) {
        return fail("REG-CONFIG-LIVE-001: Appearance reset must use typed sparse override removal");
    }
    std::cout << "source-contract ok\n";
    return 0;
}
