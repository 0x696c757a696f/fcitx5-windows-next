#include <windows.h>
#include <wincrypt.h>

#include <array>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
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

std::string read_file(const std::filesystem::path& path) {
  std::error_code error;
  const auto size = std::filesystem::file_size(path, error);
  if (error || size > 1024U * 1024U) {
    throw std::runtime_error("input is missing or too large");
  }
  std::ifstream input(path, std::ios::binary);
  std::string result(static_cast<std::size_t>(size), '\0');
  if (!result.empty()) {
    input.read(result.data(), static_cast<std::streamsize>(result.size()));
  }
  if (!input) throw std::runtime_error("input read failed");
  return result;
}

void write_file(const std::filesystem::path& path, std::string_view bytes) {
  std::filesystem::create_directories(path.parent_path());
  std::ofstream output(path, std::ios::binary | std::ios::trunc);
  output.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
  if (!output) throw std::runtime_error("output write failed");
}

std::string base64(std::span<const std::byte> value) {
  DWORD size = 0;
  if (!CryptBinaryToStringA(reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, nullptr, &size) ||
      size == 0U) {
    throw std::runtime_error("base64 sizing failed");
  }
  std::string result(size, '\0');
  if (!CryptBinaryToStringA(reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, result.data(), &size)) {
    throw std::runtime_error("base64 encoding failed");
  }
  if (!result.empty() && result.back() == '\0') result.pop_back();
  return result;
}

std::string object_name(std::string_view kind) {
  if (kind == "repository-index") return "repository-index";
  if (kind == "package-manifest") return "package-manifest";
  throw std::runtime_error("object type must be repository-index or package-manifest");
}

struct FixtureKey {
  std::vector<std::uint8_t> public_key = std::vector<std::uint8_t>(MLDSA65_PUBLICKEYBYTES);
  std::vector<std::uint8_t> secret_key = std::vector<std::uint8_t>(MLDSA65_SECRETKEYBYTES);
};

FixtureKey deterministic_key() {
  FixtureKey key;
  std::array<std::uint8_t, MLDSA65_SEEDBYTES> seed{};
  seed.fill(0x5AU);
  if (fcitx5_mldsa65_test_keypair_internal(key.public_key.data(), key.secret_key.data(),
                                           seed.data()) != 0) {
    throw std::runtime_error("ML-DSA fixture key generation failed");
  }
  return key;
}

std::vector<std::byte> sign(std::string_view bytes, const FixtureKey& key) {
  std::vector<std::byte> signature(MLDSA65_BYTES);
  std::array<std::uint8_t, MLDSA65_RNDBYTES> randomness{};
  std::array<std::uint8_t, MLD_DOMAIN_SEPARATION_MAX_BYTES> prefix{};
  randomness.fill(0xA5U);
  const auto prefix_size = fcitx5_mldsa65_test_prepare_domain_separation_prefix(
      prefix.data(), nullptr, 0U, nullptr, 0U, MLD_PREHASH_NONE);
  if (prefix_size == 0U) {
    throw std::runtime_error("ML-DSA fixture prefix generation failed");
  }
  if (fcitx5_mldsa65_test_signature_internal(
          reinterpret_cast<std::uint8_t*>(signature.data()),
          reinterpret_cast<const std::uint8_t*>(bytes.data()), bytes.size(), prefix.data(),
          prefix_size, randomness.data(), key.secret_key.data(), 0) != 0) {
    throw std::runtime_error("ML-DSA fixture signing failed");
  }
  return signature;
}

int usage() {
  std::wcerr << L"Usage:\n"
                L"  fcitx5-pqc-fixture-signer --sign repository-index|package-manifest "
                L"INPUT_JSON SIG_JSON KEYRING_JSON [KEY_ID]\n";
  return 1;
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
  try {
    if (argc != 6 && argc != 7) return usage();
    if (std::wstring_view(argv[1]) != L"--sign") return usage();
    const auto kind = object_name(std::filesystem::path(argv[2]).string());
    const auto input = read_file(argv[3]);
    const auto key_id = argc == 7 ? std::filesystem::path(argv[6]).string()
                                  : std::string("official-test-2026-mldsa65");
    if (key_id.empty() || key_id.find_first_not_of("abcdefghijklmnopqrstuvwxyz0123456789-_.") !=
                              std::string::npos) {
      throw std::runtime_error("key id is invalid");
    }
    const auto key = deterministic_key();
    const auto signature = sign(input, key);
    const std::string signature_envelope =
        "{\n"
        "  \"format_version\": 2,\n"
        "  \"signed_object\": \"" +
        kind +
        "\",\n"
        "  \"canonicalization\": \"fcitx5-windows-next-json-v1\",\n"
        "  \"signatures\": [\n"
        "    {\"key_id\":\"" +
        key_id +
        "\",\"algorithm\":\"mldsa65\",\"signature_base64\":\"" +
        base64(signature) +
        "\"}\n"
        "  ]\n"
        "}\n";
    const std::span<const std::byte> public_bytes(
        reinterpret_cast<const std::byte*>(key.public_key.data()), key.public_key.size());
    const std::string keyring =
        "{\n"
        "  \"format_version\": 2,\n"
        "  \"policy\": {\"official_required_signatures\":[\"mldsa65\"],"
        "\"compatibility_hashes\":[\"sha256\"],\"default_payload_hash\":\"blake3\"},\n"
        "  \"keys\": [\n"
        "    {\"key_id\":\"" +
        key_id +
        "\",\"algorithm\":\"mldsa65\",\"status\":\"trusted\",\"public_key_base64\":\"" +
        base64(public_bytes) +
        "\",\"scope\":[\"repository\",\"package\"],\"channels\":[\"stable\"]}\n"
        "  ]\n"
        "}\n";
    write_file(argv[4], signature_envelope);
    write_file(argv[5], keyring);
    std::cout << "signed_object=" << kind << "\nkey_id=" << key_id << '\n';
    return 0;
  } catch (const std::exception& error) {
    std::cerr << "fixture_sign_failed: " << error.what() << '\n';
    return 2;
  }
}
