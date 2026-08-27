// Repository channel binding + anti-rollback integration test against the
// real fcitx5-control.exe:
//  - a stable build rejects a signed beta index (channel binding)
//  - a signed index whose release_sequence is lower than the highest
//    previously accepted for the channel is rejected (rollback_rejected)
//  - a newer sequence is accepted and advances the accepted maximum
#include "package_core.h"

#include <windows.h>
#include <wincrypt.h>

#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <span>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

#define MLD_CONFIG_FILE "fcitx5_mldsa65_test_config.h"
extern "C" {
#include <mldsa/mldsa_native.h>
}
#undef MLD_CONFIG_FILE

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

class MldsaSigningFixture final {
 public:
  MldsaSigningFixture() {
    std::array<std::uint8_t, MLDSA65_SEEDBYTES> seed{};
    seed.fill(0x5AU);
    if (fcitx5_mldsa65_test_keypair_internal(public_key_.data(), secret_key_.data(),
                                             seed.data()) != 0) {
      throw std::runtime_error("ML-DSA fixture key generation failed");
    }
    public_key_bytes_.assign(reinterpret_cast<const std::byte*>(public_key_.data()),
                             reinterpret_cast<const std::byte*>(public_key_.data() +
                                                                public_key_.size()));
  }
  MldsaSigningFixture(const MldsaSigningFixture&) = delete;
  MldsaSigningFixture& operator=(const MldsaSigningFixture&) = delete;

  [[nodiscard]] const std::vector<std::byte>& public_key() const noexcept {
    return public_key_bytes_;
  }

  [[nodiscard]] std::vector<std::byte> sign(std::string_view bytes) const {
    std::vector<std::byte> result(MLDSA65_BYTES);
    std::array<std::uint8_t, MLDSA65_RNDBYTES> randomness{};
    std::array<std::uint8_t, MLD_DOMAIN_SEPARATION_MAX_BYTES> prefix{};
    const auto prefix_size = fcitx5_mldsa65_test_prepare_domain_separation_prefix(
        prefix.data(), nullptr, 0U, nullptr, 0U, MLD_PREHASH_NONE);
    if (prefix_size == 0U) {
      throw std::runtime_error("ML-DSA fixture prefix preparation failed");
    }
    randomness.fill(0x7BU);
    if (fcitx5_mldsa65_test_signature_internal(
            reinterpret_cast<std::uint8_t*>(result.data()),
            reinterpret_cast<const std::uint8_t*>(bytes.data()), bytes.size(), prefix.data(),
            prefix_size, randomness.data(), secret_key_.data(), 0) != 0) {
      throw std::runtime_error("ML-DSA fixture signing failed");
    }
    return result;
  }

 private:
  std::vector<std::uint8_t> public_key_ = std::vector<std::uint8_t>(MLDSA65_PUBLICKEYBYTES);
  std::vector<std::uint8_t> secret_key_ = std::vector<std::uint8_t>(MLDSA65_SECRETKEYBYTES);
  std::vector<std::byte> public_key_bytes_;
};

void write_bytes(const fs::path& path, std::string_view bytes) {
  fs::create_directories(path.parent_path());
  std::ofstream output(path, std::ios::binary);
  output.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
  if (!output) throw std::runtime_error("fixture write failed");
}

std::string index_json(std::string_view channel, std::uint64_t sequence) {
  return std::string("{\"format_version\":1,\"channel\":\"") + std::string(channel) +
         "\",\"generated_at\":\"2026-08-17T00:00:00Z\",\"key_id\":\"official-test-2026-mldsa65\","
         "\"packages\":[{\"id\":\"fcitx5-rime\",\"title\":\"Rime\","
         "\"summary\":\"Rime input engine\",\"version\":\"1.0.0\","
         "\"release_sequence\":" + std::to_string(sequence) +
         ",\"type\":\"addon\",\"architecture\":\"any\","
         "\"download_url\":\"https://packages.example.invalid/fcitx5-rime.fcpkg\","
         "\"sha256\":\"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef\","
         "\"dependencies\":[]}]}";
}

