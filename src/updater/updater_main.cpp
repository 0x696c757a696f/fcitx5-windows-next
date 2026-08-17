#include "deployment_core.h"
#include "package_core.h"

#include <fcitx5_windows/version.h>

#include <filesystem>
#include <algorithm>
#include <fstream>
#include <iostream>
#include <span>
#include <string>
#include <string_view>

namespace {

std::string read_bounded(const std::filesystem::path& path, std::size_t maximum) {
  std::error_code error;
  const auto size = std::filesystem::file_size(path, error);
  if (error || size > maximum) throw std::runtime_error("release metadata is unavailable");
  std::ifstream input(path, std::ios::binary);
  std::string bytes(static_cast<std::size_t>(size), '\0');
  if (!bytes.empty()) input.read(bytes.data(), static_cast<std::streamsize>(bytes.size()));
  if (!input) throw std::runtime_error("release metadata read failed");
  return bytes;
}

int usage() {
  std::wcerr << L"Usage:\n"
                L"  fcitx5-updater --record-owner ROOT OWNER\n"
                L"  fcitx5-updater --activate ARCHIVE ROOT TRANSACTION KEYRING CHANNEL\n"
                L"  fcitx5-updater --health ROOT CHANNEL\n"
                L"  fcitx5-updater --rollback ROOT CHANNEL CORE_PACKAGE_ID KEYRING\n"
                L"  fcitx5-updater --cleanup-previous ROOT CHANNEL CORE_PACKAGE_ID\n"
                L"  fcitx5-updater --status ROOT CHANNEL\n";
  return 1;
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
  try {
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
      std::cout << "fcitx5-updater " << fcitx::windows::version() << '\n';
      return 0;
    }
    if (argc == 4 && std::wstring_view(argv[1]) == L"--record-owner") {
      fcitx::update::write_update_owner(argv[2],
                                        fcitx::update::parse_owner(std::filesystem::path(argv[3]).string()));
      return 0;
    }
    if (argc == 7 && std::wstring_view(argv[1]) == L"--activate") {
      const std::filesystem::path root(argv[3]);
      const auto owner = fcitx::update::read_update_owner(root);
      if (owner != fcitx::update::UpdateOwner::builtin)
        throw std::runtime_error("Core update is owned by an external package manager");
      const auto keys = fcitx::package::read_trusted_keys(argv[5]);
      const auto oldLock = fcitx::package::read_lockfile(root / "packages");
      const auto staged = fcitx::package::stage_verified_archive(
          argv[2], root / "packages", std::filesystem::path(argv[4]).string(), keys);
      const auto manifest = fcitx::package::parse_manifest(
          read_bounded(staged / "manifest.json", fcitx::package::kMaximumManifestBytes));
      if (manifest.type != fcitx::package::PackageType::core)
        throw std::runtime_error("updater accepts only a complete Core release package");
      fcitx::package::activate_staged_package(staged, root / "packages", keys);
      try {
        fcitx::update::begin_activation(root, std::filesystem::path(argv[6]).string(),
                                        manifest.version, owner);
      } catch (...) {
        const auto old = std::ranges::find_if(oldLock, [&](const fcitx::package::LockEntry& item) {
          return item.id == manifest.id;
        });
        if (old != oldLock.end())
          fcitx::package::activate_installed_version(root / "packages", old->id, old->version, keys);
        throw;
      }
      std::cout << "activation=pending_health\nversion=" << manifest.version << '\n';
      return 0;
    }
    if (argc == 4 && std::wstring_view(argv[1]) == L"--health") {
      fcitx::update::mark_current_healthy(argv[2], std::filesystem::path(argv[3]).string());
      return 0;
    }
    if (argc == 6 && std::wstring_view(argv[1]) == L"--rollback") {
      const std::filesystem::path root(argv[2]);
      const auto channel = std::filesystem::path(argv[3]).string();
      const auto target = fcitx::update::rollback_target(root, channel);
      const auto keys = fcitx::package::read_trusted_keys(argv[5]);
      fcitx::package::activate_installed_version(root / "packages",
                                                 std::filesystem::path(argv[4]).string(), target,
                                                 keys);
      fcitx::update::finish_rollback(root, channel);
      std::cout << "rollback=" << target << '\n';
      return 0;
    }
    if (argc == 5 && std::wstring_view(argv[1]) == L"--cleanup-previous") {
      const std::filesystem::path root(argv[2]);
      const auto channel = std::filesystem::path(argv[3]).string();
      const auto state = fcitx::update::read_deployment_state(root, channel);
      fcitx::update::clear_previous_known_good(root, channel);
      if (!state.previous.empty()) {
        std::error_code ignored;
        std::filesystem::remove_all(root / "packages/versions" /
                                        std::filesystem::path(argv[4]) / state.previous,
                                    ignored);
      }
      return 0;
    }
    if (argc == 4 && std::wstring_view(argv[1]) == L"--status") {
      const auto state = fcitx::update::read_deployment_state(
          argv[2], std::filesystem::path(argv[3]).string());
      std::cout << "channel=" << state.channel << "\nupdate_owner="
                << fcitx::update::owner_name(state.owner) << "\ncurrent=" << state.current
                << "\nprevious=" << state.previous << "\npending=" << state.pending
                << "\nhealthy=" << (state.healthy ? "true" : "false") << '\n';
      return 0;
    }
    return usage();
  } catch (const std::exception& error) {
    std::cerr << "update_failed: " << error.what() << '\n';
    return 2;
  }
}
