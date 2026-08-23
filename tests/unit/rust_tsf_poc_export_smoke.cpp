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
    using ProfileIdentityReport = const char*(__stdcall*)(size_t*);
    using IpcBoundaryReport = const char*(__stdcall*)(size_t*);
    using CompositionTranscriptReport = const char*(__stdcall*)(size_t*);
    using DifferentialSummaryReport = const char*(__stdcall*)(size_t*);
    using ForcedFailure = HRESULT(__stdcall*)();
    const auto getClassObject = resolveProcAddress<GetClassObject>(module, "DllGetClassObject");
    const auto canUnloadNow = resolveProcAddress<CanUnloadNow>(module, "DllCanUnloadNow");
    const auto behaviorReport =
        resolveProcAddress<BehaviorReport>(module, "Fcitx5TsfPocBehaviorReport");
    const auto profileIdentityReport = resolveProcAddress<ProfileIdentityReport>(
        module, "Fcitx5TsfPocProfileIdentityReport");
    const auto ipcBoundaryReport =
        resolveProcAddress<IpcBoundaryReport>(module, "Fcitx5TsfPocIpcBoundaryReport");
    const auto compositionTranscriptReport = resolveProcAddress<CompositionTranscriptReport>(
        module, "Fcitx5TsfPocCompositionTranscriptReport");
    const auto differentialSummaryReport = resolveProcAddress<DifferentialSummaryReport>(
        module, "Fcitx5TsfPocDifferentialSummaryReport");
    const auto forcedFailure =
        resolveProcAddress<ForcedFailure>(module, "Fcitx5TsfPocForcedFailureForTest");
    if (!getClassObject || !canUnloadNow || !behaviorReport || !profileIdentityReport ||
        !ipcBoundaryReport || !compositionTranscriptReport || !differentialSummaryReport ||
        !forcedFailure) {
        std::cerr << "Rust TSF PoC exports missing\n";
        FreeLibrary(module);
        return 1;
    }
    if (forcedFailure() != E_UNEXPECTED) {
        std::cerr << "Rust TSF PoC forced internal failure should convert panic to HRESULT\n";
        FreeLibrary(module);
        return 1;
    }
    size_t reportLength = 0;
    const char* reportBytes = behaviorReport(&reportLength);
    const std::string report =
        reportBytes && reportLength > 0 ? std::string(reportBytes, reportLength) : std::string();
    if (report.find("\"corpus\":\"tsf_behavior_corpus.json\"") == std::string::npos ||
        report.find("\"case_count\":10") == std::string::npos ||
        report.find("\"rust_case_passes\":10") == std::string::npos ||
        report.find("\"uiless_candidate_show_false_preserves_metadata\"") ==
            std::string::npos ||
        report.find("\"key_busy_focus_change_does_not_clear_composition\"") ==
            std::string::npos ||
        report.find("\"single_edit_session_commit_preedit_update\"") == std::string::npos ||
        report.find("\"cpp_baseline_ctest\":\"tsf-key-commit-e2e\"") == std::string::npos ||
        report.find("\"cpp_baseline_consumes_same_corpus\":true") == std::string::npos ||
        report.find("\"full_host_differential_pending\":true") == std::string::npos ||
        report.find("\"report_export\":\"panic_contained\"") == std::string::npos) {
        std::cerr << "Rust TSF PoC behavior report should expose same-corpus case results\n";
        FreeLibrary(module);
        return 1;
    }
    size_t profileReportLength = 0;
    const char* profileReportBytes = profileIdentityReport(&profileReportLength);
    const std::string profileReport = profileReportBytes && profileReportLength > 0
                                          ? std::string(profileReportBytes, profileReportLength)
                                          : std::string();
    if (profileReport.find("\"product_display_name\":\"Fcitx5 for Windows Next\"") ==
            std::string::npos ||
        profileReport.find("\"profile_display_name\":\"Fcitx5\"") == std::string::npos ||
        profileReport.find(
            "\"text_service_clsid\":\"3a21b9e2-4f47-4c36-8bfa-91d7d3b3e901\"") ==
            std::string::npos ||
        profileReport.find(
            "\"language_profile_guid\":\"6c2ac726-7703-4b65-89af-a77e9e0da102\"") ==
            std::string::npos ||
        profileReport.find("\"windows_profile_count\":1") == std::string::npos ||
        profileReport.find("\"dynamic_profile_registration\":false") == std::string::npos ||
        profileReport.find("\"rust_poc_registers_profile\":false") == std::string::npos ||
        profileReport.find("\"shipping_cxx_authoritative\":true") == std::string::npos ||
        profileReport.find("\"release_identity_source\":\"cmake/release_identity.h.in\"") ==
            std::string::npos) {
        std::cerr << "Rust TSF PoC profile identity report should match stable release identity\n";
        FreeLibrary(module);
        return 1;
    }
    size_t ipcReportLength = 0;
    const char* ipcReportBytes = ipcBoundaryReport(&ipcReportLength);
    const std::string ipcReport =
        ipcReportBytes && ipcReportLength > 0 ? std::string(ipcReportBytes, ipcReportLength)
                                              : std::string();
    if (ipcReport.find("\"bounded_ipc_client_model\":true") == std::string::npos ||
        ipcReport.find("\"timeout_fails_open\":true") == std::string::npos ||
        ipcReport.find("\"malformed_fails_open\":true") == std::string::npos ||
        ipcReport.find("\"generation_mismatch_fails_open\":true") == std::string::npos ||
        ipcReport.find("\"network_imports\":false") == std::string::npos ||
        ipcReport.find("\"external_engine_link\":false") == std::string::npos ||
        ipcReport.find("\"host_blocking_call\":false") == std::string::npos ||
        ipcReport.find("\"shipping_cxx_authoritative\":true") == std::string::npos) {
        std::cerr << "Rust TSF PoC IPC boundary report should fail open for slow or invalid engine replies\n";
        FreeLibrary(module);
        return 1;
    }
    size_t compositionReportLength = 0;
    const char* compositionReportBytes = compositionTranscriptReport(&compositionReportLength);
    const std::string compositionReport =
        compositionReportBytes && compositionReportLength > 0
            ? std::string(compositionReportBytes, compositionReportLength)
            : std::string();
    if (compositionReport.find("\"single_edit_session\":true") == std::string::npos ||
        compositionReport.find(
            "\"operation_order\":[\"begin_edit_session\",\"commit_text\",\"update_preedit_start_composition\",\"end_edit_session\"]") ==
            std::string::npos ||
        compositionReport.find("\"commit_text\":\"你\"") == std::string::npos ||
        compositionReport.find("\"preedit_text\":\"hao\"") == std::string::npos ||
        compositionReport.find("\"composition_active_after\":true") == std::string::npos ||
        compositionReport.find("\"host_differential_pending\":true") == std::string::npos ||
        compositionReport.find("\"shipping_cxx_authoritative\":true") == std::string::npos) {
        std::cerr << "Rust TSF PoC composition transcript should preserve single edit-session operation order\n";
        FreeLibrary(module);
        return 1;
    }
    size_t summaryReportLength = 0;
    const char* summaryReportBytes = differentialSummaryReport(&summaryReportLength);
    const std::string summaryReport =
        summaryReportBytes && summaryReportLength > 0
            ? std::string(summaryReportBytes, summaryReportLength)
            : std::string();
    if (summaryReport.find("\"component\":\"fcitx5-tsf-poc\"") == std::string::npos ||
        summaryReport.find("\"same_corpus_case_count\":10") == std::string::npos ||
        summaryReport.find("\"same_corpus_rust_passes\":10") == std::string::npos ||
        summaryReport.find("\"profile_identity\":true") == std::string::npos ||
        summaryReport.find("\"ipc_boundary\":true") == std::string::npos ||
        summaryReport.find("\"composition_transcript\":true") == std::string::npos ||
        summaryReport.find("\"artifact_audit_ctest\":\"rust-tsf-poc-artifact-audit\"") ==
            std::string::npos ||
        summaryReport.find("\"arm64_artifact_pending\":true") == std::string::npos ||
        summaryReport.find("\"real_host_matrix_pending\":true") == std::string::npos ||
        summaryReport.find("\"product_decision\":\"pending\"") == std::string::npos) {
        std::cerr << "Rust TSF PoC differential summary should list green and pending evidence\n";
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
    if (FAILED(textService->ActivateEx(nullptr, 42, TF_TMF_UIELEMENTENABLEDONLY)) ||
        FAILED(textService->Deactivate())) {
        std::cerr << "Rust TSF PoC should fail open for minimal ActivateEx/Deactivate callbacks\n";
        keySink->Release();
        threadSink->Release();
        focusSink->Release();
        textService->Release();
        factory->Release();
        FreeLibrary(module);
        return 1;
    }
    BOOL testDownEaten = TRUE;
    BOOL keyDownEaten = TRUE;
    BOOL testUpEaten = TRUE;
    BOOL keyUpEaten = TRUE;
    if (FAILED(keySink->OnTestKeyDown(nullptr, 'A', 0, &testDownEaten)) ||
        FAILED(keySink->OnKeyDown(nullptr, 'A', 0, &keyDownEaten)) ||
        FAILED(keySink->OnTestKeyUp(nullptr, 'A', 0, &testUpEaten)) ||
        FAILED(keySink->OnKeyUp(nullptr, 'A', 0, &keyUpEaten)) || testDownEaten ||
        keyDownEaten || testUpEaten || keyUpEaten) {
        std::cerr << "Rust TSF PoC key callbacks should fail open on unexpected COM state\n";
        keySink->Release();
        threadSink->Release();
        focusSink->Release();
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
