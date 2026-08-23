#pragma once

#include <algorithm>
#include <array>
#include <cstdint>
#include <filesystem>
#include <iterator>
#include <stdexcept>
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

namespace detail {

struct Fcitx5DeploymentState {
  std::uint32_t format_version{};
  std::uint8_t channel[65]{};
  std::uint32_t owner{};
  std::uint8_t current[65]{};
  std::uint8_t previous[65]{};
  std::uint8_t pending[65]{};
  std::uint8_t healthy{};
};

struct Fcitx5GenerationState {
  std::uint32_t format_version{};
  std::uint8_t current_generation[33]{};
  std::uint8_t previous_generation[33]{};
  std::uint8_t build_id[65]{};
};

struct Fcitx5StringResult {
  std::int32_t status{};
  char error_code[64]{};
  char error_message[512]{};
  char value[65]{};
};

struct Fcitx5TsfDllUpdateResult {
  std::int32_t status{};
  std::uint8_t old_dll_renamed{};
  std::uint8_t new_dll_installed{};
  std::uint8_t old_cleanup_pending{};
  std::uint8_t old_cleanup_scheduled_for_reboot{};
  std::array<std::uint16_t, 32768> renamed_old_path{};
};

struct Fcitx5RuntimeGenerationInstallResult {
  std::int32_t status{};
  std::uint8_t runtime_installed{};
  std::uint8_t tsf_x64_installed{};
  std::uint8_t tsf_x86_installed{};
  std::uint8_t current_published{};
  std::uint8_t tsf_x64_old_dll_renamed{};
  std::uint8_t tsf_x64_old_cleanup_pending{};
  std::uint8_t tsf_x64_old_cleanup_scheduled_for_reboot{};
  std::uint8_t tsf_x86_old_dll_renamed{};
  std::uint8_t tsf_x86_old_cleanup_pending{};
  std::uint8_t tsf_x86_old_cleanup_scheduled_for_reboot{};
  std::array<std::uint16_t, 32768> generation_directory{};
};

extern "C" {
int fcitx5_update_write_update_owner_utf16(const std::uint16_t* root, std::size_t root_len,
                                           std::uint32_t owner);
int fcitx5_update_read_update_owner_utf16(const std::uint16_t* root, std::size_t root_len,
                                          std::uint32_t* out_owner);
int fcitx5_update_read_deployment_state_utf16(const std::uint16_t* root, std::size_t root_len,
                                              const std::uint16_t* channel,
                                              std::size_t channel_len,
                                              Fcitx5DeploymentState* out_state);
int fcitx5_update_begin_activation_utf16(const std::uint16_t* root, std::size_t root_len,
                                         const std::uint16_t* channel,
                                         std::size_t channel_len, const std::uint16_t* version,
                                         std::size_t version_len, std::uint32_t caller);
int fcitx5_update_mark_current_healthy_utf16(const std::uint16_t* root, std::size_t root_len,
                                             const std::uint16_t* channel,
                                             std::size_t channel_len);
int fcitx5_update_rollback_target_utf16(const std::uint16_t* root, std::size_t root_len,
                                        const std::uint16_t* channel,
                                        std::size_t channel_len, Fcitx5StringResult* out_target);
int fcitx5_update_finish_rollback_utf16(const std::uint16_t* root, std::size_t root_len,
                                        const std::uint16_t* channel,
                                        std::size_t channel_len);
int fcitx5_update_clear_previous_known_good_utf16(const std::uint16_t* root,
                                                  std::size_t root_len,
                                                  const std::uint16_t* channel,
                                                  std::size_t channel_len);
int fcitx5_update_cleanup_previous_known_good_utf16(const std::uint16_t* root,
                                                    std::size_t root_len,
                                                    const std::uint16_t* channel,
                                                    std::size_t channel_len,
                                                    const std::uint16_t* package_id,
                                                    std::size_t package_id_len);
int fcitx5_update_install_tsf_dll_generation_utf16(
    const std::uint16_t* registered_dll_path,
    std::size_t registered_dll_path_len,
    const std::uint16_t* verified_new_dll_path,
    std::size_t verified_new_dll_path_len,
    const std::uint16_t* generation,
    std::size_t generation_len,
    Fcitx5TsfDllUpdateResult* out_result);
int fcitx5_update_cleanup_old_tsf_dlls_utf16(const std::uint16_t* tsf_arch_directory,
                                             std::size_t tsf_arch_directory_len,
                                             std::size_t* out_pending_count);
int fcitx5_update_runtime_generation_directory_utf16(const std::uint16_t* root,
                                                     std::size_t root_len,
                                                     const std::uint16_t* generation,
                                                     std::size_t generation_len,
                                                     std::uint16_t* out_path,
                                                     std::size_t out_path_len);
int fcitx5_update_install_runtime_generation_utf16(
    const std::uint16_t* root,
    std::size_t root_len,
    const std::uint16_t* verified_payload_root,
    std::size_t verified_payload_root_len,
    const std::uint16_t* generation,
    std::size_t generation_len,
    const std::uint16_t* build_id,
    std::size_t build_id_len,
    Fcitx5RuntimeGenerationInstallResult* out_result);
int fcitx5_update_read_runtime_generation_state_utf16(const std::uint16_t* root,
                                                      std::size_t root_len,
                                                      Fcitx5GenerationState* out_state);
int fcitx5_update_publish_runtime_generation_utf16(const std::uint16_t* root,
                                                   std::size_t root_len,
                                                   const std::uint16_t* generation,
                                                   std::size_t generation_len,
                                                   const std::uint16_t* build_id,
                                                   std::size_t build_id_len);
}

inline bool token(std::string_view value) {
  return !value.empty() && value.size() <= 64U &&
         std::ranges::all_of(value, [](unsigned char character) {
           return (character >= 'a' && character <= 'z') ||
                  (character >= 'A' && character <= 'Z') ||
                  (character >= '0' && character <= '9') || character == '.' ||
                  character == '-' || character == '_' || character == '+';
         });
}

inline std::wstring widen_ascii(std::string_view value) {
  std::wstring result;
  result.reserve(value.size());
  for (const unsigned char character : value) result.push_back(static_cast<wchar_t>(character));
  return result;
}

[[nodiscard]] inline std::string bounded_ascii(const char* value, std::size_t maximum) {
  const auto end = std::find(value, value + maximum, '\0');
  return {value, end};
}

[[nodiscard]] inline const std::uint16_t* utf16_ptr(const std::wstring& value) {
  return reinterpret_cast<const std::uint16_t*>(value.c_str());
}

inline std::wstring bounded_wide(const std::uint16_t* value, std::size_t maximum) {
  const auto end = std::find(value, value + maximum, 0);
  return {reinterpret_cast<const wchar_t*>(value), reinterpret_cast<const wchar_t*>(end)};
}

inline UpdateOwner owner_from_value(std::uint32_t owner) {
  switch (owner) {
  case 0: return UpdateOwner::builtin;
  case 1: return UpdateOwner::chocolatey;
  case 2: return UpdateOwner::winget;
  case 3: return UpdateOwner::enterprise;
  case 4: return UpdateOwner::manual;
  default: throw std::runtime_error("deployment state schema is invalid");
  }
}

}  // namespace detail

