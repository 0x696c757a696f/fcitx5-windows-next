#include "pipe_security.h"

#include <sddl.h>

#include <string>

namespace fcitx::windows::platform {

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
    if (identity.userSid.empty() || identity.serviceAccount || identity.sessionId == 0) return false;
    try {
        const std::wstring sddl = L"D:P(A;;GA;;;SY)(A;;GA;;;" + identity.userSid +
                                  L")S:(ML;;NW;;;ME)";
        PSECURITY_DESCRIPTOR descriptor = nullptr;
        if (!ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.c_str(), SDDL_REVISION_1, &descriptor, nullptr)) {
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
