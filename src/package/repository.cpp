#include "package_core.h"

#include <Windows.h>
#include <wincrypt.h>

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <span>
#include <ranges>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include <nlohmann/json.hpp>

namespace fcitx::package {
namespace {

using Json = nlohmann::json;

[[noreturn]] void repository_fail(std::string message) {
  throw PackageError("invalid_repository", std::move(message));
}

[[nodiscard]] std::string json_string(std::string_view value) {
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
          repository_fail("verified repository envelope contains a control character");
        }
        result.push_back(static_cast<char>(character));
        break;
    }
  }
  result.push_back('"');
  return result;
}

[[nodiscard]] std::string base64(std::span<const std::byte> value) {
  DWORD size = 0;
  if (!CryptBinaryToStringA(reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, nullptr, &size)) {
    repository_fail("repository signature envelope encoding failed");
  }
  std::string result(size, '\0');
  if (!CryptBinaryToStringA(reinterpret_cast<const BYTE*>(value.data()),
                            static_cast<DWORD>(value.size()),
                            CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, result.data(), &size)) {
    repository_fail("repository signature envelope encoding failed");
  }
  result.resize(size);
  return result;
}

struct RepositoryBlob {
  std::uint8_t* data{};
  std::size_t len{};
};

struct RepositoryResult {
  int status{};
  char error_code[64]{};
  char error_message[512]{};
  RepositoryBlob blob{};
};

struct FfiByteSlice {
  const std::uint8_t* data{};
  std::size_t len{};
};

struct FfiTrustedKey {
  FfiByteSlice id{};
  FfiByteSlice algorithm{};
  FfiByteSlice public_key{};
  FfiByteSlice rsa_public_blob{};
  std::uint8_t revoked{};
};

extern "C" RepositoryResult fcitx5_repository_verify_index_utf8(
    const std::uint8_t* index_data, std::size_t index_len, const std::uint8_t* signature_data,
    std::size_t signature_len, const FfiTrustedKey* trusted_keys, std::size_t trusted_key_count,
    const std::uint8_t* expected_channel, std::size_t expected_channel_len);
extern "C" RepositoryResult fcitx5_repository_verify_index_envelope_utf8(
    const std::uint8_t* index_data, std::size_t index_len, const std::uint8_t* envelope_data,
    std::size_t envelope_len, const FfiTrustedKey* trusted_keys, std::size_t trusted_key_count,
    const std::uint8_t* expected_channel, std::size_t expected_channel_len);
extern "C" void fcitx5_repository_blob_free(std::uint8_t* data, std::size_t len);

struct RustTrustedKey {
  std::string id;
  std::string algorithm;
  std::vector<std::byte> public_key;
  std::vector<std::byte> rsa_public_blob;
  bool revoked{};
};

