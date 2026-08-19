// Repository channel binding + anti-rollback integration test against the
// real fcitx5-control.exe:
//  - a stable build rejects a signed beta index (channel binding)
//  - a signed index whose release_sequence is lower than the highest
//    previously accepted for the channel is rejected (rollback_rejected)
//  - a newer sequence is accepted and advances the accepted maximum
#include "package_core.h"

#include <windows.h>
#include <bcrypt.h>
#include <wincrypt.h>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <span>
#include <string>
#include <string_view>
#include <vector>

namespace {

namespace fs = std::filesystem;

std::string base64(std::span<const std::byte> value) {
  DWORD size = 0;
  if (!CryptBinaryToStringA(reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, nullptr, &size)) {
    throw std::runtime_error("base64 sizing failed");
  }
  std::string result(size, '\0');
  if (!CryptBinaryToStringA(reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, result.data(), &size)) {
    throw std::runtime_error("base64 encoding failed");
  }
  result.resize(size);
  return result;
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
    if (key_ != nullptr) BCryptDestroyKey(key_);
    if (algorithm_ != nullptr) BCryptCloseAlgorithmProvider(algorithm_, 0);
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

void write_bytes(const fs::path& path, std::string_view bytes) {
  fs::create_directories(path.parent_path());
  std::ofstream output(path, std::ios::binary);
  output.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
  if (!output) throw std::runtime_error("fixture write failed");
}

std::string index_json(std::string_view channel, std::uint64_t sequence) {
  return std::string("{\"format_version\":1,\"channel\":\"") + std::string(channel) +
         "\",\"generated_at\":\"2026-08-17T00:00:00Z\",\"key_id\":\"release-2026\","
         "\"packages\":[{\"id\":\"fcitx5-rime\",\"title\":\"Rime\","
         "\"summary\":\"Rime input engine\",\"version\":\"1.0.0\","
         "\"release_sequence\":" + std::to_string(sequence) +
         ",\"type\":\"addon\",\"architecture\":\"any\","
         "\"download_url\":\"https://packages.example.invalid/fcitx5-rime.fcpkg\","
         "\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\","
         "\"dependencies\":[]}]}";
}

DWORD run_control(const fs::path& control, const fs::path& dataRoot, std::string& output) {
  std::wstring command = L"\"" + control.wstring() + L"\" --data-root \"" + dataRoot.wstring() +
                         L"\" --packages-list";
  std::vector<wchar_t> mutableCommand(command.begin(), command.end());
  mutableCommand.push_back(L'\0');
  SECURITY_ATTRIBUTES attributes{sizeof(attributes), nullptr, TRUE};
  HANDLE readPipe = nullptr;
  HANDLE writePipe = nullptr;
  if (!CreatePipe(&readPipe, &writePipe, &attributes, 0)) return 0xFFFFFFFFU;
  SetHandleInformation(readPipe, HANDLE_FLAG_INHERIT, 0);
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  startup.dwFlags = STARTF_USESTDHANDLES;
  startup.hStdOutput = writePipe;
  PROCESS_INFORMATION process{};
  const BOOL created = CreateProcessW(control.c_str(), mutableCommand.data(), nullptr, nullptr,
                                      TRUE, CREATE_NO_WINDOW, nullptr, nullptr, &startup,
                                      &process);
  CloseHandle(writePipe);
  if (!created) {
    CloseHandle(readPipe);
    return 0xFFFFFFFFU;
  }
  std::array<char, 2048> buffer{};
  DWORD count = 0;
  while (ReadFile(readPipe, buffer.data(), static_cast<DWORD>(buffer.size()), &count, nullptr) &&
         count != 0)
    output.append(buffer.data(), count);
  CloseHandle(readPipe);
  CloseHandle(process.hThread);
  WaitForSingleObject(process.hProcess, 30'000);
  DWORD exitCode = 1;
  GetExitCodeProcess(process.hProcess, &exitCode);
  CloseHandle(process.hProcess);
  return exitCode;
}

bool repository_available(std::string_view output) {
  return output.find("repository_available\":true") != std::string_view::npos;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
  if (argc != 2) {
    std::cerr << "expected control executable path\n";
    return 1;
  }
  const fs::path control = argv[1];
  const fs::path installRoot = control.parent_path();
  const fs::path keyringPath = installRoot / "security" / "trusted-keys.json";
  const fs::path dataRoot =
      fs::temp_directory_path() /
      (L"control-repo-rollback-" + std::to_wstring(GetCurrentProcessId()));
  std::error_code ignored;
  fs::remove_all(dataRoot, ignored);

  try {
    SigningFixture signer;
    const auto keyring =
        "{\n  \"format_version\": 1,\n  \"keys\": [\n"
        "    {\"key_id\":\"release-2026\",\"algorithm\":\"rsa-2048-sha256\","
        "\"status\":\"trusted\",\"public_key_base64\":\"" +
        base64(signer.public_blob()) + "\"}\n  ]\n}\n";
    write_bytes(keyringPath, keyring);

    // Channel binding: a stable build must refuse a signed beta index.
    const auto beta_index = index_json("beta", 10U);
    write_bytes(dataRoot / "repository/index.json", beta_index);
    const auto beta_sig = signer.sign(beta_index);
    write_bytes(dataRoot / "repository/index.sig",
                std::string_view(reinterpret_cast<const char*>(beta_sig.data()),
                                 beta_sig.size()));
    std::string output;
    (void)run_control(control, dataRoot, output);
    if (repository_available(output)) {
      std::cerr << "beta index was accepted by a stable build\n";
      return 1;
    }

    // Anti-rollback: sequence 3 is older than the accepted maximum 5.
    const auto old_index = index_json("stable", 3U);
    write_bytes(dataRoot / "repository/index.json", old_index);
    const auto old_sig = signer.sign(old_index);
    write_bytes(dataRoot / "repository/index.sig",
                std::string_view(reinterpret_cast<const char*>(old_sig.data()),
                                 old_sig.size()));
    write_bytes(dataRoot / "repository/sequence-stable.json",
                "format_version=1\nchannel=stable\nmax_release_sequence=5\n");
    output.clear();
    (void)run_control(control, dataRoot, output);
    if (repository_available(output)) {
      std::cerr << "stale repository index (sequence 3 < accepted 5) was accepted\n";
      return 1;
    }

    // A newer sequence is accepted.
    const auto fresh_index = index_json("stable", 6U);
    write_bytes(dataRoot / "repository/index.json", fresh_index);
    const auto fresh_sig = signer.sign(fresh_index);
    write_bytes(dataRoot / "repository/index.sig",
                std::string_view(reinterpret_cast<const char*>(fresh_sig.data()),
                                 fresh_sig.size()));
    write_bytes(dataRoot / "repository/sequence-stable.json",
                "format_version=1\nchannel=stable\nmax_release_sequence=6\n");
    output.clear();
    (void)run_control(control, dataRoot, output);
    if (!repository_available(output)) {
      std::cerr << "fresh repository index (sequence 6) was rejected\n";
      return 1;
    }
  } catch (const std::exception& error) {
    std::cerr << "repository rollback test threw: " << error.what() << '\n';
    fs::remove_all(keyringPath, ignored);
    fs::remove_all(dataRoot, ignored);
    return 1;
  }

  fs::remove_all(keyringPath, ignored);
  fs::remove_all(dataRoot, ignored);
  std::cout << "control-repository-rollback ok\n";
  return 0;
}
