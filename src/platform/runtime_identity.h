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
[[nodiscard]] std::wstring makeLocalEndpointName(const RuntimeIdentity& identity,
                                                 std::wstring_view channel);
[[nodiscard]] bool pathsReferToSameFile(std::wstring_view left,
                                       std::wstring_view right) noexcept;

} // namespace fcitx::windows::platform
