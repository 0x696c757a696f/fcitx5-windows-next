#include "package_core.h"

#include <windows.h>
#include <bcrypt.h>
#include <wincrypt.h>

#include <filesystem>
#include <fstream>
#include <cstring>
#include <iostream>
#include <iterator>
#include <span>
#include <string>
#include <utility>
#include <vector>

#define MLD_CONFIG_FILE "fcitx5_mldsa65_test_config.h"
extern "C" {
#include <mldsa/mldsa_native.h>
}
#undef MLD_CONFIG_FILE

#include <miniz.h>
#include <nlohmann/json.hpp>

namespace {

class TemporaryDirectory final {
 public:
  TemporaryDirectory() {
    path_ = std::filesystem::temp_directory_path() /
            (L"fcitx5-package-test-" + std::to_wstring(GetCurrentProcessId()));
    std::error_code ignored;
    std::filesystem::remove_all(path_, ignored);
    std::filesystem::create_directories(path_);
  }
  ~TemporaryDirectory() {
    std::error_code ignored;
    std::filesystem::remove_all(path_, ignored);
  }
  [[nodiscard]] const std::filesystem::path& path() const noexcept { return path_; }

 private:
  std::filesystem::path path_;
};

void expect(bool condition, const char* message) {
  if (!condition) {
    throw std::runtime_error(message);
  }
}

template <typename Callable>
void expect_error(std::string_view code, Callable&& callable) {
  try {
    callable();
  } catch (const fcitx::package::PackageError& error) {
    expect(error.code() == code, "unexpected package error code");
    return;
  }
  throw std::runtime_error("expected PackageError");
}

std::string make_manifest(std::string_view id, std::string_view version,
                          std::string_view file_hash, std::uint64_t file_size,
                          std::string_view dependencies = "[]") {
  return "{\n"
         "  \"format_version\": 1,\n"
         "  \"id\": \"" +
         std::string(id) +
         "\",\n"
         "  \"version\": \"" +
         std::string(version) +
         "\",\n"
         "  \"type\": \"addon\",\n"
#if defined(_WIN64)
         "  \"architecture\": \"x64\",\n"
#else
         "  \"architecture\": \"x86\",\n"
#endif
         "  \"min_os\": \"6.1-sp1\",\n"
         "  \"core_api\": \"1\",\n"
         "  \"addon_abi\": \"1\",\n"
         "  \"dependencies\": " +
         std::string(dependencies) +
         ",\n"
         "  \"license\": \"MIT\",\n"
         "  \"source_commit\": \"0123456789abcdef\",\n"
         "  \"permissions\": [\"native-code\", \"input-data\"],\n"
         "  \"files\": [{\"path\": \"bin/addon.dll\", \"size\": " +
         std::to_string(file_size) + ", \"sha256\": \"" + std::string(file_hash) +
         "\"}],\n"
         "  \"key_id\": \"release-2026\"\n"
         "}\n";
}

std::string make_manifest_v2(std::string_view id, std::string_view version,
                             std::string_view blake3_hash, std::uint64_t file_size,
                             std::string_view key_id,
                             std::string_view sha256_hash = {}) {
  std::string hashes =
      "{\"blake3\":\"" + std::string(blake3_hash) + "\"";
  if (!sha256_hash.empty()) {
    hashes += ",\"sha256\":\"" + std::string(sha256_hash) + "\"";
  }
  hashes += "}";
  return "{\n"
         "  \"format_version\": 2,\n"
         "  \"id\": \"" +
         std::string(id) +
         "\",\n"
         "  \"version\": \"" +
         std::string(version) +
         "\",\n"
         "  \"type\": \"addon\",\n"
#if defined(_WIN64)
         "  \"architecture\": \"x64\",\n"
#else
         "  \"architecture\": \"x86\",\n"
#endif
         "  \"min_os\": \"6.1-sp1\",\n"
         "  \"core_api\": \"1\",\n"
         "  \"addon_abi\": \"1\",\n"
         "  \"dependencies\": [],\n"
         "  \"license\": \"MIT\",\n"
         "  \"source_commit\": \"0123456789abcdef\",\n"
         "  \"permissions\": [\"native-code\", \"input-data\"],\n"
         "  \"payload\": [{\"path\": \"bin/addon.dll\", \"size\": " +
         std::to_string(file_size) + ", \"hashes\": " + hashes +
         "}],\n"
         "  \"key_id\": \"" + std::string(key_id) + "\"\n"
         "}\n";
}

void write_bytes(const std::filesystem::path& path, std::string_view bytes) {
  std::filesystem::create_directories(path.parent_path());
  std::ofstream output(path, std::ios::binary);
  output.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
  if (!output) {
    throw std::runtime_error("fixture write failed");
  }
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

class MldsaSigningFixture final {
 public:
  explicit MldsaSigningFixture(std::byte seed_fill) {
    seed_.fill(static_cast<std::uint8_t>(std::to_integer<unsigned int>(seed_fill)));
    if (fcitx5_mldsa65_test_keypair_internal(public_key_.data(), secret_key_.data(),
                                             seed_.data()) != 0) {
      throw std::runtime_error("ML-DSA fixture key generation failed");
    }
    public_key_bytes_.assign(reinterpret_cast<const std::byte*>(public_key_.data()),
                             reinterpret_cast<const std::byte*>(
                                 public_key_.data() + public_key_.size()));
  }

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
  std::array<std::uint8_t, MLDSA65_SEEDBYTES> seed_{};
  std::vector<std::uint8_t> public_key_ = std::vector<std::uint8_t>(MLDSA65_PUBLICKEYBYTES);
  std::vector<std::uint8_t> secret_key_ = std::vector<std::uint8_t>(MLDSA65_SECRETKEYBYTES);
  std::vector<std::byte> public_key_bytes_;
};

using ArchiveEntry = std::pair<std::string, std::vector<std::byte>>;

std::vector<std::byte> as_bytes(std::string_view value) {
  const auto bytes = std::as_bytes(std::span(value));
  return {bytes.begin(), bytes.end()};
}

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
  if (!result.empty() && result.back() == '\0') result.pop_back();
  return result;
}

void create_archive(const std::filesystem::path& path,
                    const std::vector<ArchiveEntry>& entries) {
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

void run_path_corpus(const std::filesystem::path& corpus_path) {
  std::ifstream input(corpus_path, std::ios::binary);
  if (!input) {
    throw std::runtime_error("package path corpus is missing");
  }
  const std::string bytes((std::istreambuf_iterator<char>(input)),
                          std::istreambuf_iterator<char>());
  const auto document = nlohmann::json::parse(bytes);
  expect(document.at("version").get<int>() == 1, "unexpected package path corpus version");
  std::size_t case_count = 0;
  for (const auto& item : document.at("path_cases")) {
    const auto path = item.at("path").get<std::string>();
    const auto accepted = item.at("accepted").get<bool>();
    if (fcitx::package::is_safe_relative_package_path(path) != accepted) {
      throw std::runtime_error("package path corpus mismatch: " + path);
    }
    ++case_count;
  }
  expect(case_count >= 20U, "package path corpus is incomplete");
  expect(!document.at("case_collision_sets").empty(),
         "package path corpus lacks case-collision fixtures");
}

}  // namespace

int main(int argc, char** argv) {
  try {
    if (argc != 2) {
      throw std::runtime_error("package path corpus argument required");
    }
    using namespace fcitx::package;
    run_path_corpus(argv[1]);
    expect(is_safe_relative_package_path("bin/addon.dll"), "valid package path rejected");
    expect(!is_safe_relative_package_path("../addon.dll"), "parent traversal accepted");
    expect(!is_safe_relative_package_path("C:/addon.dll"), "drive path accepted");
    expect(!is_safe_relative_package_path("//server/share"), "UNC path accepted");
    expect(!is_safe_relative_package_path("bin\\addon.dll"), "backslash path accepted");
    expect(!is_safe_relative_package_path("bin/addon.dll:evil"), "ADS path accepted");
    expect(!is_safe_relative_package_path("bin/addon.dll."), "trailing dot path accepted");
    expect(!is_safe_relative_package_path("bin/addon.dll "), "trailing space path accepted");
    expect(!is_safe_relative_package_path("bin/CON"), "DOS device path accepted");
    expect(!is_safe_relative_package_path("bin/con.txt"), "DOS device path with extension accepted");
    expect(!is_safe_relative_package_path("bin/PRN"), "uppercase DOS device path accepted");
    expect(!is_safe_relative_package_path("bin/AUX.dat"), "AUX device path accepted");
    expect(!is_safe_relative_package_path("bin/NUL"), "NUL device path accepted");
    expect(!is_safe_relative_package_path("bin/COM1"), "COM device path accepted");
    expect(!is_safe_relative_package_path("bin/LPT9.log"), "LPT device path accepted");
    expect(!is_safe_relative_package_path(std::string_view("bin/bad\x01name.dll", 16)),
           "control-character path accepted");

    TemporaryDirectory temporary;
    const auto payload = temporary.path() / "source";
    constexpr std::string_view file_bytes = "verified addon fixture\n";
    write_bytes(payload / "bin/addon.dll", file_bytes);
    const auto file_hash = hex_sha256(sha256(std::as_bytes(std::span(file_bytes))));
    const auto file_blake3 = hex_blake3(blake3(std::as_bytes(std::span(file_bytes))));
    const auto manifest_bytes = make_manifest("fcitx5-rime", "1.0.0", file_hash,
                                              static_cast<std::uint64_t>(file_bytes.size()));
    const auto manifest = parse_manifest(manifest_bytes);
    expect(manifest.id == "fcitx5-rime", "manifest id mismatch");
#if defined(_WIN64)
    validate_manifest_compatibility(manifest, "x64");
#else
    validate_manifest_compatibility(manifest, "x86");
#endif
    auto incompatible = manifest;
    incompatible.addon_abi = "999";
#if defined(_WIN64)
    expect_error("incompatible_package", [&] { validate_manifest_compatibility(incompatible, "x64"); });
#else
    expect_error("incompatible_package", [&] { validate_manifest_compatibility(incompatible, "x86"); });
#endif
    verify_payload(manifest, payload);

    auto unknown = manifest_bytes;
    const auto closing = unknown.rfind('}');
    unknown.insert(closing, ",\n  \"unexpected\": true\n");
    expect_error("invalid_manifest", [&] { static_cast<void>(parse_manifest(unknown)); });

    auto traversal = manifest_bytes;
    traversal.replace(traversal.find("bin/addon.dll"), std::string_view("bin/addon.dll").size(),
                      "../addon.dll");
    expect_error("invalid_manifest", [&] { static_cast<void>(parse_manifest(traversal)); });
    auto manifest_case_collision = manifest_bytes;
    const auto file_array_end = manifest_case_collision.find("}],\n  \"key_id\"");
    expect(file_array_end != std::string::npos, "manifest fixture shape changed");
    manifest_case_collision.insert(
        file_array_end + 1,
        ", {\"path\": \"BIN/ADDON.DLL\", \"size\": " +
            std::to_string(file_bytes.size()) + ", \"sha256\": \"" + file_hash + "\"}");
    expect_error("invalid_manifest",
                 [&] { static_cast<void>(parse_manifest(manifest_case_collision)); });

    write_bytes(payload / "bin/undeclared.dll", "unexpected");
    expect_error("payload_mismatch", [&] { verify_payload(manifest, payload); });
    std::filesystem::remove(payload / "bin/undeclared.dll");
    const auto symlink_path = payload / "bin/symlink.dll";
    if (CreateSymbolicLinkW(symlink_path.c_str(), (payload / "bin/addon.dll").c_str(), 0)) {
      expect_error("unsafe_payload", [&] { verify_payload(manifest, payload); });
      std::filesystem::remove(symlink_path);
    } else {
      const DWORD error = GetLastError();
      if (error != ERROR_PRIVILEGE_NOT_HELD && error != ERROR_INVALID_PARAMETER) {
        throw std::runtime_error("unexpected symlink fixture failure");
      }
    }

    const auto manifest_v2_blake3_only =
        make_manifest_v2("fcitx5-rime", "2.0.0", file_blake3,
                         static_cast<std::uint64_t>(file_bytes.size()), "release-2026");
    const auto parsed_v2_blake3_only = parse_manifest(manifest_v2_blake3_only);
    expect(parsed_v2_blake3_only.format_version == kManifestV2FormatVersion &&
               parsed_v2_blake3_only.files.front().blake3 == file_blake3 &&
               parsed_v2_blake3_only.files.front().sha256.empty(),
           "v2 BLAKE3-only manifest parsing failed");
    verify_payload(parsed_v2_blake3_only, payload);

    const auto manifest_v2_dual_hash =
        make_manifest_v2("fcitx5-rime", "2.0.1", file_blake3,
                         static_cast<std::uint64_t>(file_bytes.size()), "release-2026",
                         file_hash);
    const auto parsed_v2_dual_hash = parse_manifest(manifest_v2_dual_hash);
    expect(parsed_v2_dual_hash.files.front().blake3 == file_blake3 &&
               parsed_v2_dual_hash.files.front().sha256 == file_hash,
           "v2 dual-hash manifest parsing failed");
    verify_payload(parsed_v2_dual_hash, payload);

    auto manifest_v2_missing_blake3 = manifest_v2_dual_hash;
    manifest_v2_missing_blake3.replace(manifest_v2_missing_blake3.find("\"blake3\""),
                                       std::strlen("\"blake3\""), "\"b3\"");
    expect_error("invalid_manifest",
                 [&] { static_cast<void>(parse_manifest(manifest_v2_missing_blake3)); });
    auto manifest_v2_bad_sha256 = manifest_v2_dual_hash;
    manifest_v2_bad_sha256.replace(manifest_v2_bad_sha256.find(file_hash), file_hash.size(),
                                   std::string(64U, '0'));
    expect_error("payload_mismatch", [&] {
      verify_payload(parse_manifest(manifest_v2_bad_sha256), payload);
    });

    const auto install = temporary.path() / "program";
    SigningFixture signer;
    TrustedKey trusted{"release-2026", signer.public_blob(), false};
    const auto signature = signer.sign(manifest_bytes);
    SigningFixture old_signer;
    const auto v2_signature = signer.sign(manifest_v2_dual_hash);
    const auto corrupt_v2_payload = temporary.path() / "source-v2-corrupt";
    write_bytes(corrupt_v2_payload / "bin/addon.dll", "corrupted v2 payload\n");
    expect_error("payload_mismatch", [&] {
      static_cast<void>(stage_verified_payload(parsed_v2_dual_hash, manifest_v2_dual_hash,
                                               corrupt_v2_payload, install, "tx-v2-corrupt",
                                               v2_signature, trusted));
    });
    const auto keyring_path = temporary.path() / "trusted-keys.json";
    const auto keyring_bytes =
        "{\n  \"format_version\": 1,\n  \"keys\": [\n"
        "    {\"key_id\":\"release-2026\",\"algorithm\":\"rsa-2048-sha256\","
        "\"status\":\"trusted\",\"public_key_base64\":\"" +
        base64(trusted.rsa_public_blob) +
        "\"},\n"
        "    {\"key_id\":\"release-2025\",\"algorithm\":\"rsa-2048-sha256\","
        "\"status\":\"revoked\",\"public_key_base64\":\"" +
        base64(old_signer.public_blob()) + "\"}\n  ]\n}\n";
    write_bytes(keyring_path, keyring_bytes);
    const auto parsed_keys = read_trusted_keys(keyring_path);
    expect(parsed_keys.size() == 2U && !parsed_keys.front().revoked &&
               parsed_keys.back().revoked,
           "trusted key rotation/revocation state mismatch");
    const auto mldsa = std::vector<std::byte>(1952U, std::byte{0x41});
    const auto slhdsa = std::vector<std::byte>(32U, std::byte{0x42});
    const auto pqc_keyring_path = temporary.path() / "trusted-keys-v2.json";
    const auto pqc_keyring_bytes =
        "{\n"
        "  \"format_version\": 2,\n"
        "  \"policy\": {\"official_required_signatures\":[\"mldsa65\"],"
        "\"compatibility_hashes\":[\"sha256\"],\"default_payload_hash\":\"blake3\"},\n"
        "  \"keys\": [\n"
        "    {\"key_id\":\"official-2026-mldsa65\",\"algorithm\":\"mldsa65\","
        "\"status\":\"trusted\",\"public_key_base64\":\"" +
        base64(mldsa) +
        "\",\"scope\":[\"repository\",\"package\"],\"channels\":[\"stable\"]},\n"
        "    {\"key_id\":\"official-2026-mldsa65-revoked\",\"algorithm\":\"mldsa65\","
        "\"status\":\"revoked\",\"public_key_base64\":\"" +
        base64(mldsa) +
        "\",\"scope\":[\"repository\",\"package\"],\"channels\":[\"stable\"]},\n"
        "    {\"key_id\":\"official-2026-slh-dsa-recovery\","
        "\"algorithm\":\"slhdsa-sha2-128s\",\"status\":\"trusted\","
        "\"public_key_base64\":\"" +
        base64(slhdsa) + "\",\"scope\":[\"repository\"],\"channels\":[\"stable\"]}\n"
        "  ]\n"
        "}\n";
    write_bytes(pqc_keyring_path, pqc_keyring_bytes);
    const auto pqc_keys = read_trusted_keys(pqc_keyring_path);
    expect(pqc_keys.size() == 3U && pqc_keys[0].algorithm == "mldsa65" &&
               pqc_keys[0].public_key.size() == 1952U && pqc_keys[1].revoked &&
               pqc_keys[2].algorithm == "slhdsa-sha2-128s" &&
               pqc_keys[2].public_key.size() == 32U,
           "PQC trusted keyring v2 parsing failed");
    auto bad_pqc = pqc_keyring_bytes;
    bad_pqc.replace(bad_pqc.find(base64(mldsa)), base64(mldsa).size(), base64(slhdsa));
    write_bytes(pqc_keyring_path, bad_pqc);
    expect_error("invalid_keyring", [&] { static_cast<void>(read_trusted_keys(pqc_keyring_path)); });
    auto duplicate_pqc = pqc_keyring_bytes;
    duplicate_pqc.replace(duplicate_pqc.find("official-2026-mldsa65-revoked"),
                          std::string_view("official-2026-mldsa65-revoked").size(),
                          "official-2026-mldsa65");
    write_bytes(pqc_keyring_path, duplicate_pqc);
    expect_error("invalid_keyring", [&] { static_cast<void>(read_trusted_keys(pqc_keyring_path)); });
    auto unsupported_required = pqc_keyring_bytes;
    unsupported_required.replace(unsupported_required.find("\"mldsa65\""),
                                 std::string_view("\"mldsa65\"").size(), "\"ed25519\"");
    write_bytes(pqc_keyring_path, unsupported_required);
    expect_error("invalid_keyring", [&] { static_cast<void>(read_trusted_keys(pqc_keyring_path)); });

    const auto mldsa_signature = std::vector<std::byte>(3309U, std::byte{0x43});
    const auto slhdsa_signature = std::vector<std::byte>(7856U, std::byte{0x44});
    const auto index_envelope_bytes =
        "{\n"
        "  \"format_version\": 2,\n"
        "  \"signed_object\": \"repository-index\",\n"
        "  \"canonicalization\": \"fcitx5-windows-next-json-v1\",\n"
        "  \"signatures\": [\n"
        "    {\"key_id\":\"official-2026-mldsa65\",\"algorithm\":\"mldsa65\","
        "\"signature_base64\":\"" +
        base64(mldsa_signature) + "\"},\n"
        "    {\"key_id\":\"official-2026-slh-dsa-recovery\","
        "\"algorithm\":\"slhdsa-sha2-128s\",\"signature_base64\":\"" +
        base64(slhdsa_signature) + "\"}\n"
        "  ]\n"
        "}\n";
    const auto index_envelope_path = temporary.path() / "index.sig.json";
    write_bytes(index_envelope_path, index_envelope_bytes);
    const auto index_envelope =
        read_signature_envelope(index_envelope_path, "repository-index");
    expect(index_envelope.format_version == 2U &&
               index_envelope.signed_object == "repository-index" &&
               index_envelope.canonicalization == "fcitx5-windows-next-json-v1" &&
               index_envelope.signatures.size() == 2U &&
               index_envelope.signatures.front().algorithm == "mldsa65" &&
               index_envelope.signatures.front().signature.size() == mldsa_signature.size(),
           "repository signature envelope parsing failed");

    const auto manifest_envelope_bytes =
        "{\n"
        "  \"format_version\": 2,\n"
        "  \"signed_object\": \"package-manifest\",\n"
        "  \"canonicalization\": \"fcitx5-windows-next-json-v1\",\n"
        "  \"signatures\": [\n"
        "    {\"key_id\":\"official-2026-mldsa65\",\"algorithm\":\"mldsa65\","
        "\"signature_base64\":\"" +
        base64(mldsa_signature) + "\"}\n"
        "  ]\n"
        "}\n";
    const auto manifest_envelope_path = temporary.path() / "manifest.sig.json";
    write_bytes(manifest_envelope_path, manifest_envelope_bytes);
    const auto manifest_envelope =
        read_signature_envelope(manifest_envelope_path, "package-manifest");
    expect(manifest_envelope.signed_object == "package-manifest" &&
               manifest_envelope.signatures.size() == 1U,
           "manifest signature envelope parsing failed");
    expect_error("invalid_signature", [&] {
      static_cast<void>(parse_signature_envelope(index_envelope_bytes, "package-manifest"));
    });
    auto missing_mldsa_signature = manifest_envelope_bytes;
    missing_mldsa_signature.replace(missing_mldsa_signature.find("official-2026-mldsa65"),
                                    std::string_view("official-2026-mldsa65").size(),
                                    "official-2026-slh-dsa-recovery");
    missing_mldsa_signature.replace(missing_mldsa_signature.find("\"mldsa65\""),
                                    std::string_view("\"mldsa65\"").size(),
                                    "\"slhdsa-sha2-128s\"");
    expect_error("invalid_signature", [&] {
      static_cast<void>(
          parse_signature_envelope(missing_mldsa_signature, "package-manifest"));
    });
    auto unsupported_signature_algorithm = manifest_envelope_bytes;
    unsupported_signature_algorithm.replace(unsupported_signature_algorithm.find("\"mldsa65\""),
                                            std::string_view("\"mldsa65\"").size(),
                                            "\"ed25519\"");
    expect_error("invalid_signature", [&] {
      static_cast<void>(
          parse_signature_envelope(unsupported_signature_algorithm, "package-manifest"));
    });
    auto duplicate_signature_key = index_envelope_bytes;
    duplicate_signature_key.replace(duplicate_signature_key.find("official-2026-slh-dsa-recovery"),
                                    std::string_view("official-2026-slh-dsa-recovery").size(),
                                    "official-2026-mldsa65");
    expect_error("invalid_signature", [&] {
      static_cast<void>(parse_signature_envelope(duplicate_signature_key, "repository-index"));
    });
    auto malformed_signature_base64 = manifest_envelope_bytes;
    malformed_signature_base64.replace(malformed_signature_base64.find(base64(mldsa_signature)),
                                       base64(mldsa_signature).size(), "not base64!");
    expect_error("invalid_signature", [&] {
      static_cast<void>(
          parse_signature_envelope(malformed_signature_base64, "package-manifest"));
    });
    const auto repository_bytes =
        "{\"format_version\":1,\"channel\":\"stable\","
        "\"generated_at\":\"2026-08-17T00:00:00Z\",\"key_id\":\"release-2026\","
        "\"packages\":[{\"id\":\"fcitx5-rime\",\"title\":\"Rime\","
        "\"summary\":\"Rime input engine\",\"version\":\"1.0.0\","
        "\"release_sequence\":1,\"type\":\"addon\",\"architecture\":\"x64\","
        "\"download_url\":\"https://packages.example.invalid/fcitx5-rime.fcpkg\","
        "\"sha256\":\"" + file_hash + "\",\"dependencies\":[]}]}";
    const auto repository_signature = signer.sign(repository_bytes);
    const auto repository = verify_repository_index(repository_bytes, repository_signature,
                                                    std::span(&trusted, 1U), "stable");
    expect(repository.packages.size() == 1U &&
               find_repository_package(repository, "fcitx5-rime", "x64") != nullptr,
           "signed repository did not expose its package");
    auto repository_tampered = repository_bytes;
    repository_tampered.replace(repository_tampered.find("Rime input"), 4U, "Fake");
    expect_error("invalid_signature", [&] {
      static_cast<void>(verify_repository_index(repository_tampered, repository_signature,
                                                std::span(&trusted, 1U), "stable"));
    });
    // Channel binding: a stable build must reject a beta/nightly index even
    // though the signature is valid.
    auto repository_beta = repository_bytes;
    repository_beta.replace(repository_beta.find("\"channel\":\"stable\""),
                            std::strlen("\"channel\":\"stable\""), "\"channel\":\"beta\"");
    const auto beta_signature = signer.sign(repository_beta);
    expect_error("invalid_repository", [&] {
      static_cast<void>(verify_repository_index(repository_beta, beta_signature,
                                                std::span(&trusted, 1U), "stable"));
    });

    MldsaSigningFixture mldsa_signer{std::byte{0x11}};
    MldsaSigningFixture other_mldsa_signer{std::byte{0x22}};
    TrustedKey mldsa_trusted{"official-2026-mldsa65", "mldsa65",
                             mldsa_signer.public_key(), false};
    TrustedKey other_mldsa_trusted{"official-2026-mldsa65-other", "mldsa65",
                                   other_mldsa_signer.public_key(), false};
    const auto mldsa_repository_bytes =
        "{\"format_version\":1,\"channel\":\"stable\","
        "\"generated_at\":\"2026-08-17T00:00:00Z\",\"key_id\":\"official-2026-mldsa65\","
        "\"packages\":[{\"id\":\"fcitx5-rime\",\"title\":\"Rime\","
        "\"summary\":\"Rime input engine\",\"version\":\"1.0.0\","
        "\"release_sequence\":1,\"type\":\"addon\",\"architecture\":\"x64\","
        "\"download_url\":\"https://packages.example.invalid/fcitx5-rime.fcpkg\","
        "\"sha256\":\"" + file_hash + "\",\"dependencies\":[]}]}";
    const auto mldsa_repository_signature = mldsa_signer.sign(mldsa_repository_bytes);
    const auto mldsa_repository_envelope_bytes =
        "{"
        "\"format_version\":2,"
        "\"signed_object\":\"repository-index\","
        "\"canonicalization\":\"fcitx5-windows-next-json-v1\","
        "\"signatures\":[{\"key_id\":\"official-2026-mldsa65\","
        "\"algorithm\":\"mldsa65\",\"signature_base64\":\"" +
        base64(mldsa_repository_signature) + "\"}]}";
    const auto mldsa_repository = verify_repository_index_envelope(
        mldsa_repository_bytes, mldsa_repository_envelope_bytes,
        std::span(&mldsa_trusted, 1U), "stable");
    expect(mldsa_repository.packages.size() == 1U,
           "ML-DSA repository signature did not verify");
    auto bad_mldsa_repository_signature = mldsa_repository_signature;
    bad_mldsa_repository_signature.front() ^= std::byte{0x01};
    const auto bad_mldsa_repository_envelope_bytes =
        "{"
        "\"format_version\":2,"
        "\"signed_object\":\"repository-index\","
        "\"canonicalization\":\"fcitx5-windows-next-json-v1\","
        "\"signatures\":[{\"key_id\":\"official-2026-mldsa65\","
        "\"algorithm\":\"mldsa65\",\"signature_base64\":\"" +
        base64(bad_mldsa_repository_signature) + "\"}]}";
    expect_error("invalid_signature", [&] {
      static_cast<void>(verify_repository_index_envelope(
          mldsa_repository_bytes, bad_mldsa_repository_envelope_bytes,
          std::span(&mldsa_trusted, 1U), "stable"));
    });
    auto revoked_mldsa = mldsa_trusted;
    revoked_mldsa.revoked = true;
    expect_error("revoked_key", [&] {
      static_cast<void>(verify_repository_index_envelope(
          mldsa_repository_bytes, mldsa_repository_envelope_bytes,
          std::span(&revoked_mldsa, 1U), "stable"));
    });
    const auto wrong_key_repository_envelope_bytes =
        "{"
        "\"format_version\":2,"
        "\"signed_object\":\"repository-index\","
        "\"canonicalization\":\"fcitx5-windows-next-json-v1\","
        "\"signatures\":[{\"key_id\":\"official-2026-mldsa65-other\","
        "\"algorithm\":\"mldsa65\",\"signature_base64\":\"" +
        base64(other_mldsa_signer.sign(mldsa_repository_bytes)) + "\"}]}";
    expect_error("untrusted_key", [&] {
      static_cast<void>(verify_repository_index_envelope(
          mldsa_repository_bytes, wrong_key_repository_envelope_bytes,
          std::span(&other_mldsa_trusted, 1U), "stable"));
    });
    auto mldsa_repository_beta = mldsa_repository_bytes;
    mldsa_repository_beta.replace(mldsa_repository_beta.find("\"channel\":\"stable\""),
                                  std::strlen("\"channel\":\"stable\""),
                                  "\"channel\":\"beta\"");
    const auto mldsa_beta_signature = mldsa_signer.sign(mldsa_repository_beta);
    const auto mldsa_beta_envelope_bytes =
        "{"
        "\"format_version\":2,"
        "\"signed_object\":\"repository-index\","
        "\"canonicalization\":\"fcitx5-windows-next-json-v1\","
        "\"signatures\":[{\"key_id\":\"official-2026-mldsa65\","
        "\"algorithm\":\"mldsa65\",\"signature_base64\":\"" +
        base64(mldsa_beta_signature) + "\"}]}";
    expect_error("invalid_repository", [&] {
      static_cast<void>(verify_repository_index_envelope(
          mldsa_repository_beta, mldsa_beta_envelope_bytes,
          std::span(&mldsa_trusted, 1U), "stable"));
    });

    auto mldsa_manifest_bytes = manifest_v2_dual_hash;
    mldsa_manifest_bytes.replace(mldsa_manifest_bytes.find("\"key_id\": \"release-2026\""),
                                 std::strlen("\"key_id\": \"release-2026\""),
                                 "\"key_id\": \"official-2026-mldsa65\"");
    const auto mldsa_manifest_signature = mldsa_signer.sign(mldsa_manifest_bytes);
    const auto mldsa_manifest_envelope_bytes =
        "{"
        "\"format_version\":2,"
        "\"signed_object\":\"package-manifest\","
        "\"canonicalization\":\"fcitx5-windows-next-json-v1\","
        "\"signatures\":[{\"key_id\":\"official-2026-mldsa65\","
        "\"algorithm\":\"mldsa65\",\"signature_base64\":\"" +
        base64(mldsa_manifest_signature) + "\"}]}";
    verify_manifest_signature_envelope(
        mldsa_manifest_bytes,
        parse_signature_envelope(mldsa_manifest_envelope_bytes, "package-manifest"),
        std::span(&mldsa_trusted, 1U));
    auto tampered_mldsa_manifest = mldsa_manifest_bytes;
    tampered_mldsa_manifest.replace(tampered_mldsa_manifest.find("fcitx5-rime"),
                                    std::strlen("fcitx5-rime"), "fcitx5-lime");
    expect_error("invalid_signature", [&] {
      verify_manifest_signature_envelope(
          tampered_mldsa_manifest,
          parse_signature_envelope(mldsa_manifest_envelope_bytes, "package-manifest"),
          std::span(&mldsa_trusted, 1U));
    });
    const auto staged = stage_verified_payload(manifest, manifest_bytes, payload, install, "tx-one",
                                               signature, trusted);
    expect(std::filesystem::exists(staged / "payload/bin/addon.dll"), "staged payload missing");
    activate_staged_package(staged, install, std::span(&trusted, 1U));
    expect(std::filesystem::exists(install / "versions/fcitx5-rime/1.0.0/bin/addon.dll"),
           "activated payload missing");
    const auto lock = read_lockfile(install);
    expect(lock.size() == 1U && lock.front().id == "fcitx5-rime" &&
               lock.front().version == "1.0.0",
           "lockfile did not publish active version");

    const auto user_data = temporary.path() / "user-data/rime/user.dict.yaml";
    write_bytes(user_data, "irreplaceable user data\n");
    const auto update_manifest_bytes =
        make_manifest("fcitx5-rime", "1.1.0", file_hash,
                      static_cast<std::uint64_t>(file_bytes.size()));
    const auto update_manifest = parse_manifest(update_manifest_bytes);
    const auto update_signature = signer.sign(update_manifest_bytes);
    const auto failed_stage = stage_verified_payload(update_manifest, update_manifest_bytes,
                                                     payload, install, "tx-failed",
                                                     update_signature, trusted);
    write_bytes(failed_stage / "payload/bin/addon.dll", "tampered after staging\n");
    expect_error("payload_mismatch", [&] {
      activate_staged_package(failed_stage, install, std::span(&trusted, 1U));
    });
    const auto lock_after_failure = read_lockfile(install);
    expect(lock_after_failure.size() == 1U && lock_after_failure.front().version == "1.0.0",
           "failed activation changed the active lockfile");
    expect(std::filesystem::exists(user_data), "program activation touched separate user data");

    const auto dependency_text =
        "[{\"id\":\"fcitx5-rime\",\"version\":\"1.0.0\"}]";
    const auto schema_manifest =
        parse_manifest(make_manifest("rime-schema-luna", "1.0.0", file_hash,
                                     static_cast<std::uint64_t>(file_bytes.size()),
                                     dependency_text));
    const auto order = resolve_exact_dependencies({schema_manifest, manifest}, {"rime-schema-luna"});
    expect(order == std::vector<std::string>({"fcitx5-rime", "rime-schema-luna"}),
           "dependency order mismatch");

    auto missing = schema_manifest;
    missing.dependencies.front().version = "2.0.0";
    expect_error("resolution_failed", [&] {
      static_cast<void>(resolve_exact_dependencies({missing, manifest}, {"rime-schema-luna"}));
    });

    const auto archive_path = temporary.path() / "valid.fcpkg";
    create_archive(archive_path,
                   {{"manifest.json", as_bytes(mldsa_manifest_bytes)},
                    {"manifest.sig.json", as_bytes(mldsa_manifest_envelope_bytes)},
                    {"payload/bin/addon.dll", as_bytes(file_bytes)}});
    const auto archive_stage =
        stage_verified_archive(archive_path, install, "tx-two", std::span(&mldsa_trusted, 1U));
    expect(std::filesystem::exists(archive_stage / "payload/bin/addon.dll"),
           "verified archive was not staged");
    activate_staged_package(archive_stage, install, std::span(&mldsa_trusted, 1U));
    verify_installed_packages(install, std::span(&mldsa_trusted, 1U));
    set_package_state(install, "fcitx5-rime", "disabled");
    expect(read_lockfile(install).front().state == "disabled", "package state was not persisted");
    activate_installed_version(install, "fcitx5-rime", "2.0.1", std::span(&mldsa_trusted, 1U));
    expect(read_lockfile(install).front().state == "installed",
           "rollback activation did not restore installed state");

    auto revoked = mldsa_trusted;
    revoked.revoked = true;
    expect_error("revoked_key", [&] {
      static_cast<void>(
          stage_verified_archive(archive_path, install, "tx-revoked", std::span(&revoked, 1U)));
    });

    const auto malicious_path = temporary.path() / "traversal.fcpkg";
    create_archive(malicious_path,
                   {{"manifest.json", as_bytes(mldsa_manifest_bytes)},
                    {"manifest.sig.json", as_bytes(mldsa_manifest_envelope_bytes)},
                    {"payload/bin/addon.dll", as_bytes(file_bytes)},
                    {"payload/../escape.dll", as_bytes("escape")}});
    expect_error("unsafe_archive_path", [&] {
      static_cast<void>(stage_verified_archive(malicious_path, install, "tx-traversal",
                                                std::span(&mldsa_trusted, 1U)));
    });
    const auto collision_path = temporary.path() / "case-collision.fcpkg";
    create_archive(collision_path,
                   {{"manifest.json", as_bytes(mldsa_manifest_bytes)},
                    {"manifest.sig.json", as_bytes(mldsa_manifest_envelope_bytes)},
                    {"payload/bin/addon.dll", as_bytes(file_bytes)},
                    {"payload/BIN/ADDON.DLL", as_bytes(file_bytes)}});
    expect_error("unsafe_archive_path", [&] {
      static_cast<void>(stage_verified_archive(collision_path, install, "tx-case",
                                                std::span(&mldsa_trusted, 1U)));
    });

    mark_package_for_removal(install, "fcitx5-rime");
    expect(read_lockfile(install).front().state == "pending_remove",
           "uninstall did not enter the restart-safe pending state");
    finalize_package_removal(install, "fcitx5-rime");
    expect(read_lockfile(install).empty() &&
               !std::filesystem::exists(install / "versions/fcitx5-rime") &&
               std::filesystem::exists(user_data),
           "uninstall did not deactivate payload or preserve user data");

    // Identifier validators shared with the updater/deployer for CLI-supplied
    // values that later become filesystem paths.
    expect(is_lower_package_id("fcitx5-rime") && is_lower_package_id("core") &&
               is_lower_package_id("a-b.c_d"),
           "valid package ids rejected");
    expect(!is_lower_package_id("") && !is_lower_package_id("..") &&
               !is_lower_package_id("a/../b") && !is_lower_package_id("A.b") &&
               !is_lower_package_id("1abc") && !is_lower_package_id("a\\b") &&
               !is_lower_package_id("a b"),
           "path-escaping or invalid package ids accepted");
    expect(is_ascii_token("1.0.0-beta.2", ".+-_") && !is_ascii_token("../1.0.0", ".+-_") &&
               !is_ascii_token("1.0.0/../x", ".+-_"),
           "version token validation mismatch");

    std::cout << "package core contract passed\n";
    return 0;
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
