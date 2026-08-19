#include "package_core.h"

#include <algorithm>
#include <set>

#include <nlohmann/json.hpp>

namespace fcitx::package {
namespace {

using Json = nlohmann::json;

[[noreturn]] void repository_fail(std::string message) {
  throw PackageError("invalid_repository", std::move(message));
}

void exact_keys(const Json& object, std::initializer_list<std::string_view> keys) {
  if (!object.is_object() || object.size() != keys.size()) repository_fail("object shape is invalid");
  for (const auto key : keys) {
    if (!object.contains(key)) repository_fail("required repository field is missing");
  }
}

std::string text(const Json& object, std::string_view key, std::size_t maximum,
                 bool empty = false) {
  const auto& value = object.at(key);
  if (!value.is_string()) repository_fail("repository text field has the wrong type");
  auto result = value.get<std::string>();
  if ((!empty && result.empty()) || result.size() > maximum ||
      result.find('\0') != std::string::npos) repository_fail("repository text field is invalid");
  return result;
}

bool token(std::string_view value, std::string_view extra = {}) {
  return !value.empty() && std::ranges::all_of(value, [extra](unsigned char character) {
    return (character >= 'a' && character <= 'z') ||
           (character >= 'A' && character <= 'Z') ||
           (character >= '0' && character <= '9') ||
           extra.find(static_cast<char>(character)) != std::string_view::npos;
  });
}

bool hex_digest(std::string_view value) {
  return value.size() == 64U && std::ranges::all_of(value, [](unsigned char character) {
    return (character >= '0' && character <= '9') ||
           (character >= 'a' && character <= 'f') ||
           (character >= 'A' && character <= 'F');
  });
}

PackageType package_type(std::string_view value) {
  if (value == "core") return PackageType::core;
  if (value == "addon") return PackageType::addon;
  if (value == "inputmethod-data") return PackageType::input_method_data;
  if (value == "theme") return PackageType::theme;
  if (value == "translation") return PackageType::translation;
  repository_fail("repository package type is unsupported");
}

bool https_url(std::string_view value) {
  if (!value.starts_with("https://") || value.size() > 2048U || value.find('@') != value.npos ||
      value.find('#') != value.npos || value.find('\\') != value.npos) return false;
  return std::ranges::all_of(value, [](unsigned char character) {
    return character >= 0x21U && character <= 0x7eU;
  });
}

}  // namespace

RepositoryIndex verify_repository_index(std::string_view index_bytes,
                                        std::span<const std::byte> signature,
                                        std::span<const TrustedKey> trusted_keys,
                                        std::string_view expectedChannel) {
  if (index_bytes.empty() || index_bytes.size() > kMaximumManifestBytes) {
    repository_fail("repository index exceeds its resource budget");
  }
  Json document;
  try {
    document = Json::parse(index_bytes.begin(), index_bytes.end(), nullptr, true, true);
  } catch (const Json::exception&) {
    repository_fail("repository index is not strict JSON");
  }
  exact_keys(document, {"format_version", "channel", "generated_at", "key_id", "packages"});
  if (!document["format_version"].is_number_unsigned() ||
      document["format_version"].get<std::uint32_t>() != 1U) {
    repository_fail("repository format_version must be exactly 1");
  }
  RepositoryIndex result;
  result.format_version = 1U;
  result.channel = text(document, "channel", 16U);
  result.generated_at = text(document, "generated_at", 64U);
  result.key_id = text(document, "key_id", 64U);
  // The repository channel must match the release identity of this build, not
  // merely be one of the three known names: a stable build must never accept
  // a beta or nightly index (and vice versa), so a stale or misdirected index
  // cannot steer a different channel's packages into this installation.
  if (result.channel != expectedChannel || !token(result.key_id, "-_.")) {
    repository_fail("repository identity is invalid or channel mismatch");
  }
  const auto key = std::ranges::find_if(trusted_keys, [&](const TrustedKey& candidate) {
    return candidate.id == result.key_id;
  });
  if (key == trusted_keys.end()) throw PackageError("untrusted_key", "repository key is not trusted");
  verify_manifest_signature(index_bytes, signature, *key);

  const auto& packages = document["packages"];
  if (!packages.is_array() || packages.size() > 4096U) repository_fail("package catalog is invalid");
  std::set<std::pair<std::string, std::string>> identities;
  for (const auto& item : packages) {
    exact_keys(item, {"id", "title", "summary", "version", "release_sequence", "type",
                      "architecture", "download_url", "sha256", "dependencies"});
    RepositoryEntry entry;
    entry.id = text(item, "id", 64U);
    entry.title = text(item, "title", 128U);
    entry.summary = text(item, "summary", 512U, true);
    entry.version = text(item, "version", 64U);
    entry.type = package_type(text(item, "type", 32U));
    entry.architecture = text(item, "architecture", 8U);
    entry.download_url = text(item, "download_url", 2048U);
    entry.sha256 = text(item, "sha256", 64U);
    const auto& dependencies = item["dependencies"];
    if (!dependencies.is_array() || dependencies.size() > 256U) {
      repository_fail("repository dependency list is invalid");
    }
    std::set<std::string> dependency_ids;
    for (const auto& dependency : dependencies) {
      exact_keys(dependency, {"id", "version"});
      Dependency parsed{text(dependency, "id", 64U), text(dependency, "version", 64U)};
      if (!token(parsed.id, "-_.") || !token(parsed.version, ".+-_") ||
          !dependency_ids.emplace(parsed.id).second) {
        repository_fail("repository dependency is invalid or duplicated");
      }
      entry.dependencies.push_back(std::move(parsed));
    }
    if (!item["release_sequence"].is_number_unsigned()) repository_fail("release sequence is invalid");
    entry.release_sequence = item["release_sequence"].get<std::uint64_t>();
    if (!token(entry.id, "-_.") || !token(entry.version, ".+-_") ||
        (entry.architecture != "any" && entry.architecture != "x86" &&
         entry.architecture != "x64") || !https_url(entry.download_url) ||
        !hex_digest(entry.sha256) || entry.release_sequence == 0U ||
        !identities.emplace(entry.id, entry.architecture).second) {
      repository_fail("repository package record is invalid or duplicated");
    }
    result.packages.push_back(std::move(entry));
  }
  std::ranges::sort(result.packages, {}, &RepositoryEntry::id);
  return result;
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
