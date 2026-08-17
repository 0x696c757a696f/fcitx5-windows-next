#pragma once

#include <guiddef.h>
#include <Windows.h>

#include <array>

namespace fcitx::windows::tsf {

// {3A21B9E2-4F47-4C36-8BFA-91D7D3B3E901}
inline constexpr GUID kTextServiceClsid = {0x3a21b9e2,
                                          0x4f47,
                                          0x4c36,
                                          {0x8b, 0xfa, 0x91, 0xd7, 0xd3, 0xb3, 0xe9, 0x01}};

// {6C2AC726-7703-4B65-89AF-A77E9E0DA102}
inline constexpr GUID kLanguageProfileGuid = {
    0x6c2ac726,
    0x7703,
    0x4b65,
    {0x89, 0xaf, 0xa7, 0x7e, 0x9e, 0x0d, 0xa1, 0x02}};

inline constexpr wchar_t kServiceDescription[] = L"Fcitx5 for Windows Next (Development)";

struct InputProfile {
    const char* id;
    const char* bcp47;
    LANGID language;
    GUID guid;
    const wchar_t* description;
    const char* engine;
    bool candidates;
};

inline constexpr std::array<InputProfile, 1> kInputProfiles{{
    {"zh-cn-pinyin", "zh-CN", MAKELANGID(LANG_CHINESE, SUBLANG_CHINESE_SIMPLIFIED),
     kLanguageProfileGuid, L"Fcitx5 Pinyin", "pinyin", true},
}};

} // namespace fcitx::windows::tsf
