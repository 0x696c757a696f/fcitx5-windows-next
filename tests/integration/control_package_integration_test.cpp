#include "package_core.h"

#include <windows.h>
#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <span>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include <miniz.h>

namespace {

namespace fs = std::filesystem;

class TemporaryDirectory final {
 public:
  TemporaryDirectory() {
    path_ = fs::current_path() /
            (L"control-package-integration-" + std::to_wstring(GetCurrentProcessId()));
    std::error_code ignored;
    fs::remove_all(path_, ignored);
    fs::create_directories(path_);
  }

  ~TemporaryDirectory() {
    std::error_code ignored;
    fs::remove_all(path_, ignored);
  }

  [[nodiscard]] const fs::path& path() const noexcept { return path_; }

 private:
  fs::path path_;
};

void expect(bool condition, std::string_view message) {
  if (!condition) {
    throw std::runtime_error(std::string(message));
  }
}

void write_bytes(const fs::path& path, std::span<const std::byte> bytes) {
  fs::create_directories(path.parent_path());
  std::ofstream output(path, std::ios::binary);
  if (!bytes.empty()) {
    output.write(reinterpret_cast<const char*>(bytes.data()),
                 static_cast<std::streamsize>(bytes.size()));
  }
  if (!output) {
    throw std::runtime_error("fixture write failed");
  }
}

void write_text(const fs::path& path, std::string_view text) {
  write_bytes(path, std::as_bytes(std::span(text)));
}

std::string read_text(const fs::path& path) {
  std::ifstream input(path, std::ios::binary);
  if (!input) {
    throw std::runtime_error("fixture read failed");
  }
  return {std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>()};
}

std::string theme_fixture(std::string_view id, std::string_view name) {
  return "format_version = 1\n"
         "[theme]\n"
         "id = \"" +
         std::string(id) +
         "\"\n"
         "name = \"" +
         std::string(name) +
         "\"\n"
         "version = \"1.0.0\"\n"
         "license = \"MIT\"\n"
         "description = \"Theme fixture\"\n"
         "[common.candidate]\n"
         "orientation = \"vertical\"\n"
         "[common.fonts.candidate]\n"
         "families = [\"Microsoft YaHei\", \"system\"]\n"
         "size_dip = 18.0\n"
         "[light.candidate.colors]\n"
         "background = \"#FFFFFFFF\"\n"
         "candidate_text = \"#222222FF\"\n"
         "[dark.candidate.colors]\n"
         "background = \"#222222FF\"\n"
         "candidate_text = \"#FFFFFFFF\"\n";
}

using ArchiveEntry = std::pair<std::string, std::vector<std::byte>>;

std::vector<std::byte> as_bytes(std::string_view value) {
  const auto bytes = std::as_bytes(std::span(value));
  return {bytes.begin(), bytes.end()};
}

void create_archive(const fs::path& path, const std::vector<ArchiveEntry>& entries) {
  mz_zip_archive archive{};
  const auto narrow_path = path.string();
  if (mz_zip_writer_init_file(&archive, narrow_path.c_str(), 0) != MZ_TRUE) {
    throw std::runtime_error("ZIP fixture initialization failed");
  }
  bool success = true;
  for (const auto& [name, contents] : entries) {
    if (mz_zip_writer_add_mem(&archive, name.c_str(), contents.data(), contents.size(),
                              MZ_BEST_COMPRESSION) != MZ_TRUE) {
      success = false;
      break;
    }
  }
  if (success) {
    success = mz_zip_writer_finalize_archive(&archive) == MZ_TRUE;
  }
  static_cast<void>(mz_zip_writer_end(&archive));
  if (!success) {
    throw std::runtime_error("ZIP fixture creation failed");
  }
}

std::wstring quote_argument(std::wstring_view value) {
  std::wstring result = L"\"";
  unsigned slashes = 0;
  for (const auto character : value) {
    if (character == L'\\') {
      ++slashes;
      continue;
    }
    if (character == L'\"') {
      result.append(slashes + 1U, L'\\');
    } else {
      result.append(slashes, L'\\');
    }
    slashes = 0;
    result.push_back(character);
  }
  result.append(slashes * 2U, L'\\');
  result.push_back(L'\"');
  return result;
}

DWORD run_process(const fs::path& executable, const std::vector<std::wstring>& arguments) {
  std::wstring command = quote_argument(executable.wstring());
  for (const auto& argument : arguments) {
    command.push_back(L' ');
    command += quote_argument(argument);
  }
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION process{};
  if (!CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, FALSE,
                      CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process)) {
    throw std::runtime_error("control process creation failed");
  }
  const DWORD wait = WaitForSingleObject(process.hProcess, 20000U);
  if (wait != WAIT_OBJECT_0) {
    TerminateProcess(process.hProcess, ERROR_TIMEOUT);
  }
  DWORD exit_code = ERROR_TIMEOUT;
  if (wait == WAIT_OBJECT_0) {
    GetExitCodeProcess(process.hProcess, &exit_code);
  }
  CloseHandle(process.hThread);
  CloseHandle(process.hProcess);
  return exit_code;
}

