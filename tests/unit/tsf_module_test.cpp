#include "tsf_test_identity.h"

#include <Windows.h>
#include <msctf.h>
#include <objbase.h>

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
              L"Fcitx5");
static_assert(fcitx::windows::tsf::profileForGuid(GUID{}) == nullptr);
static_assert(fcitx::windows::tsf::kObsoleteInputProfiles.size() == 3);
static_assert(fcitx::windows::tsf::kObsoleteInputProfiles[0].language ==
              MAKELANGID(LANG_ENGLISH, SUBLANG_ENGLISH_US));

namespace {

template <typename Function>
Function resolveProcAddress(HMODULE module, const char* name) noexcept {
#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wcast-function-type-mismatch"
#endif
    const auto function = reinterpret_cast<Function>(GetProcAddress(module, name));
#if defined(__clang__)
#pragma clang diagnostic pop
#endif
    return function;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    const GUID retiredRime = fcitx::windows::tsf::deterministicProfileGuid("zh-cn-rime-addon");
    const GUID retiredRimeAgain =
        fcitx::windows::tsf::deterministicProfileGuid("zh-cn-rime-addon");
    if (!fcitx::windows::tsf::equalGuid(retiredRime, retiredRimeAgain) ||
        fcitx::windows::tsf::equalGuid(retiredRime,
                                       fcitx::windows::tsf::kInputProfiles[0].guid)) {
        std::cerr << "retired TSF profile GUID contract failed\n";
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
        file << "zh-cn-custom\tzh-CN\tcustom\tFcitx5 Custom\n"
                "ja-jp-mozc\tja-JP\tmozc\tFcitx5 Mozc\n";
    }
    if (!SetEnvironmentVariableW(L"LOCALAPPDATA", profileRoot.c_str())) {
        std::cerr << "could not redirect LOCALAPPDATA for profile surface test\n";
        return 1;
    }
    const auto runtimeProfiles = fcitx::windows::tsf::loadInputProfiles();
    if (runtimeProfiles.size() != 1U || runtimeProfiles[0].id != "zh-cn-fcitx5" ||
        runtimeProfiles[0].description != L"Fcitx5" || runtimeProfiles[0].dynamic ||
        fcitx::windows::tsf::inputMethodForProfileGuid(retiredRime) != std::nullopt) {
        std::cerr << "retired dynamic TSF profile surface should not create profiles\n";
        return 1;
    }
    const auto singleProfileEngine =
        fcitx::windows::tsf::inputMethodForProfileGuid(runtimeProfiles[0].guid);
    if (!singleProfileEngine || !singleProfileEngine->empty()) {
        std::cerr << "single Windows profile should not select an internal engine\n";
        return 1;
    }
    {
        std::ofstream file(dataDirectory / L"tsf-profile-ledger.tsv");
        file << "# language\tprofile-guid\tprofile-id\tengine\n"
             << static_cast<unsigned>(MAKELANGID(LANG_JAPANESE, SUBLANG_DEFAULT))
             << "\t{90672AA7-DB8C-45F9-8E97-27866570A8FA}\tja-jp-mozc\tmozc\n";
    }
    if (fcitx::windows::tsf::loadDynamicProfileLedger().empty() ||
        !fcitx::windows::tsf::storeDynamicProfileLedger(runtimeProfiles) ||
        !fcitx::windows::tsf::loadDynamicProfileLedger().empty()) {
        std::cerr << "retired dynamic TSF profile ledger cleanup contract failed\n";
        return 1;
    }
    const auto defaultRegistrable = fcitx::windows::tsf::loadRegistrableInputProfiles();
    if (defaultRegistrable.size() != 1U || defaultRegistrable[0].id != "zh-cn-fcitx5" ||
        defaultRegistrable[0].engine != "" ||
        defaultRegistrable[0].description != L"Fcitx5") {
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
    if (customRegistrable.size() != 1U || customRegistrable[0].id != "zh-cn-fcitx5" ||
        customRegistrable[0].description != L"Fcitx5") {
        std::cerr
            << "enabled Chinese and custom input methods should not create extra TSF profiles\n";
        return 1;
    }
    {
        std::ofstream file(dataDirectory / L"config.toml", std::ios::trunc);
        file << "format_version = 1\n[input_methods]\n"
                "enabled = [\"rime\"]\n"
                "default = \"rime\"\n";
    }
    const auto rimeOnlyRegistrable = fcitx::windows::tsf::loadRegistrableInputProfiles();
    if (rimeOnlyRegistrable.size() != 1U || rimeOnlyRegistrable[0].id != "zh-cn-fcitx5" ||
        rimeOnlyRegistrable[0].description != L"Fcitx5") {
        std::cerr << "Rime should be selected inside Fcitx, not as a separate TSF profile\n";
        return 1;
    }
    {
        std::ofstream file(dataDirectory / L"config.toml", std::ios::trunc);
        file << "format_version = 1\n[input_methods]\n"
                "enabled = [\"pinyin\", \"mozc\"]\n"
                "default = \"mozc\"\n";
    }
    const auto nonChineseRegistrable = fcitx::windows::tsf::loadRegistrableInputProfiles();
    if (nonChineseRegistrable.size() != 1U ||
        nonChineseRegistrable[0].id != "zh-cn-fcitx5" ||
        nonChineseRegistrable[0].language !=
            MAKELANGID(LANG_CHINESE, SUBLANG_CHINESE_SIMPLIFIED) ||
        nonChineseRegistrable[0].description != L"Fcitx5") {
        std::cerr << "non-Chinese engines should remain internal metadata under one TSF profile\n";
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
    const auto getClassObject = resolveProcAddress<GetClassObject>(module, "DllGetClassObject");
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
