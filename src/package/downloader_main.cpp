#include "package_core.h"

#include <windows.h>
#include <winhttp.h>

#include <fcitx5_windows/version.h>

#include <array>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <string_view>
#include <vector>

namespace {

constexpr std::uint64_t kMaximumDownloadBytes = 128ULL * 1024ULL * 1024ULL;

class InternetHandle final {
 public:
  explicit InternetHandle(HINTERNET handle = nullptr) : handle_(handle) {}
  ~InternetHandle() {
    if (handle_ != nullptr) WinHttpCloseHandle(handle_);
  }
  InternetHandle(const InternetHandle&) = delete;
  InternetHandle& operator=(const InternetHandle&) = delete;
  [[nodiscard]] HINTERNET get() const noexcept { return handle_; }

 private:
  HINTERNET handle_{};
};

bool is_elevated() {
  HANDLE token = nullptr;
  if (!OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &token)) return true;
  TOKEN_ELEVATION elevation{};
  DWORD size = 0;
  const bool result = GetTokenInformation(token, TokenElevation, &elevation, sizeof(elevation),
                                          &size) != FALSE &&
                      elevation.TokenIsElevated != 0U;
  CloseHandle(token);
  return result;
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

bool crack_https_url(std::wstring_view url, std::wstring& host, INTERNET_PORT& port,
                     std::wstring& target) {
  if (url.size() > 4096U) return false;
  URL_COMPONENTS components{};
  components.dwStructSize = sizeof(components);
  components.dwSchemeLength = static_cast<DWORD>(-1);
  components.dwHostNameLength = static_cast<DWORD>(-1);
  components.dwUserNameLength = static_cast<DWORD>(-1);
  components.dwPasswordLength = static_cast<DWORD>(-1);
  components.dwUrlPathLength = static_cast<DWORD>(-1);
  components.dwExtraInfoLength = static_cast<DWORD>(-1);
  if (url.find(L'#') != std::wstring_view::npos ||
      !WinHttpCrackUrl(url.data(), static_cast<DWORD>(url.size()), 0, &components) ||
      components.nScheme != INTERNET_SCHEME_HTTPS || components.lpszHostName == nullptr ||
      components.dwHostNameLength == 0U || components.lpszUserName != nullptr ||
      components.lpszPassword != nullptr) {
    return false;
  }
  host.assign(components.lpszHostName, components.dwHostNameLength);
  target.assign(components.lpszUrlPath, components.dwUrlPathLength);
  if (components.lpszExtraInfo != nullptr) {
    target.append(components.lpszExtraInfo, components.dwExtraInfoLength);
  }
  port = components.nPort;
  return !target.empty();
}

void download(std::wstring_view url, std::string_view expected_hash,
              const std::filesystem::path& destination) {
  if (is_elevated()) {
    throw fcitx::package::PackageError("privilege_boundary",
                                       "downloader refuses to run elevated");
  }
  if (!destination.is_absolute() || (!expected_hash.empty() && !is_hex_digest(expected_hash)) ||
      std::filesystem::exists(destination)) {
    throw fcitx::package::PackageError("invalid_download", "destination or hash is invalid");
  }
  std::wstring host;
  std::wstring target;
  INTERNET_PORT port = 0;
  if (!crack_https_url(url, host, port, target)) {
    throw fcitx::package::PackageError("invalid_download", "only credential-free HTTPS is allowed");
  }
  InternetHandle session(WinHttpOpen(L"Fcitx5-Package/1", WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                                     WINHTTP_NO_PROXY_NAME, WINHTTP_NO_PROXY_BYPASS, 0));
  if (session.get() == nullptr) throw fcitx::package::PackageError("network_error", "session failed");
  InternetHandle connection(WinHttpConnect(session.get(), host.c_str(), port, 0));
  if (connection.get() == nullptr)
    throw fcitx::package::PackageError("network_error", "connection failed");
  InternetHandle request(WinHttpOpenRequest(connection.get(), L"GET", target.c_str(), nullptr,
                                            WINHTTP_NO_REFERER, WINHTTP_DEFAULT_ACCEPT_TYPES,
                                            WINHTTP_FLAG_SECURE));
  if (request.get() == nullptr)
    throw fcitx::package::PackageError("network_error", "request failed");
  DWORD redirect_policy = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
  if (!WinHttpSetOption(request.get(), WINHTTP_OPTION_REDIRECT_POLICY, &redirect_policy,
                        sizeof(redirect_policy)) ||
      !WinHttpSendRequest(request.get(), WINHTTP_NO_ADDITIONAL_HEADERS, 0,
                          WINHTTP_NO_REQUEST_DATA, 0, 0, 0) ||
      !WinHttpReceiveResponse(request.get(), nullptr)) {
    throw fcitx::package::PackageError("network_error", "HTTPS request failed");
  }
  DWORD status = 0;
  DWORD status_size = sizeof(status);
  if (!WinHttpQueryHeaders(request.get(), WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                           WINHTTP_HEADER_NAME_BY_INDEX, &status, &status_size,
                           WINHTTP_NO_HEADER_INDEX) ||
      status != 200U) {
    throw fcitx::package::PackageError("network_error", "repository returned a non-200 response");
  }

  std::filesystem::create_directories(destination.parent_path());
  const auto partial = std::filesystem::path(destination.wstring() + L".download");
  if (std::filesystem::exists(partial)) {
    throw fcitx::package::PackageError("invalid_download", "partial destination already exists");
  }
  try {
    std::ofstream output(partial, std::ios::binary | std::ios::trunc);
    std::vector<char> buffer(64U * 1024U);
    const std::uint64_t maximum = expected_hash.empty() ? fcitx::package::kMaximumManifestBytes
                                                        : kMaximumDownloadBytes;
    std::uint64_t total = 0;
    for (;;) {
      DWORD read = 0;
      if (!WinHttpReadData(request.get(), buffer.data(), static_cast<DWORD>(buffer.size()), &read)) {
        throw fcitx::package::PackageError("network_error", "response read failed");
      }
      if (read == 0U) break;
      if (total > maximum - read) {
        throw fcitx::package::PackageError("invalid_download", "download exceeds size budget");
      }
      total += read;
      output.write(buffer.data(), static_cast<std::streamsize>(read));
      if (!output) throw fcitx::package::PackageError("io_error", "download write failed");
    }
    output.close();
    if (!expected_hash.empty() &&
        fcitx::package::hex_sha256(fcitx::package::sha256_file(partial)) != expected_hash) {
      throw fcitx::package::PackageError("hash_mismatch", "download SHA-256 differs from metadata");
    }
    if (!MoveFileExW(partial.c_str(), destination.c_str(), MOVEFILE_WRITE_THROUGH)) {
      throw fcitx::package::PackageError("io_error", "download publication failed");
    }
  } catch (...) {
    std::error_code ignored;
    std::filesystem::remove(partial, ignored);
    throw;
  }
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
  try {
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
      std::cout << "fcitx5-downloader " << fcitx::windows::version() << '\n';
      return 0;
    }
    if (argc == 2 && std::wstring_view(argv[1]) == L"--self-test") {
      std::wstring host;
      std::wstring target;
      INTERNET_PORT port = 0;
      if (crack_https_url(L"http://example.invalid/file", host, port, target) ||
          !crack_https_url(L"https://example.invalid/file", host, port, target)) {
        return 1;
      }
      return 0;
    }
    if (argc == 5 && std::wstring_view(argv[1]) == L"--download") {
      download(argv[2], std::filesystem::path(argv[3]).string(), argv[4]);
      return 0;
    }
    if (argc == 4 && std::wstring_view(argv[1]) == L"--download-signed-metadata") {
      download(argv[2], {}, argv[3]);
      return 0;
    }
    std::wcerr << L"Usage:\n"
                  L"  fcitx5-downloader --download HTTPS_URL SHA256 ABSOLUTE_DESTINATION\n"
                  L"  fcitx5-downloader --download-signed-metadata HTTPS_URL ABSOLUTE_DESTINATION\n";
    return 1;
  } catch (const fcitx::package::PackageError& error) {
    std::cerr << error.code() << ": " << error.what() << '\n';
    return 2;
  } catch (const std::exception& error) {
    std::cerr << "internal_error: " << error.what() << '\n';
    return 3;
  }
}
