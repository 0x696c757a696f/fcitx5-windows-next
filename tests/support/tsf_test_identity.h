#pragma once

#include <fcitx5_windows/release_identity.h>

#include <Windows.h>
#include <objbase.h>

#include <array>
#include <charconv>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace fcitx::windows::tsf {

inline constexpr GUID kTextServiceClsid = kReleaseIdentity.text_service_clsid;
inline constexpr GUID kLanguageProfileGuid = kReleaseIdentity.language_profile_guid;

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

inline constexpr GUID kJapaneseProfileGuid = japaneseProfileGuid();
inline constexpr GUID kRimeProfileGuid = rimeProfileGuid();

inline constexpr std::array<InputProfile, 1> kInputProfiles{{
    {"zh-cn-fcitx5", "zh-CN", MAKELANGID(LANG_CHINESE, SUBLANG_CHINESE_SIMPLIFIED),
     kLanguageProfileGuid, L"Fcitx5", "", true},
}};

inline constexpr std::array<ProfileIdentity, 3> kObsoleteInputProfiles{{
    {MAKELANGID(LANG_ENGLISH, SUBLANG_ENGLISH_US), kLanguageProfileGuid},
    {MAKELANGID(LANG_CHINESE, SUBLANG_CHINESE_SIMPLIFIED), kRimeProfileGuid},
    {MAKELANGID(LANG_JAPANESE, SUBLANG_DEFAULT), kJapaneseProfileGuid},
}};

constexpr const InputProfile* profileForGuid(REFGUID guid) noexcept {
    for (const auto& profile : kInputProfiles) {
        if (equalGuid(profile.guid, guid)) return &profile;
    }
    return nullptr;
}

inline std::wstring wideFromUtf8(std::string_view input) {
    if (input.empty()) return {};
    const int required = MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                                            static_cast<int>(input.size()), nullptr, 0);
    if (required <= 0) return {};
    std::wstring output(static_cast<std::size_t>(required), L'\0');
    if (MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, input.data(),
                            static_cast<int>(input.size()), output.data(), required) !=
        required) {
        return {};
    }
    return output;
}

inline std::string utf8FromWide(std::wstring_view input) {
    if (input.empty()) return {};
    const int required = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, input.data(),
                                            static_cast<int>(input.size()), nullptr, 0,
                                            nullptr, nullptr);
    if (required <= 0) return {};
    std::string output(static_cast<std::size_t>(required), '\0');
    if (WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, input.data(),
                            static_cast<int>(input.size()), output.data(), required,
                            nullptr, nullptr) != required) {
        return {};
    }
    return output;
}

inline std::uint64_t fnv1a(std::uint64_t seed, const void* data,
                           std::size_t size) noexcept {
    constexpr std::uint64_t kPrime = 1099511628211ULL;
    const auto* bytes = static_cast<const std::uint8_t*>(data);
    std::uint64_t hash = seed;
    for (std::size_t index = 0; index < size; ++index) {
        hash ^= bytes[index];
        hash *= kPrime;
    }
    return hash;
}

inline std::uint64_t hashProfile(std::string_view id, std::uint64_t salt) noexcept {
    constexpr std::uint64_t kOffset = 14695981039346656037ULL;
    std::uint64_t hash = fnv1a(kOffset ^ salt, &kTextServiceClsid.Data1,
                               sizeof(kTextServiceClsid.Data1));
    hash = fnv1a(hash, &kTextServiceClsid.Data2, sizeof(kTextServiceClsid.Data2));
    hash = fnv1a(hash, &kTextServiceClsid.Data3, sizeof(kTextServiceClsid.Data3));
    hash = fnv1a(hash, kTextServiceClsid.Data4, sizeof(kTextServiceClsid.Data4));
    hash = fnv1a(hash, id.data(), id.size());
    return hash;
}

inline GUID deterministicProfileGuid(std::string_view profileId) noexcept {
    const std::uint64_t low = hashProfile(profileId, 0x54534650524f4649ULL);
    const std::uint64_t high = hashProfile(profileId, 0x464349545835ULL);
    GUID guid{};
    guid.Data1 = static_cast<std::uint32_t>(low);
    guid.Data2 = static_cast<std::uint16_t>(low >> 32U);
    guid.Data3 = static_cast<std::uint16_t>((low >> 48U) & 0x0fffU);
    guid.Data3 |= 0x5000U;
    for (std::size_t index = 0; index < 8; ++index) {
        guid.Data4[index] = static_cast<std::uint8_t>((high >> (index * 8U)) & 0xffU);
    }
    guid.Data4[0] = static_cast<std::uint8_t>((guid.Data4[0] & 0x3fU) | 0x80U);
    return guid;
}

