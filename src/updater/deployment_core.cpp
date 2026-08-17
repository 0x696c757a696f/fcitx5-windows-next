#include "deployment_core.h"

#include <windows.h>

#include <fstream>
#include <algorithm>
#include <set>
#include <stdexcept>

#include <nlohmann/json.hpp>

namespace fcitx::update {
namespace {

using Json = nlohmann::json;

bool token(std::string_view value) {
  return !value.empty() && value.size() <= 64U &&
         std::ranges::all_of(value, [](unsigned char character) {
           return (character >= 'a' && character <= 'z') ||
                  (character >= 'A' && character <= 'Z') ||
                  (character >= '0' && character <= '9') || character == '.' ||
                  character == '-' || character == '_' || character == '+';
         });
}

std::filesystem::path state_path(const std::filesystem::path& root) {
  return root / "deployment.json";
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

Json state_json(const DeploymentState& state) {
  return {{"format_version", 1}, {"channel", state.channel},
          {"update_owner", owner_name(state.owner)}, {"current", state.current},
          {"previous", state.previous}, {"pending", state.pending},
          {"healthy", state.healthy}};
}

void write_state(const std::filesystem::path& root, const DeploymentState& state) {
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

DeploymentState read_deployment_state(const std::filesystem::path& root,
                                      std::string_view channel) {
  const auto path = state_path(root);
  if (!std::filesystem::exists(path)) {
    return {1, std::string(channel), read_update_owner(root), {}, {}, {}, false};
  }
  std::ifstream input(path, std::ios::binary);
  const std::string bytes(std::istreambuf_iterator<char>(input), {});
  if (!input.good() && !input.eof()) throw std::runtime_error("deployment state read failed");
  Json document = Json::parse(bytes);
  static const std::set<std::string> expected{"format_version", "channel", "update_owner",
                                               "current", "previous", "pending", "healthy"};
  std::set<std::string> actual;
  for (const auto& [key, value] : document.items()) { (void)value; actual.emplace(key); }
  if (!document.is_object() || actual != expected || document.at("format_version") != 1 ||
      !document.at("healthy").is_boolean()) throw std::runtime_error("deployment state schema is invalid");
  DeploymentState result{1, document.at("channel").get<std::string>(),
                         parse_owner(document.at("update_owner").get<std::string>()),
                         document.at("current").get<std::string>(),
                         document.at("previous").get<std::string>(),
                         document.at("pending").get<std::string>(),
                         document.at("healthy").get<bool>()};
  if (result.channel != channel || (!result.current.empty() && !token(result.current)) ||
      (!result.previous.empty() && !token(result.previous)) ||
      (!result.pending.empty() && !token(result.pending))) {
    throw std::runtime_error("deployment state identity is invalid");
  }
  return result;
}

void write_update_owner(const std::filesystem::path& root, UpdateOwner owner) {
  publish(root / "update-owner.json",
          Json{{"format_version", 1}, {"update_owner", owner_name(owner)}}.dump(2) + "\n");
}

UpdateOwner read_update_owner(const std::filesystem::path& root) {
  const auto path = root / "update-owner.json";
  if (!std::filesystem::exists(path)) return UpdateOwner::manual;
  std::ifstream input(path, std::ios::binary);
  Json document = Json::parse(input);
  if (!document.is_object() || document.size() != 2U || document.at("format_version") != 1 ||
      !document.contains("update_owner")) throw std::runtime_error("update owner schema is invalid");
  return parse_owner(document.at("update_owner").get<std::string>());
}

void begin_activation(const std::filesystem::path& root, std::string_view channel,
                      std::string_view version, UpdateOwner caller) {
  if (!token(version)) throw std::invalid_argument("release version is invalid");
  auto state = read_deployment_state(root, channel);
  state.owner = read_update_owner(root);
  if (state.owner != caller || state.owner != UpdateOwner::builtin)
    throw std::runtime_error("builtin updater does not own Core updates");
  if (!state.pending.empty()) throw std::runtime_error("another activation is pending");
  state.previous = state.healthy ? state.current : state.previous;
  state.current = std::string(version);
  state.pending = state.current;
  state.healthy = false;
  write_state(root, state);
}

void mark_current_healthy(const std::filesystem::path& root, std::string_view channel) {
  auto state = read_deployment_state(root, channel);
  if (state.pending.empty() || state.pending != state.current)
    throw std::runtime_error("no matching pending release");
  state.pending.clear();
  state.healthy = true;
  write_state(root, state);
}

std::string rollback_target(const std::filesystem::path& root, std::string_view channel) {
  const auto state = read_deployment_state(root, channel);
  if (state.previous.empty() || state.previous == state.current)
    throw std::runtime_error("no previous-known-good release exists");
  return state.previous;
}

void finish_rollback(const std::filesystem::path& root, std::string_view channel) {
  auto state = read_deployment_state(root, channel);
  const auto target = rollback_target(root, channel);
  state.current = target;
  state.previous.clear();
  state.pending.clear();
  state.healthy = true;
  write_state(root, state);
}

void clear_previous_known_good(const std::filesystem::path& root, std::string_view channel) {
  auto state = read_deployment_state(root, channel);
  if (!state.healthy || !state.pending.empty()) throw std::runtime_error("deployment is not stable");
  state.previous.clear();
  write_state(root, state);
}

}  // namespace fcitx::update