DWORD run_control(const fs::path& control, const fs::path& dataRoot,
                  std::span<const std::wstring> arguments, std::string& output) {
  std::wstring command = L"\"" + control.wstring() + L"\" --data-root \"" + dataRoot.wstring() +
                         L"\"";
  for (const auto& argument : arguments) {
    command += L" ";
    command += argument;
  }
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

DWORD run_packages_list(const fs::path& control, const fs::path& dataRoot, std::string& output) {
  const std::array arguments{std::wstring(L"--packages-list")};
  return run_control(control, dataRoot, arguments, output);
}

DWORD run_packages_repair(const fs::path& control, const fs::path& dataRoot, std::string& output) {
  const std::array arguments{std::wstring(L"--packages-repair")};
  return run_control(control, dataRoot, arguments, output);
}

bool repository_available(std::string_view output) {
  return output.find("repository_available\":true") != std::string_view::npos;
}

std::string signature_envelope(std::span<const std::byte> signature) {
  return "{\"format_version\":2,\"signed_object\":\"repository-index\","
         "\"canonicalization\":\"fcitx5-windows-next-json-v1\",\"signatures\":[{"
         "\"key_id\":\"official-test-2026-mldsa65\",\"algorithm\":\"mldsa65\","
         "\"signature_base64\":\"" +
         base64(signature) + "\"}]}";
}

void write_repository(const fs::path& dataRoot, const MldsaSigningFixture& signer,
                      std::string_view channel, std::uint64_t sequence) {
  const auto index = index_json(channel, sequence);
  write_bytes(dataRoot / "repository/index.json", index);
  const auto signature = signer.sign(index);
  write_bytes(dataRoot / "repository/index.sig.json", signature_envelope(signature));
}

void write_sequence(const fs::path& dataRoot, std::string_view text) {
  write_bytes(dataRoot / "repository/sequence-stable.json", text);
}

bool sequence_contains(const fs::path& dataRoot, std::string_view text) {
  std::ifstream input(dataRoot / "repository/sequence-stable.json", std::ios::binary);
  const std::string bytes{std::istreambuf_iterator<char>(input), {}};
  return bytes.find(text) != std::string::npos;
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
    MldsaSigningFixture signer;
    const auto keyring =
        "{\n  \"format_version\": 2,\n  \"policy\": {"
        "\"official_required_signatures\":[\"mldsa65\"],"
        "\"compatibility_hashes\":[\"sha256\"],\"default_payload_hash\":\"blake3\"},"
        "\n  \"keys\": [\n    {\"key_id\":\"official-test-2026-mldsa65\","
        "\"algorithm\":\"mldsa65\",\"status\":\"trusted\",\"public_key_base64\":\"" +
        base64(signer.public_key()) +
        "\",\"scope\":[\"repository\",\"package\"],\"channels\":[\"stable\"]}\n  ]\n}\n";
    write_bytes(keyringPath, keyring);

    // First-run with no repository cache and no sequence state is allowed to
    // report offline/unavailable rather than fail as corrupt established state.
    std::string output;
    if (run_packages_list(control, dataRoot, output) != 0 || repository_available(output)) {
      std::cerr << "empty first-run repository state did not report cleanly unavailable\n";
      return 1;
    }

    // Channel binding: a stable build must refuse a signed beta index.
    write_repository(dataRoot, signer, "beta", 10U);
    output.clear();
    (void)run_packages_list(control, dataRoot, output);
    if (repository_available(output)) {
      std::cerr << "beta index was accepted by a stable build\n";
      return 1;
    }

    // Explicit repair/reset drops an invalid cached repository and returns to
    // the first-run unavailable state instead of accepting the wrong channel.
    output.clear();
    if (run_packages_repair(control, dataRoot, output) != 0 ||
        output.find("\"repository_sequence_state\":\"reset\"") == std::string::npos) {
      std::cerr << "explicit repair did not reset invalid repository state\n";
      return 1;
    }
    output.clear();
    if (run_packages_list(control, dataRoot, output) != 0 || repository_available(output)) {
      std::cerr << "reset invalid repository cache was still available\n";
      return 1;
    }

    // Explicit repair can also rebuild missing sequence state from a valid,
    // signed, channel-bound cache.
    write_repository(dataRoot, signer, "stable", 8U);
    output.clear();
    if (run_packages_list(control, dataRoot, output) != 0 || repository_available(output)) {
      std::cerr << "established cache with missing sequence state was accepted before repair\n";
      return 1;
    }
    output.clear();
    if (run_packages_repair(control, dataRoot, output) != 0 ||
        output.find("\"repository_sequence_state\":\"repaired\"") == std::string::npos ||
        !sequence_contains(dataRoot, "max_release_sequence=8\n")) {
      std::cerr << "explicit repair did not rebuild accepted sequence state\n";
      return 1;
    }

    // Interrupted atomic write simulation: an orphaned temporary file from an
    // incomplete publication must not replace or poison the committed state.
    write_bytes(dataRoot / "repository/sequence-stable.json.new",
                "format_version=1\nchannel=stable\nmax_release_sequence=0\n");
    output.clear();
    if (run_packages_list(control, dataRoot, output) != 0 || !repository_available(output)) {
      std::cerr << "orphaned sequence temp file affected committed state\n";
      return 1;
    }

    // Anti-rollback: sequence 3 is older than the accepted maximum 8.
    write_repository(dataRoot, signer, "stable", 3U);
    write_sequence(dataRoot, "format_version=1\nchannel=stable\nmax_release_sequence=8\n");
    output.clear();
    (void)run_packages_list(control, dataRoot, output);
    if (repository_available(output)) {
      std::cerr << "stale repository index (sequence 3 < accepted 8) was accepted\n";
      return 1;
    }

    // Once a high sequence has been accepted, deleting the sequence state must
    // not silently turn the stale cache into a first-run sequence zero.
    fs::remove(dataRoot / "repository/sequence-stable.json", ignored);
    output.clear();
    (void)run_packages_list(control, dataRoot, output);
    if (repository_available(output)) {
      std::cerr << "repository cache with missing sequence state was accepted\n";
      return 1;
    }

    // Truncated sequence state also fails closed.
    write_sequence(dataRoot, "format_version=1\nchannel=stable\nmax_release_sequence=");
    output.clear();
    (void)run_packages_list(control, dataRoot, output);
    if (repository_available(output)) {
      std::cerr << "repository cache with truncated sequence state was accepted\n";
      return 1;
    }

    // Corrupt sequence state also fails closed.
    write_sequence(dataRoot, "format_version=1\nchannel=stable\nmax_release_sequence=not-a-number\n");
    output.clear();
    (void)run_packages_list(control, dataRoot, output);
    if (repository_available(output)) {
      std::cerr << "repository cache with corrupt sequence state was accepted\n";
      return 1;
    }

    // A valid newer cache can be explicitly repaired after corruption.
    write_repository(dataRoot, signer, "stable", 9U);
    output.clear();
    if (run_packages_repair(control, dataRoot, output) != 0 ||
        output.find("\"repository_sequence_state\":\"repaired\"") == std::string::npos ||
        !sequence_contains(dataRoot, "max_release_sequence=9\n")) {
      std::cerr << "explicit repair did not recover from corrupt sequence state\n";
      return 1;
    }

    // A newer sequence is accepted after state repair.
    output.clear();
    (void)run_packages_list(control, dataRoot, output);
    if (!repository_available(output)) {
      std::cerr << "fresh repository index (sequence 9) was rejected\n";
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
