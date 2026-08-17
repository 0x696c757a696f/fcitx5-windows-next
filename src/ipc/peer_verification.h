#pragma once

#include "runtime_identity.h"

#include <Windows.h>

#include <string>
#include <utility>

namespace fcitx::windows::ipc {

enum class PeerVerificationMode {
    exactExecutable,
    developmentSameUserSession,
};

struct PeerPolicy {
    PeerVerificationMode mode{PeerVerificationMode::exactExecutable};
    std::wstring expectedExecutablePath;

    [[nodiscard]] static PeerPolicy exact(std::wstring path) {
        return {PeerVerificationMode::exactExecutable, std::move(path)};
    }
    [[nodiscard]] static PeerPolicy development() {
        return {PeerVerificationMode::developmentSameUserSession, {}};
    }
};

[[nodiscard]] bool verifyPipeServer(HANDLE pipe,
                                    const platform::RuntimeIdentity& clientIdentity,
                                    const PeerPolicy& policy) noexcept;
[[nodiscard]] bool verifyPipeClient(HANDLE pipe,
                                    const platform::RuntimeIdentity& serverIdentity,
                                    platform::ProcessIdentity* verifiedClient = nullptr) noexcept;

} // namespace fcitx::windows::ipc
