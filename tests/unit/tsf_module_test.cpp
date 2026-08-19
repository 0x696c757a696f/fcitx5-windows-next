#include "guids.h"
#include "input_profiles.h"

#include <Windows.h>
#include <msctf.h>
#include <objbase.h>

#include <algorithm>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <optional>
#include <string>
#include <string_view>

static_assert(fcitx::windows::tsf::kInputProfiles.size() == 1);
static_assert(fcitx::windows::tsf::kInputProfiles[0].language ==
              MAKELANGID(LANG_CHINESE, SUBLANG_CHINESE_SIMPLIFIED));
static_assert(fcitx::windows::tsf::profileForGuid(
                  fcitx::windows::tsf::kInputProfiles[0].guid)
                  ->id == std::string_view("zh-cn-fcitx5"));
static_assert(std::wstring_view(fcitx::windows::tsf::kInputProfiles[0].description) ==
              L"Fcitx5 for Windows Next");
static_assert(fcitx::windows::tsf::profileForGuid(GUID{}) == nullptr);
static_assert(fcitx::windows::tsf::kObsoleteInputProfiles.size() == 3);
static_assert(fcitx::windows::tsf::kObsoleteInputProfiles[0].language ==
              MAKELANGID(LANG_ENGLISH, SUBLANG_ENGLISH_US));

