#include "provider_policy.h"

#include "package_core.h"

#include <algorithm>
#include <cctype>

namespace fcitx::package {
namespace {

bool contains_cmd_metacharacters(std::wstring_view value) {
  return value.find_first_of(L"%!?&|<>^\"") != std::wstring_view::npos;
}

bool valid_package_spec(std::string_view value) {
  if (value.empty() || value.size() > 256U || value.find("..") != std::string_view::npos) {
    return false;
  }
  return std::ranges::all_of(value, [](char raw) {
    const auto character = static_cast<unsigned char>(raw);
    return std::isalnum(character) != 0 || character == '.' || character == '_' ||
           character == '-' || character == '/' || character == '@' || character == ':';
  });
}

ProviderTrust classify(std::string_view value) {
  if (value == ":preset" || value == ":extra" || value == ":all" ||
      value.find('/') == std::string_view::npos || value.starts_with("rime/")) {
    return ProviderTrust::official;
  }
  return ProviderTrust::unverified;
}

}  // namespace

PlumPlan make_plum_plan(const std::filesystem::path& provider_root,
                        const std::filesystem::path& rime_user_directory,
                        const std::filesystem::path& download_cache_directory,
                        std::string_view package_spec) {
  if (!provider_root.is_absolute() || !rime_user_directory.is_absolute() ||
      !download_cache_directory.is_absolute() || provider_root == provider_root.root_path() ||
      rime_user_directory == rime_user_directory.root_path() ||
      download_cache_directory == download_cache_directory.root_path() ||
      contains_cmd_metacharacters(provider_root.native()) ||
      path_contains_reparse_point(provider_root) ||
      path_contains_reparse_point(rime_user_directory) ||
      path_contains_reparse_point(download_cache_directory) || !valid_package_spec(package_spec)) {
    throw PackageError("invalid_provider_request",
                       "Plum requires explicit safe provider, Rime user and cache paths");
  }
  const auto script = provider_root / "rime-install.bat";
  if (!std::filesystem::is_regular_file(script) || path_contains_reparse_point(script)) {
    throw PackageError("invalid_provider_request", "pinned Plum entry point is missing or unsafe");
  }
  return PlumPlan{script, provider_root, rime_user_directory, download_cache_directory,
                  std::string(package_spec), classify(package_spec)};
}

}  // namespace fcitx::package
