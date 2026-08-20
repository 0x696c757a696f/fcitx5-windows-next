#include "package_core.h"

#include <windows.h>
#include <bcrypt.h>
#include <wincrypt.h>

#include <filesystem>
#include <fstream>
#include <cstring>
#include <iostream>
#include <span>
#include <string>
#include <utility>
#include <vector>

#include <miniz.h>

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

}  // namespace

int main() {
  try {
    using namespace fcitx::package;
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

    write_bytes(payload / "bin/undeclared.dll", "unexpected");
    expect_error("payload_mismatch", [&] { verify_payload(manifest, payload); });
    std::filesystem::remove(payload / "bin/undeclared.dll");

    const auto install = temporary.path() / "program";
    SigningFixture signer;
    TrustedKey trusted{"release-2026", signer.public_blob(), false};
    const auto signature = signer.sign(manifest_bytes);
    SigningFixture old_signer;
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
                   {{"manifest.json", as_bytes(manifest_bytes)},
                    {"manifest.sig", signature},
                    {"payload/bin/addon.dll", as_bytes(file_bytes)}});
    const auto archive_stage =
        stage_verified_archive(archive_path, install, "tx-two", std::span(&trusted, 1U));
    expect(std::filesystem::exists(archive_stage / "payload/bin/addon.dll"),
           "verified archive was not staged");
    activate_staged_package(archive_stage, install, std::span(&trusted, 1U));
    verify_installed_packages(install, std::span(&trusted, 1U));
    set_package_state(install, "fcitx5-rime", "disabled");
    expect(read_lockfile(install).front().state == "disabled", "package state was not persisted");

    auto revoked = trusted;
    revoked.revoked = true;
    expect_error("revoked_key", [&] {
      static_cast<void>(
          stage_verified_archive(archive_path, install, "tx-revoked", std::span(&revoked, 1U)));
    });

    const auto malicious_path = temporary.path() / "traversal.fcpkg";
    create_archive(malicious_path,
                   {{"manifest.json", as_bytes(manifest_bytes)},
                    {"manifest.sig", signature},
                    {"payload/bin/addon.dll", as_bytes(file_bytes)},
                    {"payload/../escape.dll", as_bytes("escape")}});
    expect_error("unsafe_archive_path", [&] {
      static_cast<void>(stage_verified_archive(malicious_path, install, "tx-traversal",
                                                std::span(&trusted, 1U)));
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
