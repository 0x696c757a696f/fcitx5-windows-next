#include "deployment_core.h"

#include <windows.h>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>

namespace {

void write_text(const std::filesystem::path& path, std::string_view text) {
  std::filesystem::create_directories(path.parent_path());
  std::ofstream output(path, std::ios::binary | std::ios::trunc);
  output << text;
  if (!output) throw std::runtime_error("test fixture write failed");
}

std::string read_text(const std::filesystem::path& path) {
  std::ifstream input(path, std::ios::binary);
  return {std::istreambuf_iterator<char>(input), {}};
}

bool deployment_state_contract(const std::filesystem::path& root) {
  fcitx::update::write_update_owner(root, fcitx::update::UpdateOwner::builtin);
  fcitx::update::begin_activation(root, "stable", "1.0.0", fcitx::update::UpdateOwner::builtin);
  fcitx::update::mark_current_healthy(root, "stable");
  fcitx::update::begin_activation(root, "stable", "1.1.0", fcitx::update::UpdateOwner::builtin);
  auto state = fcitx::update::read_deployment_state(root, "stable");
  if (state.current != "1.1.0" || state.previous != "1.0.0" || state.healthy) return false;
  fcitx::update::finish_rollback(root, "stable");
  state = fcitx::update::read_deployment_state(root, "stable");
  if (state.current != "1.0.0" || !state.previous.empty() || !state.healthy) return false;
  fcitx::update::write_update_owner(root, fcitx::update::UpdateOwner::winget);
  bool refused = false;
  try {
    fcitx::update::begin_activation(root, "stable", "2.0.0",
                                    fcitx::update::UpdateOwner::builtin);
  } catch (const std::exception&) { refused = true; }
  return refused;
}

bool tsf_dll_update_contract(const std::filesystem::path& root) {
  const auto registered = root / L"tsf" / L"x64" / L"fcitx5-tsf.dll";
  const auto incoming = root / L"staging" / L"fcitx5-tsf.dll";
  write_text(registered, "old-generation");
  write_text(incoming, "new-generation");
  const auto result = fcitx::update::install_tsf_dll_generation(registered, incoming, "00000042");
  if (!result.old_dll_renamed || !result.new_dll_installed || result.old_cleanup_pending ||
      read_text(registered) != "new-generation" ||
      read_text(registered.parent_path() / L"fcitx5-tsf.generation") != "00000042\n" ||
      std::filesystem::exists(result.renamed_old_path)) {
    return false;
  }
  const auto first_install = root / L"tsf" / L"x86" / L"fcitx5-tsf.dll";
  const auto first_result =
      fcitx::update::install_tsf_dll_generation(first_install, incoming, "00000042");
  if (first_result.old_dll_renamed || !first_result.new_dll_installed ||
      read_text(first_install) != "new-generation") {
    return false;
  }
  bool refused = false;
  try {
    (void)fcitx::update::install_tsf_dll_generation(root / L"tsf" / L"x64" / L"evil.dll",
                                                    incoming, "00000042");
  } catch (const std::exception&) { refused = true; }
  return refused;
}

void write_runtime_payload(const std::filesystem::path& payload, std::string_view marker) {
  write_text(payload / L"bin" / L"fcitx5-engine.exe", marker);
  write_text(payload / L"bin" / L"fcitx5-launcher.exe", marker);
  write_text(payload / L"bin" / L"fcitx5-ui.exe", marker);
  write_text(payload / L"bin" / L"fcitx5-control.exe", marker);
  write_text(payload / L"lib" / L"fcitx5" / L"addon.dll", marker);
  write_text(payload / L"share" / L"fcitx5" / L"profile", marker);
  write_text(payload / L"themes" / L"default" / L"theme.conf", marker);
  write_text(payload / L"tsf" / L"x64" / L"fcitx5-tsf.dll", std::string(marker) + "-x64");
  write_text(payload / L"tsf" / L"x86" / L"fcitx5-tsf.dll", std::string(marker) + "-x86");
}

bool runtime_generation_install_contract(const std::filesystem::path& root) {
  const auto payload41 = root / L"payload-41";
  const auto payload42 = root / L"payload-42";
  write_runtime_payload(payload41, "payload-41");
  write_runtime_payload(payload42, "payload-42");
  const auto installed41 =
      fcitx::update::install_runtime_generation(root, payload41, "00000041", "build-41");
  auto state = fcitx::update::read_runtime_generation_state(root);
  if (!installed41.runtime_installed || !installed41.tsf_x64_installed ||
      !installed41.tsf_x86_installed || !installed41.current_published ||
      state.current_generation != "00000041" || !state.previous_generation.empty() ||
      read_text(root / L"runtime" / L"00000041" / L"bin" / L"fcitx5-engine.exe") !=
          "payload-41" ||
      read_text(root / L"tsf" / L"x64" / L"fcitx5-tsf.dll") != "payload-41-x64" ||
      read_text(root / L"tsf" / L"x64" / L"fcitx5-tsf.generation") != "00000041\n") {
    return false;
  }
  const auto installed42 =
      fcitx::update::install_runtime_generation(root, payload42, "00000042", "build-42");
  state = fcitx::update::read_runtime_generation_state(root);
  if (!installed42.current_published || state.current_generation != "00000042" ||
      state.previous_generation != "00000041" ||
      read_text(root / L"runtime" / L"00000042" / L"bin" / L"fcitx5-ui.exe") !=
          "payload-42" ||
      read_text(root / L"tsf" / L"x86" / L"fcitx5-tsf.dll") != "payload-42-x86" ||
      read_text(root / L"tsf" / L"x86" / L"fcitx5-tsf.generation") != "00000042\n") {
    return false;
  }
  const auto badPayload = root / L"bad-payload";
  write_text(badPayload / L"bin" / L"fcitx5-engine.exe", "bad");
  bool refusedBad = false;
  try {
    (void)fcitx::update::install_runtime_generation(root, badPayload, "00000043", "build-43");
  } catch (const std::exception&) { refusedBad = true; }
  state = fcitx::update::read_runtime_generation_state(root);
  if (!refusedBad || state.current_generation != "00000042" ||
      std::filesystem::exists(root / L"runtime" / L"00000043")) {
    return false;
  }
  const auto badTsfPayload = root / L"bad-tsf-payload";
  write_runtime_payload(badTsfPayload, "bad-tsf");
  std::filesystem::remove(badTsfPayload / L"tsf" / L"x86" / L"fcitx5-tsf.dll");
  bool refusedBadTsf = false;
  try {
    (void)fcitx::update::install_runtime_generation(root, badTsfPayload, "00000043",
                                                    "build-43");
  } catch (const std::exception&) { refusedBadTsf = true; }
  state = fcitx::update::read_runtime_generation_state(root);
  return refusedBadTsf && state.current_generation == "00000042" &&
         state.previous_generation == "00000041" &&
         read_text(root / L"tsf" / L"x64" / L"fcitx5-tsf.dll") == "payload-42-x64" &&
         !std::filesystem::exists(root / L"runtime" / L"00000043");
}

bool runtime_generation_contract(const std::filesystem::path& root) {
  auto missing = fcitx::update::read_runtime_generation_state(root);
  if (!missing.current_generation.empty() || !missing.previous_generation.empty() ||
      !missing.build_id.empty()) {
    return false;
  }
  bool refusedMissing = false;
  try {
    fcitx::update::publish_runtime_generation(root, "00000041", "build-41");
  } catch (const std::exception&) { refusedMissing = true; }
  std::filesystem::create_directories(
      fcitx::update::runtime_generation_directory(root, "00000041"));
  std::filesystem::create_directories(
      fcitx::update::runtime_generation_directory(root, "00000042"));
  fcitx::update::publish_runtime_generation(root, "00000041", "build-41");
  auto state = fcitx::update::read_runtime_generation_state(root);
  if (!refusedMissing || state.current_generation != "00000041" ||
      !state.previous_generation.empty() || state.build_id != "build-41") {
    return false;
  }
  fcitx::update::publish_runtime_generation(root, "00000042", "build-42");
  state = fcitx::update::read_runtime_generation_state(root);
  if (state.current_generation != "00000042" ||
      state.previous_generation != "00000041" || state.build_id != "build-42") {
    return false;
  }
  bool refusedBad = false;
  try {
    (void)fcitx::update::runtime_generation_directory(root, "../bad");
  } catch (const std::exception&) { refusedBad = true; }
  return refusedBad;
}

}  // namespace

int main() {
  try {
    const auto root = std::filesystem::temp_directory_path() /
                      (L"fcitx5-deployment-test-" + std::to_wstring(GetCurrentProcessId()));
    std::error_code ignored;
    std::filesystem::remove_all(root, ignored);
    std::filesystem::create_directories(root);
    const bool ok = deployment_state_contract(root / L"deployment") &&
                    tsf_dll_update_contract(root / L"tsf") &&
                    runtime_generation_contract(root / L"runtime-metadata") &&
                    runtime_generation_install_contract(root / L"runtime-install");
    std::filesystem::remove_all(root, ignored);
    return ok ? 0 : 1;
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
