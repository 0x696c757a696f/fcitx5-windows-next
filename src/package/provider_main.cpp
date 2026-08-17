#include "package_core.h"
#include "provider_policy.h"

#include <windows.h>

#include <fcitx5_windows/version.h>

#include <filesystem>
#include <iostream>
#include <string>
#include <string_view>
#include <vector>

namespace {

class EnvironmentOverride final {
 public:
  EnvironmentOverride(const wchar_t* name, const std::wstring& value) : name_(name) {
    const DWORD required = GetEnvironmentVariableW(name, nullptr, 0);
    if (required > 0U) {
      old_.resize(required);
      const DWORD copied = GetEnvironmentVariableW(name, old_.data(), required);
      if (copied > 0U) {
        old_.resize(copied);
        existed_ = true;
      }
    }
    if (!SetEnvironmentVariableW(name, value.c_str())) {
      throw fcitx::package::PackageError("provider_failed", "provider environment setup failed");
    }
  }
  ~EnvironmentOverride() {
    static_cast<void>(SetEnvironmentVariableW(name_.c_str(), existed_ ? old_.c_str() : nullptr));
  }
  EnvironmentOverride(const EnvironmentOverride&) = delete;
  EnvironmentOverride& operator=(const EnvironmentOverride&) = delete;

 private:
  std::wstring name_;
  std::wstring old_;
  bool existed_{};
};

bool is_elevated() {
  HANDLE token = nullptr;
  if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) return true;
  TOKEN_ELEVATION elevation{};
  DWORD returned = 0;
  const bool result = GetTokenInformation(token, TokenElevation, &elevation, sizeof(elevation),
                                          &returned) != FALSE &&
                      elevation.TokenIsElevated != 0U;
  CloseHandle(token);
  return result;
}

int execute(const fcitx::package::PlumPlan& plan, bool allow_unverified) {
  if (is_elevated()) {
    throw fcitx::package::PackageError("privilege_boundary", "Plum provider refuses elevation");
  }
  if (plan.trust == fcitx::package::ProviderTrust::unverified && !allow_unverified) {
    throw fcitx::package::PackageError(
        "trust_confirmation_required", "third-party Rime source requires explicit confirmation");
  }
  wchar_t system_directory[MAX_PATH]{};
  if (GetSystemDirectoryW(system_directory, MAX_PATH) == 0U) {
    throw fcitx::package::PackageError("provider_failed", "System32 path is unavailable");
  }
  const auto command_processor = std::filesystem::path(system_directory) / "cmd.exe";
  EnvironmentOverride rime_dir(L"rime_dir", plan.rime_user_directory.wstring());
  EnvironmentOverride frontend(L"rime_frontend", L"fcitx5-rime");
  EnvironmentOverride cache(L"download_cache_dir", plan.download_cache_directory.wstring());
  std::wstring command = L"\"" + command_processor.wstring() + L"\" /d /s /c \"\"" +
                         plan.script.wstring() + L"\" " +
                         std::filesystem::path(plan.package_spec).wstring() + L"\"";
  std::vector<wchar_t> mutable_command(command.begin(), command.end());
  mutable_command.push_back(L'\0');
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION process{};
  if (!CreateProcessW(command_processor.c_str(), mutable_command.data(), nullptr, nullptr, FALSE,
                      CREATE_NO_WINDOW, nullptr, plan.working_directory.c_str(), &startup,
                      &process)) {
    throw fcitx::package::PackageError("provider_failed", "Plum process launch failed");
  }
  CloseHandle(process.hThread);
  const DWORD wait = WaitForSingleObject(process.hProcess, 10U * 60U * 1000U);
  DWORD exit_code = 1;
  if (wait != WAIT_OBJECT_0 || !GetExitCodeProcess(process.hProcess, &exit_code)) {
    TerminateProcess(process.hProcess, 2U);
    WaitForSingleObject(process.hProcess, 5000U);
    CloseHandle(process.hProcess);
    throw fcitx::package::PackageError("provider_failed", "Plum timed out");
  }
  CloseHandle(process.hProcess);
  return static_cast<int>(exit_code);
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
  try {
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
      std::cout << "fcitx5-provider " << fcitx::windows::version() << '\n';
      return 0;
    }
    bool allow_unverified = false;
    int offset = 0;
    if (argc == 7 && std::wstring_view(argv[1]) == L"--allow-unverified") {
      allow_unverified = true;
      offset = 1;
    }
    if (argc == 6 + offset && std::wstring_view(argv[1 + offset]) == L"--plum") {
      const auto plan = fcitx::package::make_plum_plan(
          argv[2 + offset], argv[3 + offset], argv[4 + offset],
          std::filesystem::path(argv[5 + offset]).string());
      std::cout << "provider=plum\nsource=" << plan.package_spec
                << "\ntrust="
                << (plan.trust == fcitx::package::ProviderTrust::official ? "official"
                                                                           : "unverified")
                << "\nrime_user_dir_explicit=true\n";
      return execute(plan, allow_unverified);
    }
    std::wcerr << L"Usage: fcitx5-provider [--allow-unverified] --plum PROVIDER_ROOT "
                  L"RIME_USER_DIR CACHE_DIR PACKAGE_SPEC\n";
    return 1;
  } catch (const fcitx::package::PackageError& error) {
    std::cerr << error.code() << ": " << error.what() << '\n';
    return 2;
  } catch (const std::exception& error) {
    std::cerr << "internal_error: " << error.what() << '\n';
    return 3;
  }
}
