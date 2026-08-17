#include <fcitx5_windows/version.h>

#include <cstdint>

namespace fcitx::windows {

std::string_view version() noexcept { return kVersion; }

Architecture architecture() noexcept {
    static_assert(sizeof(void *) == 4 || sizeof(void *) == 8);
    if constexpr (sizeof(void *) == 8) {
        return Architecture::x64;
    } else {
        return Architecture::x86;
    }
}

} // namespace fcitx::windows