DWORD run_process_capture(const fs::path& executable,
                          const std::vector<std::wstring>& arguments,
                          std::string& output) {
  SECURITY_ATTRIBUTES attributes{sizeof(attributes), nullptr, TRUE};
  HANDLE readPipe = nullptr;
  HANDLE writePipe = nullptr;
  if (!CreatePipe(&readPipe, &writePipe, &attributes, 0)) {
    throw std::runtime_error("control output pipe creation failed");
  }
  SetHandleInformation(readPipe, HANDLE_FLAG_INHERIT, 0);
  std::wstring command = quote_argument(executable.wstring());
  for (const auto& argument : arguments) {
    command.push_back(L' ');
    command += quote_argument(argument);
  }
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  startup.dwFlags = STARTF_USESTDHANDLES;
  startup.hStdOutput = writePipe;
  startup.hStdError = writePipe;
  PROCESS_INFORMATION process{};
  if (!CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, TRUE,
                      CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process)) {
    CloseHandle(readPipe);
    CloseHandle(writePipe);
    throw std::runtime_error("control capture process creation failed");
  }
  CloseHandle(writePipe);
  CloseHandle(process.hThread);
  output.clear();
  std::array<char, 4096> buffer{};
  DWORD count = 0;
  while (ReadFile(readPipe, buffer.data(), static_cast<DWORD>(buffer.size()), &count, nullptr) &&
         count != 0) {
    output.append(buffer.data(), buffer.data() + count);
    if (output.size() > 1024U * 1024U) {
      TerminateProcess(process.hProcess, ERROR_FILE_TOO_LARGE);
      break;
    }
  }
  CloseHandle(readPipe);
  const DWORD wait = WaitForSingleObject(process.hProcess, 20000U);
  if (wait != WAIT_OBJECT_0) {
    TerminateProcess(process.hProcess, ERROR_TIMEOUT);
  }
  DWORD exit_code = ERROR_TIMEOUT;
  if (wait == WAIT_OBJECT_0) {
    GetExitCodeProcess(process.hProcess, &exit_code);
  }
  CloseHandle(process.hProcess);
  return exit_code;
}

std::string architecture() {
#if defined(_WIN64)
  return "x64";
#else
  return "x86";
#endif
}

std::string make_manifest(std::string_view version, std::string_view blake3_hash,
                          std::string_view sha256_hash, std::uint64_t file_size) {
  return "{\n"
         "  \"format_version\": 2,\n"
         "  \"id\": \"fcitx5-rime\",\n"
         "  \"version\": \"" +
         std::string(version) +
         "\",\n"
         "  \"type\": \"addon\",\n"
         "  \"architecture\": \"" +
         architecture() +
         "\",\n"
         "  \"min_os\": \"6.1-sp1\",\n"
         "  \"core_api\": \"1\",\n"
         "  \"addon_abi\": \"1\",\n"
         "  \"dependencies\": [],\n"
         "  \"license\": \"MIT\",\n"
         "  \"source_commit\": \"0123456789abcdef\",\n"
         "  \"runtime_abi\": \"1\",\n"
         "  \"runtime_build\": \"0123456789abcdef+tools/bootstrap-fcitx.ps1\",\n"
         "  \"source\": {\"repository\": \"https://github.com/fcitx/fcitx5-rime.git\",\"commit\": \"0123456789abcdef\",\"build_script\": \"tools/bootstrap-fcitx.ps1\"},\n"
         "  \"data_policy\": {\"program\": \"versioned\",\"user_data\": \"durable\"},\n"
         "  \"permissions\": [\"native-code\", \"input-data\"],\n"
         "  \"payload\": [{\"path\": \"bin/addon.dll\", \"size\": " +
         std::to_string(file_size) + ", \"hashes\": {\"blake3\": \"" +
         std::string(blake3_hash) + "\", \"sha256\": \"" + std::string(sha256_hash) +
         "\"}}],\n"
         "  \"key_id\": \"official-test-2026-mldsa65\"\n"
         "}\n";
}

