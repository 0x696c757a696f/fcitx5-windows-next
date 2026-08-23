#include "package_core.h"

#include <windows.h>
#include <bcrypt.h>
#include <wincrypt.h>

#include <algorithm>
#include <array>
#include <cstring>
#include <fstream>
#include <functional>
#include <limits>
#include <map>
#include <set>
#include <sstream>
#include <system_error>
#include <unordered_map>
#include <unordered_set>

#define MLD_CONFIG_FILE "fcitx5_mldsa65_config.h"
extern "C" {
#include <blake3.h>
#include <mldsa/mldsa_native.h>
}
#undef MLD_CONFIG_FILE

#include <nlohmann/json.hpp>

extern "C" {
struct Fcitx5ByteSlice {
  const std::uint8_t* data;
  std::size_t len;
};

struct Fcitx5TrustedKeyNative {
  Fcitx5ByteSlice id;
  Fcitx5ByteSlice algorithm;
  Fcitx5ByteSlice public_key;
  Fcitx5ByteSlice rsa_public_blob;
  std::uint8_t revoked;
};

struct Fcitx5PackageLifecycleResult {
  int status;
  std::uint8_t error_code[64];
  std::uint8_t error_message[512];
};

struct Fcitx5PackageLockEntry {
  std::uint8_t id[65];
  std::uint8_t version[65];
  std::uint8_t manifest_sha256[65];
  std::uint8_t state[33];
};

struct Fcitx5PackageLockfileResult {
  int status;
  std::uint8_t error_code[64];
  std::uint8_t error_message[512];
  Fcitx5PackageLockEntry* entries;
  std::size_t entry_count;
};

struct Fcitx5PackageTrustedKeyResult {
  int status;
  std::uint8_t error_code[64];
  std::uint8_t error_message[512];
  Fcitx5TrustedKeyNative* keys;
  std::size_t key_count;
};

struct Fcitx5PackageManifestDependency {
  Fcitx5ByteSlice id;
  Fcitx5ByteSlice version;
};

struct Fcitx5PackageManifestFile {
  Fcitx5ByteSlice path;
  std::uint64_t size;
  Fcitx5ByteSlice sha256;
  Fcitx5ByteSlice blake3;
};

struct Fcitx5PackageParsedManifestResult {
  int status;
  std::uint8_t error_code[64];
  std::uint8_t error_message[512];
  std::uint32_t format_version;
  std::uint32_t package_type;
  Fcitx5ByteSlice id;
  Fcitx5ByteSlice version;
  Fcitx5ByteSlice architecture;
  Fcitx5ByteSlice min_os;
  Fcitx5ByteSlice core_api;
  Fcitx5ByteSlice addon_abi;
  Fcitx5PackageManifestDependency* dependencies;
  std::size_t dependency_count;
  Fcitx5ByteSlice license;
  Fcitx5ByteSlice source_commit;
  Fcitx5ByteSlice* permissions;
  std::size_t permission_count;
  Fcitx5PackageManifestFile* files;
  std::size_t file_count;
  Fcitx5ByteSlice key_id;
};

struct Fcitx5RepositorySequenceRepairResult {
  std::uint8_t repaired;
};

Fcitx5PackageLockfileResult fcitx5_package_read_lockfile_utf16(
    const wchar_t* install_root, std::size_t install_root_len);
void fcitx5_package_lockfile_free(Fcitx5PackageLockEntry* entries, std::size_t entry_count);
Fcitx5PackageTrustedKeyResult fcitx5_package_read_trusted_keys_utf16(
    const wchar_t* keyring_path, std::size_t keyring_path_len);
void fcitx5_package_trusted_keys_free(Fcitx5TrustedKeyNative* keys, std::size_t key_count);
Fcitx5PackageParsedManifestResult fcitx5_package_parse_manifest_utf8(
    const std::uint8_t* manifest_bytes, std::size_t manifest_len);
void fcitx5_package_manifest_free(const Fcitx5PackageParsedManifestResult* manifest);
Fcitx5PackageLifecycleResult fcitx5_package_set_state_utf16(
    const wchar_t* install_root, std::size_t install_root_len, const std::uint8_t* package_id,
    std::size_t package_id_len, const std::uint8_t* state, std::size_t state_len);
Fcitx5PackageLifecycleResult fcitx5_package_mark_remove_utf16(
    const wchar_t* install_root, std::size_t install_root_len, const std::uint8_t* package_id,
    std::size_t package_id_len);
Fcitx5PackageLifecycleResult fcitx5_package_finalize_remove_utf16(
    const wchar_t* install_root, std::size_t install_root_len, const std::uint8_t* package_id,
    std::size_t package_id_len);
Fcitx5PackageLifecycleResult fcitx5_package_verify_installed_utf16(
    const wchar_t* install_root, std::size_t install_root_len,
    const Fcitx5TrustedKeyNative* trusted_keys, std::size_t trusted_key_count);
Fcitx5PackageLifecycleResult fcitx5_package_activate_staged_utf16(
    const wchar_t* staged_root, std::size_t staged_root_len, const wchar_t* install_root,
    std::size_t install_root_len, const Fcitx5TrustedKeyNative* trusted_keys,
    std::size_t trusted_key_count);
Fcitx5PackageLifecycleResult fcitx5_package_activate_installed_version_utf16(
    const wchar_t* install_root, std::size_t install_root_len, const std::uint8_t* package_id,
    std::size_t package_id_len, const std::uint8_t* version, std::size_t version_len,
    const Fcitx5TrustedKeyNative* trusted_keys, std::size_t trusted_key_count);
Fcitx5RepositorySequenceRepairResult fcitx5_repository_sequence_repair_utf16(
    const wchar_t* data_root, std::size_t data_root_len, const wchar_t* index_path,
    std::size_t index_path_len, const wchar_t* signature_path, std::size_t signature_path_len,
    const Fcitx5TrustedKeyNative* trusted_keys, std::size_t trusted_key_count,
    const std::uint8_t* channel, std::size_t channel_len);
}

