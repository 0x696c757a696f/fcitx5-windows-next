#pragma once

#include <Windows.h>

#include <string>
#include <string_view>

namespace fcitx::windows::platform {

struct ProcessIdentity {
    DWORD processId{};
    DWORD sessionId{};
    std::wstring userSid;
    std::wstring executablePath;
    bool serviceAccount{};
};

struct RuntimeIdentity : ProcessIdentity {
    bool secureDesktop{};

    [[nodiscard]] bool mayUseUserEngine() const noexcept;
};

[[nodiscard]] bool mayLaunchUserEngine(const RuntimeIdentity& identity) noexcept;

[[nodiscard]] bool queryProcessIdentity(DWORD processId, ProcessIdentity& output) noexcept;
[[nodiscard]] bool queryCurrentIdentity(RuntimeIdentity& output) noexcept;
// Returns an empty string in production. Integration tests may set the validated
// FCITX5_TEST_NAMESPACE environment variable to prevent named-pipe/object collisions
// with a desktop instance that is already running for the same user and session.
[[nodiscard]] std::wstring localTestNamespace();
[[nodiscard]] std::wstring makeLocalEndpointName(const RuntimeIdentity& identity,
                                                 std::wstring_view channel);
[[nodiscard]] std::wstring makeLocalObjectName(const RuntimeIdentity& identity,
                                               std::wstring_view channel);
[[nodiscard]] bool pathsReferToSameFile(std::wstring_view left,
                                       std::wstring_view right) noexcept;

} // namespace fcitx::windows::platform