inline RuntimeInputProfile runtimeProfileFromBuiltin(const InputProfile& profile) {
    return RuntimeInputProfile{profile.id,
                               profile.bcp47,
                               profile.language,
                               profile.guid,
                               profile.description,
                               profile.engine,
                               profile.candidates,
                               false};
}

inline std::vector<RuntimeInputProfile> loadRegistrableInputProfiles() {
    std::vector<RuntimeInputProfile> registrable;
    registrable.reserve(kInputProfiles.size());
    for (const auto& profile : kInputProfiles) {
        registrable.push_back(runtimeProfileFromBuiltin(profile));
    }
    return registrable;
}

inline std::vector<RuntimeInputProfile> loadInputProfiles() {
    return loadRegistrableInputProfiles();
}

inline std::optional<std::string> inputMethodForProfileGuid(REFGUID guid) {
    if (const InputProfile* profile = profileForGuid(guid)) {
        return profile->engine;
    }
    return std::nullopt;
}

inline std::filesystem::path profileDataDirectory() {
    std::wstring localAppData(32768, L'\0');
    const DWORD length = GetEnvironmentVariableW(
        L"LOCALAPPDATA", localAppData.data(), static_cast<DWORD>(localAppData.size()));
    if (length == 0 || length >= localAppData.size()) return {};
    localAppData.resize(length);
    return std::filesystem::path(localAppData) / kReleaseIdentity.data_directory;
}

inline std::filesystem::path profileLedgerPath() {
    const auto directory = profileDataDirectory();
    return directory.empty() ? std::filesystem::path{} : directory / "tsf-profile-ledger.tsv";
}

inline std::vector<std::string> splitTsvLine(const std::string& line) {
    std::vector<std::string> fields;
    std::size_t start = 0;
    while (start <= line.size()) {
        const std::size_t tab = line.find('\t', start);
        fields.push_back(line.substr(start, tab == std::string::npos ? tab : tab - start));
        if (tab == std::string::npos) break;
        start = tab + 1;
    }
    return fields;
}

inline bool parseGuid(std::string_view text, GUID& guid) noexcept {
    const auto wide = wideFromUtf8(text);
    if (wide.empty()) return false;
    return SUCCEEDED(CLSIDFromString(wide.c_str(), &guid));
}

inline std::wstring guidString(REFGUID guid) {
    std::array<wchar_t, 40> buffer{};
    if (StringFromGUID2(guid, buffer.data(), static_cast<int>(buffer.size())) == 0)
        return {};
    return buffer.data();
}

inline std::vector<ProfileIdentityRecord> loadDynamicProfileLedger() {
    std::vector<ProfileIdentityRecord> records;
    const auto path = profileLedgerPath();
    if (path.empty()) return records;
    std::ifstream file(path);
    if (!file) return records;
    std::string line;
    while (std::getline(file, line)) {
        if (line.empty() || line[0] == '#') continue;
        const auto fields = splitTsvLine(line);
        if (fields.size() < 2) continue;
        unsigned language = 0;
        const auto* first = fields[0].data();
        const auto* last = fields[0].data() + fields[0].size();
        if (std::from_chars(first, last, language).ec != std::errc{} ||
            language > 0xffffU) {
            continue;
        }
        GUID guid{};
        if (!parseGuid(fields[1], guid)) continue;
        records.push_back(ProfileIdentityRecord{static_cast<LANGID>(language), guid});
    }
    return records;
}

inline bool storeDynamicProfileLedger(
    const std::vector<RuntimeInputProfile>& profiles) noexcept {
    try {
        const auto path = profileLedgerPath();
        if (path.empty()) return false;
        std::filesystem::create_directories(path.parent_path());
        std::ofstream file(path, std::ios::trunc);
        if (!file) return false;
        file << "# language\\tprofile-guid\\tprofile-id\\tengine\n";
        for (const auto& profile : profiles) {
            if (!profile.dynamic) continue;
            file << static_cast<unsigned>(profile.language) << '\t'
                 << utf8FromWide(guidString(profile.guid)) << '\t' << profile.id << '\t'
                 << profile.engine << '\n';
        }
        return static_cast<bool>(file);
    } catch (...) {
        return false;
    }
}

} // namespace fcitx::windows::tsf
