// Updater --cleanup-previous path safety: the core package id comes from the
// CLI and is later turned into a remove_all() path, so it must be validated
// (canonical lowercase package id) and the resolved target must stay inside
// the versions directory. A malicious id must be refused without deleting
// anything; a valid id cleans up the previous version as intended.
#include "deployment_core.h"

#include <windows.h>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <vector>

namespace {

std::wstring quote(const std::wstring& value) {
  return L"\"" + value + L"\"";
}

DWORD run_updater(const std::filesystem::path& updater,
                  const std::vector<std::wstring>& arguments) {
  std::wstring command = quote(updater.wstring());
  for (const auto& argument : arguments)
    command += L" " + quote(argument);
  std::vector<wchar_t> mutableCommand(command.begin(), command.end());
  mutableCommand.push_back(L'\0');
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION process{};
  if (!CreateProcessW(updater.c_str(), mutableCommand.data(), nullptr, nullptr, FALSE,
                      CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process))
    return 0xFFFFFFFFU;
  CloseHandle(process.hThread);
  WaitForSingleObject(process.hProcess, 30'000);
  DWORD exitCode = 1;
  GetExitCodeProcess(process.hProcess, &exitCode);
  CloseHandle(process.hProcess);
  return exitCode;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
  if (argc != 2) {
    std::cerr << "expected updater executable path\n";
    return 1;
  }
  const std::filesystem::path updater = argv[1];
  const std::filesystem::path root =
      std::filesystem::temp_directory_path() /
      (L"fcitx5-updater-cleanup-" + std::to_wstring(GetCurrentProcessId()));
  std::error_code ignored;
  std::filesystem::remove_all(root, ignored);
  std::filesystem::create_directories(root);

  try {
    fcitx::update::write_update_owner(root, fcitx::update::UpdateOwner::builtin);
    fcitx::update::begin_activation(root, "stable", "1.0.0",
                                    fcitx::update::UpdateOwner::builtin);
    fcitx::update::mark_current_healthy(root, "stable");
    fcitx::update::begin_activation(root, "stable", "1.1.0",
                                    fcitx::update::UpdateOwner::builtin);
    // 1.1.0 becomes healthy, leaving 1.0.0 as the previous-known-good that a
    // later --cleanup-previous is allowed to delete.
    fcitx::update::mark_current_healthy(root, "stable");
    const auto state = fcitx::update::read_deployment_state(root, "stable");
    if (state.previous != "1.0.0" || !state.healthy) {
      std::cerr << "deployment state setup failed\n";
      return 1;
    }
  } catch (const std::exception& error) {
    std::cerr << "deployment state setup threw: " << error.what() << '\n';
    return 1;
  }

  // Simulate the installed previous version directory.
  const auto previousDir = root / "packages" / "versions" / "core" / "1.0.0";
  std::filesystem::create_directories(previousDir);
  {
    std::ofstream marker(previousDir / "marker.txt");
    marker << "keep";
  }

  // Malicious package id (".." escapes the versions directory): the updater
  // must refuse, and the previous version directory must survive.
  const DWORD malicious =
      run_updater(updater, {L"--cleanup-previous", root.wstring(), L"stable", L".."});
  const bool survived =
      std::filesystem::exists(previousDir / "marker.txt");
  if (malicious == 0 || malicious == 0xFFFFFFFFU || !survived) {
    std::cerr << "malicious cleanup-previous was not refused (exit=" << malicious
              << ", survived=" << survived << ")\n";
    return 1;
  }

  // Valid core package id: the previous version directory is removed.
  const DWORD valid =
      run_updater(updater, {L"--cleanup-previous", root.wstring(), L"stable", L"core"});
  const bool removed = !std::filesystem::exists(previousDir / "marker.txt");
  if (valid != 0 || !removed) {
    std::cerr << "valid cleanup-previous failed (exit=" << valid
              << ", removed=" << removed << ")\n";
    return 1;
  }

  std::filesystem::remove_all(root, ignored);
  std::cout << "updater-cleanup-previous ok\n";
  return 0;
}
