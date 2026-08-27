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
#include <utility>
#include <vector>

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

struct Fcitx5RepositoryFindEntry {
  Fcitx5PackageByteSlice id{};
  Fcitx5PackageByteSlice architecture{};
};

struct Fcitx5RepositorySignatureEnvelopeEntry {
  Fcitx5PackageByteSlice key_id{};
  Fcitx5PackageByteSlice algorithm{};
  Fcitx5PackageByteSlice signature{};
};

struct Fcitx5RepositoryDependencyResult {
  Fcitx5PackageByteSlice id{};
  Fcitx5PackageByteSlice version{};
};

struct Fcitx5RepositoryEntryResult {
  Fcitx5PackageByteSlice id{};
  Fcitx5PackageByteSlice title{};
  Fcitx5PackageByteSlice summary{};
  Fcitx5PackageByteSlice version{};
  std::uint64_t release_sequence{};
  std::uint32_t package_type{};
  Fcitx5PackageByteSlice architecture{};
  Fcitx5PackageByteSlice download_url{};
  Fcitx5PackageByteSlice sha256{};
  Fcitx5RepositoryDependencyResult* dependencies{};
  std::size_t dependency_count{};
};

struct Fcitx5RepositoryIndexResult {
  int status{};
  char error_code[64]{};
  char error_message[512]{};
  std::uint32_t format_version{};
  Fcitx5PackageByteSlice channel{};
  Fcitx5PackageByteSlice generated_at{};
  Fcitx5PackageByteSlice key_id{};
  Fcitx5RepositoryEntryResult* packages{};
  std::size_t package_count{};
};

extern "C" Fcitx5PackageStageResult fcitx5_package_stage_archive_utf16(
    const wchar_t* archive_path, std::size_t archive_path_len, const wchar_t* install_root,
    std::size_t install_root_len, const std::uint8_t* transaction_id,
    std::size_t transaction_id_len, const Fcitx5PackageTrustedKey* trusted_keys,
    std::size_t trusted_key_count);
extern "C" Fcitx5PackageStageResult fcitx5_package_stage_payload_utf16(
    const std::uint8_t* manifest_bytes, std::size_t manifest_len, const wchar_t* payload_root,
    std::size_t payload_root_len, const wchar_t* install_root, std::size_t install_root_len,
    const std::uint8_t* transaction_id, std::size_t transaction_id_len,
    const std::uint8_t* signature, std::size_t signature_len,
    const Fcitx5PackageTrustedKey* trusted_key);
extern "C" void fcitx5_package_wide_free(wchar_t* ptr, std::size_t len);
extern "C" Fcitx5RepositoryIndexResult fcitx5_repository_verify_index_struct_utf8(
    const std::uint8_t* index_data, std::size_t index_len, const std::uint8_t* signature_data,
    std::size_t signature_len, const Fcitx5PackageTrustedKey* trusted_keys,
    std::size_t trusted_key_count, const std::uint8_t* expected_channel,
    std::size_t expected_channel_len);
extern "C" Fcitx5RepositoryIndexResult fcitx5_repository_verify_index_envelope_struct_utf8(
    const std::uint8_t* index_data, std::size_t index_len, const std::uint8_t* envelope_data,
    std::size_t envelope_len, const Fcitx5PackageTrustedKey* trusted_keys,
    std::size_t trusted_key_count, const std::uint8_t* expected_channel,
    std::size_t expected_channel_len);
extern "C" Fcitx5RepositoryIndexResult fcitx5_repository_verify_index_parsed_envelope_struct_utf8(
    const std::uint8_t* index_data, std::size_t index_len, std::uint32_t format_version,
    Fcitx5PackageByteSlice signed_object, Fcitx5PackageByteSlice canonicalization,
    const Fcitx5RepositorySignatureEnvelopeEntry* signatures, std::size_t signature_count,
    const Fcitx5PackageTrustedKey* trusted_keys, std::size_t trusted_key_count,
    const std::uint8_t* expected_channel,
    std::size_t expected_channel_len);
extern "C" std::size_t fcitx5_repository_find_package_index_utf8(
    const Fcitx5RepositoryFindEntry* entries, std::size_t entry_count,
    const std::uint8_t* package_id, std::size_t package_id_len,
    const std::uint8_t* architecture, std::size_t architecture_len);
extern "C" void fcitx5_repository_index_free(const Fcitx5RepositoryIndexResult* index);

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

[[nodiscard]] inline std::string repository_string(Fcitx5PackageByteSlice bytes) {
  if (bytes.len == 0) {
    return {};
  }
  if (bytes.data == nullptr) {
    throw PackageError("invalid_repository", "verified repository payload contains invalid data");
  }
  return {reinterpret_cast<const char*>(bytes.data), bytes.len};
}

class RepositoryIndexResultGuard final {
 public:
  explicit RepositoryIndexResultGuard(Fcitx5RepositoryIndexResult result) : result_(result) {}
  ~RepositoryIndexResultGuard() { fcitx5_repository_index_free(&result_); }
  RepositoryIndexResultGuard(const RepositoryIndexResultGuard&) = delete;
  RepositoryIndexResultGuard& operator=(const RepositoryIndexResultGuard&) = delete;
  [[nodiscard]] const Fcitx5RepositoryIndexResult& get() const noexcept { return result_; }

 private:
  Fcitx5RepositoryIndexResult result_{};
};

[[nodiscard]] inline PackageType repository_package_type(std::uint32_t package_type) {
  switch (package_type) {
    case 0:
      return PackageType::core;
    case 1:
      return PackageType::addon;
    case 2:
      return PackageType::input_method_data;
    case 3:
      return PackageType::theme;
    case 4:
      return PackageType::translation;
    default:
      throw PackageError("invalid_repository",
                         "verified repository payload has an unsupported package type");
  }
}

