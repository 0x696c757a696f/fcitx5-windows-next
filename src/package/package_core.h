#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <span>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace fcitx::package {

inline constexpr std::uint32_t kManifestFormatVersion = 1;
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
  std::vector<std::byte> rsa_public_blob;
  bool revoked{};
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
[[nodiscard]] std::vector<std::byte> decode_base64(std::string_view encoded);

void verify_manifest_signature(std::string_view manifest_bytes,
                               std::span<const std::byte> signature,
                               const TrustedKey& key);
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
    std::span<const TrustedKey> trusted_keys);
[[nodiscard]] const RepositoryEntry* find_repository_package(
    const RepositoryIndex& index, std::string_view package_id,
    std::string_view architecture) noexcept;
void verify_installed_packages(const std::filesystem::path& install_root,
                               std::span<const TrustedKey> trusted_keys);
void set_package_state(const std::filesystem::path& install_root, std::string_view package_id,
                       std::string_view state);
void mark_package_for_removal(const std::filesystem::path& install_root,
                              std::string_view package_id);
void finalize_package_removal(const std::filesystem::path& install_root,
                              std::string_view package_id);
void activate_installed_version(const std::filesystem::path& install_root,
                                std::string_view package_id, std::string_view version,
                                std::span<const TrustedKey> trusted_keys);

}  // namespace fcitx::package
