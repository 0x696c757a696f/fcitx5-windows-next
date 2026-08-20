#include "package_core.h"

#include <windows.h>
#include <bcrypt.h>
#include <wincrypt.h>

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

class SigningFixture final {
 public:
  SigningFixture() {
    if (BCryptOpenAlgorithmProvider(&algorithm_, BCRYPT_RSA_ALGORITHM, nullptr, 0) < 0 ||
        BCryptGenerateKeyPair(algorithm_, &key_, 2048U, 0) < 0 ||
        BCryptFinalizeKeyPair(key_, 0) < 0) {
      throw std::runtime_error("RSA fixture generation failed");
    }
  }

  ~SigningFixture() {
    if (key_ != nullptr) {
      BCryptDestroyKey(key_);
    }
    if (algorithm_ != nullptr) {
      BCryptCloseAlgorithmProvider(algorithm_, 0);
    }
  }

  SigningFixture(const SigningFixture&) = delete;
  SigningFixture& operator=(const SigningFixture&) = delete;

  [[nodiscard]] std::vector<std::byte> public_blob() const {
    ULONG size = 0;
    if (BCryptExportKey(key_, nullptr, BCRYPT_RSAPUBLIC_BLOB, nullptr, 0, &size, 0) < 0) {
      throw std::runtime_error("RSA public key sizing failed");
    }
    std::vector<std::byte> result(size);
    if (BCryptExportKey(key_, nullptr, BCRYPT_RSAPUBLIC_BLOB,
                        reinterpret_cast<PUCHAR>(result.data()), size, &size, 0) < 0) {
      throw std::runtime_error("RSA public key export failed");
    }
    result.resize(size);
    return result;
  }

  [[nodiscard]] std::vector<std::byte> sign(std::string_view bytes) const {
    const auto digest = fcitx::package::sha256(std::as_bytes(std::span(bytes)));
    BCRYPT_PKCS1_PADDING_INFO padding{BCRYPT_SHA256_ALGORITHM};
    ULONG size = 0;
    if (BCryptSignHash(key_, &padding,
                       reinterpret_cast<PUCHAR>(const_cast<std::byte*>(digest.data())),
                       static_cast<ULONG>(digest.size()), nullptr, 0, &size,
                       BCRYPT_PAD_PKCS1) < 0) {
      throw std::runtime_error("RSA signature sizing failed");
    }
    std::vector<std::byte> result(size);
    if (BCryptSignHash(key_, &padding,
                       reinterpret_cast<PUCHAR>(const_cast<std::byte*>(digest.data())),
                       static_cast<ULONG>(digest.size()),
                       reinterpret_cast<PUCHAR>(result.data()), size, &size,
                       BCRYPT_PAD_PKCS1) < 0) {
      throw std::runtime_error("RSA signing failed");
    }
    result.resize(size);
    return result;
  }

 private:
  BCRYPT_ALG_HANDLE algorithm_{};
  BCRYPT_KEY_HANDLE key_{};
};

