#pragma once

#include "candidate_ui_element.h"
#include "pipe_client.h"
#include "guids.h"

#include <Windows.h>
#include <msctf.h>
#include <wrl/client.h>

#include <atomic>
#include <cstdint>
#include <string>
#include <unordered_map>

namespace fcitx::windows::tsf {

class TextService final : public ITfTextInputProcessorEx,
                          public ITfKeyEventSink,
                          public ITfCompositionSink,
                          public ITfThreadMgrEventSink,
                          public ITfThreadFocusSink,
                          public ITfActiveLanguageProfileNotifySink {
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

    HRESULT STDMETHODCALLTYPE OnInitDocumentMgr(ITfDocumentMgr* documentManager) noexcept override;
    HRESULT STDMETHODCALLTYPE OnUninitDocumentMgr(ITfDocumentMgr* documentManager) noexcept override;
    HRESULT STDMETHODCALLTYPE OnSetFocus(ITfDocumentMgr* focusedDocumentManager,
                                         ITfDocumentMgr* previousDocumentManager) noexcept override;
    HRESULT STDMETHODCALLTYPE OnPushContext(ITfContext* context) noexcept override;
    HRESULT STDMETHODCALLTYPE OnPopContext(ITfContext* context) noexcept override;

    HRESULT STDMETHODCALLTYPE OnSetThreadFocus() noexcept override;
    HRESULT STDMETHODCALLTYPE OnKillThreadFocus() noexcept override;

    HRESULT STDMETHODCALLTYPE OnActivated(REFCLSID classId, REFGUID profileGuid,
                                          BOOL activated) noexcept override;

private:
    ~TextService();

    [[nodiscard]] bool shouldRouteToEngine(WPARAM virtualKey, bool alt, bool rightAlt,
                                           bool win) const noexcept;
    void dismissCandidatePresentation(bool disconnectEngine,
                                      std::uint64_t contextId = 0) noexcept;
    void dismissForFocusLoss(ITfContext* context) noexcept;
    [[nodiscard]] bool initializeCandidateNotification() noexcept;
    void shutdownCandidateNotification() noexcept;
    void applyPendingCandidateState() noexcept;
    static LRESULT CALLBACK notificationWindowProcedure(HWND window, UINT message,
                                                        WPARAM wparam,
                                                        LPARAM lparam) noexcept;
    static VOID CALLBACK notificationWaitCallback(PVOID context,
                                                   BOOLEAN timedOut) noexcept;

    std::atomic<ULONG> referenceCount_{1};
    Microsoft::WRL::ComPtr<ITfThreadMgr> threadManager_;
    Microsoft::WRL::ComPtr<ITfUIElementMgr> uiElementManager_;
    TfClientId clientId_{TF_CLIENTID_NULL};
    ipc::PipeClient client_;
    Microsoft::WRL::ComPtr<ITfComposition> composition_;
    Microsoft::WRL::ComPtr<CandidateUiElement> candidateUiElement_;
    DWORD candidateUiElementId_{TF_INVALID_UIELEMENTID};
    DWORD threadManagerEventSinkCookie_{TF_INVALID_COOKIE};
    DWORD threadFocusSinkCookie_{TF_INVALID_COOKIE};
    DWORD activeProfileSinkCookie_{TF_INVALID_COOKIE};
    std::uint64_t lastPresentedContextId_{};
    std::unordered_map<std::uint64_t, protocol::CaretRect> lastCaretRects_;
    std::unordered_map<std::uint64_t, bool> popupAllowedByContext_;
    Microsoft::WRL::ComPtr<ITfContext> activeContext_;
    std::uint64_t activeContextId_{};
    std::string activeInputMethod_{kInputProfiles[0].engine};
    bool imeActive_{};
    HANDLE notificationEvent_{};
    HANDLE notificationWait_{};
    HWND notificationWindow_{};
    bool keyEventBusy_{};
};

} // namespace fcitx::windows::tsf
