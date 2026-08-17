#include "text_service.h"

#include "input_scope_policy.h"
#include "module.h"

#include <OleAuto.h>
#include <cstdint>
#include <memory>
#include <new>
#include <string>
#include <utility>

namespace fcitx::windows::tsf {
namespace {

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

} // namespace

TextService::TextService() { moduleAddRef(); }

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
    threadManager_ = threadManager;
    (void)threadManager_.As(&uiElementManager_);
    clientId_ = clientId;
    result = keystrokeManager->AdviseKeyEventSink(clientId_, this, TRUE);
    if (FAILED(result)) {
        threadManager_.Reset();
        clientId_ = TF_CLIENTID_NULL;
    }
    return result;
}

HRESULT TextService::Deactivate() noexcept {
    client_.disconnect();
    composition_.Reset();
    if (uiElementManager_ && candidateUiElementId_ != TF_INVALID_UIELEMENTID) {
        (void)uiElementManager_->EndUIElement(candidateUiElementId_);
    }
    candidateUiElementId_ = TF_INVALID_UIELEMENTID;
    candidateUiElement_.Reset();
    lastCaretRects_.clear();
    uiElementManager_.Reset();
    HRESULT result = S_OK;
    if (threadManager_ && clientId_ != TF_CLIENTID_NULL) {
        Microsoft::WRL::ComPtr<ITfKeystrokeMgr> keystrokeManager;
        if (SUCCEEDED(threadManager_.As(&keystrokeManager))) {
            result = keystrokeManager->UnadviseKeyEventSink(clientId_);
        }
    }
    threadManager_.Reset();
    clientId_ = TF_CLIENTID_NULL;
    return result;
}

bool TextService::canHandle(WPARAM virtualKey) const noexcept {
    if ((GetKeyState(VK_CONTROL) & 0x8000) != 0 ||
        (GetKeyState(VK_MENU) & 0x8000) != 0) {
        return false;
    }
    if ((virtualKey >= 'A' && virtualKey <= 'Z') ||
        (virtualKey >= '0' && virtualKey <= '9')) {
        return true;
    }
    if (!composition_) {
        return false;
    }
    switch (virtualKey) {
    case VK_SPACE:
    case VK_BACK:
    case VK_RETURN:
    case VK_ESCAPE:
    case VK_LEFT:
    case VK_RIGHT:
    case VK_UP:
    case VK_DOWN:
    case VK_PRIOR:
    case VK_NEXT:
        return true;
    default:
        return false;
    }
}

HRESULT TextService::OnSetFocus(BOOL /*foreground*/) noexcept { return S_OK; }

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
    *eaten = !sensitive && canHandle(virtualKey) ? TRUE : FALSE;
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
    if (!context || !canHandle(virtualKey) || clientId_ == TF_CLIENTID_NULL) {
        return S_OK;
    }
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
        if (!client_.processKey(contextId, static_cast<std::uint32_t>(virtualKey),
                                static_cast<std::uint32_t>(keyData), keyResult, caret) ||
            !keyResult.handled) {
            return S_OK;
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
        auto* editSession = new (std::nothrow) CompositionEditSession(
            context, this, std::addressof(composition_), std::move(keyResult.commit),
            std::move(keyResult.preedit), keyResult.preeditCaretUtf16);
        if (!editSession) {
            return E_OUTOFMEMORY;
        }
        HRESULT sessionResult = E_FAIL;
        const HRESULT requestResult = context->RequestEditSession(
            clientId_, editSession, TF_ES_SYNC | TF_ES_READWRITE, &sessionResult);
        const bool applied = SUCCEEDED(requestResult) && SUCCEEDED(sessionResult) &&
                             editSession->applied();
        editSession->Release();
        *eaten = applied ? TRUE : FALSE;
        return S_OK;
    } catch (...) {
        client_.disconnect();
        return E_UNEXPECTED;
    }
}

HRESULT TextService::OnKeyUp(ITfContext* /*context*/, WPARAM /*virtualKey*/,
                             LPARAM /*keyData*/, BOOL* eaten) noexcept {
    if (!eaten) {
        return E_POINTER;
    }
    *eaten = FALSE;
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
    return S_OK;
}

} // namespace fcitx::windows::tsf