std::string make_repository(std::string_view version, std::uint64_t release_sequence,
                            std::string_view archive_hash) {
  const auto target = "fcitx5-rime\t" + std::string(version) + "\t" +
                      std::to_string(release_sequence) + "\t" + architecture() + "\t" +
                      std::string(archive_hash) + "\n";
  const auto target_hash = fcitx::package::hex_sha256(
      fcitx::package::sha256(std::as_bytes(std::span(target))));
  return "{\"format_version\":1,\"channel\":\"stable\","
         "\"repository_id\":\"fcitx5-windows-next\",\"mirror_id\":\"official\","
         "\"sequence\":" +
         std::to_string(release_sequence) +
         ",\"generated_at\":\"2026-08-28T00:00:00Z\",\"expires_at\":\"2026-09-01T00:00:00Z\","
         "\"key_id\":\"official-test-2026-mldsa65\",\"targets\":{\"count\":1,\"sha256\":\"" +
         target_hash + "\"},"
         "\"packages\":[{\"id\":\"fcitx5-rime\",\"title\":\"Rime\","
         "\"summary\":\"Rime input engine\",\"version\":\"" +
         std::string(version) +
         "\",\"release_sequence\":" +
         std::to_string(release_sequence) +
         ",\"type\":\"addon\",\"architecture\":\"" + architecture() +
         "\",\"download_url\":\"https://packages.example.invalid/fcitx5-rime.fcpkg\","
         "\"sha256\":\"" + std::string(archive_hash) + "\",\"dependencies\":[]}]}";
}

DWORD run_process(const fs::path& executable, const std::vector<std::wstring>& arguments);

void run_required(const fs::path& executable, const std::vector<std::wstring>& arguments) {
  if (run_process(executable, arguments) != 0U) {
    throw std::runtime_error("required signing fixture command failed");
  }
}

