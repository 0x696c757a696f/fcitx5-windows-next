#include "package_core.h"
#include "provider_policy.h"

#include <windows.h>

#include <filesystem>
#include <fstream>
#include <iostream>

namespace {

template <typename Callable>
void expect_rejected(Callable&& callable) {
  try {
    callable();
  } catch (const fcitx::package::PackageError&) {
    return;
  }
  throw std::runtime_error("unsafe Plum plan was accepted");
}

}  // namespace

int main() {
  const auto root = std::filesystem::temp_directory_path() /
                    (L"fcitx5-provider-test-" + std::to_wstring(GetCurrentProcessId()));
  try {
    std::error_code ignored;
    std::filesystem::remove_all(root, ignored);
    const auto provider = root / "plum";
    const auto user = root / "rime-user";
    const auto cache = root / "cache";
    std::filesystem::create_directories(provider);
    std::filesystem::create_directories(user);
    std::filesystem::create_directories(cache);
    std::ofstream(provider / "rime-install.bat") << "@exit /b 0\r\n";
    const auto official = fcitx::package::make_plum_plan(provider, user, cache, ":preset");
    if (official.trust != fcitx::package::ProviderTrust::official ||
        official.rime_user_directory != user) {
      throw std::runtime_error("official Plum plan mismatch");
    }
    const auto community =
        fcitx::package::make_plum_plan(provider, user, cache, "someone/schema");
    if (community.trust != fcitx::package::ProviderTrust::unverified) {
      throw std::runtime_error("community source trust mismatch");
    }
    expect_rejected([&] {
      static_cast<void>(fcitx::package::make_plum_plan(provider, {}, cache, ":preset"));
    });
    expect_rejected([&] {
      static_cast<void>(
          fcitx::package::make_plum_plan(provider, user, cache, "repo & calc.exe"));
    });
    std::filesystem::remove_all(root, ignored);
    std::cout << "provider policy contract passed\n";
    return 0;
  } catch (const std::exception& error) {
    std::error_code ignored;
    std::filesystem::remove_all(root, ignored);
    std::cerr << error.what() << '\n';
    return 1;
  }
}
