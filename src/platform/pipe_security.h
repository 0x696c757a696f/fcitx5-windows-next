#pragma once

#include "runtime_identity.h"

#include <Windows.h>

namespace fcitx::windows::platform {

class PipeSecurity final {
public:
    PipeSecurity() noexcept = default;
    ~PipeSecurity();
    PipeSecurity(const PipeSecurity&) = delete;
    PipeSecurity& operator=(const PipeSecurity&) = delete;
    PipeSecurity(PipeSecurity&& other) noexcept;
    PipeSecurity& operator=(PipeSecurity&& other) noexcept;

    [[nodiscard]] static bool create(const RuntimeIdentity& identity,
                                     PipeSecurity& output) noexcept;
    [[nodiscard]] SECURITY_ATTRIBUTES* attributes() noexcept { return &attributes_; }
    [[nodiscard]] bool valid() const noexcept { return descriptor_ != nullptr; }

private:
    void reset() noexcept;

    PSECURITY_DESCRIPTOR descriptor_{};
    SECURITY_ATTRIBUTES attributes_{};
};

} // namespace fcitx::windows::platform