void publish_package_fixture(const fs::path& fixture_root, const fs::path& data_root,
                             const fs::path& signer, std::string_view version,
                             std::uint64_t release_sequence, std::string_view payload) {
  const auto payload_sha256 = fcitx::package::hex_sha256(
      fcitx::package::sha256(std::as_bytes(std::span(payload))));
  const auto payload_blake3 = fcitx::package::hex_blake3(
      fcitx::package::blake3(std::as_bytes(std::span(payload))));
  const auto manifest = make_manifest(version, payload_blake3, payload_sha256, payload.size());
  const std::wstring wide_version(version.begin(), version.end());
  const auto archive = fixture_root / (L"fcitx5-rime-" + wide_version + L".fcpkg");
  const auto manifest_path = fixture_root / (L"manifest-" + wide_version + L".json");
  const auto manifest_signature = fixture_root / (L"manifest-" + wide_version + L".sig.json");
  const auto keyring = fixture_root / L"trusted-keys.json";
  write_text(manifest_path, manifest);
  run_required(signer, {L"--sign", L"package-manifest", manifest_path.wstring(),
                        manifest_signature.wstring(), keyring.wstring(),
                        L"official-test-2026-mldsa65"});
  create_archive(archive,
                 {{"manifest.json", as_bytes(manifest)},
                  {"manifest.sig.json", as_bytes(read_text(manifest_signature))},
                  {"payload/bin/addon.dll", as_bytes(payload)}});
  const auto archive_hash =
      fcitx::package::hex_sha256(fcitx::package::sha256_file(archive));
  const auto repository = make_repository(version, release_sequence, archive_hash);
  const auto index = fixture_root / L"index.json";
  const auto index_signature = fixture_root / L"index.sig.json";
  write_text(index, repository);
  run_required(signer, {L"--sign", L"repository-index", index.wstring(),
                        index_signature.wstring(), keyring.wstring(),
                        L"official-test-2026-mldsa65"});
  write_text(data_root / L"repository/index.json", repository);
  write_text(data_root / L"repository/index.sig.json", read_text(index_signature));
  fs::create_directories(data_root / L"app-security");
  fs::copy_file(keyring, data_root / L"app-security/trusted-keys.json",
                fs::copy_options::overwrite_existing);
  write_text(data_root / L"repository/sequence-stable.json",
             "format_version=1\nchannel=stable\nmax_release_sequence=" +
                 std::to_string(release_sequence) + "\n");
  fs::create_directories(data_root / L"downloads");
  fs::copy_file(archive,
                data_root / L"downloads" / (L"fcitx5-rime-" + wide_version + L".fcpkg"),
                fs::copy_options::overwrite_existing);
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
  try {
    expect(argc == 4, "expected control, downloader, and signing fixture paths");
    const fs::path control_source = argv[1];
    const fs::path downloader_source = argv[2];
    const fs::path signer = argv[3];
    expect(fs::is_regular_file(control_source) && fs::is_regular_file(downloader_source) &&
               fs::is_regular_file(signer),
           "required package lifecycle executable is missing");

    TemporaryDirectory temporary;
    const auto app_root = temporary.path() / L"app";
    const auto app_bin = app_root / L"bin";
    const auto data_root = temporary.path() / L"data";
    fs::create_directories(app_bin);
    const auto control = app_bin / L"fcitx5-control.exe";
    fs::copy_file(control_source, control, fs::copy_options::overwrite_existing);
    fs::copy_file(downloader_source, app_bin / L"fcitx5-downloader.exe",
                  fs::copy_options::overwrite_existing);
    fs::create_directories(app_root / L"security");
    write_text(app_root / L"share/fcitx5/addon/pinyin.conf",
               "[Addon]\n"
               "Name=Pinyin\n"
               "Category=InputMethod\n"
               "Version=5.1.12\n"
               "Library=libpinyin\n"
               "Type=SharedLibrary\n"
               "Configurable=True\n"
               "OnDemand=True\n");
    write_text(app_root / L"share/fcitx5/addon/clipboard.conf",
               "[Addon]\n"
               "Name=Clipboard\n"
               "Category=Module\n"
               "Library=libclipboard\n"
               "Type=SharedLibrary\n"
               "Configurable=False\n");
    write_text(app_root / L"lib/fcitx5/libpinyin.dll", "fixture");
    write_text(app_root / L"resources/themes/default/theme.toml",
               theme_fixture("builtin.default", "Fcitx5 Default"));
    write_text(data_root / L"themes/eosphoros-night/theme.toml",
               theme_fixture("eosphoros-night", "Eosphoros Night"));
    expect(!fs::exists(app_bin / L"fcitx5-launcher.exe"),
           "fixture must exercise package management with no launcher service");

    std::string themes_list;
    const DWORD themes_list_exit =
        run_process_capture(control, {L"--data-root", data_root.wstring(), L"--themes-list"},
                            themes_list);
    expect(themes_list_exit == 0, "theme list must succeed with builtin and user themes");
    expect(themes_list.find("\"id\":\"builtin:default\"") != std::string::npos &&
               themes_list.find("\"id\":\"eosphoros-night\"") != std::string::npos &&
               themes_list.find("\"source\":\"user\"") != std::string::npos,
           "theme list did not expose builtin and imported user themes: " + themes_list);

    std::string theme_detail;
    const DWORD theme_detail_exit = run_process_capture(
        control, {L"--data-root", data_root.wstring(), L"--themes-detail", L"eosphoros-night"},
        theme_detail);
    expect(theme_detail_exit == 0, "theme detail must succeed for an imported user theme");
    expect(theme_detail.find("\"editable_fields\"") != std::string::npos &&
               theme_detail.find("\"candidate.colors.background\"") != std::string::npos &&
               theme_detail.find("\"network_allowed\":false") != std::string::npos,
           "theme detail did not expose the safe editor surface: " + theme_detail);

    std::string theme_export;
    const DWORD theme_export_exit = run_process_capture(
        control, {L"--data-root", data_root.wstring(), L"--themes-export", L"eosphoros-night"},
        theme_export);
    expect(theme_export_exit == 0 && theme_export.find("Eosphoros Night") != std::string::npos,
           "theme export must return the selected user theme TOML: " + theme_export);
    const auto theme_export_path = temporary.path() / L"exported-eosphoros-night.toml";
    std::string theme_export_to;
    const DWORD theme_export_to_exit = run_process_capture(
        control,
        {L"--data-root", data_root.wstring(), L"--themes-export-to", L"eosphoros-night",
         theme_export_path.wstring()},
        theme_export_to);
    expect(theme_export_to_exit == 0 &&
               theme_export_to.find("\"operation\":\"export\"") != std::string::npos &&
               read_text(theme_export_path).find("Eosphoros Night") != std::string::npos,
           "theme export-to must atomically write the selected user theme: " +
               theme_export_to);

    write_text(temporary.path() / L"theme-import.toml", theme_fixture("soft-blue", "Soft Blue"));
    std::string theme_import;
    const DWORD theme_import_exit = run_process_capture(
        control,
        {L"--data-root", data_root.wstring(), L"--themes-import",
         (temporary.path() / L"theme-import.toml").wstring()},
        theme_import);
    expect(theme_import_exit == 0 &&
               theme_import.find("\"operation\":\"import\"") != std::string::npos &&
               fs::exists(data_root / L"themes/soft-blue/theme.toml"),
           "theme import must publish a validated user theme: " + theme_import);

    std::string theme_duplicate;
    const DWORD theme_duplicate_exit =
        run_process_capture(control,
                            {L"--data-root", data_root.wstring(), L"--themes-duplicate",
                             L"builtin:default", L"default-copy"},
                            theme_duplicate);
    expect(theme_duplicate_exit == 0 &&
               theme_duplicate.find("\"operation\":\"duplicate\"") != std::string::npos &&
               fs::exists(data_root / L"themes/default-copy/theme.toml"),
           "theme duplicate must copy a builtin theme into user scope: " + theme_duplicate);

    std::string theme_delete_readonly;
    const DWORD theme_delete_readonly_exit =
        run_process_capture(control,
                            {L"--data-root", data_root.wstring(), L"--themes-delete",
                             L"builtin:default"},
                            theme_delete_readonly);
    expect(theme_delete_readonly_exit != 0,
           "theme delete must reject the read-only builtin theme");

    std::string theme_delete;
    const DWORD theme_delete_exit =
        run_process_capture(control,
                            {L"--data-root", data_root.wstring(), L"--themes-delete",
                             L"soft-blue"},
                            theme_delete);
    expect(theme_delete_exit == 0 &&
               theme_delete.find("\"operation\":\"delete\"") != std::string::npos &&
               !fs::exists(data_root / L"themes/soft-blue"),
           "theme delete must remove only user-owned theme directories: " + theme_delete);

    const auto config_update_path = temporary.path() / L"candidate-font-config.toml";
    write_text(config_update_path,
               "format_version = 1\n"
               "[fonts.candidate]\n"
               "families = [\"Segoe UI Emoji\", \"system\"]\n"
               "size_dip = 20.0\n");
    std::string config_validate;
    const DWORD config_validate_exit = run_process_capture(
        control, {L"--data-root", data_root.wstring(), L"--validate-config",
                  config_update_path.wstring()},
        config_validate);
    expect(config_validate_exit == 0,
           "typed Config validation must accept candidate font overrides: " + config_validate);
    std::string config_apply;
    const DWORD config_apply_exit = run_process_capture(
        control, {L"--data-root", data_root.wstring(), L"--apply-config",
                  config_update_path.wstring()},
        config_apply);
    expect(config_apply_exit == 0,
           "typed Config apply must work with the launcher service stopped: " + config_apply);
    const std::string persisted_config = read_text(data_root / L"config.toml");
    expect(persisted_config.find("Segoe UI Emoji") != std::string::npos &&
               persisted_config.find("size_dip = 20.0") != std::string::npos,
           "typed Config apply did not persist candidate font overrides: " + persisted_config);

    std::string addons_list;
    const DWORD addons_list_exit =
        run_process_capture(control, {L"--data-root", data_root.wstring(), L"--addons-list"},
                            addons_list);
    expect(addons_list_exit == 0, "addon descriptor inventory must succeed");
    expect(addons_list.find("\"surface\":\"descriptor-inventory\"") != std::string::npos &&
               addons_list.find("\"typed_config\":\"not_available\"") != std::string::npos &&
               addons_list.find("\"id\":\"pinyin\"") != std::string::npos &&
               addons_list.find("\"category\":\"InputMethod\"") != std::string::npos &&
               addons_list.find("\"configurable\":true") != std::string::npos &&
               addons_list.find("\"library_present\":true") != std::string::npos &&
               addons_list.find("\"id\":\"clipboard\"") != std::string::npos,
           "addon descriptor inventory did not expose safe Advanced R1 surface: " +
               addons_list);

    constexpr std::string_view initial_payload = "verified control package fixture v1\n";
    publish_package_fixture(temporary.path(), data_root, signer, "1.0.0", 1U,
                            initial_payload);
    fs::copy_file(data_root / L"app-security/trusted-keys.json",
                  app_root / L"security/trusted-keys.json", fs::copy_options::overwrite_existing);

    std::string install_output;
    const DWORD install_exit =
        run_process_capture(control, {L"--data-root", data_root.wstring(), L"--packages-install",
                                      L"fcitx5-rime"},
                            install_output);
    expect(install_exit == 0,
           "package install must succeed when the launcher service is not running: " +
               install_output);
    const auto installed = fcitx::package::read_lockfile(data_root / L"packages");
    expect(installed.size() == 1U && installed.front().id == "fcitx5-rime" &&
               installed.front().version == "1.0.0" && installed.front().state == "installed",
           "control install did not publish the expected active package");
    expect(fs::is_regular_file(data_root /
                               L"packages/versions/fcitx5-rime/1.0.0/bin/addon.dll"),
           "control install did not activate the verified payload");
    std::string detail;
    const DWORD detail_exit =
        run_process_capture(control, {L"--data-root", data_root.wstring(), L"--packages-detail",
                                      L"fcitx5-rime"},
                            detail);
    expect(detail_exit == 0, "package detail must succeed for an installed package");
    expect(detail.find("\"permissions\":[\"native-code\",\"input-data\"]") !=
               std::string::npos &&
               detail.find("\"source_commit\":\"0123456789abcdef\"") != std::string::npos &&
               detail.find("\"kind\":\"fcitx-addon\"") != std::string::npos,
           "package detail did not expose generic addon config surface: " + detail);

    const DWORD disable_exit =
        run_process(control, {L"--data-root", data_root.wstring(), L"--packages-state",
                              L"fcitx5-rime", L"disabled"});
    expect(disable_exit == 0, "package disable must succeed with no launcher service");
    expect(fcitx::package::read_lockfile(data_root / L"packages").front().state == "disabled",
           "control disable did not persist the disabled state");

    const DWORD enable_exit =
        run_process(control, {L"--data-root", data_root.wstring(), L"--packages-state",
                              L"fcitx5-rime", L"enabled"});
    expect(enable_exit == 0, "package enable must succeed with no launcher service");
    expect(fcitx::package::read_lockfile(data_root / L"packages").front().state == "enabled",
           "control enable did not persist the enabled state");

    constexpr std::string_view updated_payload = "verified control package fixture v2\n";
    publish_package_fixture(temporary.path(), data_root, signer, "1.1.0", 2U,
                            updated_payload);
    const DWORD update_exit =
        run_process(control, {L"--data-root", data_root.wstring(), L"--packages-update",
                              L"fcitx5-rime"});
    expect(update_exit == 0, "package update must succeed with no launcher service");
    const auto updated = fcitx::package::read_lockfile(data_root / L"packages");
    expect(updated.size() == 1U && updated.front().version == "1.1.0" &&
               updated.front().state == "installed",
           "control update did not atomically publish the new version");
    expect(read_text(data_root /
                     L"packages/versions/fcitx5-rime/1.1.0/bin/addon.dll") ==
               updated_payload,
           "control update did not activate the new verified payload");

    const DWORD repair_exit =
        run_process(control,
                    {L"--data-root", data_root.wstring(), L"--packages-repair"});
    expect(repair_exit == 0, "package repair did not verify the installed package set");

    const auto user_data = data_root / L"rime/user.dict.yaml";
    write_text(user_data, "irreplaceable user dictionary\n");
    const DWORD remove_exit =
        run_process(control, {L"--data-root", data_root.wstring(), L"--packages-remove",
                              L"fcitx5-rime"});
    expect(remove_exit == 0, "package removal must succeed with no launcher service");
    expect(fcitx::package::read_lockfile(data_root / L"packages").empty(),
           "control removal did not remove the lockfile entry");
    expect(!fs::exists(data_root / L"packages/versions/fcitx5-rime"),
           "control removal left executable package payload behind");
    expect(read_text(user_data) == "irreplaceable user dictionary\n",
           "control removal modified package-owned user data");

    std::cout << "control package lifecycle with stopped service passed\n";
    return 0;
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
