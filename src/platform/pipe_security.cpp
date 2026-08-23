#include "pipe_security.h"

#include <cstdint>

namespace fcitx::windows::platform {
namespace {

extern "C" void* fcitx5_windows_common_pipe_security_create_utf16(
    std::uint8_t service_account,
    std::uint32_t session_id,
    const std::uint16_t* user_sid,
    std::size_t user_sid_len);
extern "C" void* fcitx5_windows_common_pipe_security_attributes(void* state);
extern "C" void fcitx5_windows_common_pipe_security_destroy(void* state);

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

void* pipeSecurityState(const RuntimeIdentity& identity) noexcept {
    return fcitx5_windows_common_pipe_security_create_utf16(
        identity.serviceAccount ? 1 : 0, identity.sessionId, wideData(identity.userSid),
        identity.userSid.size());
}

} // namespace

PipeSecurity::~PipeSecurity() { reset(); }

PipeSecurity::PipeSecurity(PipeSecurity&& other) noexcept
    : state_(other.state_) {
    other.state_ = nullptr;
}

PipeSecurity& PipeSecurity::operator=(PipeSecurity&& other) noexcept {
    if (this != &other) {
        reset();
        state_ = other.state_;
        other.state_ = nullptr;
    }
    return *this;
}

void PipeSecurity::reset() noexcept {
    if (state_) {
        fcitx5_windows_common_pipe_security_destroy(state_);
    }
    state_ = nullptr;
}

SECURITY_ATTRIBUTES* PipeSecurity::attributes() noexcept {
    return reinterpret_cast<SECURITY_ATTRIBUTES*>(
        fcitx5_windows_common_pipe_security_attributes(state_));
}

bool PipeSecurity::create(const RuntimeIdentity& identity, PipeSecurity& output) noexcept {
    output.reset();
    try {
        void* state = pipeSecurityState(identity);
        if (!state) {
            return false;
        }
        output.state_ = state;
        return true;
    } catch (...) {
        output.reset();
        return false;
    }
}

} // namespace fcitx::windows::platform
