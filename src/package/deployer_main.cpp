#include "package_core.h"

#include <windows.h>
#include <shlobj.h>

#include <fcitx5_windows/version.h>

#include <array>
#include <filesystem>
#include <iostream>
#include <string>
#include <string_view>
#include <vector>

namespace {

constexpr std::uint64_t kMaximumArtifactBytes = 128ULL * 1024ULL * 1024ULL;

class Handle final {
 public:
  explicit Handle(HANDLE handle = INVALID_HANDLE_VALUE) : handle_(handle) {}
  ~Handle() {
    if (handle_ != INVALID_HANDLE_VALUE && handle_ != nullptr) CloseHandle(handle_);
  }
  Handle(const Handle&) = delete;
  Handle& operator=(const Handle&) = delete;
  [[nodiscard]] HANDLE get() const noexcept { return handle_; }

 private:
  HANDLE handle_;
};

bool is_elevated() {
  HANDLE raw_token = nullptr;
  if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw_token)) return false;
  Handle token(raw_token);
  TOKEN_ELEVATION elevation{};
  DWORD returned = 0;
  return GetTokenInformation(token.get(), TokenElevation, &elevation, sizeof(elevation),
                             &returned) != FALSE &&
         elevation.TokenIsElevated != 0U;
}

std::filesystem::path module_path() {
  std::wstring buffer(32'768U, L'\0');
  const DWORD length =
      GetModuleFileNameW(nullptr, buffer.data(), static_cast<DWORD>(buffer.size()));
  if (length == 0U || length >= buffer.size()) {
    throw fcitx::package::PackageError("privilege_boundary", "module path is unavailable");
  }
  buffer.resize(length);
  return std::filesystem::path(buffer);
}

bool ordinal_path_prefix(const std::filesystem::path& candidate,
                         const std::filesystem::path& parent) {
  const auto normalized_candidate = std::filesystem::weakly_canonical(candidate).wstring();
  auto normalized_parent = std::filesystem::weakly_canonical(parent).wstring();
  if (!normalized_parent.ends_with(L'\\')) normalized_parent.push_back(L'\\');
  if (normalized_candidate.size() <= normalized_parent.size()) return false;
  return CompareStringOrdinal(normalized_candidate.data(), static_cast<int>(normalized_parent.size()),
                              normalized_parent.data(), static_cast<int>(normalized_parent.size()),
                              TRUE) == CSTR_EQUAL;
}

std::filesystem::path protected_install_root() {
  const auto executable = module_path();
  const auto root = executable.parent_path().parent_path();
  wchar_t program_files[MAX_PATH]{};
  if (SHGetFolderPathW(nullptr, CSIDL_PROGRAM_FILES, nullptr, SHGFP_TYPE_CURRENT, program_files) !=
          S_OK ||
      !ordinal_path_prefix(root, program_files) ||
      _wcsicmp(executable.filename().c_str(), L"fcitx5-deployer.exe") != 0 ||
      _wcsicmp(executable.parent_path().filename().c_str(), L"bin") != 0) {
    throw fcitx::package::PackageError(
        "privilege_boundary", "deployer must run from Program Files/Fcitx5/bin");
  }
  return root;
}

void copy_exclusive_artifact(const std::filesystem::path& source,
                             const std::filesystem::path& destination) {
  Handle input(CreateFileW(source.c_str(), GENERIC_READ, 0, nullptr, OPEN_EXISTING,
                           FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OPEN_REPARSE_POINT, nullptr));
  if (input.get() == INVALID_HANDLE_VALUE) {
    throw fcitx::package::PackageError("artifact_changed", "artifact cannot be opened exclusively");
  }
  FILE_ATTRIBUTE_TAG_INFO tag{};
  LARGE_INTEGER size{};
  if (!GetFileInformationByHandleEx(input.get(), FileAttributeTagInfo, &tag, sizeof(tag)) ||
      (tag.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT) != 0U ||
      !GetFileSizeEx(input.get(), &size) || size.QuadPart <= 0 ||
      static_cast<std::uint64_t>(size.QuadPart) > kMaximumArtifactBytes) {
    throw fcitx::package::PackageError("artifact_changed", "artifact identity or size is unsafe");
  }
  Handle output(CreateFileW(destination.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                            FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH, nullptr));
  if (output.get() == INVALID_HANDLE_VALUE) {
    throw fcitx::package::PackageError("io_error", "protected artifact copy cannot be created");
  }
  std::vector<std::byte> buffer(64U * 1024U);
  for (;;) {
    DWORD read = 0;
    if (!ReadFile(input.get(), buffer.data(), static_cast<DWORD>(buffer.size()), &read, nullptr)) {
      throw fcitx::package::PackageError("io_error", "artifact read failed");
    }
    if (read == 0U) break;
    DWORD written = 0;
    if (!WriteFile(output.get(), buffer.data(), read, &written, nullptr) || written != read) {
      throw fcitx::package::PackageError("io_error", "protected artifact copy failed");
    }
  }
  if (!FlushFileBuffers(output.get())) {
    throw fcitx::package::PackageError("io_error", "protected artifact flush failed");
  }
}

bool is_hex_digest(std::string_view value) {
  if (value.size() != 64U) return false;
  for (const char character : value) {
    if (!((character >= '0' && character <= '9') ||
          (character >= 'a' && character <= 'f') ||
          (character >= 'A' && character <= 'F'))) {
      return false;
    }
  }
  return true;
}

void activate(const std::filesystem::path& source, std::string_view expected_hash,
              std::string_view transaction_id) {
  if (!is_elevated() || !is_hex_digest(expected_hash) ||
      !fcitx::package::is_safe_relative_package_path(transaction_id) ||
      transaction_id.find('/') != std::string_view::npos) {
    throw fcitx::package::PackageError("privilege_boundary", "deployer request is invalid");
  }
  const auto root = protected_install_root();
  const auto keyring = root / "security/trusted-keys.json";
  const auto keys = fcitx::package::read_trusted_keys(keyring);
  const auto transactions = root / ".transactions" / std::filesystem::path(transaction_id);
  if (std::filesystem::exists(transactions)) {
    throw fcitx::package::PackageError("transaction_exists", "protected transaction exists");
  }
  std::filesystem::create_directories(transactions);
  const auto protected_archive = transactions / "artifact.fcpkg";
  try {
    copy_exclusive_artifact(source, protected_archive);
    if (fcitx::package::hex_sha256(fcitx::package::sha256_file(protected_archive)) !=
        expected_hash) {
      throw fcitx::package::PackageError("hash_mismatch",
                                         "artifact changed across elevation boundary");
    }
    const auto staged = fcitx::package::stage_verified_archive(
        protected_archive, root, transaction_id, keys);
    fcitx::package::activate_staged_package(staged, root, keys);
    std::error_code ignored;
    std::filesystem::remove_all(transactions, ignored);
  } catch (...) {
    std::error_code ignored;
    std::filesystem::remove_all(transactions, ignored);
    throw;
  }
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
  try {
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
      std::cout << "fcitx5-deployer " << fcitx::windows::version() << '\n';
      return 0;
    }
    if (argc == 2 && std::wstring_view(argv[1]) == L"--self-test") {
      try {
        static_cast<void>(protected_install_root());
        return 1;
      } catch (const fcitx::package::PackageError&) {
        return 0;
      }
    }
    if (argc == 5 && std::wstring_view(argv[1]) == L"--activate") {
      activate(argv[2], std::filesystem::path(argv[3]).string(),
               std::filesystem::path(argv[4]).string());
      return 0;
    }
    std::wcerr << L"Usage: fcitx5-deployer --activate LOCAL_ARCHIVE SHA256 TRANSACTION_ID\n";
    return 1;
  } catch (const fcitx::package::PackageError& error) {
    std::cerr << error.code() << ": " << error.what() << '\n';
    return 2;
  } catch (const std::exception& error) {
    std::cerr << "internal_error: " << error.what() << '\n';
    return 3;
  }
}
