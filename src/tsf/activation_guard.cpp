#include "activation_guard.h"

#include <fcitx5_windows/release_identity.h>

#include <ShlObj.h>
#include <Windows.h>

#include <algorithm>
#include <array>
#include <cwctype>
#include <fstream>
#include <iterator>

namespace fcitx::windows::tsf {
namespace {

namespace fs = std::filesystem;

constexpr wchar_t kRecoveryDirectory[] = L"recovery";
constexpr wchar_t kDisabledMarker[] = L"tsf-activation-disabled.v1";
constexpr wchar_t kAttemptPrefix[] = L"tsf-activation-attempt.";
constexpr wchar_t kAttemptSuffix[] = L".v1";

[[nodiscard]] std::string sanitizeReason(std::string_view reason) {
    std::string output;
    output.reserve((std::min)(reason.size(), std::size_t{160}));
    for (const unsigned char value : reason) {
        if (output.size() >= 160) break;
        if (value < 0x20U || value == '=')
            output.push_back('_');
        else
            output.push_back(static_cast<char>(value));
    }
    return output.empty() ? "unspecified" : output;
}

[[nodiscard]] fs::path recoveryDirectory(const fs::path& dataRoot) {
    return dataRoot / kRecoveryDirectory;
}

[[nodiscard]] bool validTestNamespace(std::wstring_view value) {
    return !value.empty() && value.size() <= 32 &&
           std::ranges::all_of(value, [](wchar_t character) {
               return (character >= L'a' && character <= L'z') ||
                      (character >= L'0' && character <= L'9') || character == L'-';
           });
}

[[nodiscard]] fs::path testDataRoot() noexcept {
    std::array<wchar_t, 34> testNamespace{};
    const DWORD namespaceLength =
        GetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", testNamespace.data(),
                                static_cast<DWORD>(testNamespace.size()));
    if (namespaceLength == 0 || namespaceLength >= testNamespace.size() ||
        !validTestNamespace(std::wstring_view(testNamespace.data(), namespaceLength))) {
        return {};
    }
    std::wstring root(32768, L'\0');
    const DWORD rootLength =
        GetEnvironmentVariableW(L"FCITX5_TEST_DATA_ROOT", root.data(),
                                static_cast<DWORD>(root.size()));
    if (rootLength == 0 || rootLength >= root.size()) return {};
    root.resize(rootLength);
    try {
        fs::path path(root);
        return path.is_absolute() ? path : fs::path{};
    } catch (...) {
        return {};
    }
}

[[nodiscard]] bool isAttemptFile(const fs::path& path) {
    const auto name = path.filename().wstring();
    constexpr std::wstring_view prefix(kAttemptPrefix);
    constexpr std::wstring_view suffix(kAttemptSuffix);
    return name.size() > prefix.size() + suffix.size() &&
           name.starts_with(prefix) && name.ends_with(suffix);
}

[[nodiscard]] bool atomicWriteUtf8(const fs::path& destination, std::string_view text) noexcept {
    try {
        std::error_code error;
        fs::create_directories(destination.parent_path(), error);
        if (error) return false;
        const fs::path temporary =
            destination.wstring() + L".tmp." + std::to_wstring(GetCurrentProcessId()) +
            L"." + std::to_wstring(GetTickCount64());
        HANDLE file = CreateFileW(temporary.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_NEW,
                                  FILE_ATTRIBUTE_NORMAL | FILE_FLAG_WRITE_THROUGH, nullptr);
        if (file == INVALID_HANDLE_VALUE) return false;
        DWORD written = 0;
        const bool writtenOk =
            text.size() <= MAXDWORD &&
            WriteFile(file, text.data(), static_cast<DWORD>(text.size()), &written, nullptr) &&
            written == text.size() && FlushFileBuffers(file);
        CloseHandle(file);
        if (!writtenOk) {
            DeleteFileW(temporary.c_str());
            return false;
        }
        if (!MoveFileExW(temporary.c_str(), destination.c_str(),
                         MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
            DeleteFileW(temporary.c_str());
            return false;
        }
        return true;
    } catch (...) {
        return false;
    }
}

[[nodiscard]] std::string readReason(const fs::path& marker) noexcept {
    try {
        std::ifstream input(marker, std::ios::binary);
        if (!input) return {};
        std::string text(std::istreambuf_iterator<char>(input), {});
        constexpr std::string_view key = "reason=";
        const auto position = text.find(key);
        if (position == std::string::npos) return {};
        auto end = text.find('\n', position + key.size());
        if (end == std::string::npos) end = text.size();
        return text.substr(position + key.size(), end - position - key.size());
    } catch (...) {
        return {};
    }
}

[[nodiscard]] bool staleAttemptExists(const fs::path& directory,
                                      std::chrono::seconds threshold) noexcept {
    try {
        std::error_code error;
        if (!fs::is_directory(directory, error)) return false;
        const auto now = fs::file_time_type::clock::now();
        for (const auto& entry : fs::directory_iterator(directory, error)) {
            if (error) return false;
            if (!entry.is_regular_file(error) || error || !isAttemptFile(entry.path())) continue;
            const auto timestamp = entry.last_write_time(error);
            if (!error && now - timestamp > threshold) return true;
        }
    } catch (...) {
    }
    return false;
}

void removeAttemptFiles(const fs::path& directory) noexcept {
    try {
        std::error_code error;
        if (!fs::is_directory(directory, error)) return;
        for (const auto& entry : fs::directory_iterator(directory, error)) {
            if (error) return;
            if (entry.is_regular_file(error) && !error && isAttemptFile(entry.path())) {
                fs::remove(entry.path(), error);
            }
        }
    } catch (...) {
    }
}

} // namespace

std::filesystem::path defaultActivationGuardDataRoot() noexcept {
    if (const auto root = testDataRoot(); !root.empty()) return root;
    PWSTR localAppData = nullptr;
    if (FAILED(SHGetKnownFolderPath(FOLDERID_LocalAppData, KF_FLAG_CREATE, nullptr,
                                    &localAppData))) {
        return {};
    }
    fs::path result;
    try {
        result = fs::path(localAppData) / kReleaseIdentity.data_directory;
    } catch (...) {
        result.clear();
    }
    CoTaskMemFree(localAppData);
    return result;
}

std::filesystem::path activationGuardMarkerPath(const std::filesystem::path& dataRoot) {
    return recoveryDirectory(dataRoot) / kDisabledMarker;
}

ActivationGuardStatus activationGuardStatus(const std::filesystem::path& dataRoot) noexcept {
    ActivationGuardStatus status;
    try {
        status.markerPath = activationGuardMarkerPath(dataRoot);
        std::error_code error;
        status.disabled = fs::is_regular_file(status.markerPath, error) && !error;
        if (status.disabled) status.reason = readReason(status.markerPath);
    } catch (...) {
        status = {};
    }
    return status;
}

bool disableActivationGuard(const std::filesystem::path& dataRoot,
                            std::string_view reason) noexcept {
    const std::string sanitized = sanitizeReason(reason);
    return atomicWriteUtf8(activationGuardMarkerPath(dataRoot),
                           "format_version=1\nreason=" + sanitized + "\n");
}

bool clearActivationGuard(const std::filesystem::path& dataRoot) noexcept {
    try {
        const auto directory = recoveryDirectory(dataRoot);
        std::error_code error;
        fs::remove(activationGuardMarkerPath(dataRoot), error);
        removeAttemptFiles(directory);
        return true;
    } catch (...) {
        return false;
    }
}

ActivationAttempt::~ActivationAttempt() { finish(); }

ActivationAttempt::ActivationAttempt(ActivationAttempt&& other) noexcept
    : dataRoot_(std::move(other.dataRoot_)),
      attemptPath_(std::move(other.attemptPath_)),
      reason_(std::move(other.reason_)),
      failOpen_(other.failOpen_),
      active_(other.active_) {
    other.active_ = false;
}

ActivationAttempt& ActivationAttempt::operator=(ActivationAttempt&& other) noexcept {
    if (this != &other) {
        finish();
        dataRoot_ = std::move(other.dataRoot_);
        attemptPath_ = std::move(other.attemptPath_);
        reason_ = std::move(other.reason_);
        failOpen_ = other.failOpen_;
        active_ = other.active_;
        other.active_ = false;
    }
    return *this;
}

ActivationAttempt ActivationAttempt::begin(const std::filesystem::path& dataRoot,
                                           std::chrono::seconds staleThreshold) noexcept {
    ActivationAttempt attempt;
    attempt.dataRoot_ = dataRoot;
    const auto status = activationGuardStatus(dataRoot);
    if (status.disabled) {
        attempt.failOpen_ = true;
        attempt.reason_ = status.reason.empty() ? "disabled_marker" : status.reason;
        return attempt;
    }
    const auto directory = recoveryDirectory(dataRoot);
    if (staleAttemptExists(directory, staleThreshold)) {
        (void)disableActivationGuard(dataRoot, "previous_activation_did_not_finish");
        removeAttemptFiles(directory);
        attempt.failOpen_ = true;
        attempt.reason_ = "previous_activation_did_not_finish";
        return attempt;
    }
    try {
        std::error_code error;
        fs::create_directories(directory, error);
        if (error) return attempt;
        attempt.attemptPath_ = directory / (std::wstring(kAttemptPrefix) +
                                            std::to_wstring(GetCurrentProcessId()) + L"." +
                                            std::to_wstring(GetTickCount64()) +
                                            std::wstring(kAttemptSuffix));
        if (atomicWriteUtf8(attempt.attemptPath_, "format_version=1\nstate=activating\n")) {
            attempt.active_ = true;
        }
    } catch (...) {
        attempt.attemptPath_.clear();
        attempt.active_ = false;
    }
    return attempt;
}

void ActivationAttempt::finish() noexcept {
    if (!active_ || attemptPath_.empty()) return;
    try {
        std::error_code error;
        fs::remove(attemptPath_, error);
    } catch (...) {
    }
    active_ = false;
}

void ActivationAttempt::disableAndFinish(std::string_view reason) noexcept {
    reason_ = sanitizeReason(reason);
    failOpen_ = true;
    if (!dataRoot_.empty()) (void)disableActivationGuard(dataRoot_, reason_);
    finish();
}

} // namespace fcitx::windows::tsf