inline std::string_view owner_name(UpdateOwner owner) noexcept {
  switch (owner) {
  case UpdateOwner::builtin: return "builtin";
  case UpdateOwner::chocolatey: return "chocolatey";
  case UpdateOwner::winget: return "winget";
  case UpdateOwner::enterprise: return "enterprise";
  case UpdateOwner::manual: return "manual";
  }
  return "manual";
}

inline UpdateOwner parse_owner(std::string_view owner) {
  if (owner == "builtin") return UpdateOwner::builtin;
  if (owner == "chocolatey") return UpdateOwner::chocolatey;
  if (owner == "winget") return UpdateOwner::winget;
  if (owner == "enterprise") return UpdateOwner::enterprise;
  if (owner == "manual") return UpdateOwner::manual;
  throw std::invalid_argument("unknown update owner");
}

inline DeploymentState read_deployment_state(const std::filesystem::path& root,
                                             std::string_view channel) {
  detail::Fcitx5DeploymentState state{};
  const auto channel_wide = detail::widen_ascii(channel);
  if (detail::fcitx5_update_read_deployment_state_utf16(
          detail::utf16_ptr(root), root.native().size(), detail::utf16_ptr(channel_wide),
          channel_wide.size(), &state) != 0) {
    throw std::runtime_error("deployment state read failed");
  }
  return {state.format_version,
          detail::bounded_ascii(reinterpret_cast<const char*>(state.channel),
                                std::size(state.channel)),
          detail::owner_from_value(state.owner),
          detail::bounded_ascii(reinterpret_cast<const char*>(state.current),
                                std::size(state.current)),
          detail::bounded_ascii(reinterpret_cast<const char*>(state.previous),
                                std::size(state.previous)),
          detail::bounded_ascii(reinterpret_cast<const char*>(state.pending),
                                std::size(state.pending)),
          state.healthy != 0};
}

