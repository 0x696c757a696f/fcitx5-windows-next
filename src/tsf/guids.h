#pragma once

#include <Windows.h>
#include <guiddef.h>

#include <fcitx5_windows/release_identity.h>

#include <array>

namespace fcitx::windows::tsf {

// {3A21B9E2-4F47-4C36-8BFA-91D7D3B3E901}
inline constexpr GUID kTextServiceClsid = kReleaseIdentity.text_service_clsid;

// {6C2AC726-7703-4B65-89AF-A77E9E0DA102}
inline constexpr GUID kLanguageProfileGuid = kReleaseIdentity.language_profile_guid;

inline constexpr const wchar_t* kServiceDescription = kReleaseIdentity.service_description;

struct InputProfile {
    const char* id;
    const char* bcp47;
    LANGID language;
    GUID guid;
    const wchar_t* description;
    const char* engine;
    bool candidates;
};

struct ProfileIdentity {
    LANGID language;
    GUID guid;
};

inline constexpr std::array<InputProfile, 1> kInputProfiles{{
    {"zh-cn-pinyin", "zh-CN", MAKELANGID(LANG_CHINESE, SUBLANG_CHINESE_SIMPLIFIED),
     kLanguageProfileGuid, L"Fcitx5 Pinyin", "pinyin", true},
}};

// Migration-only identities which were registered by development builds but are
// no longer product input profiles. Keep this list so repair and uninstall can
// remove the stale entry without touching unrelated keyboards or the current
// Simplified Chinese profile.
inline constexpr std::array<ProfileIdentity, 1> kObsoleteInputProfiles{{
    {MAKELANGID(LANG_ENGLISH, SUBLANG_ENGLISH_US), kLanguageProfileGuid},
}};

} // namespace fcitx::windows::tsf
