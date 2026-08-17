#include "guids.h"

#include <Windows.h>
#include <msctf.h>
#include <objbase.h>
#include <wrl/client.h>

#include <atomic>
#include <cstdint>
#include <iostream>
#include <string>
#include <vector>

namespace {

using Microsoft::WRL::ComPtr;

class TestRange final : public ITfRange {
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** object) noexcept override {
        if (!object) return E_POINTER;
        *object = nullptr;
        if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITfRange)) {
            *object = static_cast<ITfRange*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() noexcept override {
        return references_.fetch_add(1, std::memory_order_relaxed) + 1;
    }
    ULONG STDMETHODCALLTYPE Release() noexcept override {
        return references_.fetch_sub(1, std::memory_order_acq_rel) - 1;
    }
    HRESULT STDMETHODCALLTYPE GetText(TfEditCookie, DWORD, WCHAR*, ULONG,
                                      ULONG*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE SetText(TfEditCookie, DWORD, const WCHAR* text,
                                      LONG length) noexcept override {
        if (length < 0 || (!text && length != 0)) return E_INVALIDARG;
        try {
            text_.assign(text ? text : L"", static_cast<std::size_t>(length));
            return S_OK;
        } catch (...) {
            return E_OUTOFMEMORY;
        }
    }
    HRESULT STDMETHODCALLTYPE GetFormattedText(TfEditCookie, IDataObject**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetEmbedded(TfEditCookie, REFGUID, REFIID,
                                          IUnknown**) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE InsertEmbedded(TfEditCookie, DWORD,
                                             IDataObject*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE ShiftStart(TfEditCookie, LONG, LONG*,
                                         const TF_HALTCOND*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE ShiftEnd(TfEditCookie, LONG, LONG*,
                                       const TF_HALTCOND*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE ShiftStartToRange(TfEditCookie, ITfRange*,
                                                TfAnchor) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE ShiftEndToRange(TfEditCookie, ITfRange*,
                                              TfAnchor) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE ShiftStartRegion(TfEditCookie, TfShiftDir,
                                               BOOL*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE ShiftEndRegion(TfEditCookie, TfShiftDir,
                                             BOOL*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE IsEmpty(TfEditCookie, BOOL*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE Collapse(TfEditCookie, TfAnchor) noexcept override { return S_OK; }
    HRESULT STDMETHODCALLTYPE IsEqualStart(TfEditCookie, ITfRange*, TfAnchor,
                                           BOOL*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE IsEqualEnd(TfEditCookie, ITfRange*, TfAnchor,
                                         BOOL*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE CompareStart(TfEditCookie, ITfRange*, TfAnchor,
                                           LONG*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE CompareEnd(TfEditCookie, ITfRange*, TfAnchor,
                                         LONG*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE AdjustForInsert(TfEditCookie, ULONG,
                                              BOOL*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetGravity(TfGravity*, TfGravity*) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE SetGravity(TfEditCookie, TfGravity,
                                         TfGravity) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE Clone(ITfRange**) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetContext(ITfContext**) noexcept override { return E_NOTIMPL; }

    [[nodiscard]] const std::wstring& text() const noexcept { return text_; }

private:
    std::atomic<ULONG> references_{1};
    std::wstring text_;
};

class TestContext final : public ITfContext {
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** object) noexcept override {
        if (!object) return E_POINTER;
        *object = nullptr;
        if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITfContext)) {
            *object = static_cast<ITfContext*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() noexcept override {
        return references_.fetch_add(1, std::memory_order_relaxed) + 1;
    }
    ULONG STDMETHODCALLTYPE Release() noexcept override {
        return references_.fetch_sub(1, std::memory_order_acq_rel) - 1;
    }
    HRESULT STDMETHODCALLTYPE RequestEditSession(TfClientId, ITfEditSession* session,
                                                 DWORD flags,
                                                 HRESULT* sessionResult) noexcept override {
        if (!session || !sessionResult) return E_POINTER;
        if ((flags & (TF_ES_SYNC | TF_ES_READWRITE)) != (TF_ES_SYNC | TF_ES_READWRITE)) {
            *sessionResult = E_INVALIDARG;
            return E_INVALIDARG;
        }
        *sessionResult = session->DoEditSession(1);
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE InWriteSession(TfClientId, BOOL*) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetSelection(TfEditCookie, ULONG, ULONG count,
                                           TF_SELECTION* selection,
                                           ULONG* fetched) noexcept override {
        if (!selection || !fetched) return E_POINTER;
        if (count != 1) return E_INVALIDARG;
        range_.AddRef();
        selection->range = &range_;
        selection->style.ase = TF_AE_END;
        selection->style.fInterimChar = FALSE;
        *fetched = 1;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE SetSelection(TfEditCookie, ULONG count,
                                           const TF_SELECTION* selection) noexcept override {
        return count == 1 && selection && selection->range == &range_ ? S_OK : E_INVALIDARG;
    }
    HRESULT STDMETHODCALLTYPE GetStart(TfEditCookie, ITfRange**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetEnd(TfEditCookie, ITfRange**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetActiveView(ITfContextView**) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE EnumViews(IEnumTfContextViews**) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetStatus(TF_STATUS*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetProperty(REFGUID, ITfProperty**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetAppProperty(REFGUID, ITfReadOnlyProperty**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE TrackProperties(const GUID**, ULONG, const GUID**, ULONG,
                                              ITfReadOnlyProperty**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE EnumProperties(IEnumTfProperties**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetDocumentMgr(ITfDocumentMgr**) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE CreateRangeBackup(TfEditCookie, ITfRange*,
                                                ITfRangeBackup**) noexcept override {
        return E_NOTIMPL;
    }

    [[nodiscard]] const std::wstring& text() const noexcept { return range_.text(); }

private:
    std::atomic<ULONG> references_{1};
    TestRange range_;
};

class TestThreadManager final : public ITfThreadMgr, public ITfKeystrokeMgr {
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** object) noexcept override {
        if (!object) return E_POINTER;
        *object = nullptr;
        if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITfThreadMgr)) {
            *object = static_cast<ITfThreadMgr*>(this);
        } else if (IsEqualIID(iid, IID_ITfKeystrokeMgr)) {
            *object = static_cast<ITfKeystrokeMgr*>(this);
        }
        if (!*object) return E_NOINTERFACE;
        AddRef();
        return S_OK;
    }
    ULONG STDMETHODCALLTYPE AddRef() noexcept override {
        return references_.fetch_add(1, std::memory_order_relaxed) + 1;
    }
    ULONG STDMETHODCALLTYPE Release() noexcept override {
        return references_.fetch_sub(1, std::memory_order_acq_rel) - 1;
    }
    HRESULT STDMETHODCALLTYPE Activate(TfClientId* clientId) noexcept override {
        if (!clientId) return E_POINTER;
        *clientId = 1;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE Deactivate() noexcept override { return S_OK; }
    HRESULT STDMETHODCALLTYPE CreateDocumentMgr(ITfDocumentMgr**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE EnumDocumentMgrs(IEnumTfDocumentMgrs**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetFocus(ITfDocumentMgr**) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE SetFocus(ITfDocumentMgr*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE AssociateFocus(HWND, ITfDocumentMgr*,
                                             ITfDocumentMgr**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE IsThreadFocus(BOOL*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetFunctionProvider(REFCLSID,
                                                  ITfFunctionProvider**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE EnumFunctionProviders(
        IEnumTfFunctionProviders**) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetGlobalCompartment(ITfCompartmentMgr**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE AdviseKeyEventSink(TfClientId clientId, ITfKeyEventSink* sink,
                                                BOOL foreground) noexcept override {
        if (clientId == TF_CLIENTID_NULL || !sink || !foreground || advised_) return E_INVALIDARG;
        advised_ = true;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE UnadviseKeyEventSink(TfClientId clientId) noexcept override {
        if (clientId == TF_CLIENTID_NULL || !advised_) return E_INVALIDARG;
        advised_ = false;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetForeground(CLSID*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE TestKeyDown(WPARAM, LPARAM, BOOL*) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE TestKeyUp(WPARAM, LPARAM, BOOL*) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE KeyDown(WPARAM, LPARAM, BOOL*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE KeyUp(WPARAM, LPARAM, BOOL*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetPreservedKey(ITfContext*, const TF_PRESERVEDKEY*,
                                              GUID*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE IsPreservedKey(REFGUID, const TF_PRESERVEDKEY*,
                                             BOOL*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE PreserveKey(TfClientId, REFGUID, const TF_PRESERVEDKEY*,
                                          const WCHAR*, ULONG) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE UnpreserveKey(REFGUID,
                                            const TF_PRESERVEDKEY*) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE SetPreservedKeyDescription(REFGUID, const WCHAR*,
                                                         ULONG) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE GetPreservedKeyDescription(REFGUID, BSTR*) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE SimulatePreservedKey(ITfContext*, REFGUID,
                                                   BOOL*) noexcept override { return E_NOTIMPL; }

private:
    std::atomic<ULONG> references_{1};
    bool advised_{};
};

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

struct EngineProcess {
    HANDLE process{};
    bool ready{};
};

EngineProcess startEngine(const wchar_t* executable) {
    const std::wstring eventName =
        L"Local\\Fcitx5WindowsNext.TsfE2E.Ready." + std::to_wstring(GetCurrentProcessId());
    HANDLE readyEvent = CreateEventW(nullptr, TRUE, FALSE, eventName.c_str());
    if (!readyEvent) {
        std::cerr << "readiness event creation failed: " << GetLastError() << '\n';
        return {};
    }
    std::wstring command = quote(executable) + L" --test-once --ready-event " + quote(eventName);
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(executable, mutableCommand.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, nullptr, &startup, &process)) {
        std::cerr << "mock engine creation failed: " << GetLastError() << '\n';
        CloseHandle(readyEvent);
        return {};
    }
    CloseHandle(process.hThread);
    const bool ready = WaitForSingleObject(readyEvent, 2000) == WAIT_OBJECT_0;
    if (!ready) std::cerr << "mock engine readiness timed out\n";
    CloseHandle(readyEvent);
    return {process.hProcess, ready};
}

int exercise(const wchar_t* dllPath) {
    HMODULE module = LoadLibraryExW(dllPath, nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR |
                                                        LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!module) {
        std::cerr << "TSF DLL load failed: " << GetLastError() << '\n';
        return 1;
    }
    using GetClassObject = HRESULT(STDAPICALLTYPE*)(REFCLSID, REFIID, void**);
    const auto getClassObject =
        reinterpret_cast<GetClassObject>(GetProcAddress(module, "DllGetClassObject"));
    ComPtr<IClassFactory> factory;
    ComPtr<ITfTextInputProcessorEx> service;
    ComPtr<ITfKeyEventSink> keySink;
    TestThreadManager threadManager;
    constexpr TfClientId clientId = 1;
    bool activated = false;
    int result = 1;
    HRESULT stepResult = getClassObject
                             ? getClassObject(fcitx::windows::tsf::kTextServiceClsid,
                                              IID_PPV_ARGS(&factory))
                             : HRESULT_FROM_WIN32(GetLastError());
    if (FAILED(stepResult)) {
        std::cerr << "DllGetClassObject failed: 0x" << std::hex << stepResult << '\n';
    } else if (FAILED(stepResult = factory->CreateInstance(nullptr, IID_PPV_ARGS(&service)))) {
        std::cerr << "CreateInstance failed: 0x" << std::hex << stepResult << '\n';
    } else if (FAILED(stepResult = service.As(&keySink))) {
        std::cerr << "ITfKeyEventSink query failed: 0x" << std::hex << stepResult << '\n';
    } else if (FAILED(stepResult = service->ActivateEx(&threadManager, clientId, 0))) {
        std::cerr << "TextService activation failed: 0x" << std::hex << stepResult
                  << ", clientId=0x" << clientId << '\n';
    } else {
        activated = true;
        TestContext context;
        BOOL testEaten = FALSE;
        BOOL eaten = FALSE;
        const HRESULT testResult = keySink->OnTestKeyDown(&context, 'A', 0, &testEaten);
        const HRESULT keyResult = keySink->OnKeyDown(&context, 'A', 0, &eaten);
        if (SUCCEEDED(testResult) && SUCCEEDED(keyResult) && testEaten && eaten &&
            context.text() == L"a") {
            result = 0;
        } else {
            std::cerr << "key path failed: test=0x" << std::hex << testResult << ", key=0x"
                      << keyResult << ", testEaten=" << std::dec << testEaten
                      << ", eaten=" << eaten << ", textLength=" << context.text().size()
                      << '\n';
        }
    }
    if (activated) service->Deactivate();
    keySink.Reset();
    service.Reset();
    factory.Reset();
    FreeLibrary(module);
    return result;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 3) {
        std::cerr << "TSF DLL and mock engine arguments required\n";
        return 1;
    }
    const HRESULT initialized = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    if (FAILED(initialized)) return 1;
    EngineProcess engine = startEngine(argv[2]);
    if (!engine.process) std::cerr << "mock engine process unavailable\n";
    int result = engine.process && engine.ready ? exercise(argv[1]) : 1;
    if (engine.process) {
        if (WaitForSingleObject(engine.process, 2000) != WAIT_OBJECT_0) {
            TerminateProcess(engine.process, 2);
            WaitForSingleObject(engine.process, 1000);
            result = 1;
        }
        DWORD exitCode = 1;
        GetExitCodeProcess(engine.process, &exitCode);
        CloseHandle(engine.process);
        if (exitCode != 0) result = 1;
    }
    CoUninitialize();
    if (result != 0) std::cerr << "TSF key-to-edit-session commit E2E failed\n";
    return result;
}
