#pragma once

#include <Windows.h>

#include <filesystem>
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
// Current runtime generation used to isolate IPC/object names during
// deployment-level side-by-side draining. Explicitly launched generation
// processes may set the validated FCITX5_RELEASE_GENERATION environment
// variable. Host-loaded modules and management tools otherwise discover the
// active generation from the installed current.json beside the runtime layout.
[[nodiscard]] std::wstring currentRuntimeGeneration();
[[nodiscard]] std::wstring currentRuntimeGenerationForModule(std::wstring_view modulePath);
[[nodiscard]] std::wstring currentRuntimeGenerationFromInstallRoot(std::wstring_view installRoot);
[[nodiscard]] std::filesystem::path installationRootForModule(std::wstring_view modulePath);
[[nodiscard]] std::filesystem::path portableDataRootForModule(std::wstring_view modulePath);
// Returns an empty string in production. Integration tests may set the validated
// FCITX5_TEST_NAMESPACE environment variable to prevent named-pipe/object collisions
// with a desktop instance that is already running for the same user and session.
[[nodiscard]] std::wstring localTestNamespace();
[[nodiscard]] std::wstring makeLocalEndpointName(const RuntimeIdentity& identity,
                                                 std::wstring_view channel);
[[nodiscard]] std::wstring makeLocalEndpointName(const RuntimeIdentity& identity,
                                                 std::wstring_view generation,
                                                 std::wstring_view channel);
[[nodiscard]] std::wstring makeLocalObjectName(const RuntimeIdentity& identity,
                                               std::wstring_view channel);
[[nodiscard]] std::wstring makeLocalObjectName(const RuntimeIdentity& identity,
                                               std::wstring_view generation,
                                               std::wstring_view channel);
[[nodiscard]] bool pathsReferToSameFile(std::wstring_view left,
                                       std::wstring_view right) noexcept;

} // namespace fcitx::windows::platform
