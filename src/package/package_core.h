#pragma once

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <span>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>
#include <utility>

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

extern "C" Fcitx5PackageStageResult fcitx5_package_stage_archive_utf16(
    const wchar_t* archive_path, std::size_t archive_path_len, const wchar_t* install_root,
    std::size_t install_root_len, const std::uint8_t* transaction_id,
    std::size_t transaction_id_len, const Fcitx5PackageTrustedKey* trusted_keys,
    std::size_t trusted_key_count);
extern "C" void fcitx5_package_wide_free(wchar_t* ptr, std::size_t len);

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

}  // namespace fcitx::package
