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
#include <iterator>
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

void write_text(const std::filesystem::path& path, std::string_view text) {
  std::filesystem::create_directories(path.parent_path());
  std::ofstream output(path, std::ios::binary | std::ios::trunc);
  output << text;
}

std::string read_text(const std::filesystem::path& path) {
  std::ifstream input(path, std::ios::binary);
  return {std::istreambuf_iterator<char>(input), {}};
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

  const auto registeredTsf = root / L"tsf" / L"x64" / L"fcitx5-tsf.dll";
  const auto incomingTsf = root / L"staging" / L"fcitx5-tsf.dll";
  write_text(registeredTsf, "old-tsf");
  write_text(incomingTsf, "new-tsf");
  const DWORD tsfInstall =
      run_updater(updater, {L"--install-tsf-dll", registeredTsf.wstring(),
                            incomingTsf.wstring(), L"00000042"});
  if (tsfInstall != 0 || read_text(registeredTsf) != "new-tsf") {
    std::cerr << "updater TSF DLL install failed (exit=" << tsfInstall << ")\n";
    return 1;
  }
  const DWORD maliciousTsfInstall =
      run_updater(updater, {L"--install-tsf-dll", root.wstring(), incomingTsf.wstring(),
                            L"00000042"});
  if (maliciousTsfInstall == 0) {
    std::cerr << "updater accepted invalid TSF registered path\n";
    return 1;
  }
  const auto staleOldTsf = root / L"tsf" / L"x64" / L"fcitx5-tsf.old.00000041.test.dll";
  const auto unrelatedDll = root / L"tsf" / L"x64" / L"not-owned.dll";
  write_text(staleOldTsf, "stale");
  write_text(unrelatedDll, "keep");
  const DWORD tsfCleanup =
      run_updater(updater, {L"--cleanup-old-tsf-dlls", (root / L"tsf" / L"x64").wstring()});
  if (tsfCleanup != 0 || std::filesystem::exists(staleOldTsf) ||
      !std::filesystem::exists(unrelatedDll)) {
    std::cerr << "updater old TSF cleanup failed (exit=" << tsfCleanup << ")\n";
    return 1;
  }
  const DWORD missingGeneration =
      run_updater(updater, {L"--publish-generation", root.wstring(), L"00000042", L"build-42"});
  if (missingGeneration == 0) {
    std::cerr << "updater published a missing runtime generation\n";
    return 1;
  }
  std::filesystem::create_directories(root / L"runtime" / L"00000042");
  const DWORD publishGeneration =
      run_updater(updater, {L"--publish-generation", root.wstring(), L"00000042", L"build-42"});
  const DWORD generationStatus =
      run_updater(updater, {L"--generation-status", root.wstring()});
  if (publishGeneration != 0 || generationStatus != 0) {
    std::cerr << "updater runtime generation publication failed (publish="
              << publishGeneration << ", status=" << generationStatus << ")\n";
    return 1;
  }
  const auto runtimePayload = root / L"runtime-payload";
  write_text(runtimePayload / L"bin" / L"fcitx5-engine.exe", "engine-43");
  write_text(runtimePayload / L"bin" / L"fcitx5-launcher.exe", "launcher-43");
  write_text(runtimePayload / L"bin" / L"fcitx5-ui.exe", "ui-43");
  write_text(runtimePayload / L"lib" / L"fcitx5" / L"addon.dll", "addon-43");
  write_text(runtimePayload / L"share" / L"fcitx5" / L"profile", "share-43");
  write_text(runtimePayload / L"tsf" / L"x64" / L"fcitx5-tsf.dll", "tsf-x64-43");
  write_text(runtimePayload / L"tsf" / L"x86" / L"fcitx5-tsf.dll", "tsf-x86-43");
  const DWORD activateRuntime =
      run_updater(updater, {L"--activate-runtime-generation", root.wstring(),
                            runtimePayload.wstring(), L"00000043", L"build-43"});
  if (activateRuntime != 0 ||
      read_text(root / L"runtime" / L"00000043" / L"bin" / L"fcitx5-engine.exe") !=
          "engine-43" ||
      read_text(root / L"tsf" / L"x64" / L"fcitx5-tsf.dll") != "tsf-x64-43" ||
      read_text(root / L"tsf" / L"x64" / L"fcitx5-tsf.generation") != "00000043\n") {
    std::cerr << "updater runtime generation activation failed (exit="
              << activateRuntime << ")\n";
    return 1;
  }

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
