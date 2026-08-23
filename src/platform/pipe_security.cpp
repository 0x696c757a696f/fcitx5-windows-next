#include "pipe_security.h"

#include <cstdint>

namespace fcitx::windows::platform {
namespace {

extern "C" void* fcitx5_windows_common_pipe_security_descriptor_utf16(
    std::uint8_t service_account,
    std::uint32_t session_id,
    const std::uint16_t* user_sid,
    std::size_t user_sid_len);

const std::uint16_t* wideData(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return reinterpret_cast<const std::uint16_t*>(value.data());
}

PSECURITY_DESCRIPTOR pipeSecurityDescriptor(const RuntimeIdentity& identity) noexcept {
    return reinterpret_cast<PSECURITY_DESCRIPTOR>(
        fcitx5_windows_common_pipe_security_descriptor_utf16(
            identity.serviceAccount ? 1 : 0, identity.sessionId, wideData(identity.userSid),
            identity.userSid.size()));
}

} // namespace

PipeSecurity::~PipeSecurity() { reset(); }

PipeSecurity::PipeSecurity(PipeSecurity&& other) noexcept
    : descriptor_(other.descriptor_), attributes_(other.attributes_) {
    other.descriptor_ = nullptr;
    other.attributes_ = {};
    attributes_.lpSecurityDescriptor = descriptor_;
}

PipeSecurity& PipeSecurity::operator=(PipeSecurity&& other) noexcept {
    if (this != &other) {
        reset();
        descriptor_ = other.descriptor_;
        attributes_ = other.attributes_;
        other.descriptor_ = nullptr;
        other.attributes_ = {};
        attributes_.lpSecurityDescriptor = descriptor_;
    }
    return *this;
}

void PipeSecurity::reset() noexcept {
    if (descriptor_) LocalFree(descriptor_);
    descriptor_ = nullptr;
    attributes_ = {};
}

bool PipeSecurity::create(const RuntimeIdentity& identity, PipeSecurity& output) noexcept {
    output.reset();
    try {
        PSECURITY_DESCRIPTOR descriptor = pipeSecurityDescriptor(identity);
        if (!descriptor) {
            return false;
        }
        output.descriptor_ = descriptor;
        output.attributes_.nLength = sizeof(SECURITY_ATTRIBUTES);
        output.attributes_.lpSecurityDescriptor = descriptor;
        output.attributes_.bInheritHandle = FALSE;
        return true;
    } catch (...) {
        output.reset();
        return false;
    }
}

} // namespace fcitx::windows::platform
