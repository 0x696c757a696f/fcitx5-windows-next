#pragma once

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <ranges>
#include <span>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>
#include <utility>

#include <Windows.h>
#include <wincrypt.h>
#include <nlohmann/json.hpp>

namespace fcitx::package {

// Package identifiers are strictly lowercase alphanumerics with '.', '-', '_',
// and must start with a lowercase letter. Everything that is later turned into
// a filesystem path (package versions dir, transaction id, ...) goes through
// this check so a CLI-supplied value cannot escape the versions directory.
[[nodiscard]] bool is_lower_package_id(std::string_view value) noexcept;
// Generic restricted token check (letters/digits plus an explicit extra set),
// used for version strings.
[[nodiscard]] bool is_ascii_token(std::string_view value,
                                  std::string_view extra) noexcept;

inline constexpr std::uint32_t kManifestFormatVersion = 1;
inline constexpr std::uint32_t kManifestV2FormatVersion = 2;
inline constexpr std::size_t kMaximumManifestBytes = 1024U * 1024U;
inline constexpr std::size_t kMaximumFileCount = 4096U;
inline constexpr std::uint64_t kMaximumFileBytes = 64ULL * 1024ULL * 1024ULL;
inline constexpr std::uint64_t kMaximumPayloadBytes = 512ULL * 1024ULL * 1024ULL;
inline constexpr std::string_view kSupportedCoreApi = "1";
inline constexpr std::string_view kSupportedAddonAbi = "1";

class PackageError final : public std::runtime_error {
 public:
  PackageError(std::string code, std::string message);

  [[nodiscard]] const std::string& code() const noexcept;

 private:
  std::string code_;
};

enum class PackageType {
  core,
  addon,
  input_method_data,
  theme,
  translation,
};

struct Dependency {
  std::string id;
  std::string version;
};

struct FileEntry {
  std::string path;
  std::uint64_t size{};
  std::string sha256;
  std::string blake3;
};

struct Manifest {
  std::uint32_t format_version{};
  std::string id;
  std::string version;
  PackageType type{};
  std::string architecture;
  std::string min_os;
  std::string core_api;
  std::string addon_abi;
  std::vector<Dependency> dependencies;
  std::string license;
  std::string source_commit;
  std::vector<std::string> permissions;
  std::vector<FileEntry> files;
  std::string key_id;
};

struct TrustedKey {
  std::string id;
  std::string algorithm{"rsa-2048-sha256"};
  std::vector<std::byte> public_key;
  std::vector<std::byte> rsa_public_blob;
  bool revoked{};

