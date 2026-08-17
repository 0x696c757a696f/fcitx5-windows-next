#pragma once

#include <filesystem>
#include <string>
#include <string_view>

namespace fcitx::update {

enum class UpdateOwner { builtin, chocolatey, winget, enterprise, manual };

struct DeploymentState {
  std::uint32_t format_version{1};
  std::string channel;
  UpdateOwner owner{UpdateOwner::manual};
  std::string current;
  std::string previous;
  std::string pending;
  bool healthy{};
};

[[nodiscard]] std::string_view owner_name(UpdateOwner owner) noexcept;
[[nodiscard]] UpdateOwner parse_owner(std::string_view owner);
[[nodiscard]] DeploymentState read_deployment_state(const std::filesystem::path& root,
                                                     std::string_view channel);
void write_update_owner(const std::filesystem::path& root, UpdateOwner owner);
[[nodiscard]] UpdateOwner read_update_owner(const std::filesystem::path& root);
void begin_activation(const std::filesystem::path& root, std::string_view channel,
                      std::string_view version, UpdateOwner caller);
void mark_current_healthy(const std::filesystem::path& root, std::string_view channel);
[[nodiscard]] std::string rollback_target(const std::filesystem::path& root,
                                          std::string_view channel);
void finish_rollback(const std::filesystem::path& root, std::string_view channel);
void clear_previous_known_good(const std::filesystem::path& root, std::string_view channel);

}  // namespace fcitx::update