[[nodiscard]] FfiByteSlice slice_of(const std::string& value) {
  return {reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

[[nodiscard]] FfiByteSlice slice_of(std::span<const std::byte> value) {
  return {reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

[[nodiscard]] FfiTrustedKey to_ffi(const TrustedKey& key) {
  return {slice_of(key.id), slice_of(key.algorithm), slice_of(key.public_key),
          slice_of(key.rsa_public_blob), static_cast<std::uint8_t>(key.revoked)};
}

[[nodiscard]] std::vector<FfiTrustedKey> to_ffi(std::span<const TrustedKey> keys) {
  std::vector<FfiTrustedKey> result;
  result.reserve(keys.size());
  for (const auto& key : keys) {
    result.push_back(to_ffi(key));
  }
  return result;
}

[[nodiscard]] RepositoryIndex parse_verified_repository_json(std::string_view bytes) {
  Json document;
  try {
    document = Json::parse(bytes.begin(), bytes.end(), nullptr, true, true);
  } catch (const Json::exception&) {
    repository_fail("verified repository payload is not strict JSON");
  }
  if (!document.is_object()) {
    repository_fail("verified repository payload is malformed");
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
        repository_fail("verified repository payload has an unsupported package type");
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
  } catch (const Json::exception&) {
    repository_fail("verified repository payload is malformed");
  }
  return result;
}

[[nodiscard]] std::string serialize_signature_envelope(const SignatureEnvelope& envelope) {
  std::string result = "{\"format_version\":" + std::to_string(envelope.format_version) +
                       ",\"signed_object\":" + json_string(envelope.signed_object) +
                       ",\"canonicalization\":" + json_string(envelope.canonicalization) +
                       ",\"signatures\":[";
  bool first = true;
  for (const auto& signature : envelope.signatures) {
    if (!first) {
      result.push_back(',');
    }
    first = false;
    result += "{\"key_id\":" + json_string(signature.key_id) + ",\"algorithm\":" +
              json_string(signature.algorithm) + ",\"signature_base64\":" +
              json_string(base64(std::as_bytes(std::span(signature.signature)))) + '}';
  }
  result += "]}";
  return result;
}

[[nodiscard]] RepositoryIndex verify_repository_via_rust(std::string_view index_bytes,
                                                         std::string_view signature_bytes,
                                                         std::span<const TrustedKey> trusted_keys,
                                                         std::string_view expected_channel) {
  const auto ffi_keys = to_ffi(trusted_keys);
  const auto result = fcitx5_repository_verify_index_utf8(
      reinterpret_cast<const std::uint8_t*>(index_bytes.data()), index_bytes.size(),
      reinterpret_cast<const std::uint8_t*>(signature_bytes.data()), signature_bytes.size(),
      ffi_keys.data(), ffi_keys.size(), reinterpret_cast<const std::uint8_t*>(expected_channel.data()),
      expected_channel.size());
  if (result.status != 0) {
    throw PackageError(result.error_code, result.error_message);
  }
  std::string verified_blob(reinterpret_cast<const char*>(result.blob.data), result.blob.len);
  fcitx5_repository_blob_free(result.blob.data, result.blob.len);
  return parse_verified_repository_json(verified_blob);
}

[[nodiscard]] RepositoryIndex verify_repository_via_rust(std::string_view index_bytes,
                                                         const SignatureEnvelope& envelope,
                                                         std::span<const TrustedKey> trusted_keys,
                                                         std::string_view expected_channel) {
  const auto envelope_bytes = serialize_signature_envelope(envelope);
  const auto ffi_keys = to_ffi(trusted_keys);
  const auto result = fcitx5_repository_verify_index_envelope_utf8(
      reinterpret_cast<const std::uint8_t*>(index_bytes.data()), index_bytes.size(),
      reinterpret_cast<const std::uint8_t*>(envelope_bytes.data()), envelope_bytes.size(),
      ffi_keys.data(), ffi_keys.size(), reinterpret_cast<const std::uint8_t*>(expected_channel.data()),
      expected_channel.size());
  if (result.status != 0) {
    throw PackageError(result.error_code, result.error_message);
  }
  std::string verified_blob(reinterpret_cast<const char*>(result.blob.data), result.blob.len);
  fcitx5_repository_blob_free(result.blob.data, result.blob.len);
  return parse_verified_repository_json(verified_blob);
}

}  // namespace

RepositoryIndex verify_repository_index(std::string_view index_bytes,
                                        std::span<const std::byte> signature,
                                        std::span<const TrustedKey> trusted_keys,
                                        std::string_view expectedChannel) {
  return verify_repository_via_rust(index_bytes, std::string_view{
                                                      reinterpret_cast<const char*>(signature.data()),
                                                      signature.size()},
                                    trusted_keys, expectedChannel);
}

RepositoryIndex verify_repository_index(std::string_view index_bytes,
                                        const SignatureEnvelope& envelope,
                                        std::span<const TrustedKey> trusted_keys,
                                        std::string_view expectedChannel) {
  return verify_repository_via_rust(index_bytes, envelope, trusted_keys, expectedChannel);
}

const RepositoryEntry* find_repository_package(const RepositoryIndex& index,
                                               std::string_view package_id,
                                               std::string_view architecture) noexcept {
  const auto match = std::ranges::find_if(index.packages, [&](const RepositoryEntry& entry) {
    return entry.id == package_id &&
           (entry.architecture == "any" || entry.architecture == architecture);
  });
  return match == index.packages.end() ? nullptr : &*match;
}

}  // namespace fcitx::package
