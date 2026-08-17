#pragma once

#include <inputscope.h>

namespace fcitx::windows::tsf {

[[nodiscard]] constexpr bool isSensitiveInputScope(InputScope scope) noexcept {
    switch (scope) {
    case IS_PASSWORD:
    case IS_PRIVATE:
    case IS_NUMERIC_PASSWORD:
    case IS_NUMERIC_PIN:
    case IS_ALPHANUMERIC_PIN:
    case IS_ALPHANUMERIC_PIN_SET:
        return true;
    default:
        return false;
    }
}

} // namespace fcitx::windows::tsf
