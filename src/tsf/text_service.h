#pragma once

#include "candidate_ui_element.h"
#include "pipe_client.h"

#include <Windows.h>
#include <msctf.h>
#include <wrl/client.h>

#include <atomic>
#include <cstdint>
#include <unordered_map>

namespace fcitx::windows::tsf {

class TextService final : public ITfTextInputProcessorEx,
                          public ITfKeyEventSink,
                          public ITfCompositionSink {
public:
    TextService();

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID interfaceId, void** object) noexcept override;
    ULONG STDMETHODCALLTYPE AddRef() noexcept override;
    ULONG STDMETHODCALLTYPE Release() noexcept override;

    HRESULT STDMETHODCALLTYPE Activate(ITfThreadMgr* threadManager,
                                       TfClientId clientId) noexcept override;
    HRESULT STDMETHODCALLTYPE ActivateEx(ITfThreadMgr* threadManager, TfClientId clientId,
                                         DWORD flags) noexcept override;
    HRESULT STDMETHODCALLTYPE Deactivate() noexcept override;

    HRESULT STDMETHODCALLTYPE OnSetFocus(BOOL foreground) noexcept override;
    HRESULT STDMETHODCALLTYPE OnTestKeyDown(ITfContext* context, WPARAM virtualKey, LPARAM keyData,
                                            BOOL* eaten) noexcept override;
    HRESULT STDMETHODCALLTYPE OnTestKeyUp(ITfContext* context, WPARAM virtualKey, LPARAM keyData,
                                          BOOL* eaten) noexcept override;
    HRESULT STDMETHODCALLTYPE OnKeyDown(ITfContext* context, WPARAM virtualKey, LPARAM keyData,
                                        BOOL* eaten) noexcept override;
    HRESULT STDMETHODCALLTYPE OnKeyUp(ITfContext* context, WPARAM virtualKey, LPARAM keyData,
                                      BOOL* eaten) noexcept override;
    HRESULT STDMETHODCALLTYPE OnPreservedKey(ITfContext* context, REFGUID keyGuid,
                                             BOOL* eaten) noexcept override;
    HRESULT STDMETHODCALLTYPE OnCompositionTerminated(
        TfEditCookie editCookie, ITfComposition* composition) noexcept override;

private:
    ~TextService();

    [[nodiscard]] bool canHandle(WPARAM virtualKey) const noexcept;

    std::atomic<ULONG> referenceCount_{1};
    Microsoft::WRL::ComPtr<ITfThreadMgr> threadManager_;
    Microsoft::WRL::ComPtr<ITfUIElementMgr> uiElementManager_;
    TfClientId clientId_{TF_CLIENTID_NULL};
    ipc::PipeClient client_;
    Microsoft::WRL::ComPtr<ITfComposition> composition_;
    Microsoft::WRL::ComPtr<CandidateUiElement> candidateUiElement_;
    DWORD candidateUiElementId_{TF_INVALID_UIELEMENTID};
    std::unordered_map<std::uint64_t, protocol::CaretRect> lastCaretRects_;
};

} // namespace fcitx::windows::tsf
