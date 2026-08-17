#pragma once

#include <filesystem>
#include <string>
#include <string_view>

namespace fcitx::package {

enum class ProviderTrust { official, unverified };

struct PlumPlan {
  std::filesystem::path script;
  std::filesystem::path working_directory;
  std::filesystem::path rime_user_directory;
  std::filesystem::path download_cache_directory;
  std::string package_spec;
  ProviderTrust trust{};
};

[[nodiscard]] PlumPlan make_plum_plan(const std::filesystem::path& provider_root,
                                      const std::filesystem::path& rime_user_directory,
                                      const std::filesystem::path& download_cache_directory,
                                      std::string_view package_spec);

}  // namespace fcitx::package
