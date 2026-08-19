#include "text_service.h"

#include "input_scope_policy.h"
#include "module.h"
#include "pipe_security.h"

#include <fcitx5_windows/release_identity.h>

#include <OleAuto.h>
#include <cstdint>
#include <filesystem>
#include <iterator>
#include <memory>
#include <new>
#include <string>
#include <utility>

namespace fcitx::windows::tsf {
namespace {

constexpr wchar_t kCandidateDismissMessageName[] =
    L"Fcitx5WindowsNext.CandidateDismiss.v1";
constexpr UINT kCandidateStateAvailableMessage = WM_APP + 0x315;

std::wstring modulePath() {
    std::wstring path(32'768, L'\0');
    const DWORD size = GetModuleFileNameW(moduleHandle(), path.data(),
                                         static_cast<DWORD>(path.size()));
    if (size == 0 || size >= path.size()) return {};
    path.resize(size);
    return path;
}

std::wstring expectedEnginePath() {
#if defined(FCITX_DEVELOPMENT_PEER_EXCEPTION)
    if (!platform::localTestNamespace().empty()) {
        std::wstring testPath(32'768, L'\0');
        const DWORD size = GetEnvironmentVariableW(
            L"FCITX5_TEST_ENGINE_PATH", testPath.data(),
            static_cast<DWORD>(testPath.size()));
        if (size > 0 && size < testPath.size()) {
            testPath.resize(size);
            if (std::filesystem::path(testPath).is_absolute() &&
                std::filesystem::exists(testPath)) return testPath;
        }
    }
#endif
    const std::filesystem::path dll(modulePath());
    if (dll.empty()) return {};
    const auto sibling = dll.parent_path() / L"fcitx5-engine.exe";
    if (std::filesystem::exists(sibling)) return sibling.wstring();
    const auto packaged = dll.parent_path().parent_path().parent_path() / L"bin" /
                          L"fcitx5-engine.exe";
    return packaged.wstring();
}

std::wstring engineEndpoint() {
    platform::RuntimeIdentity identity;
    return platform::queryCurrentIdentity(identity)
               ? platform::makeLocalEndpointName(identity, L"engine")
               : std::wstring{};
}

void broadcastCandidateDismiss(std::uint64_t contextId) noexcept {
    const UINT message = RegisterWindowMessageW(kCandidateDismissMessageName);
    if (message != 0) {
        (void)PostMessageW(HWND_BROADCAST, message,
                           static_cast<WPARAM>(GetCurrentProcessId()),
                           static_cast<LPARAM>(contextId));
    }
}

class ScopedBusyFlag final {
public:
    explicit ScopedBusyFlag(bool& flag) noexcept : flag_(flag) { flag_ = true; }
    ~ScopedBusyFlag() { flag_ = false; }

    ScopedBusyFlag(const ScopedBusyFlag&) = delete;
    ScopedBusyFlag& operator=(const ScopedBusyFlag&) = delete;

private:
    bool& flag_;
};

Microsoft::WRL::ComPtr<ITfContext> topContext(ITfDocumentMgr* documentManager) noexcept {
    Microsoft::WRL::ComPtr<ITfContext> context;
    if (documentManager) (void)documentManager->GetTop(&context);
    return context;
}

UINT windowDpi(HWND window) noexcept {
    using GetDpiForWindowFunction = UINT(WINAPI*)(HWND);
    const HMODULE user32 = GetModuleHandleW(L"user32.dll");
    const auto getDpiForWindow = user32
                                     ? reinterpret_cast<GetDpiForWindowFunction>(
                                           GetProcAddress(user32, "GetDpiForWindow"))
                                     : nullptr;
    if (getDpiForWindow && window) {
        const UINT dpi = getDpiForWindow(window);
        if (dpi != 0) return dpi;
    }
    HDC device = GetDC(window);
    const int dpi = device ? GetDeviceCaps(device, LOGPIXELSX) : 96;
    if (device) ReleaseDC(window, device);
    return dpi > 0 ? static_cast<UINT>(dpi) : 96U;
}

bool readSensitiveInputScope(ITfContext* context, TfEditCookie editCookie,
                             ITfRange* range) noexcept {
    Microsoft::WRL::ComPtr<ITfReadOnlyProperty> property;
    if (FAILED(context->GetAppProperty(GUID_PROP_INPUTSCOPE, &property)) || !property) {
        return false;
    }
    VARIANT value;
    VariantInit(&value);
    const HRESULT valueResult = property->GetValue(editCookie, range, &value);
    if (FAILED(valueResult) || value.vt != VT_UNKNOWN || !value.punkVal) {
        VariantClear(&value);
        return false;
    }
    Microsoft::WRL::ComPtr<ITfInputScope> inputScope;
    const HRESULT queryResult = value.punkVal->QueryInterface(IID_PPV_ARGS(&inputScope));
    VariantClear(&value);
    if (FAILED(queryResult) || !inputScope) return false;
    InputScope* scopes = nullptr;
    UINT count = 0;
    if (FAILED(inputScope->GetInputScopes(&scopes, &count))) return false;
    if (count != 0 && !scopes) return false;
    bool sensitive = false;
    for (UINT index = 0; index < count; ++index) {
        sensitive = sensitive || isSensitiveInputScope(scopes[index]);
    }
    CoTaskMemFree(scopes);
    return sensitive;
}

class ContextReadSession final : public ITfEditSession {
public:
    ContextReadSession(ITfContext* context, protocol::CaretRect* caret, bool* sensitive)
        : context_(context), caret_(caret), sensitive_(sensitive) {}

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID interfaceId, void** object) noexcept override {
        if (!object) return E_POINTER;
        *object = nullptr;
        if (IsEqualIID(interfaceId, IID_IUnknown) ||
            IsEqualIID(interfaceId, IID_ITfEditSession)) {
            *object = static_cast<ITfEditSession*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    ULONG STDMETHODCALLTYPE AddRef() noexcept override {
        return referenceCount_.fetch_add(1, std::memory_order_relaxed) + 1;
    }

    ULONG STDMETHODCALLTYPE Release() noexcept override {
        const ULONG remaining = referenceCount_.fetch_sub(1, std::memory_order_acq_rel) - 1;
        if (remaining == 0) delete this;
        return remaining;
    }

    HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie editCookie) noexcept override {
        TF_SELECTION selection{};
        ULONG fetched = 0;
        HRESULT result = context_->GetSelection(editCookie, TF_DEFAULT_SELECTION, 1,
                                                &selection, &fetched);
        Microsoft::WRL::ComPtr<ITfRange> range;
        if (FAILED(result) || fetched != 1 || !selection.range) {
            return FAILED(result) ? result : E_FAIL;
        }
        range.Attach(selection.range);
        *sensitive_ = readSensitiveInputScope(context_.Get(), editCookie, range.Get());
        if (*sensitive_) return S_OK;
        Microsoft::WRL::ComPtr<ITfContextView> view;
        result = context_->GetActiveView(&view);
        if (FAILED(result) || !view) return FAILED(result) ? result : E_FAIL;
        RECT rect{};
        BOOL clipped = FALSE;
        result = view->GetTextExt(editCookie, range.Get(), &rect, &clipped);
        if (FAILED(result)) return result;
        if (rect.left == 0 && rect.top == 0 && rect.right == 0 && rect.bottom == 0) {
            return TS_E_NOLAYOUT;
        }
        HWND window = nullptr;
        (void)view->GetWnd(&window);
        *caret_ = protocol::CaretRect{true, rect.left, rect.top, rect.right,
                                      rect.bottom, windowDpi(window)};
        return S_OK;
    }

private:
    ~ContextReadSession() = default;
    std::atomic<ULONG> referenceCount_{1};
    Microsoft::WRL::ComPtr<ITfContext> context_;
    protocol::CaretRect* caret_{};
    bool* sensitive_{};
};

bool queryContext(ITfContext* context, TfClientId clientId, protocol::CaretRect* caret,
                  bool* sensitive) noexcept {
    auto* session = new (std::nothrow) ContextReadSession(context, caret, sensitive);
    if (!session) return false;
    HRESULT sessionResult = E_FAIL;
    const HRESULT requestResult = context->RequestEditSession(
        clientId, session, TF_ES_SYNC | TF_ES_READ, &sessionResult);
    session->Release();
    return SUCCEEDED(requestResult) && SUCCEEDED(sessionResult);
}

class CompositionEditSession final : public ITfEditSession {
public:
    CompositionEditSession(ITfContext* context, ITfCompositionSink* sink,
                           Microsoft::WRL::ComPtr<ITfComposition>* composition,
                           std::wstring commit, std::wstring preedit,
                           std::uint32_t caret)
        : context_(context), sink_(sink), composition_(composition),
          commit_(std::move(commit)), preedit_(std::move(preedit)), caret_(caret) {}

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID interfaceId, void** object) noexcept override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;
        if (IsEqualIID(interfaceId, IID_IUnknown) ||
            IsEqualIID(interfaceId, IID_ITfEditSession)) {
            *object = static_cast<ITfEditSession*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    ULONG STDMETHODCALLTYPE AddRef() noexcept override {
        return referenceCount_.fetch_add(1, std::memory_order_relaxed) + 1;
    }

    ULONG STDMETHODCALLTYPE Release() noexcept override {
        const ULONG remaining = referenceCount_.fetch_sub(1, std::memory_order_acq_rel) - 1;
        if (remaining == 0) {
            delete this;
        }
        return remaining;
    }

    HRESULT STDMETHODCALLTYPE DoEditSession(TfEditCookie editCookie) noexcept override {
        TF_SELECTION selection{};
        ULONG fetched = 0;
        HRESULT result = context_->GetSelection(editCookie, TF_DEFAULT_SELECTION, 1, &selection,
                                                &fetched);
        if (FAILED(result) || fetched != 1 || !selection.range) {
            return FAILED(result) ? result : E_FAIL;
        }
        Microsoft::WRL::ComPtr<ITfRange> selectionRange;
        selectionRange.Attach(selection.range);

        if (*composition_) {
            Microsoft::WRL::ComPtr<ITfRange> range;
            result = (*composition_)->GetRange(&range);
            if (FAILED(result)) return result;
            if (!commit_.empty()) {
                result = range->SetText(editCookie, 0, commit_.data(),
                                        static_cast<LONG>(commit_.size()));
                if (SUCCEEDED(result)) result = (*composition_)->EndComposition(editCookie);
                if (FAILED(result)) return result;
                composition_->Reset();
                selectionRange = std::move(range);
            } else if (preedit_.empty()) {
                result = range->SetText(editCookie, 0, nullptr, 0);
                if (SUCCEEDED(result)) result = (*composition_)->EndComposition(editCookie);
                if (FAILED(result)) return result;
                composition_->Reset();
                selectionRange = std::move(range);
            }
        } else if (!commit_.empty()) {
            result = selectionRange->SetText(editCookie, 0, commit_.data(),
                                             static_cast<LONG>(commit_.size()));
            if (FAILED(result)) return result;
        }

        if (!preedit_.empty()) {
            if (!*composition_) {
                Microsoft::WRL::ComPtr<ITfContextComposition> contextComposition;
                result = context_.As(&contextComposition);
                if (FAILED(result)) return result;
                Microsoft::WRL::ComPtr<ITfComposition> started;
                result = contextComposition->StartComposition(
                    editCookie, selectionRange.Get(), sink_.Get(), &started);
                if (FAILED(result) || !started) return FAILED(result) ? result : E_FAIL;
                composition_->Attach(started.Detach());
            }
            Microsoft::WRL::ComPtr<ITfRange> range;
            result = (*composition_)->GetRange(&range);
            if (FAILED(result)) return result;
            result = range->SetText(editCookie, 0, preedit_.data(),
                                    static_cast<LONG>(preedit_.size()));
            if (FAILED(result)) return result;
            result = range->Collapse(editCookie, TF_ANCHOR_START);
            if (FAILED(result)) return result;
            LONG shifted = 0;
            result = range->ShiftEnd(editCookie, static_cast<LONG>(caret_), &shifted, nullptr);
            if (FAILED(result) || shifted != static_cast<LONG>(caret_)) {
                return FAILED(result) ? result : E_FAIL;
            }
            result = range->Collapse(editCookie, TF_ANCHOR_END);
            if (FAILED(result)) return result;
            selectionRange = std::move(range);
        } else {
            result = selectionRange->Collapse(editCookie, TF_ANCHOR_END);
            if (FAILED(result)) return result;
        }

        if (selectionRange) {
            selection.range = selectionRange.Get();
            selection.style.ase = TF_AE_END;
            selection.style.fInterimChar = FALSE;
            result = context_->SetSelection(editCookie, 1, &selection);
        }
        applied_ = SUCCEEDED(result);
        return result;
    }

    [[nodiscard]] bool applied() const noexcept { return applied_; }

private:
    ~CompositionEditSession() = default;

    std::atomic<ULONG> referenceCount_{1};
    Microsoft::WRL::ComPtr<ITfContext> context_;
    Microsoft::WRL::ComPtr<ITfCompositionSink> sink_;
    Microsoft::WRL::ComPtr<ITfComposition>* composition_{};
    std::wstring commit_;
    std::wstring preedit_;
    std::uint32_t caret_{};
    bool applied_{};
};

bool applyTextUpdate(ITfContext* context, TfClientId clientId, ITfCompositionSink* sink,
                     Microsoft::WRL::ComPtr<ITfComposition>* composition,
                     std::wstring commit, std::wstring preedit,
                     std::uint32_t caret) noexcept {
    auto* session = new (std::nothrow) CompositionEditSession(
        context, sink, composition, std::move(commit), std::move(preedit), caret);
    if (!session)
        return false;
    HRESULT sessionResult = E_FAIL;
    const HRESULT requestResult = context->RequestEditSession(
        clientId, session, TF_ES_SYNC | TF_ES_READWRITE, &sessionResult);
    const bool applied = SUCCEEDED(requestResult) && SUCCEEDED(sessionResult) &&
                         session->applied();
    session->Release();
    return applied;
}

bool translatePrintableKey(WPARAM virtualKey, LPARAM keyData,
                           std::wstring& text) noexcept {
    text.clear();
    if (!((virtualKey >= 'A' && virtualKey <= 'Z') ||
          (virtualKey >= '0' && virtualKey <= '9'))) {
        return false;
    }
    BYTE keyboardState[256]{};
    if (!GetKeyboardState(keyboardState))
        return false;
    const HKL layout = GetKeyboardLayout(0);
    UINT scanCode = static_cast<UINT>((static_cast<std::uintptr_t>(keyData) >> 16U) & 0xffU);
    if (scanCode == 0)
        scanCode = MapVirtualKeyExW(static_cast<UINT>(virtualKey), MAPVK_VK_TO_VSC, layout);
    wchar_t buffer[8]{};
    const int count = ToUnicodeEx(static_cast<UINT>(virtualKey), scanCode, keyboardState,
                                  buffer, static_cast<int>(std::size(buffer)), 0, layout);
    if (count <= 0 || count > static_cast<int>(std::size(buffer)))
        return false;
    try {
        text.assign(buffer, static_cast<std::size_t>(count));
        return true;
    } catch (...) {
        text.clear();
        return false;
    }
}

} // namespace

TextService::TextService()
    : client_(engineEndpoint(), ipc::PeerPolicy::exact(expectedEnginePath())) {
    moduleAddRef();
}

TextService::~TextService() {
    Deactivate();
    moduleRelease();
}

HRESULT TextService::QueryInterface(REFIID interfaceId, void** object) noexcept {
    if (!object) {
        return E_POINTER;
    }
    *object = nullptr;
    if (IsEqualIID(interfaceId, IID_IUnknown) ||
        IsEqualIID(interfaceId, IID_ITfTextInputProcessor) ||
        IsEqualIID(interfaceId, IID_ITfTextInputProcessorEx)) {
        *object = static_cast<ITfTextInputProcessorEx*>(this);
    } else if (IsEqualIID(interfaceId, IID_ITfKeyEventSink)) {
        *object = static_cast<ITfKeyEventSink*>(this);
    } else if (IsEqualIID(interfaceId, IID_ITfCompositionSink)) {
        *object = static_cast<ITfCompositionSink*>(this);
    } else if (IsEqualIID(interfaceId, IID_ITfThreadMgrEventSink)) {
        *object = static_cast<ITfThreadMgrEventSink*>(this);
    } else if (IsEqualIID(interfaceId, IID_ITfThreadFocusSink)) {
        *object = static_cast<ITfThreadFocusSink*>(this);
    }
    if (!*object) {
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

ULONG TextService::AddRef() noexcept {
    return referenceCount_.fetch_add(1, std::memory_order_relaxed) + 1;
}

ULONG TextService::Release() noexcept {
    const ULONG remaining = referenceCount_.fetch_sub(1, std::memory_order_acq_rel) - 1;
    if (remaining == 0) {
        delete this;
    }
    return remaining;
}

LRESULT CALLBACK TextService::notificationWindowProcedure(
    HWND window, UINT message, WPARAM wparam, LPARAM lparam) noexcept {
    TextService* service = nullptr;
    if (message == WM_NCCREATE) {
        service = static_cast<TextService*>(
            reinterpret_cast<CREATESTRUCTW*>(lparam)->lpCreateParams);
        SetWindowLongPtrW(window, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(service));
    } else {
        service = reinterpret_cast<TextService*>(GetWindowLongPtrW(window, GWLP_USERDATA));
    }
    if (service && message == kCandidateStateAvailableMessage) {
        service->applyPendingCandidateState();
        return 0;
    }
    return DefWindowProcW(window, message, wparam, lparam);
}

VOID CALLBACK TextService::notificationWaitCallback(PVOID context,
                                                     BOOLEAN timedOut) noexcept {
    if (timedOut || !context) return;
    auto* service = static_cast<TextService*>(context);
    const HWND window = service->notificationWindow_;
    if (window) (void)PostMessageW(window, kCandidateStateAvailableMessage, 0, 0);
}

bool TextService::initializeCandidateNotification() noexcept {
    try {
        platform::RuntimeIdentity identity;
        platform::PipeSecurity security;
        if (!platform::queryCurrentIdentity(identity) ||
            !platform::PipeSecurity::create(identity, security)) return false;
        const std::wstring eventName = platform::makeLocalObjectName(
            identity, L"candidate-" + std::to_wstring(GetCurrentProcessId()));
        if (eventName.empty()) return false;
        notificationEvent_ =
            CreateEventW(security.attributes(), FALSE, FALSE, eventName.c_str());
        if (!notificationEvent_) return false;
        const std::wstring className =
            std::wstring(kReleaseIdentity.local_object_prefix) + L".TsfNotification";
        WNDCLASSW windowClass{};
        windowClass.hInstance = moduleHandle();
        windowClass.lpfnWndProc = notificationWindowProcedure;
        windowClass.lpszClassName = className.c_str();
        if (!RegisterClassW(&windowClass) && GetLastError() != ERROR_CLASS_ALREADY_EXISTS) {
            shutdownCandidateNotification();
            return false;
        }
        notificationWindow_ = CreateWindowExW(
            0, className.c_str(), L"", 0, 0, 0, 0, 0, HWND_MESSAGE, nullptr,
            moduleHandle(), this);
        if (!notificationWindow_ ||
            !RegisterWaitForSingleObject(&notificationWait_, notificationEvent_,
                                         notificationWaitCallback, this, INFINITE,
                                         WT_EXECUTEDEFAULT)) {
            shutdownCandidateNotification();
            return false;
        }
        return true;
    } catch (...) {
        shutdownCandidateNotification();
        return false;
    }
}

void TextService::shutdownCandidateNotification() noexcept {
    if (notificationWait_) {
        (void)UnregisterWaitEx(notificationWait_, INVALID_HANDLE_VALUE);
        notificationWait_ = nullptr;
    }
    if (notificationWindow_) {
        DestroyWindow(notificationWindow_);
        notificationWindow_ = nullptr;
    }
    if (notificationEvent_) {
        CloseHandle(notificationEvent_);
        notificationEvent_ = nullptr;
    }
}

void TextService::applyPendingCandidateState() noexcept {
    if (!activeContext_ || activeContextId_ == 0 || clientId_ == TF_CLIENTID_NULL ||
        keyEventBusy_) return;
    ScopedBusyFlag busy(keyEventBusy_);
    ipc::KeyResult state;
    if (!client_.pollState(activeContextId_, state)) return;
    imeActive_ = state.candidateVisibility != 0 || !state.preedit.empty();
    if (state.candidateVisibility != 0)
        lastPresentedContextId_ = activeContextId_;
    else if (lastPresentedContextId_ == activeContextId_)
        lastPresentedContextId_ = 0;
    if (uiElementManager_) {
        if (state.candidateVisibility != 0) {
            if (!candidateUiElement_) {
                candidateUiElement_.Attach(new (std::nothrow) CandidateUiElement());
                if (!candidateUiElement_) return;
                candidateUiElement_->update(activeContext_.Get(), state);
                BOOL show = TRUE;
                if (FAILED(uiElementManager_->BeginUIElement(
                        candidateUiElement_.Get(), &show, &candidateUiElementId_))) {
                    candidateUiElementId_ = TF_INVALID_UIELEMENTID;
                    candidateUiElement_.Reset();
                } else {
                    (void)candidateUiElement_->Show(show);
                }
            } else {
                candidateUiElement_->update(activeContext_.Get(), state);
                (void)uiElementManager_->UpdateUIElement(candidateUiElementId_);
            }
        } else if (candidateUiElementId_ != TF_INVALID_UIELEMENTID) {
            (void)uiElementManager_->EndUIElement(candidateUiElementId_);
            candidateUiElementId_ = TF_INVALID_UIELEMENTID;
            candidateUiElement_.Reset();
        }
    }
    (void)applyTextUpdate(activeContext_.Get(), clientId_, this,
                          std::addressof(composition_), std::move(state.commit),
                          std::move(state.preedit), state.preeditCaretUtf16);
}

HRESULT TextService::Activate(ITfThreadMgr* threadManager, TfClientId clientId) noexcept {
    return ActivateEx(threadManager, clientId, 0);
}

HRESULT TextService::ActivateEx(ITfThreadMgr* threadManager, TfClientId clientId,
                                DWORD /*flags*/) noexcept {
    if (!threadManager || clientId == TF_CLIENTID_NULL) {
        return E_INVALIDARG;
    }
    if (threadManager_) {
        return E_UNEXPECTED;
    }
    Microsoft::WRL::ComPtr<ITfKeystrokeMgr> keystrokeManager;
    HRESULT result = threadManager->QueryInterface(IID_PPV_ARGS(&keystrokeManager));
    if (FAILED(result)) {
        return result;
    }
    Microsoft::WRL::ComPtr<ITfSource> source;
    result = threadManager->QueryInterface(IID_PPV_ARGS(&source));
    if (FAILED(result)) {
        return result;
    }
    threadManager_ = threadManager;
    (void)threadManager_.As(&uiElementManager_);
    clientId_ = clientId;
    result = source->AdviseSink(IID_ITfThreadMgrEventSink,
                                static_cast<ITfThreadMgrEventSink*>(this),
                                &threadManagerEventSinkCookie_);
    if (FAILED(result)) {
        threadManager_.Reset();
        uiElementManager_.Reset();
        clientId_ = TF_CLIENTID_NULL;
        return result;
    }
    result = source->AdviseSink(IID_ITfThreadFocusSink,
                                static_cast<ITfThreadFocusSink*>(this),
                                &threadFocusSinkCookie_);
    if (FAILED(result)) {
        (void)source->UnadviseSink(threadManagerEventSinkCookie_);
        threadManagerEventSinkCookie_ = TF_INVALID_COOKIE;
        threadManager_.Reset();
        uiElementManager_.Reset();
        clientId_ = TF_CLIENTID_NULL;
        return result;
    }
    result = keystrokeManager->AdviseKeyEventSink(clientId_, this, TRUE);
    if (FAILED(result)) {
        (void)source->UnadviseSink(threadFocusSinkCookie_);
        (void)source->UnadviseSink(threadManagerEventSinkCookie_);
        threadFocusSinkCookie_ = TF_INVALID_COOKIE;
        threadManagerEventSinkCookie_ = TF_INVALID_COOKIE;
        threadManager_.Reset();
        uiElementManager_.Reset();
        clientId_ = TF_CLIENTID_NULL;
    }
    if (SUCCEEDED(result) && !initializeCandidateNotification()) {
        (void)Deactivate();
        return E_FAIL;
    }
    return result;
}

HRESULT TextService::Deactivate() noexcept {
    shutdownCandidateNotification();
    dismissCandidatePresentation(true);
    composition_.Reset();
    activeContext_.Reset();
    activeContextId_ = 0;
    imeActive_ = false;
    HRESULT result = S_OK;
    if (threadManager_ && clientId_ != TF_CLIENTID_NULL) {
        Microsoft::WRL::ComPtr<ITfSource> source;
        if (SUCCEEDED(threadManager_.As(&source))) {
            if (threadFocusSinkCookie_ != TF_INVALID_COOKIE) {
                const HRESULT unadvise = source->UnadviseSink(threadFocusSinkCookie_);
                if (FAILED(unadvise) && SUCCEEDED(result)) result = unadvise;
            }
            if (threadManagerEventSinkCookie_ != TF_INVALID_COOKIE) {
                const HRESULT unadvise = source->UnadviseSink(threadManagerEventSinkCookie_);
                if (FAILED(unadvise) && SUCCEEDED(result)) result = unadvise;
            }
        }
        threadFocusSinkCookie_ = TF_INVALID_COOKIE;
        threadManagerEventSinkCookie_ = TF_INVALID_COOKIE;
        Microsoft::WRL::ComPtr<ITfKeystrokeMgr> keystrokeManager;
        if (SUCCEEDED(threadManager_.As(&keystrokeManager))) {
            const HRESULT unadvise = keystrokeManager->UnadviseKeyEventSink(clientId_);
            if (FAILED(unadvise) && SUCCEEDED(result)) result = unadvise;
        }
    }
    uiElementManager_.Reset();
    threadManager_.Reset();
    clientId_ = TF_CLIENTID_NULL;
    return result;
}

void TextService::dismissCandidatePresentation(bool disconnectEngine,
                                               std::uint64_t contextId) noexcept {
    if (uiElementManager_ && candidateUiElementId_ != TF_INVALID_UIELEMENTID) {
        (void)uiElementManager_->EndUIElement(candidateUiElementId_);
    }
    candidateUiElementId_ = TF_INVALID_UIELEMENTID;
    candidateUiElement_.Reset();
    lastCaretRects_.clear();
    imeActive_ = false;
    if (disconnectEngine) client_.disconnect();
    const std::uint64_t dismissedContext = contextId != 0 ? contextId : lastPresentedContextId_;
    broadcastCandidateDismiss(dismissedContext);
    if (dismissedContext == 0 || dismissedContext == lastPresentedContextId_)
        lastPresentedContextId_ = 0;
}

void TextService::dismissForFocusLoss(ITfContext* context) noexcept {
    if (context && composition_ && clientId_ != TF_CLIENTID_NULL) {
        (void)applyTextUpdate(context, clientId_, this, std::addressof(composition_),
                              {}, {}, 0);
    }
    dismissCandidatePresentation(
        true, static_cast<std::uint64_t>(reinterpret_cast<std::uintptr_t>(context)));
    activeContext_.Reset();
    activeContextId_ = 0;
    imeActive_ = false;
}

bool TextService::shouldRouteToEngine(WPARAM virtualKey, bool alt, bool rightAlt,
                                      bool win) const noexcept {
    // In the idle state the host editor owns navigation, Enter, Backspace,
    // punctuation and ordinary shortcuts. Route only keys that can start real
    // IME input or explicit IME hotkeys; once preedit/candidates exist, Fcitx
    // becomes the authority for candidate and composition keys.
    if (win) {
        return false;  // Win+* is reserved by the OS.
    }
    const bool ctrl = (GetKeyState(VK_CONTROL) & 0x8000) != 0;
    const bool activeImeState = imeActive_;
    if (virtualKey >= VK_F1 && virtualKey <= VK_F24) {
        return false;  // Application function keys (F5 refresh, Alt+F4, ...).
    }
    if (alt && !rightAlt && virtualKey != VK_SHIFT) {
        // Left-Alt chords are application menus / window shortcuts. Alt+Shift
        // is a configurable IME-switch hotkey, so the Shift key itself must
        // still reach Fcitx while Alt is held.
        return false;
    }
    if (!activeImeState) {
        if ((virtualKey >= 'A' && virtualKey <= 'Z') && !ctrl && !alt)
            return true;
        if (virtualKey == VK_SPACE && ctrl && !alt)
            return true;
        if (virtualKey == VK_SHIFT && (ctrl || alt))
            return true;
        return false;
    }
    switch (virtualKey) {
    case VK_LEFT:
    case VK_RIGHT:
    case VK_UP:
    case VK_DOWN:
    case VK_HOME:
    case VK_END:
    case VK_PRIOR:
    case VK_NEXT:
        // With no active IME state, navigation keys belong to the host editor.
        // Once a composition/candidate UI is active, route them so Fcitx can
        // move the preedit caret or candidate selection.
        return true;
    default:
        break;
    }
    return true;
}

HRESULT TextService::OnSetFocus(BOOL foreground) noexcept {
    if (!foreground && !keyEventBusy_) {
        dismissForFocusLoss(nullptr);
    }
    return S_OK;
}

HRESULT TextService::OnTestKeyDown(ITfContext* context, WPARAM virtualKey,
                                   LPARAM /*keyData*/, BOOL* eaten) noexcept {
    if (!eaten) {
        return E_POINTER;
    }
    bool sensitive = false;
    protocol::CaretRect ignored;
    if (context && clientId_ != TF_CLIENTID_NULL) {
        (void)queryContext(context, clientId_, &ignored, &sensitive);
    }
    const bool alt = (GetKeyState(VK_MENU) & 0x8000) != 0;
    const bool rightAlt = (GetKeyState(VK_RMENU) & 0x8000) != 0;
    const bool win = (GetKeyState(VK_LWIN) & 0x8000) != 0 ||
                     (GetKeyState(VK_RWIN) & 0x8000) != 0;
    *eaten = !sensitive && shouldRouteToEngine(virtualKey, alt, rightAlt, win) ? TRUE : FALSE;
    return S_OK;
}

HRESULT TextService::OnTestKeyUp(ITfContext* /*context*/, WPARAM /*virtualKey*/,
                                 LPARAM /*keyData*/, BOOL* eaten) noexcept {
    if (!eaten) {
        return E_POINTER;
    }
    *eaten = FALSE;
    return S_OK;
}

HRESULT TextService::OnKeyDown(ITfContext* context, WPARAM virtualKey, LPARAM keyData,
                               BOOL* eaten) noexcept {
    if (!eaten) {
        return E_POINTER;
    }
    *eaten = FALSE;
    const bool alt = (GetKeyState(VK_MENU) & 0x8000) != 0;
    const bool rightAlt = (GetKeyState(VK_RMENU) & 0x8000) != 0;
    const bool win = (GetKeyState(VK_LWIN) & 0x8000) != 0 ||
                     (GetKeyState(VK_RWIN) & 0x8000) != 0;
    if (!context || !shouldRouteToEngine(virtualKey, alt, rightAlt, win) ||
        clientId_ == TF_CLIENTID_NULL) {
        return S_OK;
    }
    ScopedBusyFlag busy(keyEventBusy_);
    try {
        ipc::KeyResult keyResult;
        const auto contextId = static_cast<std::uint64_t>(
            reinterpret_cast<std::uintptr_t>(context));
        protocol::CaretRect caret;
        if (const auto found = lastCaretRects_.find(contextId);
            found != lastCaretRects_.end()) {
            caret = found->second;
        }
        bool sensitive = false;
        if (queryContext(context, clientId_, &caret, &sensitive) && caret.valid) {
            lastCaretRects_[contextId] = caret;
        }
        if (sensitive) {
            client_.disconnect();
            if (candidateUiElementId_ != TF_INVALID_UIELEMENTID && uiElementManager_) {
                (void)uiElementManager_->EndUIElement(candidateUiElementId_);
                candidateUiElementId_ = TF_INVALID_UIELEMENTID;
                candidateUiElement_.Reset();
            }
            return S_OK;
        }
        activeContext_ = context;
        activeContextId_ = contextId;
        std::uint32_t keyFlags = 0;
        if ((GetKeyState(VK_SHIFT) & 0x8000) != 0)
            keyFlags |= protocol::kKeyFlagShift;
        if ((GetKeyState(VK_CONTROL) & 0x8000) != 0)
            keyFlags |= protocol::kKeyFlagControl;
        if ((GetKeyState(VK_MENU) & 0x8000) != 0)
            keyFlags |= protocol::kKeyFlagAlt;
        if ((GetKeyState(VK_LWIN) & 0x8000) != 0 ||
            (GetKeyState(VK_RWIN) & 0x8000) != 0)
            keyFlags |= protocol::kKeyFlagSuper;
        const bool engineResponded = client_.processKey(
            contextId, static_cast<std::uint32_t>(virtualKey), keyFlags, keyResult, caret);
        if (!engineResponded) {
            if (candidateUiElementId_ != TF_INVALID_UIELEMENTID && uiElementManager_) {
                (void)uiElementManager_->EndUIElement(candidateUiElementId_);
                candidateUiElementId_ = TF_INVALID_UIELEMENTID;
                candidateUiElement_.Reset();
            }
            std::wstring fallback;
            if (translatePrintableKey(virtualKey, keyData, fallback)) {
                *eaten = applyTextUpdate(context, clientId_, this,
                                         std::addressof(composition_),
                                         std::move(fallback), {}, 0)
                             ? TRUE
                             : FALSE;
            }
            return S_OK;
        }
        if (!keyResult.handled) {
            return S_OK;
        }
        imeActive_ = keyResult.candidateVisibility != 0 || !keyResult.preedit.empty();
        if (keyResult.candidateVisibility != 0) {
            lastPresentedContextId_ = contextId;
        } else if (lastPresentedContextId_ == contextId) {
            lastPresentedContextId_ = 0;
        }
        if (uiElementManager_) {
            if (keyResult.candidateVisibility != 0) {
                if (!candidateUiElement_) {
                    candidateUiElement_.Attach(new (std::nothrow) CandidateUiElement());
                    if (!candidateUiElement_) return E_OUTOFMEMORY;
                    candidateUiElement_->update(context, keyResult);
                    BOOL show = TRUE;
                    const HRESULT begin = uiElementManager_->BeginUIElement(
                        candidateUiElement_.Get(), &show, &candidateUiElementId_);
                    if (FAILED(begin)) {
                        candidateUiElementId_ = TF_INVALID_UIELEMENTID;
                        candidateUiElement_.Reset();
                    } else {
                        (void)candidateUiElement_->Show(show);
                    }
                } else {
                    candidateUiElement_->update(context, keyResult);
                    (void)uiElementManager_->UpdateUIElement(candidateUiElementId_);
                }
            } else if (candidateUiElementId_ != TF_INVALID_UIELEMENTID) {
                (void)uiElementManager_->EndUIElement(candidateUiElementId_);
                candidateUiElementId_ = TF_INVALID_UIELEMENTID;
                candidateUiElement_.Reset();
            }
        }
        if (keyResult.commit.empty() && keyResult.preedit.empty() && !composition_) {
            *eaten = TRUE;
            return S_OK;
        }
        *eaten = applyTextUpdate(context, clientId_, this, std::addressof(composition_),
                                 std::move(keyResult.commit), std::move(keyResult.preedit),
                                 keyResult.preeditCaretUtf16)
                     ? TRUE
                     : FALSE;
        return S_OK;
    } catch (...) {
        client_.disconnect();
        return E_UNEXPECTED;
    }
}

HRESULT TextService::OnKeyUp(ITfContext* context, WPARAM virtualKey,
                             LPARAM /*keyData*/, BOOL* eaten) noexcept {
    if (!eaten) {
        return E_POINTER;
    }
    *eaten = FALSE;
    const bool alt = (GetKeyState(VK_MENU) & 0x8000) != 0;
    const bool rightAlt = (GetKeyState(VK_RMENU) & 0x8000) != 0;
    const bool win = (GetKeyState(VK_LWIN) & 0x8000) != 0 ||
                     (GetKeyState(VK_RWIN) & 0x8000) != 0;
    if (!context || !shouldRouteToEngine(virtualKey, alt, rightAlt, win) ||
        clientId_ == TF_CLIENTID_NULL || keyEventBusy_) {
        return S_OK;
    }
    // Key-up events reach Fcitx so it can track modifier release (Ctrl+Shift
    // / Alt+Shift IME switching depends on the modifier key event, not on a
    // TSF whitelist). The application still receives the key-up: eaten stays
    // FALSE and the engine result is intentionally ignored.
    std::uint32_t keyFlags = protocol::kKeyFlagRelease;
    if ((GetKeyState(VK_SHIFT) & 0x8000) != 0)
        keyFlags |= protocol::kKeyFlagShift;
    if ((GetKeyState(VK_CONTROL) & 0x8000) != 0)
        keyFlags |= protocol::kKeyFlagControl;
    if ((GetKeyState(VK_MENU) & 0x8000) != 0)
        keyFlags |= protocol::kKeyFlagAlt;
    if ((GetKeyState(VK_LWIN) & 0x8000) != 0 ||
        (GetKeyState(VK_RWIN) & 0x8000) != 0)
        keyFlags |= protocol::kKeyFlagSuper;
    ipc::KeyResult keyResult;
    (void)client_.processKey(
        static_cast<std::uint64_t>(reinterpret_cast<std::uintptr_t>(context)),
        static_cast<std::uint32_t>(virtualKey), keyFlags, keyResult);
    return S_OK;
}

HRESULT TextService::OnPreservedKey(ITfContext* /*context*/, REFGUID /*keyGuid*/,
                                    BOOL* eaten) noexcept {
    if (!eaten) {
        return E_POINTER;
    }
    *eaten = FALSE;
    return S_OK;
}

HRESULT TextService::OnCompositionTerminated(
    TfEditCookie /*editCookie*/, ITfComposition* composition) noexcept {
    if (composition_.Get() == composition) {
        composition_.Reset();
    }
    dismissCandidatePresentation(true);
    return S_OK;
}

HRESULT TextService::OnInitDocumentMgr(ITfDocumentMgr* /*documentManager*/) noexcept {
    return S_OK;
}

HRESULT TextService::OnUninitDocumentMgr(ITfDocumentMgr* documentManager) noexcept {
    if (!keyEventBusy_) dismissForFocusLoss(topContext(documentManager).Get());
    return S_OK;
}

HRESULT TextService::OnSetFocus(ITfDocumentMgr* focusedDocumentManager,
                                ITfDocumentMgr* previousDocumentManager) noexcept {
    if (keyEventBusy_) return S_OK;
    if (!focusedDocumentManager ||
        (previousDocumentManager && focusedDocumentManager != previousDocumentManager)) {
        dismissForFocusLoss(topContext(previousDocumentManager).Get());
    }
    return S_OK;
}

HRESULT TextService::OnPushContext(ITfContext* /*context*/) noexcept { return S_OK; }

HRESULT TextService::OnPopContext(ITfContext* context) noexcept {
    if (!keyEventBusy_) dismissForFocusLoss(context);
    return S_OK;
}

HRESULT TextService::OnSetThreadFocus() noexcept { return S_OK; }

HRESULT TextService::OnKillThreadFocus() noexcept {
    if (keyEventBusy_) return S_OK;
    Microsoft::WRL::ComPtr<ITfDocumentMgr> documentManager;
    if (threadManager_) (void)threadManager_->GetFocus(&documentManager);
    dismissForFocusLoss(topContext(documentManager.Get()).Get());
    return S_OK;
}

} // namespace fcitx::windows::tsf
