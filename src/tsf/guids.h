#pragma once

#include <Windows.h>
#include <guiddef.h>

#include <fcitx5_windows/release_identity.h>

#include <array>
#include <string_view>

namespace fcitx::windows::tsf {

// {3A21B9E2-4F47-4C36-8BFA-91D7D3B3E901}
inline constexpr GUID kTextServiceClsid = kReleaseIdentity.text_service_clsid;

// {6C2AC726-7703-4B65-89AF-A77E9E0DA102}
inline constexpr GUID kLanguageProfileGuid = kReleaseIdentity.language_profile_guid;

inline constexpr const wchar_t* kServiceDescription = kReleaseIdentity.service_description;

constexpr bool equalGuid(REFGUID left, REFGUID right) noexcept {
    return left.Data1 == right.Data1 && left.Data2 == right.Data2 &&
           left.Data3 == right.Data3 &&
           left.Data4[0] == right.Data4[0] && left.Data4[1] == right.Data4[1] &&
           left.Data4[2] == right.Data4[2] && left.Data4[3] == right.Data4[3] &&
           left.Data4[4] == right.Data4[4] && left.Data4[5] == right.Data4[5] &&
           left.Data4[6] == right.Data4[6] && left.Data4[7] == right.Data4[7];
}

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

constexpr GUID japaneseProfileGuid() noexcept {
    if constexpr (kReleaseIdentity.channel == ReleaseChannel::stable) {
        return {0x90672aa7, 0xdb8c, 0x45f9,
                {0x8e, 0x97, 0x27, 0x86, 0x65, 0x70, 0xa8, 0xfa}};
    } else if constexpr (kReleaseIdentity.channel == ReleaseChannel::beta) {
        return {0x3874209c, 0xc633, 0x42cb,
                {0xb7, 0x46, 0x14, 0xb7, 0xf3, 0xc4, 0xf2, 0x73}};
    } else {
        return {0x1796964c, 0x4b36, 0x499d,
                {0xac, 0x6c, 0xbe, 0xcc, 0x28, 0x30, 0x3a, 0xad}};
    }
}

inline constexpr GUID kJapaneseProfileGuid = japaneseProfileGuid();

constexpr GUID rimeProfileGuid() noexcept {
    if constexpr (kReleaseIdentity.channel == ReleaseChannel::stable) {
        return {0xa79f94c2, 0xbd7e, 0x4498,
                {0x8f, 0xe5, 0x65, 0x22, 0xf8, 0x3c, 0xd4, 0xd0}};
    } else if constexpr (kReleaseIdentity.channel == ReleaseChannel::beta) {
        return {0xa2327e53, 0xb15c, 0x4560,
                {0x99, 0x7a, 0x7c, 0x4f, 0x56, 0x42, 0xbf, 0x75}};
    } else {
        return {0xd37249ca, 0xa365, 0x4aa2,
                {0xb8, 0xb1, 0xbe, 0x98, 0x4b, 0x6a, 0xf9, 0x5e}};
    }
}

inline constexpr GUID kRimeProfileGuid = rimeProfileGuid();

inline constexpr std::array<InputProfile, 1> kInputProfiles{{
    {"zh-cn-fcitx5", "zh-CN", MAKELANGID(LANG_CHINESE, SUBLANG_CHINESE_SIMPLIFIED),
     kLanguageProfileGuid, L"Fcitx5 for Windows Next", "", true},
}};

constexpr const InputProfile* profileForGuid(REFGUID guid) noexcept {
    for (const auto& profile : kInputProfiles) {
        if (equalGuid(profile.guid, guid)) return &profile;
    }
    return nullptr;
}

// Migration-only identities which were registered by development builds but are
// no longer product input profiles. Keep this list so repair and uninstall can
// remove the stale entry without touching unrelated keyboards or the current
// Simplified Chinese profile.
inline constexpr std::array<ProfileIdentity, 3> kObsoleteInputProfiles{{
    {MAKELANGID(LANG_ENGLISH, SUBLANG_ENGLISH_US), kLanguageProfileGuid},
    {MAKELANGID(LANG_CHINESE, SUBLANG_CHINESE_SIMPLIFIED), kRimeProfileGuid},
    {MAKELANGID(LANG_JAPANESE, SUBLANG_DEFAULT), kJapaneseProfileGuid},
}};

} // namespace fcitx::windows::tsf