inline void write_update_owner(const std::filesystem::path& root, UpdateOwner owner) {
  if (detail::fcitx5_update_write_update_owner_utf16(
          detail::utf16_ptr(root), root.native().size(), static_cast<std::uint32_t>(owner)) != 0) {
    throw std::runtime_error("update owner publication failed");
  }
}

inline UpdateOwner read_update_owner(const std::filesystem::path& root) {
  std::uint32_t owner = static_cast<std::uint32_t>(UpdateOwner::manual);
  if (detail::fcitx5_update_read_update_owner_utf16(detail::utf16_ptr(root), root.native().size(),
                                                    &owner) != 0) {
    throw std::runtime_error("update owner schema is invalid");
  }
  return detail::owner_from_value(owner);
}

inline void begin_activation(const std::filesystem::path& root, std::string_view channel,
                             std::string_view version, UpdateOwner caller) {
  if (!detail::token(version)) throw std::invalid_argument("release version is invalid");
  const auto state = read_deployment_state(root, channel);
  const auto owner = read_update_owner(root);
  if (owner != caller || owner != UpdateOwner::builtin)
    throw std::runtime_error("builtin updater does not own Core updates");
  if (!state.pending.empty()) throw std::runtime_error("another activation is pending");
  const auto channel_wide = detail::widen_ascii(channel);
  const auto version_wide = detail::widen_ascii(version);
  if (detail::fcitx5_update_begin_activation_utf16(
          detail::utf16_ptr(root), root.native().size(), detail::utf16_ptr(channel_wide),
          channel_wide.size(), detail::utf16_ptr(version_wide), version_wide.size(),
          static_cast<std::uint32_t>(caller)) != 0) {
    throw std::runtime_error("deployment activation failed");
  }
}

inline void mark_current_healthy(const std::filesystem::path& root, std::string_view channel) {
  const auto state = read_deployment_state(root, channel);
  if (state.pending.empty() || state.pending != state.current)
    throw std::runtime_error("no matching pending release");
  const auto channel_wide = detail::widen_ascii(channel);
  if (detail::fcitx5_update_mark_current_healthy_utf16(
          detail::utf16_ptr(root), root.native().size(), detail::utf16_ptr(channel_wide),
          channel_wide.size()) != 0) {
    throw std::runtime_error("deployment health update failed");
  }
}

inline std::string rollback_target(const std::filesystem::path& root, std::string_view channel) {
  detail::Fcitx5StringResult result{};
  const auto channel_wide = detail::widen_ascii(channel);
  if (detail::fcitx5_update_rollback_target_utf16(
          detail::utf16_ptr(root), root.native().size(), detail::utf16_ptr(channel_wide),
          channel_wide.size(), &result) != 0) {
    throw std::runtime_error(result.error_message[0] != '\0' ? result.error_message
                                                             : "rollback target failed");
  }
  return detail::bounded_ascii(result.value, std::size(result.value));
}