namespace fcitx::package {
namespace {

using Json = nlohmann::json;

constexpr std::size_t kMaximumIdBytes = 64U;
constexpr std::string_view kSignatureEnvelopeCanonicalization =
    "fcitx5-windows-next-json-v1";
constexpr std::size_t kMldsa65PublicKeyBytes = MLDSA65_PUBLICKEYBYTES;
constexpr std::size_t kMldsa65SignatureBytes = MLDSA65_BYTES;

class AlgorithmHandle final {
 public:
  explicit AlgorithmHandle(BCRYPT_ALG_HANDLE handle) : handle_(handle) {}
  ~AlgorithmHandle() {
    if (handle_ != nullptr) {
      BCryptCloseAlgorithmProvider(handle_, 0);
    }
  }
  AlgorithmHandle(const AlgorithmHandle&) = delete;
  AlgorithmHandle& operator=(const AlgorithmHandle&) = delete;
  [[nodiscard]] BCRYPT_ALG_HANDLE get() const noexcept { return handle_; }

 private:
  BCRYPT_ALG_HANDLE handle_{};
};

class HashHandle final {
 public:
  explicit HashHandle(BCRYPT_HASH_HANDLE handle) : handle_(handle) {}
  ~HashHandle() {
    if (handle_ != nullptr) {
      BCryptDestroyHash(handle_);
    }
  }
  HashHandle(const HashHandle&) = delete;
  HashHandle& operator=(const HashHandle&) = delete;
  [[nodiscard]] BCRYPT_HASH_HANDLE get() const noexcept { return handle_; }

 private:
  BCRYPT_HASH_HANDLE handle_{};
};

class KeyHandle final {
 public:
  explicit KeyHandle(BCRYPT_KEY_HANDLE handle) : handle_(handle) {}
  ~KeyHandle() {
    if (handle_ != nullptr) {
      BCryptDestroyKey(handle_);
    }
  }
  KeyHandle(const KeyHandle&) = delete;
  KeyHandle& operator=(const KeyHandle&) = delete;
  [[nodiscard]] BCRYPT_KEY_HANDLE get() const noexcept { return handle_; }

