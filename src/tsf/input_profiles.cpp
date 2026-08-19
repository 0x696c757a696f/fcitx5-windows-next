#include "input_profiles.h"

#include <fcitx5_windows/release_identity.h>

#include <objbase.h>

#include <algorithm>
#include <array>
#include <charconv>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <sstream>

namespace fcitx::windows::tsf {
namespace {

constexpr std::string_view kDynamicProfileFile = "tsf-profiles.tsv";
constexpr std::string_view kDynamicProfileLedger = "tsf-profile-ledger.tsv";
constexpr std::size_t kMaxProfileIdentifierUtf8 = 64;

std::wstring wideFromUtf8(std::string_view input) {
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

std::string utf8FromWide(std::wstring_view input) {
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

std::filesystem::path profileDataDirectory() {
    std::wstring localAppData(32768, L'\0');
    const DWORD length = GetEnvironmentVariableW(
        L"LOCALAPPDATA", localAppData.data(), static_cast<DWORD>(localAppData.size()));
    if (length == 0 || length >= localAppData.size()) return {};
    localAppData.resize(length);
    return std::filesystem::path(localAppData) / kReleaseIdentity.data_directory;
}

std::filesystem::path profileSurfacePath() {
    const auto directory = profileDataDirectory();
    return directory.empty() ? std::filesystem::path{} : directory / kDynamicProfileFile;
}

std::filesystem::path profileLedgerPath() {
    const auto directory = profileDataDirectory();
    return directory.empty() ? std::filesystem::path{} : directory / kDynamicProfileLedger;
}

bool validIdentifier(std::string_view value) noexcept {
    return !value.empty() && value.size() <= kMaxProfileIdentifierUtf8 &&
           std::all_of(value.begin(), value.end(), [](unsigned char ch) {
               return (ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') ||
                      (ch >= '0' && ch <= '9') || ch == '_' || ch == '-' ||
                      ch == '.' || ch == '+';
           });
}

std::vector<std::string> splitTsvLine(const std::string& line) {
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

std::wstring guidString(REFGUID guid) {
    std::array<wchar_t, 40> buffer{};
    if (StringFromGUID2(guid, buffer.data(), static_cast<int>(buffer.size())) == 0)
        return {};
    return buffer.data();
}

std::string guidUtf8(REFGUID guid) {
    return utf8FromWide(guidString(guid));
}

bool equalRuntimeGuid(REFGUID left, REFGUID right) noexcept {
    return equalGuid(left, right);
}

bool parseGuid(std::string_view text, GUID& guid) noexcept {
    const auto wide = wideFromUtf8(text);
    if (wide.empty()) return false;
    return SUCCEEDED(CLSIDFromString(wide.c_str(), &guid));
}

std::uint64_t fnv1a(std::uint64_t seed, const void* data, std::size_t size) noexcept {
    constexpr std::uint64_t kPrime = 1099511628211ULL;
    const auto* bytes = static_cast<const std::uint8_t*>(data);
    std::uint64_t hash = seed;
    for (std::size_t index = 0; index < size; ++index) {
        hash ^= bytes[index];
        hash *= kPrime;
    }
    return hash;
}

std::uint64_t hashProfile(std::string_view id, std::uint64_t salt) noexcept {
    constexpr std::uint64_t kOffset = 14695981039346656037ULL;
    std::uint64_t hash = fnv1a(kOffset ^ salt, &kTextServiceClsid.Data1,
                               sizeof(kTextServiceClsid.Data1));
    hash = fnv1a(hash, &kTextServiceClsid.Data2, sizeof(kTextServiceClsid.Data2));
    hash = fnv1a(hash, &kTextServiceClsid.Data3, sizeof(kTextServiceClsid.Data3));
    hash = fnv1a(hash, kTextServiceClsid.Data4, sizeof(kTextServiceClsid.Data4));
    hash = fnv1a(hash, id.data(), id.size());
    return hash;
}

LANGID languageFromBcp47(std::string_view bcp47) {
    const auto wide = wideFromUtf8(bcp47);
    if (wide.empty()) return 0;
    const LCID locale = LocaleNameToLCID(wide.c_str(), 0);
    return locale == 0 ? 0 : LANGIDFROMLCID(locale);
}

RuntimeInputProfile runtimeProfileFromBuiltin(const InputProfile& profile) {
    return RuntimeInputProfile{profile.id,
                               profile.bcp47,
                               profile.language,
                               profile.guid,
                               profile.description,
                               profile.engine,
                               profile.candidates,
                               false};
}

bool appendDynamicProfile(std::vector<RuntimeInputProfile>& profiles,
                          std::vector<RuntimeInputProfile>& dynamicProfiles,
                          const std::vector<std::string>& fields) {
    if (fields.size() < 4) return false;
    const std::string& id = fields[0];
    const std::string& bcp47 = fields[1];
    const std::string& engine = fields[2];
    if (!validIdentifier(id) || !validIdentifier(engine) || bcp47.empty() ||
        bcp47.size() > 35) {
        return false;
    }
    const LANGID language = languageFromBcp47(bcp47);
    if (language == 0) return false;
    std::wstring description = wideFromUtf8(fields[3]);
    if (description.empty()) {
        description = L"Fcitx5 " + wideFromUtf8(engine);
    }
    if (description.empty()) return false;
    RuntimeInputProfile profile{id, bcp47, language, deterministicProfileGuid(id),
                                description, engine, true, true};
    const auto duplicate = [&](const RuntimeInputProfile& existing) {
        return existing.id == profile.id || equalRuntimeGuid(existing.guid, profile.guid);
    };
    if (std::any_of(profiles.begin(), profiles.end(), duplicate) ||
        std::any_of(dynamicProfiles.begin(), dynamicProfiles.end(), duplicate)) {
        return false;
    }
    dynamicProfiles.push_back(std::move(profile));
    return true;
}

} // namespace

GUID deterministicProfileGuid(std::string_view profileId) noexcept {
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

std::vector<RuntimeInputProfile> loadInputProfiles() {
    std::vector<RuntimeInputProfile> profiles;
    profiles.reserve(kInputProfiles.size());
    for (const auto& profile : kInputProfiles) {
        profiles.push_back(runtimeProfileFromBuiltin(profile));
    }

    const auto path = profileSurfacePath();
    if (path.empty()) return profiles;
    std::ifstream file(path);
    if (!file) return profiles;

    std::vector<RuntimeInputProfile> dynamicProfiles;
    std::string line;
    while (std::getline(file, line)) {
        if (line.empty() || line[0] == '#') continue;
        (void)appendDynamicProfile(profiles, dynamicProfiles, splitTsvLine(line));
    }
    profiles.insert(profiles.end(), std::make_move_iterator(dynamicProfiles.begin()),
                    std::make_move_iterator(dynamicProfiles.end()));
    return profiles;
}

std::vector<RuntimeInputProfile> loadRegistrableInputProfiles() {
    std::vector<RuntimeInputProfile> registrable;
    registrable.reserve(kInputProfiles.size());
    for (const auto& profile : kInputProfiles) {
        registrable.push_back(runtimeProfileFromBuiltin(profile));
    }
    return registrable;
}

std::optional<std::string> inputMethodForProfileGuid(REFGUID guid) {
    if (const InputProfile* profile = profileForGuid(guid)) {
        return profile->engine;
    }
    for (const auto& profile : loadInputProfiles()) {
        if (profile.dynamic && equalRuntimeGuid(profile.guid, guid)) {
            return profile.engine;
        }
    }
    return std::nullopt;
}

std::vector<ProfileIdentityRecord> loadDynamicProfileLedger() {
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

bool storeDynamicProfileLedger(const std::vector<RuntimeInputProfile>& profiles) noexcept {
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
                 << guidUtf8(profile.guid) << '\t' << profile.id << '\t'
                 << profile.engine << '\n';
        }
        return static_cast<bool>(file);
    } catch (...) {
        return false;
    }
}

} // namespace fcitx::windows::tsf
