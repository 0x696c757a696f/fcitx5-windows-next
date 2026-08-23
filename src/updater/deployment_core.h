#pragma once

#include <filesystem>
#include <vector>
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

struct TsfDllUpdateResult {
  std::filesystem::path registered_path;
  std::filesystem::path renamed_old_path;
  bool old_dll_renamed{};
  bool new_dll_installed{};
  bool old_cleanup_pending{};
  bool old_cleanup_scheduled_for_reboot{};
};

struct RuntimeGenerationInstallResult {
  std::filesystem::path generation_directory;
  TsfDllUpdateResult tsf_x64;
  TsfDllUpdateResult tsf_x86;
  bool runtime_installed{};
  bool tsf_x64_installed{};
  bool tsf_x86_installed{};
  bool current_published{};
};

struct RuntimeGenerationState {
  std::uint32_t format_version{1};
  std::string current_generation;
  std::string previous_generation;
  std::string build_id;
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
void cleanup_previous_known_good(const std::filesystem::path& root,
                                 std::string_view channel,
                                 std::string_view package_id);
[[nodiscard]] TsfDllUpdateResult install_tsf_dll_generation(
    const std::filesystem::path& registered_dll_path,
    const std::filesystem::path& verified_new_dll_path,
    std::string_view generation);
[[nodiscard]] std::vector<std::filesystem::path> cleanup_old_tsf_dlls(
    const std::filesystem::path& tsf_arch_directory);
[[nodiscard]] std::filesystem::path runtime_generation_directory(
    const std::filesystem::path& root, std::string_view generation);
[[nodiscard]] RuntimeGenerationInstallResult install_runtime_generation(
    const std::filesystem::path& root,
    const std::filesystem::path& verified_payload_root,
    std::string_view generation,
    std::string_view build_id);
[[nodiscard]] RuntimeGenerationState read_runtime_generation_state(
    const std::filesystem::path& root);
void publish_runtime_generation(const std::filesystem::path& root,
                                std::string_view generation,
                                std::string_view build_id);

}  // namespace fcitx::update
