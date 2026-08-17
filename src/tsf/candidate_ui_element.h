#pragma once

#include "pipe_client.h"

#include <Windows.h>
#include <msctf.h>
#include <wrl/client.h>

#include <atomic>
#include <vector>

namespace fcitx::windows::tsf {

class CandidateUiElement final : public ITfCandidateListUIElement {
public:
    CandidateUiElement();

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID interfaceId, void** object) noexcept override;
    ULONG STDMETHODCALLTYPE AddRef() noexcept override;
    ULONG STDMETHODCALLTYPE Release() noexcept override;
    HRESULT STDMETHODCALLTYPE GetDescription(BSTR* description) noexcept override;
    HRESULT STDMETHODCALLTYPE GetGUID(GUID* guid) noexcept override;
    HRESULT STDMETHODCALLTYPE Show(BOOL show) noexcept override;
    HRESULT STDMETHODCALLTYPE IsShown(BOOL* show) noexcept override;
    HRESULT STDMETHODCALLTYPE GetUpdatedFlags(DWORD* flags) noexcept override;
    HRESULT STDMETHODCALLTYPE GetDocumentMgr(ITfDocumentMgr** manager) noexcept override;
    HRESULT STDMETHODCALLTYPE GetCount(UINT* count) noexcept override;
    HRESULT STDMETHODCALLTYPE GetSelection(UINT* index) noexcept override;
    HRESULT STDMETHODCALLTYPE GetString(UINT index, BSTR* text) noexcept override;
    HRESULT STDMETHODCALLTYPE GetPageIndex(UINT* indices, UINT size,
                                           UINT* pageCount) noexcept override;
    HRESULT STDMETHODCALLTYPE SetPageIndex(UINT* indices, UINT pageCount) noexcept override;
    HRESULT STDMETHODCALLTYPE GetCurrentPage(UINT* page) noexcept override;

    void update(ITfContext* context, const ipc::KeyResult& result);

private:
    ~CandidateUiElement() = default;

    std::atomic<ULONG> references_{1};
    Microsoft::WRL::ComPtr<ITfDocumentMgr> documentManager_;
    std::vector<std::wstring> candidates_;
    UINT selection_{};
    DWORD updatedFlags_{};
    bool shown_{};
};

} // namespace fcitx::windows::tsf