 private:
  BCRYPT_KEY_HANDLE handle_{};
};

[[noreturn]] void fail(std::string code, std::string message) {
  throw PackageError(std::move(code), std::move(message));
}

std::string ffi_string(std::span<const std::uint8_t> bytes) {
  const auto end = std::ranges::find(bytes, std::uint8_t{0});
  return {reinterpret_cast<const char*>(bytes.data()),
          static_cast<std::size_t>(end - bytes.begin())};
}

std::string ffi_string(Fcitx5ByteSlice bytes) {
  if (bytes.len == 0) {
    return {};
  }
  if (bytes.data == nullptr) {
    fail("invalid_keyring", "trusted key set is invalid");
  }
  return {reinterpret_cast<const char*>(bytes.data), bytes.len};
}

std::vector<std::byte> ffi_bytes(Fcitx5ByteSlice bytes) {
  if (bytes.len == 0) {
    return {};
  }
  if (bytes.data == nullptr) {
    fail("invalid_keyring", "trusted key set is invalid");
  }
  const auto* begin = reinterpret_cast<const std::byte*>(bytes.data);
  return {begin, begin + bytes.len};
}

std::string ffi_manifest_string(Fcitx5ByteSlice bytes) {
  if (bytes.len == 0) {
    return {};
  }
  if (bytes.data == nullptr) {
    fail("invalid_manifest", "parsed manifest contains invalid string data");
  }
  return {reinterpret_cast<const char*>(bytes.data), bytes.len};
}

Fcitx5ByteSlice ffi_slice(std::string_view value) {
  return {reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

Fcitx5ByteSlice ffi_slice(std::span<const std::byte> value) {
  return {reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

std::vector<Fcitx5TrustedKeyNative> rust_trusted_key_views(
    std::span<const TrustedKey> trusted_keys) {
  std::vector<Fcitx5TrustedKeyNative> key_views;
  key_views.reserve(trusted_keys.size());
  for (const auto& key : trusted_keys) {
    key_views.push_back(Fcitx5TrustedKeyNative{
        ffi_slice(key.id),
        ffi_slice(key.algorithm),
        ffi_slice(std::span<const std::byte>(key.public_key.data(), key.public_key.size())),
        ffi_slice(std::span<const std::byte>(key.rsa_public_blob.data(),
                                             key.rsa_public_blob.size())),
        key.revoked ? std::uint8_t{1} : std::uint8_t{0},
    });
  }
  return key_views;
}

void require_lifecycle_ok(const Fcitx5PackageLifecycleResult& result) {
  if (result.status == 0) {
    return;
  }
  std::string code = ffi_string(result.error_code);
  std::string message = ffi_string(result.error_message);
  if (code.empty()) {
    code = "lifecycle_failed";
  }
  if (message.empty()) {
    message = "package lifecycle operation failed";
  }
  fail(std::move(code), std::move(message));
}

class TrustedKeyResultGuard final {
 public:
  explicit TrustedKeyResultGuard(Fcitx5PackageTrustedKeyResult result) : result_(result) {}
  ~TrustedKeyResultGuard() {
    fcitx5_package_trusted_keys_free(result_.keys, result_.key_count);
  }
  TrustedKeyResultGuard(const TrustedKeyResultGuard&) = delete;
  TrustedKeyResultGuard& operator=(const TrustedKeyResultGuard&) = delete;
  [[nodiscard]] const Fcitx5PackageTrustedKeyResult& get() const noexcept { return result_; }

 private:
  Fcitx5PackageTrustedKeyResult result_{};
};

class ManifestResultGuard final {
 public:
  explicit ManifestResultGuard(Fcitx5PackageParsedManifestResult result) : result_(result) {}
  ~ManifestResultGuard() { fcitx5_package_manifest_free(&result_); }
  ManifestResultGuard(const ManifestResultGuard&) = delete;
  ManifestResultGuard& operator=(const ManifestResultGuard&) = delete;
  [[nodiscard]] const Fcitx5PackageParsedManifestResult& get() const noexcept { return result_; }

 private:
  Fcitx5PackageParsedManifestResult result_{};
};

std::wstring native_path_string(const std::filesystem::path& path) {
  return path.native();
}

void require_nt_success(NTSTATUS status, std::string_view operation) {
  if (status < 0) {
    fail("crypto_error", std::string(operation) + " failed");
  }
}

void require_signature_object_keys(const Json& object,
                                   std::initializer_list<std::string_view> required) {
  if (!object.is_object()) {
    fail("invalid_signature", "signature envelope entry must be a JSON object");
  }
  std::set<std::string, std::less<>> allowed;
  for (const auto key : required) {
    allowed.emplace(key);
    if (!object.contains(key)) {
      fail("invalid_signature", "signature envelope is missing required key: " +
                                    std::string(key));
    }
  }
  for (const auto& [key, unused] : object.items()) {
    static_cast<void>(unused);
    if (!allowed.contains(key)) {
      fail("invalid_signature", "signature envelope has unknown key: " + key);
    }
  }
}

std::string require_signature_string(const Json& object, std::string_view key,
                                     std::size_t maximum) {
  const auto& value = object.at(key);
  if (!value.is_string()) {
    fail("invalid_signature", std::string(key) + " must be a string");
  }
  auto result = value.get<std::string>();
  if (result.empty() || result.size() > maximum || result.find('\0') != std::string::npos) {
    fail("invalid_signature", std::string(key) + " has an invalid length");
  }
  return result;
}

bool contains_reparse_component(const std::filesystem::path& path) {
  std::filesystem::path current;
  for (const auto& component : path) {
    current /= component;
    const DWORD attributes = GetFileAttributesW(current.c_str());
    if (attributes != INVALID_FILE_ATTRIBUTES &&
        (attributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0U) {
      return true;
    }
  }
  return false;
}

std::string read_file_bounded(const std::filesystem::path& path, std::size_t maximum) {
  std::error_code error;
  const auto size = std::filesystem::file_size(path, error);
  if (error || size > maximum) {
    fail("invalid_file", "file is missing or exceeds its resource budget");
  }
  std::ifstream input(path, std::ios::binary);
  if (!input) {
    fail("io_error", "unable to open file");
  }
  std::string bytes(static_cast<std::size_t>(size), '\0');
  if (!bytes.empty()) {
    input.read(bytes.data(), static_cast<std::streamsize>(bytes.size()));
  }
  if (!input) {
    fail("io_error", "unable to read complete file");
  }
  return bytes;
}

}  // namespace

// Validators declared in package_core.h and used by the updater and deployer
// for CLI-supplied identifiers that later become filesystem paths.
bool is_ascii_token(std::string_view value, std::string_view extra) noexcept {
  return !value.empty() && std::ranges::all_of(value, [extra](char raw_character) {
           const auto character = static_cast<unsigned char>(raw_character);
           return (character >= 'a' && character <= 'z') ||
                  (character >= 'A' && character <= 'Z') ||
                  (character >= '0' && character <= '9') ||
                  extra.find(static_cast<char>(character)) != std::string_view::npos;
         });
}

bool is_lower_package_id(std::string_view value) noexcept {
  if (value.empty() || value.size() > kMaximumIdBytes || value.front() < 'a' ||
      value.front() > 'z') {
    return false;
  }
  return std::ranges::all_of(value, [](char raw_character) {
    const auto character = static_cast<unsigned char>(raw_character);
    return (character >= 'a' && character <= 'z') ||
           (character >= '0' && character <= '9') || character == '.' || character == '-' ||
           character == '_';
  });
}

PackageError::PackageError(std::string code, std::string message)
    : std::runtime_error(std::move(message)), code_(std::move(code)) {}

const std::string& PackageError::code() const noexcept { return code_; }

Manifest parse_manifest(std::string_view bytes) {
  const ManifestResultGuard guard(fcitx5_package_parse_manifest_utf8(
      reinterpret_cast<const std::uint8_t*>(bytes.data()), bytes.size()));
  const auto& manifest_result = guard.get();
  if (manifest_result.status != 0) {
    std::string code = ffi_string(manifest_result.error_code);
    std::string message = ffi_string(manifest_result.error_message);
    if (code.empty()) {
      code = "invalid_manifest";
    }
    if (message.empty()) {
      message = "manifest is invalid";
    }
    fail(std::move(code), std::move(message));
  }
  Manifest result;
  result.format_version = manifest_result.format_version;
  switch (manifest_result.package_type) {
    case 0:
      result.type = PackageType::core;
      break;
    case 1:
      result.type = PackageType::addon;
      break;
    case 2:
      result.type = PackageType::input_method_data;
      break;
    case 3:
      result.type = PackageType::theme;
      break;
    case 4:
      result.type = PackageType::translation;
      break;
    default:
      fail("invalid_manifest", "parsed manifest has an unsupported package type");
  }
  result.id = ffi_manifest_string(manifest_result.id);
  result.version = ffi_manifest_string(manifest_result.version);
  result.architecture = ffi_manifest_string(manifest_result.architecture);
  result.min_os = ffi_manifest_string(manifest_result.min_os);
  result.core_api = ffi_manifest_string(manifest_result.core_api);
  result.addon_abi = ffi_manifest_string(manifest_result.addon_abi);
  result.license = ffi_manifest_string(manifest_result.license);
  result.source_commit = ffi_manifest_string(manifest_result.source_commit);
  result.key_id = ffi_manifest_string(manifest_result.key_id);
  if (manifest_result.dependency_count != 0 && manifest_result.dependencies == nullptr) {
    fail("invalid_manifest", "parsed manifest dependency data is invalid");
  }
  result.dependencies.reserve(manifest_result.dependency_count);
  for (std::size_t index = 0; index < manifest_result.dependency_count; ++index) {
    const auto& dependency = manifest_result.dependencies[index];
    result.dependencies.push_back(
        Dependency{ffi_manifest_string(dependency.id), ffi_manifest_string(dependency.version)});
  }
  if (manifest_result.permission_count != 0 && manifest_result.permissions == nullptr) {
    fail("invalid_manifest", "parsed manifest permission data is invalid");
  }
  result.permissions.reserve(manifest_result.permission_count);
  for (std::size_t index = 0; index < manifest_result.permission_count; ++index) {
    result.permissions.push_back(ffi_manifest_string(manifest_result.permissions[index]));
  }
  if (manifest_result.file_count != 0 && manifest_result.files == nullptr) {
    fail("invalid_manifest", "parsed manifest file data is invalid");
  }
  result.files.reserve(manifest_result.file_count);
  for (std::size_t index = 0; index < manifest_result.file_count; ++index) {
    const auto& file = manifest_result.files[index];
    result.files.push_back(FileEntry{
        ffi_manifest_string(file.path),
        file.size,
        ffi_manifest_string(file.sha256),
        ffi_manifest_string(file.blake3),
    });
  }
  return result;
}

void validate_manifest_compatibility(const Manifest& manifest,
                                     std::string_view architecture) {
  if (architecture != "x64" && architecture != "x86")
    fail("incompatible_package", "runtime architecture is invalid");
  if (manifest.architecture != "any" && manifest.architecture != architecture)
    fail("incompatible_package", "package architecture does not match this runtime");
  if (manifest.core_api != kSupportedCoreApi)
    fail("incompatible_package", "package requires an unsupported Core API");
  if (manifest.type == PackageType::addon && manifest.addon_abi != kSupportedAddonAbi)
    fail("incompatible_package", "addon ABI does not match this engine");
}

bool is_safe_relative_package_path(std::string_view path) noexcept {
  if (path.empty() || path.size() > 512U || path.front() == '/' || path.front() == '\\' ||
      path.back() == '/' || path.back() == '\\' || path.find(':') != std::string_view::npos ||
      path.find('\0') != std::string_view::npos || path.find('\\') != std::string_view::npos) {
    return false;
  }
  std::size_t begin = 0;
  while (begin < path.size()) {
    const auto end = path.find('/', begin);
    const auto component = path.substr(begin, end == std::string_view::npos ? path.size() - begin
                                                                          : end - begin);
    if (component.empty() || component == "." || component == ".." ||
        component.back() == '.' || component.back() == ' ') {
      return false;
    }
    for (const unsigned char character : component) {
      if (character < 0x20U)
        return false;
    }
    auto stem = component.substr(0, component.find('.'));
    std::string lowered;
    lowered.reserve(stem.size());
    for (const unsigned char character : stem) {
      lowered.push_back(static_cast<char>(
          character >= 'A' && character <= 'Z' ? character - 'A' + 'a' : character));
    }
    if (lowered == "con" || lowered == "prn" || lowered == "aux" || lowered == "nul" ||
        (lowered.size() == 4U &&
         ((lowered.starts_with("com") && lowered[3] >= '1' && lowered[3] <= '9') ||
          (lowered.starts_with("lpt") && lowered[3] >= '1' && lowered[3] <= '9')))) {
      return false;
    }
    begin = end == std::string_view::npos ? path.size() : end + 1U;
  }
  return true;
}

bool path_contains_reparse_point(const std::filesystem::path& path) {
  return contains_reparse_component(path);
}

std::array<std::byte, 32> sha256(std::span<const std::byte> bytes) {
  BCRYPT_ALG_HANDLE raw_algorithm = nullptr;
  require_nt_success(BCryptOpenAlgorithmProvider(&raw_algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0),
                     "BCryptOpenAlgorithmProvider");
  AlgorithmHandle algorithm(raw_algorithm);
  BCRYPT_HASH_HANDLE raw_hash = nullptr;
  require_nt_success(BCryptCreateHash(algorithm.get(), &raw_hash, nullptr, 0, nullptr, 0, 0),
                     "BCryptCreateHash");
  HashHandle hash(raw_hash);
  if (!bytes.empty()) {
    require_nt_success(
        BCryptHashData(hash.get(), reinterpret_cast<PUCHAR>(const_cast<std::byte*>(bytes.data())),
                       static_cast<ULONG>(bytes.size()), 0),
        "BCryptHashData");
  }
  std::array<std::byte, 32> result{};
  require_nt_success(BCryptFinishHash(hash.get(), reinterpret_cast<PUCHAR>(result.data()),
                                     static_cast<ULONG>(result.size()), 0),
                     "BCryptFinishHash");
  return result;
}

std::array<std::byte, 32> sha256_file(const std::filesystem::path& path) {
  std::error_code error;
  const auto size = std::filesystem::file_size(path, error);
  if (error || size > kMaximumPayloadBytes) {
    fail("invalid_file", "file is missing or exceeds the hashing budget");
  }
  const auto bytes = read_file_bounded(path, static_cast<std::size_t>(size));
  return sha256(std::as_bytes(std::span(bytes)));
}

std::string hex_sha256(std::span<const std::byte> digest) {
  constexpr std::string_view digits = "0123456789abcdef";
  std::string result;
  result.reserve(digest.size() * 2U);
  for (const auto value : digest) {
    const auto byte = std::to_integer<unsigned int>(value);
    result.push_back(digits[byte >> 4U]);
    result.push_back(digits[byte & 0x0FU]);
  }
  return result;
}

std::array<std::byte, 32> blake3(std::span<const std::byte> bytes) {
  blake3_hasher hasher{};
  blake3_hasher_init(&hasher);
  blake3_hasher_update(&hasher, bytes.data(), bytes.size());
  std::array<std::byte, 32> result{};
  blake3_hasher_finalize(&hasher, reinterpret_cast<std::uint8_t*>(result.data()),
                         result.size());
  return result;
}

std::array<std::byte, 32> blake3_file(const std::filesystem::path& path) {
  const auto bytes = read_file_bounded(path, kMaximumFileBytes);
  return blake3(std::as_bytes(std::span(bytes)));
}

std::string hex_blake3(std::span<const std::byte> digest) {
  return hex_sha256(digest);
}

std::vector<std::byte> decode_base64(std::string_view encoded) {
  if (encoded.empty() || encoded.size() > 16384U) {
    fail("invalid_signature", "base64 value is empty or too large");
  }
  DWORD output_size = 0;
  if (!CryptStringToBinaryA(encoded.data(), static_cast<DWORD>(encoded.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_STRICT, nullptr, &output_size,
                            nullptr, nullptr)) {
    fail("invalid_signature", "base64 value is malformed");
  }
  std::vector<std::byte> result(output_size);
  if (!CryptStringToBinaryA(encoded.data(), static_cast<DWORD>(encoded.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_STRICT,
                            reinterpret_cast<BYTE*>(result.data()), &output_size, nullptr,
                            nullptr)) {
    fail("invalid_signature", "base64 decoding failed");
  }
  result.resize(output_size);
  return result;
}

SignatureEnvelope parse_signature_envelope(std::string_view bytes,
                                           std::string_view expected_object) {
  if (bytes.empty() || bytes.size() > kMaximumManifestBytes ||
      (expected_object != "repository-index" && expected_object != "package-manifest")) {
    fail("invalid_signature", "signature envelope identity is invalid");
  }

  Json document;
  try {
    document = Json::parse(bytes);
  } catch (const Json::exception&) {
    fail("invalid_signature", "signature envelope is not strict JSON");
  }

  require_signature_object_keys(document,
                                {"format_version", "signed_object", "canonicalization",
                                 "signatures"});
  if (!document["format_version"].is_number_unsigned() ||
      document["format_version"].get<std::uint32_t>() != 2U) {
    fail("invalid_signature", "signature envelope format version is unsupported");
  }

  SignatureEnvelope result;
  result.format_version = 2U;
  result.signed_object = require_signature_string(document, "signed_object", 64U);
  result.canonicalization = require_signature_string(document, "canonicalization", 64U);
  if (result.signed_object != expected_object ||
      (result.signed_object != "repository-index" &&
       result.signed_object != "package-manifest") ||
      result.canonicalization != kSignatureEnvelopeCanonicalization) {
    fail("invalid_signature", "signature envelope object binding is invalid");
  }

  const auto& signatures = document["signatures"];
  if (!signatures.is_array() || signatures.empty() || signatures.size() > 16U) {
    fail("invalid_signature", "signature envelope signatures array is invalid");
  }

  std::set<std::string, std::less<>> key_ids;
  bool has_required_mldsa65 = false;
  for (const auto& item : signatures) {
    require_signature_object_keys(item, {"key_id", "algorithm", "signature_base64"});
    auto key_id = require_signature_string(item, "key_id", kMaximumIdBytes);
    auto algorithm = require_signature_string(item, "algorithm", 32U);
    if (!is_lower_package_id(key_id) || !key_ids.emplace(key_id).second) {
      fail("invalid_signature", "signature envelope key id is invalid or duplicated");
    }
    if (algorithm != "mldsa65" && algorithm != "slhdsa-sha2-128s") {
      fail("invalid_signature", "signature envelope requires an unsupported algorithm");
    }
    if (algorithm == "mldsa65") {
      has_required_mldsa65 = true;
    }
    auto signature = decode_base64(
        require_signature_string(item, "signature_base64", 16384U));
    result.signatures.push_back(SignatureEnvelopeEntry{
        std::move(key_id), std::move(algorithm), std::move(signature)});
  }

  if (!has_required_mldsa65) {
    fail("invalid_signature", "signature envelope is missing required ML-DSA-65 signature");
  }
  return result;
}

SignatureEnvelope read_signature_envelope(const std::filesystem::path& path,
                                          std::string_view expected_object) {
  return parse_signature_envelope(read_file_bounded(path, kMaximumManifestBytes),
                                  expected_object);
}

void verify_manifest_signature(std::string_view manifest_bytes,
                               std::span<const std::byte> signature,
                               const TrustedKey& key) {
  if (key.revoked) {
    fail("revoked_key", "manifest key is revoked");
  }
  if (key.id.empty() || key.rsa_public_blob.empty() || signature.empty()) {
    fail("invalid_signature", "signature identity is incomplete");
  }
  BCRYPT_ALG_HANDLE raw_algorithm = nullptr;
  require_nt_success(BCryptOpenAlgorithmProvider(&raw_algorithm, BCRYPT_RSA_ALGORITHM, nullptr, 0),
                     "BCryptOpenAlgorithmProvider");
  AlgorithmHandle algorithm(raw_algorithm);
  BCRYPT_KEY_HANDLE raw_key = nullptr;
  require_nt_success(
      BCryptImportKeyPair(algorithm.get(), nullptr, BCRYPT_RSAPUBLIC_BLOB, &raw_key,
                          reinterpret_cast<PUCHAR>(
                              const_cast<std::byte*>(key.rsa_public_blob.data())),
                          static_cast<ULONG>(key.rsa_public_blob.size()), 0),
      "BCryptImportKeyPair");
  KeyHandle imported_key(raw_key);
  const auto digest = sha256(std::as_bytes(std::span(manifest_bytes)));
  BCRYPT_PKCS1_PADDING_INFO padding{BCRYPT_SHA256_ALGORITHM};
  const auto status = BCryptVerifySignature(
      imported_key.get(), &padding,
      reinterpret_cast<PUCHAR>(const_cast<std::byte*>(digest.data())),
      static_cast<ULONG>(digest.size()),
      reinterpret_cast<PUCHAR>(const_cast<std::byte*>(signature.data())),
      static_cast<ULONG>(signature.size()), BCRYPT_PAD_PKCS1);
  if (status < 0) {
    fail("invalid_signature", "manifest signature verification failed");
  }
}

void verify_mldsa65_signature(std::string_view object_bytes,
                              std::span<const std::byte> signature,
                              const TrustedKey& key) {
  if (key.revoked) {
    fail("revoked_key", "ML-DSA key is revoked");
  }
  if (key.algorithm != "mldsa65" || key.public_key.size() != kMldsa65PublicKeyBytes ||
      signature.size() != kMldsa65SignatureBytes || object_bytes.empty()) {
    fail("invalid_signature", "ML-DSA signature identity is incomplete");
  }
  const auto* message = reinterpret_cast<const std::uint8_t*>(object_bytes.data());
  const auto* signature_bytes = reinterpret_cast<const std::uint8_t*>(signature.data());
  const auto* public_key = reinterpret_cast<const std::uint8_t*>(key.public_key.data());
  if (fcitx5_mldsa65_verify(signature_bytes, message, object_bytes.size(), nullptr, 0U,
                            public_key) != 0) {
    fail("invalid_signature", "ML-DSA-65 signature verification failed");
  }
}

void verify_signature_envelope(std::string_view object_bytes,
                               const SignatureEnvelope& envelope,
                               std::span<const TrustedKey> trusted_keys,
                               std::string_view expected_object,
                               std::string_view expected_key_id) {
  if (envelope.format_version != 2U || envelope.signed_object != expected_object ||
      envelope.canonicalization != kSignatureEnvelopeCanonicalization ||
      !is_lower_package_id(expected_key_id)) {
    fail("invalid_signature", "signature envelope binding is invalid");
  }
  bool saw_required_key = false;
  for (const auto& entry : envelope.signatures) {
    if (entry.algorithm != "mldsa65") {
      continue;
    }
    if (entry.key_id != expected_key_id) {
      fail("untrusted_key", "ML-DSA signature key id does not match signed metadata");
    }
    saw_required_key = true;
    const auto trusted_key = std::ranges::find_if(trusted_keys, [&](const TrustedKey& candidate) {
      return candidate.id == entry.key_id;
    });
    if (trusted_key == trusted_keys.end()) {
      fail("untrusted_key", "ML-DSA signature key is not trusted");
    }
    verify_mldsa65_signature(object_bytes, std::span(entry.signature), *trusted_key);
    return;
  }
  if (!saw_required_key) {
    fail("invalid_signature", "signature envelope has no required ML-DSA signature");
  }
}

void verify_manifest_signature_envelope(std::string_view manifest_bytes,
                                        const SignatureEnvelope& envelope,
                                        std::span<const TrustedKey> trusted_keys) {
  const auto manifest = parse_manifest(manifest_bytes);
  verify_signature_envelope(manifest_bytes, envelope, trusted_keys, "package-manifest",
                            manifest.key_id);
}

void verify_payload(const Manifest& manifest, const std::filesystem::path& payload_root) {
  if (!std::filesystem::is_directory(payload_root) ||
      path_contains_reparse_point(payload_root)) {
    fail("unsafe_payload", "payload root is missing or contains a reparse point");
  }
  std::set<std::string, std::less<>> expected;
  for (const auto& file : manifest.files) {
    expected.emplace(file.path);
    const auto path = payload_root / std::filesystem::path(file.path);
    if (path_contains_reparse_point(path) || !std::filesystem::is_regular_file(path) ||
        std::filesystem::file_size(path) != file.size) {
      fail("payload_mismatch", "payload file does not match manifest: " + file.path);
    }
    if (manifest.format_version == kManifestFormatVersion) {
      if (hex_sha256(sha256_file(path)) != file.sha256) {
        fail("payload_mismatch", "payload file does not match manifest: " + file.path);
      }
    } else if (manifest.format_version == kManifestV2FormatVersion) {
      if (hex_blake3(blake3_file(path)) != file.blake3 ||
          (!file.sha256.empty() && hex_sha256(sha256_file(path)) != file.sha256)) {
        fail("payload_mismatch", "payload file does not match manifest: " + file.path);
      }
    } else {
      fail("invalid_manifest", "payload verifier does not support manifest version");
    }
  }
  std::error_code error;
  for (std::filesystem::recursive_directory_iterator iterator(
           payload_root, std::filesystem::directory_options::none, error),
       end;
       iterator != end; iterator.increment(error)) {
    if (error) {
      fail("unsafe_payload", "payload directory cannot be enumerated safely");
    }
    if (iterator->is_symlink(error) ||
        (GetFileAttributesW(iterator->path().c_str()) & FILE_ATTRIBUTE_REPARSE_POINT) != 0U) {
      fail("unsafe_payload", "payload contains a reparse point");
    }
    if (iterator->is_regular_file(error)) {
      const auto relative = std::filesystem::relative(iterator->path(), payload_root, error);
      if (error || !expected.contains(relative.generic_string())) {
        fail("payload_mismatch", "payload contains an undeclared file");
      }
    }
  }
}

std::vector<std::string> resolve_exact_dependencies(
    const std::vector<Manifest>& available, const std::vector<std::string>& requested_ids) {
  std::map<std::string, const Manifest*, std::less<>> packages;
  for (const auto& package : available) {
    if (!packages.emplace(package.id, &package).second) {
      fail("resolution_failed", "repository contains duplicate package id");
    }
  }
  enum class Visit { visiting, complete };
  std::unordered_map<std::string, Visit> visits;
  std::vector<std::string> result;
  std::function<void(const std::string&)> visit = [&](const std::string& id) {
    const auto state = visits.find(id);
    if (state != visits.end()) {
      if (state->second == Visit::visiting) {
        fail("resolution_failed", "dependency cycle detected");
      }
      return;
    }
    const auto found = packages.find(id);
    if (found == packages.end()) {
      fail("resolution_failed", "required package is unavailable: " + id);
    }
    visits.emplace(id, Visit::visiting);
    for (const auto& dependency : found->second->dependencies) {
      const auto target = packages.find(dependency.id);
      if (target == packages.end() || target->second->version != dependency.version) {
        fail("resolution_failed", "exact dependency version is unavailable");
      }
      visit(dependency.id);
    }
    visits[id] = Visit::complete;
    result.push_back(id);
  };
  for (const auto& id : requested_ids) {
    visit(id);
  }
  return result;
}

std::filesystem::path stage_verified_payload(
    const Manifest& manifest, std::string_view manifest_bytes,
    const std::filesystem::path& payload_root, const std::filesystem::path& install_root,
    std::string_view transaction_id, std::span<const std::byte> signature,
    const TrustedKey& trusted_key) {
  static_cast<void>(manifest);
  const detail::Fcitx5PackageTrustedKey trusted_key_view{
      detail::byte_slice(trusted_key.id),
      detail::byte_slice(trusted_key.algorithm),
      detail::byte_slice(std::span<const std::byte>(trusted_key.public_key.data(),
                                                    trusted_key.public_key.size())),
      detail::byte_slice(std::span<const std::byte>(trusted_key.rsa_public_blob.data(),
                                                    trusted_key.rsa_public_blob.size())),
      trusted_key.revoked ? std::uint8_t{1} : std::uint8_t{0},
  };
  const std::wstring payload = native_path_string(payload_root);
  const std::wstring root = native_path_string(install_root);
  const auto result = detail::fcitx5_package_stage_payload_utf16(
      reinterpret_cast<const std::uint8_t*>(manifest_bytes.data()), manifest_bytes.size(),
      payload.data(), payload.size(), root.data(), root.size(),
      reinterpret_cast<const std::uint8_t*>(transaction_id.data()), transaction_id.size(),
      reinterpret_cast<const std::uint8_t*>(signature.data()), signature.size(), &trusted_key_view);
  if (result.status != 0) {
    std::string code = detail::ffi_ascii(result.error_code);
    std::string message = detail::ffi_ascii(result.error_message);
    fail(code.empty() ? "invalid_package" : std::move(code),
         message.empty() ? "package payload staging failed" : std::move(message));
  }
  std::filesystem::path staged;
  if (result.staged_path && result.staged_path_len != 0) {
    staged = std::filesystem::path(
        std::wstring(result.staged_path, result.staged_path + result.staged_path_len));
  }
  detail::fcitx5_package_wide_free(result.staged_path, result.staged_path_len);
  return staged;
}

std::vector<LockEntry> read_lockfile(const std::filesystem::path& install_root) {
  const std::wstring root = native_path_string(install_root);
  const auto lockfile_result = fcitx5_package_read_lockfile_utf16(root.data(), root.size());
  if (lockfile_result.status != 0) {
    std::string code = ffi_string(lockfile_result.error_code);
    std::string message = ffi_string(lockfile_result.error_message);
    if (code.empty()) {
      code = "invalid_lockfile";
    }
    if (message.empty()) {
      message = "packages.lock read failed";
    }
    fail(std::move(code), std::move(message));
  }
  std::vector<LockEntry> result;
  result.reserve(lockfile_result.entry_count);
  for (std::size_t index = 0; index < lockfile_result.entry_count; ++index) {
    const auto& entry = lockfile_result.entries[index];
    result.push_back(LockEntry{
        ffi_string(entry.id),
        ffi_string(entry.version),
        ffi_string(entry.manifest_sha256),
        ffi_string(entry.state),
    });
  }
  fcitx5_package_lockfile_free(lockfile_result.entries, lockfile_result.entry_count);
  return result;
}

void activate_staged_package(const std::filesystem::path& staged_root,
                             const std::filesystem::path& install_root,
                             std::span<const TrustedKey> trusted_keys) {
  const auto key_views = rust_trusted_key_views(trusted_keys);
  const std::wstring staged = native_path_string(staged_root);
  const std::wstring root = native_path_string(install_root);
  require_lifecycle_ok(fcitx5_package_activate_staged_utf16(
      staged.data(), staged.size(), root.data(), root.size(),
      key_views.empty() ? nullptr : key_views.data(), key_views.size()));
}

std::vector<TrustedKey> read_trusted_keys(const std::filesystem::path& path) {
  const std::wstring keyring = native_path_string(path);
  const TrustedKeyResultGuard guard(
      fcitx5_package_read_trusted_keys_utf16(keyring.data(), keyring.size()));
  const auto& keyring_result = guard.get();
  if (keyring_result.status != 0) {
    std::string code = ffi_string(keyring_result.error_code);
    std::string message = ffi_string(keyring_result.error_message);
    if (code.empty()) {
      code = "invalid_keyring";
    }
    if (message.empty()) {
      message = "trusted key file is invalid";
    }
    fail(std::move(code), std::move(message));
  }
  std::vector<TrustedKey> result;
  result.reserve(keyring_result.key_count);
  for (std::size_t index = 0; index < keyring_result.key_count; ++index) {
    const auto& key = keyring_result.keys[index];
    result.push_back(TrustedKey{
        ffi_string(key.id),
        ffi_string(key.algorithm),
        ffi_bytes(key.public_key),
        key.revoked != 0,
    });
  }
  return result;
}

void verify_installed_packages(const std::filesystem::path& install_root,
                               std::span<const TrustedKey> trusted_keys) {
  const auto key_views = rust_trusted_key_views(trusted_keys);
  const std::wstring root = native_path_string(install_root);
  require_lifecycle_ok(fcitx5_package_verify_installed_utf16(
      root.data(), root.size(), key_views.empty() ? nullptr : key_views.data(),
      key_views.size()));
}

std::string repair_repository_sequence_state(const std::filesystem::path& data_root,
                                             const std::filesystem::path& index_path,
                                             const std::filesystem::path& signature_path,
                                             std::span<const TrustedKey> trusted_keys,
                                             std::string_view channel) {
  const auto key_views = rust_trusted_key_views(trusted_keys);
  const std::wstring root = native_path_string(data_root);
  const std::wstring index = native_path_string(index_path);
  const std::wstring signature = native_path_string(signature_path);
  const auto result = fcitx5_repository_sequence_repair_utf16(
      root.data(), root.size(), index.data(), index.size(), signature.data(), signature.size(),
      key_views.empty() ? nullptr : key_views.data(), key_views.size(),
      reinterpret_cast<const std::uint8_t*>(channel.data()), channel.size());
  return result.repaired != 0 ? "repaired" : "reset";
}

void set_package_state(const std::filesystem::path& install_root, std::string_view package_id,
                       std::string_view state) {
  const std::wstring root = native_path_string(install_root);
  require_lifecycle_ok(fcitx5_package_set_state_utf16(
      root.data(), root.size(), reinterpret_cast<const std::uint8_t*>(package_id.data()),
      package_id.size(), reinterpret_cast<const std::uint8_t*>(state.data()), state.size()));
}

void mark_package_for_removal(const std::filesystem::path& install_root,
                              std::string_view package_id) {
  const std::wstring root = native_path_string(install_root);
  require_lifecycle_ok(fcitx5_package_mark_remove_utf16(
      root.data(), root.size(), reinterpret_cast<const std::uint8_t*>(package_id.data()),
      package_id.size()));
}

void finalize_package_removal(const std::filesystem::path& install_root,
                              std::string_view package_id) {
  const std::wstring root = native_path_string(install_root);
  require_lifecycle_ok(fcitx5_package_finalize_remove_utf16(
      root.data(), root.size(), reinterpret_cast<const std::uint8_t*>(package_id.data()),
      package_id.size()));
}

void activate_installed_version(const std::filesystem::path& install_root,
                                std::string_view package_id, std::string_view version,
                                std::span<const TrustedKey> trusted_keys) {
  const auto key_views = rust_trusted_key_views(trusted_keys);
  const std::wstring root = native_path_string(install_root);
  require_lifecycle_ok(fcitx5_package_activate_installed_version_utf16(
      root.data(), root.size(), reinterpret_cast<const std::uint8_t*>(package_id.data()),
      package_id.size(), reinterpret_cast<const std::uint8_t*>(version.data()), version.size(),
      key_views.empty() ? nullptr : key_views.data(), key_views.size()));
}

}  // namespace fcitx::package
