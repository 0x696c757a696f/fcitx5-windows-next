#pragma once

#include "guids.h"

#include <Windows.h>

#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace fcitx::windows::tsf {

struct RuntimeInputProfile {
    std::string id;
    std::string bcp47;
    LANGID language{};
    GUID guid{};
    std::wstring description;
    std::string engine;
    bool candidates{};
    bool dynamic{};
};

struct ProfileIdentityRecord {
    LANGID language{};
    GUID guid{};
};

[[nodiscard]] GUID deterministicProfileGuid(std::string_view profileId) noexcept;
[[nodiscard]] std::vector<RuntimeInputProfile> loadInputProfiles();
[[nodiscard]] std::optional<std::string> inputMethodForProfileGuid(REFGUID guid);
[[nodiscard]] std::vector<ProfileIdentityRecord> loadDynamicProfileLedger();
bool storeDynamicProfileLedger(const std::vector<RuntimeInputProfile>& profiles) noexcept;

} // namespace fcitx::windows::tsf