std::string base64(std::span<const std::byte> value) {
  DWORD size = 0;
  if (!CryptBinaryToStringA(reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, nullptr, &size) ||
      size == 0U) {
    throw std::runtime_error("base64 fixture sizing failed");
  }
  std::string result(size, '\0');
  if (!CryptBinaryToStringA(reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, result.data(), &size)) {
    throw std::runtime_error("base64 fixture encoding failed");
  }
  if (!result.empty() && result.back() == '\0') {
    result.pop_back();
  }
  return result;
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

std::string make_manifest(std::string_view version, std::string_view file_hash,
                          std::uint64_t file_size) {
  return "{\n"
         "  \"format_version\": 1,\n"
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
         "  \"permissions\": [\"native-code\", \"input-data\"],\n"
         "  \"files\": [{\"path\": \"bin/addon.dll\", \"size\": " +
         std::to_string(file_size) + ", \"sha256\": \"" + std::string(file_hash) +
         "\"}],\n"
         "  \"key_id\": \"release-2026\"\n"
         "}\n";
}

std::string make_repository(std::string_view version, std::uint64_t release_sequence,
                            std::string_view archive_hash) {
  return "{\"format_version\":1,\"channel\":\"stable\","
         "\"generated_at\":\"2026-08-18T00:00:00Z\",\"key_id\":\"release-2026\","
         "\"packages\":[{\"id\":\"fcitx5-rime\",\"title\":\"Rime\","
         "\"summary\":\"Rime input engine\",\"version\":\"" +
         std::string(version) +
         "\",\"release_sequence\":" +
         std::to_string(release_sequence) +
         ",\"type\":\"addon\",\"architecture\":\"" + architecture() +
         "\",\"download_url\":\"https://packages.example.invalid/fcitx5-rime.fcpkg\","
         "\"sha256\":\"" +
         std::string(archive_hash) + "\",\"dependencies\":[]}]}";
}

void publish_package_fixture(const fs::path& fixture_root, const fs::path& data_root,
                             const SigningFixture& signer, std::string_view version,
                             std::uint64_t release_sequence, std::string_view payload) {
  const auto payload_hash = fcitx::package::hex_sha256(
      fcitx::package::sha256(std::as_bytes(std::span(payload))));
  const auto manifest = make_manifest(version, payload_hash, payload.size());
  const std::wstring wide_version(version.begin(), version.end());
  const auto archive = fixture_root / (L"fcitx5-rime-" + wide_version + L".fcpkg");
  create_archive(archive,
                 {{"manifest.json", as_bytes(manifest)},
                  {"manifest.sig", signer.sign(manifest)},
                  {"payload/bin/addon.dll", as_bytes(payload)}});
  const auto archive_hash =
      fcitx::package::hex_sha256(fcitx::package::sha256_file(archive));
  const auto repository = make_repository(version, release_sequence, archive_hash);
  write_text(data_root / L"repository/index.json", repository);
  write_bytes(data_root / L"repository/index.sig", signer.sign(repository));
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
    expect(argc == 3, "expected control and downloader executable paths");
    const fs::path control_source = argv[1];
    const fs::path downloader_source = argv[2];
    expect(fs::is_regular_file(control_source), "control executable is missing");
    expect(fs::is_regular_file(downloader_source), "downloader executable is missing");

    TemporaryDirectory temporary;
    const auto app_root = temporary.path() / L"app";
    const auto app_bin = app_root / L"bin";
    const auto data_root = temporary.path() / L"data";
    fs::create_directories(app_bin);
    const auto control = app_bin / L"fcitx5-control.exe";
    fs::copy_file(control_source, control, fs::copy_options::overwrite_existing);
    fs::copy_file(downloader_source, app_bin / L"fcitx5-downloader.exe",
                  fs::copy_options::overwrite_existing);
    expect(!fs::exists(app_bin / L"fcitx5-launcher.exe"),
           "fixture must exercise package management with no launcher service");

    SigningFixture signer;
    const auto public_key = signer.public_blob();
    const auto keyring =
        "{\n  \"format_version\": 1,\n  \"keys\": [\n"
        "    {\"key_id\":\"release-2026\",\"algorithm\":\"rsa-2048-sha256\"," 
        "\"status\":\"trusted\",\"public_key_base64\":\"" +
        base64(public_key) + "\"}\n  ]\n}\n";
    write_text(app_root / L"security/trusted-keys.json", keyring);

    constexpr std::string_view initial_payload = "verified control package fixture v1\n";
    publish_package_fixture(temporary.path(), data_root, signer, "1.0.0", 1U,
                            initial_payload);

    const DWORD install_exit =
        run_process(control, {L"--data-root", data_root.wstring(), L"--packages-install",
                              L"fcitx5-rime"});
    expect(install_exit == 0,
           "package install must succeed when the launcher service is not running");
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