  TrustedKey() = default;
  TrustedKey(std::string key_id, std::vector<std::byte> rsa_blob, bool is_revoked)
      : id(std::move(key_id)),
        algorithm("rsa-2048-sha256"),
        public_key(rsa_blob),
        rsa_public_blob(std::move(rsa_blob)),
        revoked(is_revoked) {}
  TrustedKey(std::string key_id, std::string key_algorithm, std::vector<std::byte> key_bytes,
             bool is_revoked)
      : id(std::move(key_id)),
        algorithm(std::move(key_algorithm)),
        public_key(std::move(key_bytes)),
        revoked(is_revoked) {
    if (algorithm == "rsa-2048-sha256")
      rsa_public_blob = public_key;
  }
};

struct SignatureEnvelopeEntry {
  std::string key_id;
  std::string algorithm;
  std::vector<std::byte> signature;
};

struct SignatureEnvelope {
  std::uint32_t format_version{};
  std::string signed_object;
  std::string canonicalization;
  std::vector<SignatureEnvelopeEntry> signatures;
};

struct LockEntry {
  std::string id;
  std::string version;
  std::string manifest_sha256;
  std::string state;
};

struct RepositoryEntry {
  std::string id;
  std::string title;
  std::string summary;
  std::string version;
  std::uint64_t release_sequence{};
  PackageType type{};
  std::string architecture;
  std::string download_url;
  std::string sha256;
  std::vector<Dependency> dependencies;
};

struct RepositoryIndex {
  std::uint32_t format_version{};
  std::string channel;
  std::string generated_at;
  std::string key_id;
  std::vector<RepositoryEntry> packages;
};

[[nodiscard]] Manifest parse_manifest(std::string_view bytes);
void validate_manifest_compatibility(const Manifest& manifest,
                                     std::string_view architecture);
[[nodiscard]] bool is_safe_relative_package_path(std::string_view path) noexcept;
[[nodiscard]] bool path_contains_reparse_point(const std::filesystem::path& path);
[[nodiscard]] std::array<std::byte, 32> sha256(std::span<const std::byte> bytes);
[[nodiscard]] std::array<std::byte, 32> sha256_file(const std::filesystem::path& path);
[[nodiscard]] std::string hex_sha256(std::span<const std::byte> digest);
[[nodiscard]] std::array<std::byte, 32> blake3(std::span<const std::byte> bytes);
[[nodiscard]] std::array<std::byte, 32> blake3_file(const std::filesystem::path& path);
[[nodiscard]] std::string hex_blake3(std::span<const std::byte> digest);
[[nodiscard]] std::vector<std::byte> decode_base64(std::string_view encoded);
[[nodiscard]] SignatureEnvelope parse_signature_envelope(
    std::string_view bytes, std::string_view expected_object);
[[nodiscard]] SignatureEnvelope read_signature_envelope(
    const std::filesystem::path& path, std::string_view expected_object);

void verify_manifest_signature(std::string_view manifest_bytes,
                               std::span<const std::byte> signature,
                               const TrustedKey& key);
void verify_signature_envelope(std::string_view object_bytes,
                               const SignatureEnvelope& envelope,
                               std::span<const TrustedKey> trusted_keys,
                               std::string_view expected_object,
                               std::string_view expected_key_id);
void verify_manifest_signature_envelope(std::string_view manifest_bytes,
                                        const SignatureEnvelope& envelope,
                                        std::span<const TrustedKey> trusted_keys);
void verify_payload(const Manifest& manifest, const std::filesystem::path& payload_root);

[[nodiscard]] std::vector<std::string> resolve_exact_dependencies(
    const std::vector<Manifest>& available, const std::vector<std::string>& requested_ids);

[[nodiscard]] std::filesystem::path stage_verified_payload(
    const Manifest& manifest, std::string_view manifest_bytes,
    const std::filesystem::path& payload_root, const std::filesystem::path& install_root,
    std::string_view transaction_id, std::span<const std::byte> signature,
    const TrustedKey& trusted_key);

[[nodiscard]] std::filesystem::path stage_verified_archive(
    const std::filesystem::path& archive_path, const std::filesystem::path& install_root,
    std::string_view transaction_id, std::span<const TrustedKey> trusted_keys);

void activate_staged_package(const std::filesystem::path& staged_root,
                             const std::filesystem::path& install_root,
                             std::span<const TrustedKey> trusted_keys);

[[nodiscard]] std::vector<LockEntry> read_lockfile(const std::filesystem::path& install_root);
[[nodiscard]] std::vector<TrustedKey> read_trusted_keys(const std::filesystem::path& path);
[[nodiscard]] RepositoryIndex verify_repository_index(
    std::string_view index_bytes, std::span<const std::byte> signature,
    std::span<const TrustedKey> trusted_keys, std::string_view expectedChannel);
[[nodiscard]] RepositoryIndex verify_repository_index(
    std::string_view index_bytes, const SignatureEnvelope& envelope,
    std::span<const TrustedKey> trusted_keys, std::string_view expectedChannel);
[[nodiscard]] const RepositoryEntry* find_repository_package(
    const RepositoryIndex& index, std::string_view package_id,
    std::string_view architecture) noexcept;
void verify_installed_packages(const std::filesystem::path& install_root,
                               std::span<const TrustedKey> trusted_keys);
[[nodiscard]] std::string repair_repository_sequence_state(
    const std::filesystem::path& data_root, const std::filesystem::path& index_path,
    const std::filesystem::path& signature_path, std::span<const TrustedKey> trusted_keys,
    std::string_view channel);
void set_package_state(const std::filesystem::path& install_root, std::string_view package_id,
                       std::string_view state);
void mark_package_for_removal(const std::filesystem::path& install_root,
                              std::string_view package_id);
void finalize_package_removal(const std::filesystem::path& install_root,
                              std::string_view package_id);
void activate_installed_version(const std::filesystem::path& install_root,
                                std::string_view package_id, std::string_view version,
                                std::span<const TrustedKey> trusted_keys);

namespace detail {

struct Fcitx5PackageByteSlice {
  const std::uint8_t* data{};
  std::size_t len{};
};

struct Fcitx5PackageTrustedKey {
  Fcitx5PackageByteSlice id{};
  Fcitx5PackageByteSlice algorithm{};
  Fcitx5PackageByteSlice public_key{};
  Fcitx5PackageByteSlice rsa_public_blob{};
  std::uint8_t revoked{};
};

struct Fcitx5PackageStageResult {
  int status{};
  std::uint8_t error_code[64]{};
  std::uint8_t error_message[512]{};
  wchar_t* staged_path{};
  std::size_t staged_path_len{};
};

struct Fcitx5RepositoryBlob {
  std::uint8_t* data{};
  std::size_t len{};
};

struct Fcitx5RepositoryResult {
  int status{};
  char error_code[64]{};
  char error_message[512]{};
  Fcitx5RepositoryBlob blob{};
};

extern "C" Fcitx5PackageStageResult fcitx5_package_stage_archive_utf16(
    const wchar_t* archive_path, std::size_t archive_path_len, const wchar_t* install_root,
    std::size_t install_root_len, const std::uint8_t* transaction_id,
    std::size_t transaction_id_len, const Fcitx5PackageTrustedKey* trusted_keys,
    std::size_t trusted_key_count);
extern "C" void fcitx5_package_wide_free(wchar_t* ptr, std::size_t len);
extern "C" Fcitx5RepositoryResult fcitx5_repository_verify_index_utf8(
    const std::uint8_t* index_data, std::size_t index_len, const std::uint8_t* signature_data,
    std::size_t signature_len, const Fcitx5PackageTrustedKey* trusted_keys,
    std::size_t trusted_key_count, const std::uint8_t* expected_channel,
    std::size_t expected_channel_len);
extern "C" Fcitx5RepositoryResult fcitx5_repository_verify_index_envelope_utf8(
    const std::uint8_t* index_data, std::size_t index_len, const std::uint8_t* envelope_data,
    std::size_t envelope_len, const Fcitx5PackageTrustedKey* trusted_keys,
    std::size_t trusted_key_count, const std::uint8_t* expected_channel,
    std::size_t expected_channel_len);
extern "C" void fcitx5_repository_blob_free(std::uint8_t* data, std::size_t len);

[[nodiscard]] inline Fcitx5PackageByteSlice byte_slice(std::string_view value) noexcept {
  return {reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

[[nodiscard]] inline Fcitx5PackageByteSlice byte_slice(
    std::span<const std::byte> value) noexcept {
  return {reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

[[nodiscard]] inline std::vector<Fcitx5PackageTrustedKey> rust_trusted_key_views(
    std::span<const TrustedKey> trusted_keys) {
  std::vector<Fcitx5PackageTrustedKey> result;
  result.reserve(trusted_keys.size());
  for (const auto& key : trusted_keys) {
    result.push_back(Fcitx5PackageTrustedKey{
        byte_slice(key.id),
        byte_slice(key.algorithm),
        byte_slice(std::span<const std::byte>(key.public_key.data(), key.public_key.size())),
        byte_slice(std::span<const std::byte>(key.rsa_public_blob.data(),
                                              key.rsa_public_blob.size())),
        key.revoked ? std::uint8_t{1} : std::uint8_t{0},
    });
  }
  return result;
}

[[nodiscard]] inline std::string ffi_ascii(std::span<const std::uint8_t> bytes) {
  const auto* begin = bytes.data();
  const auto* end = begin + bytes.size();
  const auto* nul = std::find(begin, end, std::uint8_t{0});
  return {reinterpret_cast<const char*>(begin), static_cast<std::size_t>(nul - begin)};
}

[[nodiscard]] inline std::string ffi_ascii(const char* bytes, std::size_t maximum) {
  const auto* nul = std::find(bytes, bytes + maximum, '\0');
  return {bytes, static_cast<std::size_t>(nul - bytes)};
}

[[nodiscard]] inline std::string json_string(std::string_view value) {
  std::string result = "\"";
  for (const unsigned char character : value) {
    switch (character) {
      case '\\':
        result += "\\\\";
        break;
      case '"':
        result += "\\\"";
        break;
      case '\b':
        result += "\\b";
        break;
      case '\f':
        result += "\\f";
        break;
      case '\n':
        result += "\\n";
        break;
      case '\r':
        result += "\\r";
        break;
      case '\t':
        result += "\\t";
        break;
      default:
        if (character < 0x20U) {
          throw PackageError("invalid_repository",
                             "verified repository envelope contains a control character");
        }
        result.push_back(static_cast<char>(character));
        break;
    }
  }
  result.push_back('"');
  return result;
}

[[nodiscard]] inline std::string base64(std::span<const std::byte> value) {
  DWORD size = 0;
  if (!CryptBinaryToStringA(reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, nullptr, &size)) {
    throw PackageError("invalid_repository", "repository signature envelope encoding failed");
  }
  std::string result(size, '\0');
  if (!CryptBinaryToStringA(reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, result.data(), &size)) {
    throw PackageError("invalid_repository", "repository signature envelope encoding failed");
  }
  result.resize(size);
  return result;
}

[[nodiscard]] inline RepositoryIndex parse_verified_repository_json(std::string_view bytes) {
  nlohmann::json document;
  try {
    document = nlohmann::json::parse(bytes.begin(), bytes.end(), nullptr, true, true);
  } catch (const nlohmann::json::exception&) {
    throw PackageError("invalid_repository", "verified repository payload is not strict JSON");
  }
  if (!document.is_object()) {
    throw PackageError("invalid_repository", "verified repository payload is malformed");
  }
  RepositoryIndex result;
  try {
    result.format_version = document.at("format_version").get<std::uint32_t>();
    result.channel = document.at("channel").get<std::string>();
    result.generated_at = document.at("generated_at").get<std::string>();
    result.key_id = document.at("key_id").get<std::string>();
    const auto& packages = document.at("packages");
    for (const auto& item : packages) {
      RepositoryEntry entry;
      entry.id = item.at("id").get<std::string>();
      entry.title = item.at("title").get<std::string>();
      entry.summary = item.at("summary").get<std::string>();
      entry.version = item.at("version").get<std::string>();
      entry.release_sequence = item.at("release_sequence").get<std::uint64_t>();
      const auto type = item.at("type").get<std::string>();
      if (type == "core") {
        entry.type = PackageType::core;
      } else if (type == "addon") {
        entry.type = PackageType::addon;
      } else if (type == "inputmethod-data") {
        entry.type = PackageType::input_method_data;
      } else if (type == "theme") {
        entry.type = PackageType::theme;
      } else if (type == "translation") {
        entry.type = PackageType::translation;
      } else {
        throw PackageError("invalid_repository",
                           "verified repository payload has an unsupported package type");
      }
      entry.architecture = item.at("architecture").get<std::string>();
      entry.download_url = item.at("download_url").get<std::string>();
      entry.sha256 = item.at("sha256").get<std::string>();
      for (const auto& dependency : item.at("dependencies")) {
        Dependency parsed;
        parsed.id = dependency.at("id").get<std::string>();
        parsed.version = dependency.at("version").get<std::string>();
        entry.dependencies.push_back(std::move(parsed));
      }
      result.packages.push_back(std::move(entry));
    }
  } catch (const nlohmann::json::exception&) {
    throw PackageError("invalid_repository", "verified repository payload is malformed");
  }
  return result;
}

[[nodiscard]] inline RepositoryIndex repository_result(Fcitx5RepositoryResult result) {
  if (result.status != 0) {
    throw PackageError(ffi_ascii(result.error_code, std::size(result.error_code)),
                       ffi_ascii(result.error_message, std::size(result.error_message)));
  }
  std::string verified_blob(reinterpret_cast<const char*>(result.blob.data), result.blob.len);
  fcitx5_repository_blob_free(result.blob.data, result.blob.len);
  return parse_verified_repository_json(verified_blob);
}

}  // namespace detail

[[nodiscard]] inline std::filesystem::path stage_verified_archive(
    const std::filesystem::path& archive_path, const std::filesystem::path& install_root,
    std::string_view transaction_id, std::span<const TrustedKey> trusted_keys) {
  const auto key_views = detail::rust_trusted_key_views(trusted_keys);
  const auto result = detail::fcitx5_package_stage_archive_utf16(
      archive_path.c_str(), archive_path.native().size(), install_root.c_str(),
      install_root.native().size(), reinterpret_cast<const std::uint8_t*>(transaction_id.data()),
      transaction_id.size(), key_views.data(), key_views.size());
  if (result.status != 0) {
    const std::string code = detail::ffi_ascii(result.error_code);
    const std::string message = detail::ffi_ascii(result.error_message);
    throw PackageError(code.empty() ? "invalid_archive" : code,
                       message.empty() ? "package archive validation failed" : message);
  }
  std::filesystem::path staged;
  if (result.staged_path && result.staged_path_len != 0) {
    staged = std::filesystem::path(
        std::wstring(result.staged_path, result.staged_path + result.staged_path_len));
  }
  detail::fcitx5_package_wide_free(result.staged_path, result.staged_path_len);
  return staged;
}

[[nodiscard]] inline RepositoryIndex verify_repository_index(
    std::string_view index_bytes, std::span<const std::byte> signature,
    std::span<const TrustedKey> trusted_keys, std::string_view expectedChannel) {
  const auto key_views = detail::rust_trusted_key_views(trusted_keys);
  return detail::repository_result(detail::fcitx5_repository_verify_index_utf8(
      reinterpret_cast<const std::uint8_t*>(index_bytes.data()), index_bytes.size(),
      reinterpret_cast<const std::uint8_t*>(signature.data()), signature.size(),
      key_views.data(), key_views.size(),
      reinterpret_cast<const std::uint8_t*>(expectedChannel.data()), expectedChannel.size()));
}

[[nodiscard]] inline RepositoryIndex verify_repository_index(
    std::string_view index_bytes, const SignatureEnvelope& envelope,
    std::span<const TrustedKey> trusted_keys, std::string_view expectedChannel) {
  std::string envelope_bytes = "{\"format_version\":" +
                               std::to_string(envelope.format_version) +
                               ",\"signed_object\":" +
                               detail::json_string(envelope.signed_object) +
                               ",\"canonicalization\":" +
                               detail::json_string(envelope.canonicalization) +
                               ",\"signatures\":[";
  bool first = true;
  for (const auto& signature : envelope.signatures) {
    if (!first) {
      envelope_bytes.push_back(',');
    }
    first = false;
    envelope_bytes += "{\"key_id\":" + detail::json_string(signature.key_id) +
                      ",\"algorithm\":" + detail::json_string(signature.algorithm) +
                      ",\"signature_base64\":" +
                      detail::json_string(detail::base64(std::as_bytes(std::span(signature.signature)))) +
                      '}';
  }
  envelope_bytes += "]}";
  const auto key_views = detail::rust_trusted_key_views(trusted_keys);
  return detail::repository_result(detail::fcitx5_repository_verify_index_envelope_utf8(
      reinterpret_cast<const std::uint8_t*>(index_bytes.data()), index_bytes.size(),
      reinterpret_cast<const std::uint8_t*>(envelope_bytes.data()), envelope_bytes.size(),
      key_views.data(), key_views.size(),
      reinterpret_cast<const std::uint8_t*>(expectedChannel.data()), expectedChannel.size()));
}

[[nodiscard]] inline const RepositoryEntry* find_repository_package(
    const RepositoryIndex& index, std::string_view package_id,
    std::string_view architecture) noexcept {
  const auto match = std::ranges::find_if(index.packages, [&](const RepositoryEntry& entry) {
    return entry.id == package_id &&
           (entry.architecture == "any" || entry.architecture == architecture);
  });
  return match == index.packages.end() ? nullptr : &*match;
}

}  // namespace fcitx::package
