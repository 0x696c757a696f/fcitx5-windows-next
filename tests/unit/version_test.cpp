#include <fcitx5_windows/version.h>

#include <iostream>

int main() {
    using fcitx::windows::Architecture;
    using fcitx::windows::architecture;

    if (fcitx::windows::version() != fcitx::windows::kVersion ||
        fcitx::windows::version().empty()) {
        std::cerr << "version contract failed\n";
        return 1;
    }

    const auto expected =
        sizeof(void *) == 8 ? Architecture::x64 : Architecture::x86;
    if (architecture() != expected) {
        std::cerr << "architecture contract failed\n";
        return 1;
    }

    return 0;
}
