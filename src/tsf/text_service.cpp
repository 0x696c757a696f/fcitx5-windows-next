#include "text_service.h"

#include "module.h"

#include <cstdint>
#include <memory>
#include <new>
#include <string>
#include <utility>

namespace fcitx::windows::tsf {
namespace {

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
