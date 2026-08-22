#include <Windows.h>
#include <guiddef.h>
#include <msctf.h>
#include <objbase.h>

#include <iostream>
#include <string>

namespace {

constexpr GUID kTextServiceClsid{0x3a21b9e2,
                                 0x4f47,
                                 0x4c36,
                                 {0x8b, 0xfa, 0x91, 0xd7, 0xd3, 0xb3, 0xe9, 0x01}};
constexpr GUID kUnsupportedClsid{0xaaaaaaaa,
                                 0xbbbb,
                                 0xcccc,
                                 {0xdd, 0xdd, 0xee, 0xee, 0xee, 0xee, 0xee, 0xee}};

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) {
        std::cerr << "Rust TSF PoC DLL argument required\n";
        return 1;
    }
    HMODULE module = LoadLibraryExW(argv[1], nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR |
                                                          LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!module) {
        std::cerr << "LoadLibraryExW failed: " << GetLastError() << '\n';
        return 1;
    }
    using GetClassObject = HRESULT(__stdcall*)(REFCLSID, REFIID, void**);
    using CanUnloadNow = HRESULT(__stdcall*)();
    using BehaviorReport = const char*(__stdcall*)(size_t*);
    const auto getClassObject =
        reinterpret_cast<GetClassObject>(GetProcAddress(module, "DllGetClassObject"));
    const auto canUnloadNow =
        reinterpret_cast<CanUnloadNow>(GetProcAddress(module, "DllCanUnloadNow"));
    const auto behaviorReport = reinterpret_cast<BehaviorReport>(
        GetProcAddress(module, "Fcitx5TsfPocBehaviorReport"));
    if (!getClassObject || !canUnloadNow || !behaviorReport) {
        std::cerr << "Rust TSF PoC exports missing\n";
        FreeLibrary(module);
        return 1;
    }
    size_t reportLength = 0;
    const char* reportBytes = behaviorReport(&reportLength);
    const std::string report =
        reportBytes && reportLength > 0 ? std::string(reportBytes, reportLength) : std::string();
    if (report.find("\"corpus\":\"tsf_behavior_corpus.json\"") == std::string::npos ||
        report.find("\"case_count\":7") == std::string::npos ||
        report.find("\"rust_case_passes\":7") == std::string::npos ||
        report.find("\"cpp_baseline_ctest\":\"tsf-key-commit-e2e\"") == std::string::npos ||
        report.find("\"cpp_baseline_consumes_same_corpus\":true") == std::string::npos ||
        report.find("\"full_host_differential_pending\":true") == std::string::npos ||
        report.find("\"report_export\":\"panic_contained\"") == std::string::npos) {
        std::cerr << "Rust TSF PoC behavior report should expose same-corpus case results\n";
        FreeLibrary(module);
        return 1;
    }
    if (canUnloadNow() != S_OK) {
        std::cerr << "Rust TSF PoC should be unloadable before activation\n";
        FreeLibrary(module);
        return 1;
    }
    void* object = reinterpret_cast<void*>(0x1);
    HRESULT result = getClassObject(kUnsupportedClsid, IID_IUnknown, &object);
    if (result != CLASS_E_CLASSNOTAVAILABLE || object != nullptr) {
        std::cerr << "Unsupported class should fail closed without returning an object\n";
        FreeLibrary(module);
        return 1;
    }
    IClassFactory* factory = nullptr;
    result = getClassObject(kTextServiceClsid, IID_IClassFactory,
                            reinterpret_cast<void**>(&factory));
    if (FAILED(result) || !factory) {
        std::cerr << "Rust TSF PoC should expose an IClassFactory\n";
        FreeLibrary(module);
        return 1;
    }
    if (canUnloadNow() != S_FALSE) {
        std::cerr << "Rust TSF PoC should not unload while factory is alive\n";
        factory->Release();
        FreeLibrary(module);
        return 1;
    }
    result = getClassObject(kTextServiceClsid, IID_IUnknown, nullptr);
    if (result != E_POINTER) {
        std::cerr << "Null output pointer should return E_POINTER\n";
        factory->Release();
        FreeLibrary(module);
        return 1;
    }
    ITfTextInputProcessorEx* textService = nullptr;
    result = factory->CreateInstance(nullptr, IID_ITfTextInputProcessorEx,
                                     reinterpret_cast<void**>(&textService));
    if (FAILED(result) || !textService) {
        std::cerr << "Rust TSF PoC factory should create an empty ITfTextInputProcessorEx\n";
        factory->Release();
        FreeLibrary(module);
        return 1;
    }
    if (canUnloadNow() != S_FALSE) {
        std::cerr << "Rust TSF PoC should not unload while service is alive\n";
        textService->Release();
        factory->Release();
        FreeLibrary(module);
        return 1;
    }
    ITfKeyEventSink* keySink = nullptr;
    ITfThreadMgrEventSink* threadSink = nullptr;
    ITfThreadFocusSink* focusSink = nullptr;
    if (FAILED(textService->QueryInterface(IID_ITfKeyEventSink,
                                           reinterpret_cast<void**>(&keySink))) ||
        FAILED(textService->QueryInterface(IID_ITfThreadMgrEventSink,
                                           reinterpret_cast<void**>(&threadSink))) ||
        FAILED(textService->QueryInterface(IID_ITfThreadFocusSink,
                                           reinterpret_cast<void**>(&focusSink)))) {
        std::cerr << "Rust TSF PoC service should expose key/thread/focus sinks\n";
        if (keySink) keySink->Release();
        if (threadSink) threadSink->Release();
        if (focusSink) focusSink->Release();
        textService->Release();
        factory->Release();
        FreeLibrary(module);
        return 1;
    }
    if (keySink) keySink->Release();
    if (threadSink) threadSink->Release();
    if (focusSink) focusSink->Release();
    textService->Release();
    factory->Release();
    if (canUnloadNow() != S_OK) {
        std::cerr << "Rust TSF PoC should unload after all COM objects are released\n";
        FreeLibrary(module);
        return 1;
    }
    FreeLibrary(module);
    return 0;
}
