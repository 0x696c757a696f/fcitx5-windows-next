#pragma once

#include <Windows.h>

namespace fcitx::windows::tsf {

void moduleAddRef() noexcept;
void moduleRelease() noexcept;
[[nodiscard]] long moduleReferenceCount() noexcept;
[[nodiscard]] HMODULE moduleHandle() noexcept;

} // namespace fcitx::windows::tsf
