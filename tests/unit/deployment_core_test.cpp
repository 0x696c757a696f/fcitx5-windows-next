#include "deployment_core.h"

#include <windows.h>

#include <filesystem>
#include <iostream>

int main() {
  try {
    const auto root = std::filesystem::temp_directory_path() /
                      (L"fcitx5-deployment-test-" + std::to_wstring(GetCurrentProcessId()));
    std::error_code ignored;
    std::filesystem::remove_all(root, ignored);
    std::filesystem::create_directories(root);
    fcitx::update::write_update_owner(root, fcitx::update::UpdateOwner::builtin);
    fcitx::update::begin_activation(root, "stable", "1.0.0", fcitx::update::UpdateOwner::builtin);
    fcitx::update::mark_current_healthy(root, "stable");
    fcitx::update::begin_activation(root, "stable", "1.1.0", fcitx::update::UpdateOwner::builtin);
    auto state = fcitx::update::read_deployment_state(root, "stable");
    if (state.current != "1.1.0" || state.previous != "1.0.0" || state.healthy) return 1;
    fcitx::update::finish_rollback(root, "stable");
    state = fcitx::update::read_deployment_state(root, "stable");
    if (state.current != "1.0.0" || !state.previous.empty() || !state.healthy) return 1;
    fcitx::update::write_update_owner(root, fcitx::update::UpdateOwner::winget);
    bool refused = false;
    try {
      fcitx::update::begin_activation(root, "stable", "2.0.0",
                                      fcitx::update::UpdateOwner::builtin);
    } catch (const std::exception&) { refused = true; }
    std::filesystem::remove_all(root, ignored);
    return refused ? 0 : 1;
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
