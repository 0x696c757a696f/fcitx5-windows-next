#include "candidate_ui_element.h"

#include "guids.h"

#include <OleAuto.h>

#include <algorithm>
#include <utility>

namespace fcitx::windows::tsf {

CandidateUiElement::CandidateUiElement() = default;

HRESULT CandidateUiElement::QueryInterface(REFIID interfaceId, void** object) noexcept {
    if (!object) return E_POINTER;
    *object = nullptr;
    if (IsEqualIID(interfaceId, IID_IUnknown) ||
        IsEqualIID(interfaceId, IID_ITfUIElement) ||
        IsEqualIID(interfaceId, IID_ITfCandidateListUIElement)) {
        *object = static_cast<ITfCandidateListUIElement*>(this);
        AddRef();
        return S_OK;
    }
    return E_NOINTERFACE;
}

ULONG CandidateUiElement::AddRef() noexcept {
    return references_.fetch_add(1, std::memory_order_relaxed) + 1;
}

ULONG CandidateUiElement::Release() noexcept {
    const ULONG remaining = references_.fetch_sub(1, std::memory_order_acq_rel) - 1;
    if (remaining == 0) delete this;
    return remaining;
}

HRESULT CandidateUiElement::GetDescription(BSTR* description) noexcept {
    if (!description) return E_POINTER;
    *description = SysAllocString(L"Fcitx5 candidates");
    return *description ? S_OK : E_OUTOFMEMORY;
}

HRESULT CandidateUiElement::GetGUID(GUID* guid) noexcept {
    if (!guid) return E_POINTER;
    *guid = kLanguageProfileGuid;
    return S_OK;
}

HRESULT CandidateUiElement::Show(BOOL show) noexcept {
    shown_ = show != FALSE;
    updatedFlags_ |= TF_CLUIE_STRING;
    return S_OK;
}

HRESULT CandidateUiElement::IsShown(BOOL* show) noexcept {
    if (!show) return E_POINTER;
    *show = shown_ ? TRUE : FALSE;
    return S_OK;
}

HRESULT CandidateUiElement::GetUpdatedFlags(DWORD* flags) noexcept {
    if (!flags) return E_POINTER;
    *flags = std::exchange(updatedFlags_, 0);
    return S_OK;
}

HRESULT CandidateUiElement::GetDocumentMgr(ITfDocumentMgr** manager) noexcept {
    if (!manager) return E_POINTER;
    return documentManager_.CopyTo(manager);
}

HRESULT CandidateUiElement::GetCount(UINT* count) noexcept {
    if (!count) return E_POINTER;
    *count = static_cast<UINT>(candidates_.size());
    return S_OK;
}

HRESULT CandidateUiElement::GetSelection(UINT* index) noexcept {
    if (!index) return E_POINTER;
    *index = selection_;
    return S_OK;
}

HRESULT CandidateUiElement::GetString(UINT index, BSTR* text) noexcept {
    if (!text) return E_POINTER;
    *text = nullptr;
    if (index >= candidates_.size()) return E_INVALIDARG;
    *text = SysAllocStringLen(candidates_[index].data(),
                              static_cast<UINT>(candidates_[index].size()));
    return *text ? S_OK : E_OUTOFMEMORY;
}

HRESULT CandidateUiElement::GetPageIndex(UINT* indices, UINT size,
                                         UINT* pageCount) noexcept {
    if (!pageCount) return E_POINTER;
    *pageCount = candidates_.empty() ? 0U : 1U;
    if (candidates_.empty()) return S_OK;
    if (!indices || size < 1) return E_INVALIDARG;
    indices[0] = 0;
    return S_OK;
}

HRESULT CandidateUiElement::SetPageIndex(UINT*, UINT) noexcept { return E_NOTIMPL; }

HRESULT CandidateUiElement::GetCurrentPage(UINT* page) noexcept {
    if (!page) return E_POINTER;
    // This UIElement exposes only the engine's immutable current-page snapshot.
    // GetCurrentPage is an index into GetPageIndex(), not the engine's absolute
    // page number, so the sole exposed page is always index zero.
    *page = 0;
    return S_OK;
}

void CandidateUiElement::update(ITfContext* context, const ipc::KeyResult& result) {
    documentManager_.Reset();
    if (context) (void)context->GetDocumentMgr(&documentManager_);
    candidates_.clear();
    candidates_.reserve(result.candidates.size());
    for (const auto& candidate : result.candidates) {
        std::wstring display;
        if (!candidate.label.empty()) {
            display += candidate.label;
            display += L". ";
        }
        display += candidate.text;
        if (!candidate.comment.empty()) {
            display += L"  ";
            display += candidate.comment;
        }
        candidates_.emplace_back(std::move(display));
    }
    selection_ = result.selectedCandidate == UINT32_MAX
                     ? 0U
                     : std::min(result.selectedCandidate,
                                static_cast<UINT>(candidates_.empty() ? 0 : candidates_.size() - 1));
    shown_ = result.candidateVisibility != 0;
    updatedFlags_ = TF_CLUIE_COUNT | TF_CLUIE_SELECTION | TF_CLUIE_STRING |
                    TF_CLUIE_PAGEINDEX | TF_CLUIE_CURRENTPAGE;
}

} // namespace fcitx::windows::tsf