[[nodiscard]] inline RepositoryIndex repository_result(Fcitx5RepositoryIndexResult result) {
  const RepositoryIndexResultGuard guard(result);
  const auto& verified = guard.get();
  if (verified.status != 0) {
    throw PackageError(ffi_ascii(verified.error_code, std::size(verified.error_code)),
                       ffi_ascii(verified.error_message, std::size(verified.error_message)));
  }
  RepositoryIndex parsed;
  parsed.format_version = verified.format_version;
  parsed.channel = repository_string(verified.channel);
  parsed.generated_at = repository_string(verified.generated_at);
  parsed.key_id = repository_string(verified.key_id);
  if (verified.package_count != 0 && verified.packages == nullptr) {
    throw PackageError("invalid_repository", "verified repository package data is invalid");
  }
  parsed.packages.reserve(verified.package_count);
  for (std::size_t package_index = 0; package_index < verified.package_count; ++package_index) {
    const auto& package = verified.packages[package_index];
    RepositoryEntry entry;
    entry.id = repository_string(package.id);
    entry.title = repository_string(package.title);
    entry.summary = repository_string(package.summary);
    entry.version = repository_string(package.version);
    entry.release_sequence = package.release_sequence;
    entry.type = repository_package_type(package.package_type);
    entry.architecture = repository_string(package.architecture);
    entry.download_url = repository_string(package.download_url);
    entry.sha256 = repository_string(package.sha256);
    if (package.dependency_count != 0 && package.dependencies == nullptr) {
      throw PackageError("invalid_repository", "verified repository dependency data is invalid");
    }
    entry.dependencies.reserve(package.dependency_count);
    for (std::size_t dependency_index = 0; dependency_index < package.dependency_count;
         ++dependency_index) {
      const auto& dependency = package.dependencies[dependency_index];
      entry.dependencies.push_back(
          Dependency{repository_string(dependency.id), repository_string(dependency.version)});
    }
    parsed.packages.push_back(std::move(entry));
  }
  return parsed;
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
  return detail::repository_result(detail::fcitx5_repository_verify_index_struct_utf8(
      reinterpret_cast<const std::uint8_t*>(index_bytes.data()), index_bytes.size(),
      reinterpret_cast<const std::uint8_t*>(signature.data()), signature.size(),
      key_views.data(), key_views.size(),
      reinterpret_cast<const std::uint8_t*>(expectedChannel.data()), expectedChannel.size()));
}

[[nodiscard]] inline RepositoryIndex verify_repository_index_envelope(
    std::string_view index_bytes, std::string_view envelope_bytes,
    std::span<const TrustedKey> trusted_keys, std::string_view expectedChannel) {
  const auto key_views = detail::rust_trusted_key_views(trusted_keys);
  return detail::repository_result(
      detail::fcitx5_repository_verify_index_envelope_struct_utf8(
          reinterpret_cast<const std::uint8_t*>(index_bytes.data()), index_bytes.size(),
          reinterpret_cast<const std::uint8_t*>(envelope_bytes.data()), envelope_bytes.size(),
          key_views.data(), key_views.size(),
          reinterpret_cast<const std::uint8_t*>(expectedChannel.data()), expectedChannel.size()));
}

[[nodiscard]] inline RepositoryIndex verify_repository_index(
    std::string_view index_bytes, const SignatureEnvelope& envelope,
    std::span<const TrustedKey> trusted_keys, std::string_view expectedChannel) {
  std::vector<detail::Fcitx5RepositorySignatureEnvelopeEntry> signature_views;
  signature_views.reserve(envelope.signatures.size());
  for (const auto& signature : envelope.signatures) {
    signature_views.push_back(detail::Fcitx5RepositorySignatureEnvelopeEntry{
        detail::byte_slice(signature.key_id),
        detail::byte_slice(signature.algorithm),
        detail::byte_slice(std::as_bytes(std::span(signature.signature))),
    });
  }
  const auto key_views = detail::rust_trusted_key_views(trusted_keys);
  return detail::repository_result(detail::fcitx5_repository_verify_index_parsed_envelope_struct_utf8(
      reinterpret_cast<const std::uint8_t*>(index_bytes.data()), index_bytes.size(),
      envelope.format_version, detail::byte_slice(envelope.signed_object),
      detail::byte_slice(envelope.canonicalization),
      signature_views.empty() ? nullptr : signature_views.data(), signature_views.size(),
      key_views.data(), key_views.size(),
      reinterpret_cast<const std::uint8_t*>(expectedChannel.data()), expectedChannel.size()));
}

[[nodiscard]] inline const RepositoryEntry* find_repository_package(
    const RepositoryIndex& index, std::string_view package_id,
    std::string_view architecture) noexcept {
  std::vector<detail::Fcitx5RepositoryFindEntry> entries;
  entries.reserve(index.packages.size());
  for (const auto& entry : index.packages) {
    entries.push_back(detail::Fcitx5RepositoryFindEntry{
        detail::byte_slice(entry.id),
        detail::byte_slice(entry.architecture),
    });
  }
  const auto found = detail::fcitx5_repository_find_package_index_utf8(
      entries.empty() ? nullptr : entries.data(), entries.size(),
      reinterpret_cast<const std::uint8_t*>(package_id.data()), package_id.size(),
      reinterpret_cast<const std::uint8_t*>(architecture.data()), architecture.size());
  return found < index.packages.size() ? &index.packages[found] : nullptr;
}

}  // namespace fcitx::package
