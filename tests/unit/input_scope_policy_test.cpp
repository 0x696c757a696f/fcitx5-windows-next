#include "input_scope_policy.h"

#include <array>
#include <iostream>

int main() {
    using fcitx::windows::tsf::isSensitiveInputScope;
    constexpr std::array sensitive{
        IS_PASSWORD,         IS_PRIVATE,          IS_NUMERIC_PASSWORD,
        IS_NUMERIC_PIN,      IS_ALPHANUMERIC_PIN, IS_ALPHANUMERIC_PIN_SET,
    };
    for (const auto scope : sensitive) {
        if (!isSensitiveInputScope(scope)) {
            std::cerr << "sensitive input scope was not blocked\n";
            return 1;
        }
    }
    constexpr std::array ordinary{IS_DEFAULT, IS_URL, IS_EMAIL_SMTPEMAILADDRESS,
                                  IS_CHAT, IS_NUMBER, IS_SEARCH};
    for (const auto scope : ordinary) {
        if (isSensitiveInputScope(scope)) {
            std::cerr << "ordinary input scope was blocked\n";
            return 1;
        }
    }
    return 0;
}