inline void finish_rollback(const std::filesystem::path& root, std::string_view channel) {
  const auto target = rollback_target(root, channel);
  const auto channel_wide = detail::widen_ascii(channel);
  if (detail::fcitx5_update_finish_rollback_utf16(
          detail::utf16_ptr(root), root.native().size(), detail::utf16_ptr(channel_wide),
          channel_wide.size()) != 0) {
    throw std::runtime_error("rollback completion failed");
  }
}

inline void clear_previous_known_good(const std::filesystem::path& root,
                                      std::string_view channel) {
  const auto state = read_deployment_state(root, channel);
  if (!state.healthy || !state.pending.empty()) throw std::runtime_error("deployment is not stable");
  const auto channel_wide = detail::widen_ascii(channel);
  if (detail::fcitx5_update_clear_previous_known_good_utf16(
          detail::utf16_ptr(root), root.native().size(), detail::utf16_ptr(channel_wide),
          channel_wide.size()) != 0) {
    throw std::runtime_error("previous-known-good cleanup failed");
  }
}

inline void cleanup_previous_known_good(const std::filesystem::path& root,
                                        std::string_view channel,
                                        std::string_view package_id) {
  const auto channel_wide = detail::widen_ascii(channel);
  const auto package_id_wide = detail::widen_ascii(package_id);
  if (detail::fcitx5_update_cleanup_previous_known_good_utf16(
          detail::utf16_ptr(root), root.native().size(), detail::utf16_ptr(channel_wide),
          channel_wide.size(), detail::utf16_ptr(package_id_wide), package_id_wide.size()) != 0) {
    throw std::runtime_error("previous-known-good cleanup failed");
  }
}

inline TsfDllUpdateResult install_tsf_dll_generation(
    const std::filesystem::path& registered_dll_path,
    const std::filesystem::path& verified_new_dll_path,
    std::string_view generation) {
  TsfDllUpdateResult result;
  result.registered_path = registered_dll_path;
  const auto generation_wide = detail::widen_ascii(generation);
  detail::Fcitx5TsfDllUpdateResult rust_result{};
  if (detail::fcitx5_update_install_tsf_dll_generation_utf16(
          detail::utf16_ptr(registered_dll_path), registered_dll_path.native().size(),
          detail::utf16_ptr(verified_new_dll_path), verified_new_dll_path.native().size(),
          detail::utf16_ptr(generation_wide), generation_wide.size(), &rust_result) != 0) {
    throw std::runtime_error("TSF DLL generation install failed");
  }
  result.old_dll_renamed = rust_result.old_dll_renamed != 0;
  result.new_dll_installed = rust_result.new_dll_installed != 0;
  result.old_cleanup_pending = rust_result.old_cleanup_pending != 0;
  result.old_cleanup_scheduled_for_reboot =
      rust_result.old_cleanup_scheduled_for_reboot != 0;
  result.renamed_old_path =
      std::filesystem::path(detail::bounded_wide(rust_result.renamed_old_path.data(),
                                                 rust_result.renamed_old_path.size()));
  return result;
}

inline std::vector<std::filesystem::path> cleanup_old_tsf_dlls(
    const std::filesystem::path& tsf_arch_directory) {
  std::size_t pending_count = 0;
  if (detail::fcitx5_update_cleanup_old_tsf_dlls_utf16(
          detail::utf16_ptr(tsf_arch_directory), tsf_arch_directory.native().size(),
          &pending_count) != 0) {
    throw std::runtime_error("old TSF DLL cleanup failed");
  }
  return std::vector<std::filesystem::path>(pending_count);
}

inline std::filesystem::path runtime_generation_directory(const std::filesystem::path& root,
                                                          std::string_view generation) {
  const auto generation_wide = detail::widen_ascii(generation);
  std::array<std::uint16_t, 32768> path{};
  if (detail::fcitx5_update_runtime_generation_directory_utf16(
          detail::utf16_ptr(root), root.native().size(), detail::utf16_ptr(generation_wide),
          generation_wide.size(), path.data(), path.size()) != 0) {
    throw std::invalid_argument("runtime generation is invalid");
  }
  return std::filesystem::path(detail::bounded_wide(path.data(), path.size()));
}