int wmain(int argc, wchar_t** argv) {
    const GUID dynamicRime = fcitx::windows::tsf::deterministicProfileGuid("zh-cn-rime-addon");
    const GUID dynamicRimeAgain =
        fcitx::windows::tsf::deterministicProfileGuid("zh-cn-rime-addon");
    if (!fcitx::windows::tsf::equalGuid(dynamicRime, dynamicRimeAgain) ||
        fcitx::windows::tsf::equalGuid(dynamicRime,
                                       fcitx::windows::tsf::kInputProfiles[0].guid)) {
        std::cerr << "dynamic TSF profile GUID contract failed\n";
        return 1;
    }
    std::wstring previousLocalAppData(32768, L'\0');
    const DWORD previousLocalAppDataLength =
        GetEnvironmentVariableW(L"LOCALAPPDATA", previousLocalAppData.data(),
                                static_cast<DWORD>(previousLocalAppData.size()));
    if (previousLocalAppDataLength >= previousLocalAppData.size()) {
        std::cerr << "LOCALAPPDATA was too long for the profile surface test\n";
        return 1;
    }
    previousLocalAppData.resize(previousLocalAppDataLength);
    std::wstring temporaryPath(32768, L'\0');
    const DWORD temporaryPathLength =
        GetTempPathW(static_cast<DWORD>(temporaryPath.size()), temporaryPath.data());
    if (temporaryPathLength == 0 || temporaryPathLength >= temporaryPath.size()) {
        std::cerr << "could not resolve temp path for profile surface test\n";
        return 1;
    }
    temporaryPath.resize(temporaryPathLength);
    const auto profileRoot =
        std::filesystem::path(temporaryPath) /
        (L"fcitx5-tsf-profile-surface-" + std::to_wstring(GetCurrentProcessId()));
    std::filesystem::remove_all(profileRoot);
    const auto dataDirectory = profileRoot / L"Fcitx5";
    std::filesystem::create_directories(dataDirectory);
    {
        std::ofstream file(dataDirectory / L"tsf-profiles.tsv");
        file << "zh-cn-custom\tzh-CN\tcustom\tFcitx5 Custom\n";
    }
    if (!SetEnvironmentVariableW(L"LOCALAPPDATA", profileRoot.c_str())) {
        std::cerr << "could not redirect LOCALAPPDATA for profile surface test\n";
        return 1;
    }
    const auto runtimeProfiles = fcitx::windows::tsf::loadInputProfiles();
    const auto customProfile = std::find_if(
        runtimeProfiles.begin(), runtimeProfiles.end(), [](const auto& profile) {
            return profile.id == "zh-cn-custom" && profile.engine == "custom" &&
                   profile.dynamic;
        });
    if (customProfile == runtimeProfiles.end() ||
        fcitx::windows::tsf::inputMethodForProfileGuid(customProfile->guid) !=
            std::optional<std::string>("custom") ||
        !fcitx::windows::tsf::storeDynamicProfileLedger(runtimeProfiles) ||
        fcitx::windows::tsf::loadDynamicProfileLedger().empty()) {
        std::cerr << "dynamic TSF profile surface contract failed\n";
        return 1;
    }
    const auto defaultRegistrable = fcitx::windows::tsf::loadRegistrableInputProfiles();
    if (defaultRegistrable.size() != 1U || defaultRegistrable[0].id != "zh-cn-fcitx5" ||
        defaultRegistrable[0].engine != "" ||
        defaultRegistrable[0].description != L"Fcitx5 for Windows Next") {
        std::cerr << "TSF registration should expose one product profile\n";
        return 1;
    }
    {
        std::ofstream file(dataDirectory / L"config.toml");
        file << "format_version = 1\n[input_methods]\n"
                "enabled = [\"pinyin\", \"custom\"]\n"
                "default = \"pinyin\"\n";
    }
    const auto customRegistrable = fcitx::windows::tsf::loadRegistrableInputProfiles();
    if (customRegistrable.size() != 1U || customRegistrable[0].id != "zh-cn-fcitx5") {
        std::cerr << "enabled input methods should not create extra TSF profiles\n";
        return 1;
    }
    {
        std::ofstream file(dataDirectory / L"config.toml", std::ios::trunc);
        file << "format_version = 1\n[input_methods]\n"
                "enabled = [\"rime\"]\n"
                "default = \"rime\"\n";
    }
    const auto rimeOnlyRegistrable = fcitx::windows::tsf::loadRegistrableInputProfiles();
    if (rimeOnlyRegistrable.size() != 1U || rimeOnlyRegistrable[0].id != "zh-cn-fcitx5") {
        std::cerr << "Rime should be selected inside Fcitx, not as a separate TSF profile\n";
        return 1;
    }
    if (previousLocalAppDataLength == 0) {
        SetEnvironmentVariableW(L"LOCALAPPDATA", nullptr);
    } else {
        SetEnvironmentVariableW(L"LOCALAPPDATA", previousLocalAppData.c_str());
    }
    std::filesystem::remove_all(profileRoot);
    if (argc != 2) {
        std::cerr << "TSF DLL argument required\n";
        return 1;
    }
    HMODULE module = LoadLibraryExW(argv[1], nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR |
                                                          LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!module) {
        std::cerr << "LoadLibraryExW failed: " << GetLastError() << '\n';
        return 1;
    }
    using GetClassObject = HRESULT(__stdcall*)(REFCLSID, REFIID, void**);
    const auto getClassObject =
        reinterpret_cast<GetClassObject>(GetProcAddress(module, "DllGetClassObject"));
    if (!getClassObject) {
        std::cerr << "DllGetClassObject export missing\n";
        FreeLibrary(module);
        return 1;
    }
    IClassFactory* factory = nullptr;
    HRESULT result = getClassObject(fcitx::windows::tsf::kTextServiceClsid, IID_IClassFactory,
                                    reinterpret_cast<void**>(&factory));
    ITfTextInputProcessorEx* service = nullptr;
    ITfThreadMgrEventSink* threadManagerEventSink = nullptr;
    ITfThreadFocusSink* threadFocusSink = nullptr;
    if (SUCCEEDED(result)) {
        result = factory->CreateInstance(nullptr, IID_ITfTextInputProcessorEx,
                                         reinterpret_cast<void**>(&service));
    }
    if (SUCCEEDED(result)) {
        result = service->QueryInterface(IID_ITfThreadMgrEventSink,
                                         reinterpret_cast<void**>(&threadManagerEventSink));
    }
    if (SUCCEEDED(result)) {
        result = service->QueryInterface(IID_ITfThreadFocusSink,
                                         reinterpret_cast<void**>(&threadFocusSink));
    }
    if (threadFocusSink) {
        threadFocusSink->Release();
    }
    if (threadManagerEventSink) {
        threadManagerEventSink->Release();
    }
    if (service) {
        service->Release();
    }
    if (factory) {
        factory->Release();
    }
    FreeLibrary(module);
    if (FAILED(result)) {
        std::cerr << "TSF COM activation failed: " << std::hex << result << '\n';
        return 1;
    }
    return 0;
}
