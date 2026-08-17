#include "text_service.h"

#include "module.h"

#include <cstdint>
#include <new>
#include <string>
#include <utility>

namespace fcitx::windows::tsf {
namespace {

class CommitEditSession final : public ITfEditSession {
public:
    CommitEditSession(ITfContext* context, std::wstring text)
        : context_(context), text_(std::move(text)) {}

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
        result = selection.range->SetText(editCookie, 0, text_.data(),
                                          static_cast<LONG>(text_.size()));
        if (SUCCEEDED(result)) {
            result = selection.range->Collapse(editCookie, TF_ANCHOR_END);
        }
        if (SUCCEEDED(result)) {
            selection.style.ase = TF_AE_END;
            selection.style.fInterimChar = FALSE;
            result = context_->SetSelection(editCookie, 1, &selection);
        }
        selection.range->Release();
        committed_ = SUCCEEDED(result);
        return result;
    }

    [[nodiscard]] bool committed() const noexcept { return committed_; }

private:
    ~CommitEditSession() = default;

    std::atomic<ULONG> referenceCount_{1};
    Microsoft::WRL::ComPtr<ITfContext> context_;
    std::wstring text_;
    bool committed_{};
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
    if (virtualKey < 'A' || virtualKey > 'Z') {
        return false;
    }
    return (GetKeyState(VK_CONTROL) & 0x8000) == 0 && (GetKeyState(VK_MENU) & 0x8000) == 0;
}

HRESULT TextService::OnSetFocus(BOOL /*foreground*/) noexcept { return S_OK; }

HRESULT TextService::OnTestKeyDown(ITfContext* /*context*/, WPARAM virtualKey,
                                   LPARAM /*keyData*/, BOOL* eaten) noexcept {
    if (!eaten) {
        return E_POINTER;
    }
    *eaten = canHandle(virtualKey) ? TRUE : FALSE;
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
        if (!client_.processKey(contextId, static_cast<std::uint32_t>(virtualKey),
                                static_cast<std::uint32_t>(keyData), keyResult) ||
            !keyResult.handled) {
            return S_OK;
        }
        if (keyResult.commit.empty()) {
            *eaten = TRUE;
            return S_OK;
        }
        auto* editSession = new (std::nothrow) CommitEditSession(context, std::move(keyResult.commit));
        if (!editSession) {
            return E_OUTOFMEMORY;
        }
        HRESULT sessionResult = E_FAIL;
        const HRESULT requestResult = context->RequestEditSession(
            clientId_, editSession, TF_ES_SYNC | TF_ES_READWRITE, &sessionResult);
        const bool committed = SUCCEEDED(requestResult) && SUCCEEDED(sessionResult) &&
                               editSession->committed();
        editSession->Release();
        *eaten = committed ? TRUE : FALSE;
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

} // namespace fcitx::windows::tsf
