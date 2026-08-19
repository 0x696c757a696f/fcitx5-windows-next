#include "guids.h"

#include <Windows.h>
#include <msctf.h>
#include <objbase.h>

#include <iostream>
#include <string_view>

static_assert(fcitx::windows::tsf::kInputProfiles.size() >= 2);
static_assert(fcitx::windows::tsf::kInputProfiles[0].language ==
              MAKELANGID(LANG_CHINESE, SUBLANG_CHINESE_SIMPLIFIED));
static_assert(!fcitx::windows::tsf::equalGuid(
    fcitx::windows::tsf::kInputProfiles[0].guid,
    fcitx::windows::tsf::kInputProfiles[1].guid));
static_assert(fcitx::windows::tsf::kInputProfiles[1].language ==
              MAKELANGID(LANG_JAPANESE, SUBLANG_DEFAULT));
static_assert(fcitx::windows::tsf::kInputProfiles[1].engine ==
              std::string_view("mozc"));
static_assert(fcitx::windows::tsf::profileForGuid(
                  fcitx::windows::tsf::kInputProfiles[1].guid)
                  ->bcp47 == std::string_view("ja-JP"));
static_assert(fcitx::windows::tsf::profileForGuid(GUID{}) == nullptr);
static_assert(fcitx::windows::tsf::kObsoleteInputProfiles.size() == 1);
static_assert(fcitx::windows::tsf::kObsoleteInputProfiles[0].language ==
              MAKELANGID(LANG_ENGLISH, SUBLANG_ENGLISH_US));

int wmain(int argc, wchar_t** argv) {
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