inline RuntimeGenerationInstallResult install_runtime_generation(
    const std::filesystem::path& root,
    const std::filesystem::path& verified_payload_root,
    std::string_view generation,
    std::string_view build_id) {
  const auto generation_wide = detail::widen_ascii(generation);
  const auto build_id_wide = detail::widen_ascii(build_id);
  detail::Fcitx5RuntimeGenerationInstallResult rust_result{};
  if (detail::fcitx5_update_install_runtime_generation_utf16(
          detail::utf16_ptr(root), root.native().size(),
          detail::utf16_ptr(verified_payload_root), verified_payload_root.native().size(),
          detail::utf16_ptr(generation_wide), generation_wide.size(),
          detail::utf16_ptr(build_id_wide), build_id_wide.size(), &rust_result) != 0) {
    throw std::runtime_error("runtime generation install failed");
  }
  RuntimeGenerationInstallResult result;
  result.generation_directory =
      std::filesystem::path(detail::bounded_wide(rust_result.generation_directory.data(),
                                                 rust_result.generation_directory.size()));
  result.runtime_installed = rust_result.runtime_installed != 0;
  result.tsf_x64_installed = rust_result.tsf_x64_installed != 0;
  result.tsf_x86_installed = rust_result.tsf_x86_installed != 0;
  result.current_published = rust_result.current_published != 0;
  result.tsf_x64.registered_path = root / L"tsf" / L"x64" / L"fcitx5-tsf.dll";
  result.tsf_x64.old_dll_renamed = rust_result.tsf_x64_old_dll_renamed != 0;
  result.tsf_x64.new_dll_installed = rust_result.tsf_x64_installed != 0;
  result.tsf_x64.old_cleanup_pending = rust_result.tsf_x64_old_cleanup_pending != 0;
  result.tsf_x64.old_cleanup_scheduled_for_reboot =
      rust_result.tsf_x64_old_cleanup_scheduled_for_reboot != 0;
  result.tsf_x86.registered_path = root / L"tsf" / L"x86" / L"fcitx5-tsf.dll";
  result.tsf_x86.old_dll_renamed = rust_result.tsf_x86_old_dll_renamed != 0;
  result.tsf_x86.new_dll_installed = rust_result.tsf_x86_installed != 0;
  result.tsf_x86.old_cleanup_pending = rust_result.tsf_x86_old_cleanup_pending != 0;
  result.tsf_x86.old_cleanup_scheduled_for_reboot =
      rust_result.tsf_x86_old_cleanup_scheduled_for_reboot != 0;
  return result;
}

inline RuntimeGenerationState read_runtime_generation_state(const std::filesystem::path& root) {
  detail::Fcitx5GenerationState state{};
  if (detail::fcitx5_update_read_runtime_generation_state_utf16(
          detail::utf16_ptr(root), root.native().size(), &state) != 0) {
    throw std::runtime_error("runtime generation read failed");
  }
  return {state.format_version,
          detail::bounded_ascii(reinterpret_cast<const char*>(state.current_generation),
                                std::size(state.current_generation)),
          detail::bounded_ascii(reinterpret_cast<const char*>(state.previous_generation),
                                std::size(state.previous_generation)),
          detail::bounded_ascii(reinterpret_cast<const char*>(state.build_id),
                                std::size(state.build_id))};
}

inline void publish_runtime_generation(const std::filesystem::path& root,
                                       std::string_view generation,
                                       std::string_view build_id) {
  const auto generation_wide = detail::widen_ascii(generation);
  const auto build_id_wide = detail::widen_ascii(build_id);
  if (detail::fcitx5_update_publish_runtime_generation_utf16(
          detail::utf16_ptr(root), root.native().size(), detail::utf16_ptr(generation_wide),
          generation_wide.size(), detail::utf16_ptr(build_id_wide), build_id_wide.size()) != 0) {
    throw std::runtime_error("runtime generation publication failed");
  }
}

}  // namespace fcitx::update
