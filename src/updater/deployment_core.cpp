#include "deployment_core.h"

#include <windows.h>

#include <cstdint>
#include <algorithm>
#include <fstream>
#include <iterator>
#include <set>
#include <stdexcept>

#include <nlohmann/json.hpp>

namespace fcitx::update {
namespace {

using Json = nlohmann::json;

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

bool token(std::string_view value) {
  return !value.empty() && value.size() <= 64U &&
         std::ranges::all_of(value, [](unsigned char character) {
           return (character >= 'a' && character <= 'z') ||
                  (character >= 'A' && character <= 'Z') ||
                  (character >= '0' && character <= '9') || character == '.' ||
                  character == '-' || character == '_' || character == '+';
         });
}

bool generation_token(std::string_view value) {
  return !value.empty() && value.size() <= 32U &&
         std::ranges::all_of(value, [](unsigned char character) {
           return (character >= 'a' && character <= 'z') ||
                  (character >= '0' && character <= '9') || character == '-' ||
                  character == '_';
         });
}

std::wstring widen_ascii(std::string_view value) {
  std::wstring result;
  result.reserve(value.size());
  for (const unsigned char character : value) result.push_back(static_cast<wchar_t>(character));
  return result;
}

[[nodiscard]] std::string bounded_ascii(const char* value, std::size_t maximum) {
  const auto end = std::find(value, value + maximum, '\0');
  return {value, end};
}

[[nodiscard]] const std::uint16_t* utf16_ptr(const std::wstring& value) {
  return reinterpret_cast<const std::uint16_t*>(value.c_str());
}

bool old_tsf_name(const std::filesystem::path& path) {
  const auto name = path.filename().wstring();
  constexpr std::wstring_view prefix = L"fcitx5-tsf.old.";
  constexpr std::wstring_view suffix = L".dll";
  return name.size() > prefix.size() + suffix.size() && name.starts_with(prefix) &&
         name.ends_with(suffix);
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

void validate_tsf_update_inputs(const std::filesystem::path& registered_dll_path,
                                const std::filesystem::path& verified_new_dll_path,
                                std::string_view generation) {
  if (!registered_dll_path.is_absolute() || registered_dll_path.filename() != L"fcitx5-tsf.dll" ||
      registered_dll_path.parent_path().empty()) {
    throw std::invalid_argument("registered TSF DLL path is invalid");
  }
  if (!verified_new_dll_path.is_absolute() ||
      !std::filesystem::is_regular_file(verified_new_dll_path)) {
    throw std::invalid_argument("verified TSF DLL path is invalid");
  }
  if (!generation_token(generation)) throw std::invalid_argument("TSF generation is invalid");
}

std::filesystem::path unique_old_tsf_path(const std::filesystem::path& registered_dll_path,
                                          std::string_view generation) {
  const auto directory = registered_dll_path.parent_path();
  const auto generation_text = widen_ascii(generation);
  for (unsigned attempt = 0; attempt < 64U; ++attempt) {
    const auto candidate =
        directory / (L"fcitx5-tsf.old." + generation_text + L"." +
                     std::to_wstring(GetCurrentProcessId()) + L"." +
                     std::to_wstring(GetTickCount64()) + L"." + std::to_wstring(attempt) +
                     L".dll");
    if (!std::filesystem::exists(candidate)) return candidate;
  }
  throw std::runtime_error("unable to allocate old TSF DLL path");
}

bool cleanup_old_tsf_dll(const std::filesystem::path& path, bool& scheduled_for_reboot) noexcept {
  scheduled_for_reboot = false;
  if (DeleteFileW(path.c_str())) return true;
  const DWORD error = GetLastError();
  if (error == ERROR_FILE_NOT_FOUND || error == ERROR_PATH_NOT_FOUND) return true;
  if (error == ERROR_SHARING_VIOLATION || error == ERROR_LOCK_VIOLATION ||
      error == ERROR_ACCESS_DENIED) {
    scheduled_for_reboot =
        MoveFileExW(path.c_str(), nullptr, MOVEFILE_DELAY_UNTIL_REBOOT) != FALSE;
  }
  return false;
}

void restore_renamed_tsf(const std::filesystem::path& registered_dll_path,
                         const std::filesystem::path& renamed_old_path) noexcept {
  if (renamed_old_path.empty() || !std::filesystem::exists(renamed_old_path) ||
      std::filesystem::exists(registered_dll_path)) {
    return;
  }
  (void)MoveFileExW(renamed_old_path.c_str(), registered_dll_path.c_str(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH);
}

[[maybe_unused]] std::filesystem::path state_path(const std::filesystem::path& root) {
  return root / "deployment.json";
}

[[maybe_unused]] std::filesystem::path current_generation_path(const std::filesystem::path& root) {
  return root / "current.json";
}

void copy_directory_tree(const std::filesystem::path& source,
                         const std::filesystem::path& destination) {
  if (!std::filesystem::is_directory(source)) return;
  std::filesystem::create_directories(destination);
  for (const auto& entry : std::filesystem::recursive_directory_iterator(source)) {
    const auto relative = std::filesystem::relative(entry.path(), source);
    const auto target = destination / relative;
    std::error_code error;
    if (entry.is_directory(error)) {
      if (error) throw std::runtime_error("runtime source directory inspection failed");
      std::filesystem::create_directories(target);
    } else if (entry.is_regular_file(error)) {
      if (error) throw std::runtime_error("runtime source file inspection failed");
      std::filesystem::create_directories(target.parent_path());
      std::filesystem::copy_file(entry.path(), target, std::filesystem::copy_options::none);
    } else if (entry.is_symlink(error) || entry.is_other(error)) {
      throw std::runtime_error("runtime source contains unsupported file type");
    }
  }
}

void copy_optional_file(const std::filesystem::path& source,
                        const std::filesystem::path& destination) {
  if (!std::filesystem::is_regular_file(source)) return;
  std::filesystem::create_directories(destination.parent_path());
  std::filesystem::copy_file(source, destination, std::filesystem::copy_options::none);
}

void stage_runtime_payload(const std::filesystem::path& verified_payload_root,
                           const std::filesystem::path& destination) {
  if (!std::filesystem::is_directory(verified_payload_root)) {
    throw std::runtime_error("runtime generation payload is incomplete");
  }
  copy_directory_tree(verified_payload_root / L"bin", destination / L"bin");
  copy_directory_tree(verified_payload_root / L"lib", destination / L"lib");
  copy_directory_tree(verified_payload_root / L"share", destination / L"share");
  copy_directory_tree(verified_payload_root / L"themes", destination / L"themes");
  copy_directory_tree(verified_payload_root / L"data", destination / L"data");
  copy_optional_file(verified_payload_root / L"portable.flag", destination / L"portable.flag");
}

bool runtime_payload_complete(const std::filesystem::path& root) {
  return std::filesystem::is_regular_file(root / L"bin" / L"fcitx5-engine.exe") &&
         std::filesystem::is_regular_file(root / L"bin" / L"fcitx5-launcher.exe") &&
         std::filesystem::is_regular_file(root / L"bin" / L"fcitx5-ui.exe") &&
         std::filesystem::is_directory(root / L"lib") &&
         std::filesystem::is_directory(root / L"share");
}

void validate_runtime_generation_payload(const std::filesystem::path& verified_payload_root) {
  if (!std::filesystem::is_directory(verified_payload_root) ||
      !runtime_payload_complete(verified_payload_root) ||
      !std::filesystem::is_regular_file(verified_payload_root / L"tsf" / L"x64" /
                                        L"fcitx5-tsf.dll") ||
      !std::filesystem::is_regular_file(verified_payload_root / L"tsf" / L"x86" /
                                        L"fcitx5-tsf.dll")) {
    throw std::runtime_error("runtime generation payload is incomplete");
  }
}

void publish_runtime_directory(const std::filesystem::path& verified_payload_root,
                               const std::filesystem::path& destination) {
  const auto temporary = std::filesystem::path(destination.wstring() + L".new." +
                                              std::to_wstring(GetCurrentProcessId()) + L"." +
                                              std::to_wstring(GetTickCount64()));
  std::error_code ignored;
  std::filesystem::remove_all(temporary, ignored);
  try {
    stage_runtime_payload(verified_payload_root, temporary);
    std::filesystem::create_directories(destination.parent_path());
    if (std::filesystem::exists(destination)) {
      if (!runtime_payload_complete(destination)) {
        throw std::runtime_error("existing runtime generation is incomplete");
      }
      std::filesystem::remove_all(temporary, ignored);
      return;
    }
    DWORD lastError = ERROR_SUCCESS;
    for (int attempt = 0; attempt < 5; ++attempt) {
      if (MoveFileExW(temporary.c_str(), destination.c_str(), MOVEFILE_WRITE_THROUGH)) {
        return;
      }
      lastError = GetLastError();
      if (lastError != ERROR_ACCESS_DENIED && lastError != ERROR_SHARING_VIOLATION &&
          lastError != ERROR_LOCK_VIOLATION) {
        break;
      }
      Sleep(static_cast<DWORD>(25 * (attempt + 1)));
    }
    if (lastError == ERROR_ACCESS_DENIED || lastError == ERROR_SHARING_VIOLATION ||
        lastError == ERROR_LOCK_VIOLATION) {
      if (std::filesystem::exists(destination)) {
        if (runtime_payload_complete(destination)) {
          std::filesystem::remove_all(temporary, ignored);
          return;
        }
        std::filesystem::remove_all(destination, ignored);
      }
      try {
        stage_runtime_payload(temporary, destination);
        std::filesystem::remove_all(temporary, ignored);
        return;
      } catch (...) {
        std::filesystem::remove_all(destination, ignored);
        throw;
      }
    }
    std::filesystem::remove_all(temporary, ignored);
    throw std::runtime_error("runtime generation publication failed: " +
                             std::to_string(lastError));
  } catch (...) {
    std::filesystem::remove_all(temporary, ignored);
    throw;
  }
}

void publish(const std::filesystem::path& path, std::string_view bytes) {
  std::filesystem::create_directories(path.parent_path());
  const auto temporary = std::filesystem::path(path.wstring() + L".new");
  std::ofstream output(temporary, std::ios::binary | std::ios::trunc);
  output.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
  output.close();
  if (!output || !MoveFileExW(temporary.c_str(), path.c_str(),
                              MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
    std::error_code ignored;
    std::filesystem::remove(temporary, ignored);
    throw std::runtime_error("atomic deployment state publication failed");
  }
}

[[maybe_unused]] Json state_json(const DeploymentState& state) {
  return {{"format_version", 1}, {"channel", state.channel},
          {"update_owner", owner_name(state.owner)}, {"current", state.current},
          {"previous", state.previous}, {"pending", state.pending},
          {"healthy", state.healthy}};
}

[[maybe_unused]] Json runtime_generation_json(const RuntimeGenerationState& state) {
  return {{"format_version", 1},
          {"current_generation", state.current_generation},
          {"previous_generation", state.previous_generation},
          {"build_id", state.build_id}};
}

[[maybe_unused]] void write_state(const std::filesystem::path& root, const DeploymentState& state) {
  publish(state_path(root), state_json(state).dump(2) + "\n");
}

}  // namespace

std::string_view owner_name(UpdateOwner owner) noexcept {
  switch (owner) {
  case UpdateOwner::builtin: return "builtin";
  case UpdateOwner::chocolatey: return "chocolatey";
  case UpdateOwner::winget: return "winget";
  case UpdateOwner::enterprise: return "enterprise";
  case UpdateOwner::manual: return "manual";
  }
  return "manual";
}

UpdateOwner parse_owner(std::string_view owner) {
  if (owner == "builtin") return UpdateOwner::builtin;
  if (owner == "chocolatey") return UpdateOwner::chocolatey;
  if (owner == "winget") return UpdateOwner::winget;
  if (owner == "enterprise") return UpdateOwner::enterprise;
  if (owner == "manual") return UpdateOwner::manual;
  throw std::invalid_argument("unknown update owner");
}

UpdateOwner owner_from_value(std::uint32_t owner) {
  switch (owner) {
  case 0: return UpdateOwner::builtin;
  case 1: return UpdateOwner::chocolatey;
  case 2: return UpdateOwner::winget;
  case 3: return UpdateOwner::enterprise;
  case 4: return UpdateOwner::manual;
  default: throw std::runtime_error("deployment state schema is invalid");
  }
}

DeploymentState read_deployment_state(const std::filesystem::path& root,
                                      std::string_view channel) {
  Fcitx5DeploymentState state{};
  const auto channel_wide = widen_ascii(channel);
  if (fcitx5_update_read_deployment_state_utf16(utf16_ptr(root), root.native().size(),
                                                utf16_ptr(channel_wide), channel_wide.size(),
                                                &state) != 0) {
    throw std::runtime_error("deployment state read failed");
  }
  return {state.format_version,
          bounded_ascii(reinterpret_cast<const char*>(state.channel), std::size(state.channel)),
          owner_from_value(state.owner),
          bounded_ascii(reinterpret_cast<const char*>(state.current), std::size(state.current)),
          bounded_ascii(reinterpret_cast<const char*>(state.previous), std::size(state.previous)),
          bounded_ascii(reinterpret_cast<const char*>(state.pending), std::size(state.pending)),
          state.healthy != 0};
}

void write_update_owner(const std::filesystem::path& root, UpdateOwner owner) {
  if (fcitx5_update_write_update_owner_utf16(utf16_ptr(root), root.native().size(),
                                             static_cast<std::uint32_t>(owner)) != 0) {
    throw std::runtime_error("update owner publication failed");
  }
}

UpdateOwner read_update_owner(const std::filesystem::path& root) {
  std::uint32_t owner = static_cast<std::uint32_t>(UpdateOwner::manual);
  if (fcitx5_update_read_update_owner_utf16(utf16_ptr(root), root.native().size(), &owner) != 0) {
    throw std::runtime_error("update owner schema is invalid");
  }
  return owner_from_value(owner);
}

void begin_activation(const std::filesystem::path& root, std::string_view channel,
                      std::string_view version, UpdateOwner caller) {
  if (!token(version)) throw std::invalid_argument("release version is invalid");
  const auto state = read_deployment_state(root, channel);
  const auto owner = read_update_owner(root);
  if (owner != caller || owner != UpdateOwner::builtin)
    throw std::runtime_error("builtin updater does not own Core updates");
  if (!state.pending.empty()) throw std::runtime_error("another activation is pending");
  const auto channel_wide = widen_ascii(channel);
  const auto version_wide = widen_ascii(version);
  if (fcitx5_update_begin_activation_utf16(utf16_ptr(root), root.native().size(),
                                           utf16_ptr(channel_wide), channel_wide.size(),
                                           utf16_ptr(version_wide), version_wide.size(),
                                           static_cast<std::uint32_t>(caller)) != 0) {
    throw std::runtime_error("deployment activation failed");
  }
}

void mark_current_healthy(const std::filesystem::path& root, std::string_view channel) {
  const auto state = read_deployment_state(root, channel);
  if (state.pending.empty() || state.pending != state.current)
    throw std::runtime_error("no matching pending release");
  const auto channel_wide = widen_ascii(channel);
  if (fcitx5_update_mark_current_healthy_utf16(utf16_ptr(root), root.native().size(),
                                               utf16_ptr(channel_wide), channel_wide.size()) != 0) {
    throw std::runtime_error("deployment health update failed");
  }
}

std::string rollback_target(const std::filesystem::path& root, std::string_view channel) {
  Fcitx5StringResult result{};
  const auto channel_wide = widen_ascii(channel);
  if (fcitx5_update_rollback_target_utf16(utf16_ptr(root), root.native().size(),
                                          utf16_ptr(channel_wide), channel_wide.size(),
                                          &result) != 0) {
    throw std::runtime_error(result.error_message[0] != '\0' ? result.error_message
                                                             : "rollback target failed");
  }
  return bounded_ascii(result.value, std::size(result.value));
}

void finish_rollback(const std::filesystem::path& root, std::string_view channel) {
  const auto target = rollback_target(root, channel);
  const auto channel_wide = widen_ascii(channel);
  if (fcitx5_update_finish_rollback_utf16(utf16_ptr(root), root.native().size(),
                                          utf16_ptr(channel_wide), channel_wide.size()) != 0) {
    throw std::runtime_error("rollback completion failed");
  }
}

void clear_previous_known_good(const std::filesystem::path& root, std::string_view channel) {
  const auto state = read_deployment_state(root, channel);
  if (!state.healthy || !state.pending.empty()) throw std::runtime_error("deployment is not stable");
  const auto channel_wide = widen_ascii(channel);
  if (fcitx5_update_clear_previous_known_good_utf16(utf16_ptr(root), root.native().size(),
                                                    utf16_ptr(channel_wide),
                                                    channel_wide.size()) != 0) {
    throw std::runtime_error("previous-known-good cleanup failed");
  }
}

void cleanup_previous_known_good(const std::filesystem::path& root, std::string_view channel,
                                 std::string_view package_id) {
  const auto channel_wide = widen_ascii(channel);
  const auto package_id_wide = widen_ascii(package_id);
  if (fcitx5_update_cleanup_previous_known_good_utf16(
          utf16_ptr(root), root.native().size(), utf16_ptr(channel_wide), channel_wide.size(),
          utf16_ptr(package_id_wide), package_id_wide.size()) != 0) {
    throw std::runtime_error("previous-known-good cleanup failed");
  }
}

TsfDllUpdateResult install_tsf_dll_generation(
    const std::filesystem::path& registered_dll_path,
    const std::filesystem::path& verified_new_dll_path,
    std::string_view generation) {
  validate_tsf_update_inputs(registered_dll_path, verified_new_dll_path, generation);
  std::filesystem::create_directories(registered_dll_path.parent_path());
  TsfDllUpdateResult result;
  result.registered_path = registered_dll_path;
  if (std::filesystem::exists(registered_dll_path)) {
    if (!std::filesystem::is_regular_file(registered_dll_path)) {
      throw std::runtime_error("registered TSF DLL path is not a regular file");
    }
    result.renamed_old_path = unique_old_tsf_path(registered_dll_path, generation);
    if (!MoveFileExW(registered_dll_path.c_str(), result.renamed_old_path.c_str(),
                     MOVEFILE_WRITE_THROUGH)) {
      throw std::runtime_error("old TSF DLL rename failed");
    }
    result.old_dll_renamed = true;
  }

  const auto temporary = std::filesystem::path(
      registered_dll_path.wstring() + L".new." + std::to_wstring(GetCurrentProcessId()) + L"." +
      std::to_wstring(GetTickCount64()));
  try {
    if (!CopyFileW(verified_new_dll_path.c_str(), temporary.c_str(), FALSE)) {
      restore_renamed_tsf(registered_dll_path, result.renamed_old_path);
      throw std::runtime_error("new TSF DLL staging copy failed");
    }
    if (!MoveFileExW(temporary.c_str(), registered_dll_path.c_str(),
                     MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
      std::error_code ignored;
      std::filesystem::remove(temporary, ignored);
      restore_renamed_tsf(registered_dll_path, result.renamed_old_path);
      throw std::runtime_error("new TSF DLL publication failed");
    }
    publish(registered_dll_path.parent_path() / L"fcitx5-tsf.generation",
            std::string(generation) + "\n");
    result.new_dll_installed = true;
    if (result.old_dll_renamed) {
      bool scheduled = false;
      if (!cleanup_old_tsf_dll(result.renamed_old_path, scheduled)) {
        result.old_cleanup_pending = true;
        result.old_cleanup_scheduled_for_reboot = scheduled;
      }
    }
    return result;
  } catch (...) {
    std::error_code ignored;
    std::filesystem::remove(temporary, ignored);
    throw;
  }
}

std::vector<std::filesystem::path> cleanup_old_tsf_dlls(
    const std::filesystem::path& tsf_arch_directory) {
  std::vector<std::filesystem::path> pending;
  if (!std::filesystem::is_directory(tsf_arch_directory)) return pending;
  for (const auto& entry : std::filesystem::directory_iterator(tsf_arch_directory)) {
    std::error_code error;
    if (!entry.is_regular_file(error) || error || !old_tsf_name(entry.path())) continue;
    bool scheduled = false;
    if (!cleanup_old_tsf_dll(entry.path(), scheduled)) pending.push_back(entry.path());
  }
  return pending;
}

std::filesystem::path runtime_generation_directory(const std::filesystem::path& root,
                                                   std::string_view generation) {
  if (!generation_token(generation)) throw std::invalid_argument("runtime generation is invalid");
  return root / "runtime" / std::filesystem::path(std::string(generation));
}

RuntimeGenerationInstallResult install_runtime_generation(
    const std::filesystem::path& root,
    const std::filesystem::path& verified_payload_root,
    std::string_view generation,
    std::string_view build_id) {
  if (!root.is_absolute() || !verified_payload_root.is_absolute() ||
      !generation_token(generation) || !token(build_id) || contains_reparse_component(root) ||
      contains_reparse_component(verified_payload_root)) {
    throw std::invalid_argument("runtime generation install inputs are invalid");
  }
  validate_runtime_generation_payload(verified_payload_root);
  RuntimeGenerationInstallResult result;
  result.generation_directory = runtime_generation_directory(root, generation);
  publish_runtime_directory(verified_payload_root, result.generation_directory);
  result.runtime_installed = true;
  const auto incomingX64 = verified_payload_root / L"tsf" / L"x64" / L"fcitx5-tsf.dll";
  const auto incomingX86 = verified_payload_root / L"tsf" / L"x86" / L"fcitx5-tsf.dll";
  result.tsf_x64 = install_tsf_dll_generation(root / L"tsf" / L"x64" / L"fcitx5-tsf.dll",
                                              incomingX64, generation);
  result.tsf_x64_installed = true;
  result.tsf_x86 = install_tsf_dll_generation(root / L"tsf" / L"x86" / L"fcitx5-tsf.dll",
                                              incomingX86, generation);
  result.tsf_x86_installed = true;
  publish_runtime_generation(root, generation, build_id);
  result.current_published = true;
  return result;
}

RuntimeGenerationState read_runtime_generation_state(const std::filesystem::path& root) {
  Fcitx5GenerationState state{};
  if (fcitx5_update_read_runtime_generation_state_utf16(utf16_ptr(root), root.native().size(),
                                                        &state) != 0) {
    throw std::runtime_error("runtime generation read failed");
  }
  return {state.format_version,
          bounded_ascii(reinterpret_cast<const char*>(state.current_generation),
                        std::size(state.current_generation)),
          bounded_ascii(reinterpret_cast<const char*>(state.previous_generation),
                        std::size(state.previous_generation)),
          bounded_ascii(reinterpret_cast<const char*>(state.build_id), std::size(state.build_id))};
}

void publish_runtime_generation(const std::filesystem::path& root,
                                std::string_view generation,
                                std::string_view build_id) {
  if (!generation_token(generation) || !token(build_id))
    throw std::invalid_argument("runtime generation publication identity is invalid");
  if (!std::filesystem::is_directory(runtime_generation_directory(root, generation))) {
    throw std::runtime_error("runtime generation directory is missing");
  }
  const auto generation_wide = widen_ascii(generation);
  const auto build_id_wide = widen_ascii(build_id);
  if (fcitx5_update_publish_runtime_generation_utf16(utf16_ptr(root), root.native().size(),
                                                     utf16_ptr(generation_wide),
                                                     generation_wide.size(),
                                                     utf16_ptr(build_id_wide),
                                                     build_id_wide.size()) != 0) {
    throw std::runtime_error("runtime generation publication failed");
  }
}

}  // namespace fcitx::update
