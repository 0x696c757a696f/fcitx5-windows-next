#include "guids.h"

#include <Windows.h>
#include <OleAuto.h>
#include <inputscope.h>
#include <msctf.h>
#include <objbase.h>
#include <wrl/client.h>

#include <algorithm>
#include <atomic>
#include <cstdint>
#include <iostream>
#include <memory>
#include <new>
#include <string>
#include <vector>

namespace {

using Microsoft::WRL::ComPtr;

class TestRange final : public ITfRange {
public:
    TestRange() : document_(std::make_shared<std::wstring>()) {}
    TestRange(std::shared_ptr<std::wstring> document, std::size_t start, std::size_t end)
        : document_(std::move(document)), start_(start), end_(end) {}

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
    HRESULT STDMETHODCALLTYPE GetText(TfEditCookie, DWORD, WCHAR* text, ULONG count,
                                      ULONG* fetched) noexcept override {
        if (!fetched || (!text && count != 0)) return E_POINTER;
        const auto begin = (std::min)(start_, document_->size());
        const auto finish = (std::min)((std::max)(begin, end_), document_->size());
        const auto available = finish - begin;
        const auto copied = (std::min)(available, static_cast<std::size_t>(count));
        if (copied != 0) {
            std::copy_n(document_->data() + begin, copied, text);
        }
        *fetched = static_cast<ULONG>(copied);
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE SetText(TfEditCookie, DWORD, const WCHAR* text,
                                      LONG length) noexcept override {
        if (length < 0 || (!text && length != 0)) return E_INVALIDARG;
        try {
            const std::wstring replacement(text ? text : L"",
                                           static_cast<std::size_t>(length));
            const auto begin = (std::min)(start_, document_->size());
            const auto finish = (std::min)((std::max)(begin, end_), document_->size());
            document_->replace(begin, finish - begin, replacement);
            start_ = begin;
            end_ = begin + replacement.size();
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
    HRESULT STDMETHODCALLTYPE ShiftStart(TfEditCookie, LONG count, LONG* shifted,
                                         const TF_HALTCOND*) noexcept override {
        if (!shifted) return E_POINTER;
        const auto original = start_;
        if (count < 0) {
            const auto requested = static_cast<std::size_t>(-count);
            start_ = requested > start_ ? 0 : start_ - requested;
        } else {
            start_ = (std::min)(end_, start_ + static_cast<std::size_t>(count));
        }
        *shifted = count < 0 ? -static_cast<LONG>(original - start_)
                             : static_cast<LONG>(start_ - original);
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE ShiftEnd(TfEditCookie, LONG count, LONG* shifted,
                                       const TF_HALTCOND*) noexcept override {
        if (!shifted) return E_POINTER;
        const auto original = end_;
        if (count < 0) {
            const auto requested = static_cast<std::size_t>(-count);
            end_ = requested > end_ - start_ ? start_ : end_ - requested;
        } else {
            end_ = (std::min)(document_->size(), end_ + static_cast<std::size_t>(count));
        }
        (void)original;
        *shifted = count;
        return S_OK;
    }
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
    HRESULT STDMETHODCALLTYPE Clone(ITfRange** range) noexcept override {
        if (!range) return E_POINTER;
        *range = new (std::nothrow) TestRange(document_, start_, end_);
        return *range ? S_OK : E_OUTOFMEMORY;
    }
    HRESULT STDMETHODCALLTYPE GetContext(ITfContext**) noexcept override { return E_NOTIMPL; }

    [[nodiscard]] const std::wstring& text() const noexcept { return *document_; }
    void setDocumentText(std::wstring text, std::size_t cursor) {
        *document_ = std::move(text);
        start_ = (std::min)(cursor, document_->size());
        end_ = start_;
    }

private:
    std::atomic<ULONG> references_{1};
    std::shared_ptr<std::wstring> document_;
    std::size_t start_{};
    std::size_t end_{};
};

class TestComposition final : public ITfComposition {
public:
    TestComposition(TestRange* range, bool* active, bool* ended, ITfCompositionSink* sink)
        : range_(range), active_(active), ended_(ended), sink_(sink) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** object) noexcept override {
        if (!object) return E_POINTER;
        *object = nullptr;
        if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITfComposition)) {
            *object = static_cast<ITfComposition*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }
    ULONG STDMETHODCALLTYPE AddRef() noexcept override {
        return references_.fetch_add(1, std::memory_order_relaxed) + 1;
    }
    ULONG STDMETHODCALLTYPE Release() noexcept override {
        const ULONG remaining = references_.fetch_sub(1, std::memory_order_acq_rel) - 1;
        if (remaining == 0) delete this;
        return remaining;
    }
    HRESULT STDMETHODCALLTYPE GetRange(ITfRange** range) noexcept override {
        if (!range) return E_POINTER;
        range_->AddRef();
        *range = range_;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE ShiftStart(TfEditCookie, ITfRange*) noexcept override {
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE ShiftEnd(TfEditCookie, ITfRange*) noexcept override {
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE EndComposition(TfEditCookie editCookie) noexcept override {
        *active_ = false;
        *ended_ = true;
        if (sink_) return sink_->OnCompositionTerminated(editCookie, this);
        return S_OK;
    }

private:
    ~TestComposition() = default;
    std::atomic<ULONG> references_{1};
    TestRange* range_{};
    bool* active_{};
    bool* ended_{};
    ComPtr<ITfCompositionSink> sink_;
};

class TestInputScope final : public ITfInputScope {
public:
    explicit TestInputScope(InputScope scope) : scope_(scope) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** object) noexcept override {
        if (!object) return E_POINTER;
        *object = nullptr;
        if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITfInputScope)) {
            *object = static_cast<ITfInputScope*>(this);
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
    HRESULT STDMETHODCALLTYPE GetInputScopes(InputScope** scopes, UINT* count) noexcept override {
        if (!scopes || !count) return E_POINTER;
        *scopes = static_cast<InputScope*>(CoTaskMemAlloc(sizeof(scope_)));
        if (!*scopes) return E_OUTOFMEMORY;
        **scopes = scope_;
        *count = 1;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetPhrase(BSTR**, UINT*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetRegularExpression(BSTR*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetSRGS(BSTR*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetXML(BSTR*) noexcept override { return E_NOTIMPL; }

private:
    std::atomic<ULONG> references_{1};
    InputScope scope_{};
};

class TestInputScopeProperty final : public ITfReadOnlyProperty {
public:
    explicit TestInputScopeProperty(TestInputScope* scope) : scope_(scope) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** object) noexcept override {
        if (!object) return E_POINTER;
        *object = nullptr;
        if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITfReadOnlyProperty)) {
            *object = static_cast<ITfReadOnlyProperty*>(this);
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
    HRESULT STDMETHODCALLTYPE GetType(GUID* type) noexcept override {
        if (!type) return E_POINTER;
        *type = GUID_PROP_INPUTSCOPE;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE EnumRanges(TfEditCookie, IEnumTfRanges**,
                                         ITfRange*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetValue(TfEditCookie, ITfRange*, VARIANT* value) noexcept override {
        if (!value) return E_POINTER;
        VariantInit(value);
        value->vt = VT_UNKNOWN;
        value->punkVal = scope_;
        scope_->AddRef();
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetContext(ITfContext**) noexcept override { return E_NOTIMPL; }

private:
    std::atomic<ULONG> references_{1};
    TestInputScope* scope_{};
};

class TestContext final : public ITfContext, public ITfContextComposition {
public:
    explicit TestContext(InputScope scope = IS_DEFAULT)
        : inputScope_(scope), inputScopeProperty_(&inputScope_) {}
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** object) noexcept override {
        if (!object) return E_POINTER;
        *object = nullptr;
        if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITfContext)) {
            *object = static_cast<ITfContext*>(this);
        } else if (IsEqualIID(iid, IID_ITfContextComposition)) {
            *object = static_cast<ITfContextComposition*>(this);
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
    HRESULT STDMETHODCALLTYPE RequestEditSession(TfClientId, ITfEditSession* session,
                                                 DWORD flags,
                                                 HRESULT* sessionResult) noexcept override {
        if (!session || !sessionResult) return E_POINTER;
        if ((flags & TF_ES_SYNC) == 0 ||
            (flags & (TF_ES_READ | TF_ES_READWRITE)) == 0) {
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
    HRESULT STDMETHODCALLTYPE GetAppProperty(REFGUID guid,
                                             ITfReadOnlyProperty** property) noexcept override {
        if (!property) return E_POINTER;
        *property = nullptr;
        if (!IsEqualGUID(guid, GUID_PROP_INPUTSCOPE)) return E_NOTIMPL;
        inputScopeProperty_.AddRef();
        *property = &inputScopeProperty_;
        return S_OK;
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

    HRESULT STDMETHODCALLTYPE StartComposition(TfEditCookie, ITfRange* range,
                                               ITfCompositionSink* sink,
                                               ITfComposition** composition) noexcept override {
        if (!range || !composition || active_) return E_INVALIDARG;
        active_ = true;
        started_ = true;
        ended_ = false;
        *composition = new (std::nothrow) TestComposition(&range_, &active_, &ended_, sink);
        return *composition ? S_OK : E_OUTOFMEMORY;
    }
    HRESULT STDMETHODCALLTYPE EnumCompositions(IEnumITfCompositionView**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE FindComposition(TfEditCookie, ITfRange*,
                                              IEnumITfCompositionView**) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE TakeOwnership(TfEditCookie, ITfCompositionView*,
                                            ITfCompositionSink*,
                                            ITfComposition**) noexcept override {
        return E_NOTIMPL;
    }

    [[nodiscard]] const std::wstring& text() const noexcept { return range_.text(); }
    [[nodiscard]] bool compositionEnded() const noexcept { return ended_; }
    [[nodiscard]] bool compositionStarted() const noexcept { return started_; }
    void setDocumentText(std::wstring text, std::size_t cursor) {
        range_.setDocumentText(std::move(text), cursor);
    }

private:
    std::atomic<ULONG> references_{1};
    TestRange range_;
    TestInputScope inputScope_;
    TestInputScopeProperty inputScopeProperty_;
    bool active_{};
    bool started_{};
    bool ended_{};
};

class TestDocumentManager final : public ITfDocumentMgr {
public:
    explicit TestDocumentManager(ITfContext* context) : context_(context) {}

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** object) noexcept override {
        if (!object) return E_POINTER;
        *object = nullptr;
        if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITfDocumentMgr)) {
            *object = static_cast<ITfDocumentMgr*>(this);
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
    HRESULT STDMETHODCALLTYPE CreateContext(TfClientId, DWORD, IUnknown*, ITfContext**,
                                            TfEditCookie*) noexcept override {
        return E_NOTIMPL;
    }
    HRESULT STDMETHODCALLTYPE Push(ITfContext*) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE Pop(DWORD) noexcept override { return E_NOTIMPL; }
    HRESULT STDMETHODCALLTYPE GetTop(ITfContext** context) noexcept override {
        if (!context) return E_POINTER;
        *context = context_.Get();
        if (*context) (*context)->AddRef();
        return *context ? S_OK : S_FALSE;
    }
    HRESULT STDMETHODCALLTYPE GetBase(ITfContext** context) noexcept override {
        return GetTop(context);
    }
    HRESULT STDMETHODCALLTYPE EnumContexts(IEnumTfContexts**) noexcept override {
        return E_NOTIMPL;
    }

private:
    std::atomic<ULONG> references_{1};
    ComPtr<ITfContext> context_;
};

class TestThreadManager final : public ITfThreadMgr,
                                public ITfKeystrokeMgr,
                                public ITfSource,
                                public ITfUIElementMgr {
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, void** object) noexcept override {
        if (!object) return E_POINTER;
        *object = nullptr;
        if (IsEqualIID(iid, IID_IUnknown) || IsEqualIID(iid, IID_ITfThreadMgr)) {
            *object = static_cast<ITfThreadMgr*>(this);
        } else if (IsEqualIID(iid, IID_ITfKeystrokeMgr)) {
            *object = static_cast<ITfKeystrokeMgr*>(this);
        } else if (IsEqualIID(iid, IID_ITfSource)) {
            *object = static_cast<ITfSource*>(this);
        } else if (IsEqualIID(iid, IID_ITfUIElementMgr)) {
            *object = static_cast<ITfUIElementMgr*>(this);
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
    HRESULT STDMETHODCALLTYPE AdviseSink(REFIID iid, IUnknown* unknown,
                                         DWORD* cookie) noexcept override {
        if (!unknown || !cookie) return E_POINTER;
        if (IsEqualIID(iid, IID_ITfThreadMgrEventSink) && !threadManagerEventSink_) {
            const HRESULT result = unknown->QueryInterface(
                IID_PPV_ARGS(threadManagerEventSink_.ReleaseAndGetAddressOf()));
            if (FAILED(result)) return result;
            *cookie = 1;
            return S_OK;
        }
        if (IsEqualIID(iid, IID_ITfThreadFocusSink) && !threadFocusSink_) {
            const HRESULT result = unknown->QueryInterface(
                IID_PPV_ARGS(threadFocusSink_.ReleaseAndGetAddressOf()));
            if (FAILED(result)) return result;
            *cookie = 2;
            return S_OK;
        }
        if (IsEqualIID(iid, IID_ITfActiveLanguageProfileNotifySink) &&
            !activeProfileSink_) {
            const HRESULT result = unknown->QueryInterface(
                IID_PPV_ARGS(activeProfileSink_.ReleaseAndGetAddressOf()));
            if (FAILED(result)) return result;
            *cookie = 3;
            return S_OK;
        }
        return E_UNEXPECTED;
    }
    HRESULT STDMETHODCALLTYPE UnadviseSink(DWORD cookie) noexcept override {
        if (cookie == 1 && threadManagerEventSink_) {
            threadManagerEventSink_.Reset();
            return S_OK;
        }
        if (cookie == 2 && threadFocusSink_) {
            threadFocusSink_.Reset();
            return S_OK;
        }
        if (cookie == 3 && activeProfileSink_) {
            activeProfileSink_.Reset();
            return S_OK;
        }
        return E_INVALIDARG;
    }
    HRESULT STDMETHODCALLTYPE BeginUIElement(ITfUIElement* element, BOOL* show,
                                             DWORD* id) noexcept override {
        if (!element || !show || !id) return E_POINTER;
        uiElement_ = element;
        *show = beginShow_;
        *id = 1;
        ++uiBeginCount_;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE UpdateUIElement(DWORD id) noexcept override {
        if (id != 1 || !uiElement_) return E_INVALIDARG;
        ++uiUpdateCount_;
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE EndUIElement(DWORD id) noexcept override {
        if (id != 1 || !uiElement_) return E_INVALIDARG;
        ++uiEndCount_;
        uiElement_.Reset();
        return S_OK;
    }
    HRESULT STDMETHODCALLTYPE GetUIElement(DWORD id, ITfUIElement** element) noexcept override {
        if (!element) return E_POINTER;
        *element = nullptr;
        if (id != 1 || !uiElement_) return E_INVALIDARG;
        return uiElement_.CopyTo(element);
    }
    HRESULT STDMETHODCALLTYPE EnumUIElements(IEnumTfUIElements**) noexcept override {
        return E_NOTIMPL;
    }

    [[nodiscard]] bool lifecycleSinksAdvised() const noexcept {
        return threadManagerEventSink_ && threadFocusSink_ && activeProfileSink_;
    }
    [[nodiscard]] bool lifecycleSinksReleased() const noexcept {
        return !threadManagerEventSink_ && !threadFocusSink_ && !activeProfileSink_;
    }
    HRESULT simulateDocumentFocusLost(ITfContext* context) noexcept {
        if (!threadManagerEventSink_) return E_UNEXPECTED;
        TestDocumentManager previous(context);
        return threadManagerEventSink_->OnSetFocus(nullptr, &previous);
    }
    void setBeginShow(BOOL show) noexcept { beginShow_ = show; }
    HRESULT simulateActiveProfile(REFGUID profileGuid) noexcept {
        if (!activeProfileSink_) return E_UNEXPECTED;
        return activeProfileSink_->OnActivated(
            fcitx::windows::tsf::kTextServiceClsid, profileGuid, TRUE);
    }
    [[nodiscard]] DWORD uiBeginCount() const noexcept { return uiBeginCount_; }
    [[nodiscard]] DWORD uiEndCount() const noexcept { return uiEndCount_; }
    [[nodiscard]] bool hiddenCandidateSemanticsAvailable() const noexcept {
        ComPtr<ITfCandidateListUIElement> candidates;
        if (!uiElement_ || FAILED(uiElement_.As(&candidates))) return false;
        UINT count = 0;
        UINT selection = UINT_MAX;
        BOOL shown = TRUE;
        BSTR text = nullptr;
        const bool ok = SUCCEEDED(candidates->GetCount(&count)) && count != 0 &&
                        SUCCEEDED(candidates->GetSelection(&selection)) &&
                        selection < count &&
                        SUCCEEDED(candidates->IsShown(&shown)) && !shown &&
                        SUCCEEDED(candidates->GetString(selection, &text)) && text &&
                        SysStringLen(text) != 0;
        SysFreeString(text);
        return ok;
    }

private:
    std::atomic<ULONG> references_{1};
    bool advised_{};
    BOOL beginShow_{TRUE};
    DWORD uiBeginCount_{};
    DWORD uiUpdateCount_{};
    DWORD uiEndCount_{};
    ComPtr<ITfThreadMgrEventSink> threadManagerEventSink_;
    ComPtr<ITfThreadFocusSink> threadFocusSink_;
    ComPtr<ITfActiveLanguageProfileNotifySink> activeProfileSink_;
    ComPtr<ITfUIElement> uiElement_;
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
    std::wstring command = quote(executable) +
                           L" --test-clients 2 --composition-test --ready-event " +
                           quote(eventName);
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

int exercise(const wchar_t* dllPath, HANDLE engineProcess) {
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
    bool lifecycleSinksAdvised = false;
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
        lifecycleSinksAdvised = threadManager.lifecycleSinksAdvised();
        threadManager.setBeginShow(FALSE);
        TestContext passwordContext(IS_PASSWORD);
        BOOL passwordTestEaten = TRUE;
        BOOL passwordEaten = TRUE;
        const HRESULT passwordTest =
            keySink->OnTestKeyDown(&passwordContext, 'P', 0, &passwordTestEaten);
        const HRESULT passwordResult =
            keySink->OnKeyDown(&passwordContext, 'P', 0, &passwordEaten);
        if (FAILED(passwordTest) || FAILED(passwordResult) || passwordTestEaten ||
            passwordEaten || !passwordContext.text().empty()) {
            std::cerr << "password input scope was not passed through\n";
        } else {
            TestContext context;
            const HRESULT activeProfile = threadManager.simulateActiveProfile(
                fcitx::windows::tsf::kInputProfiles[0].guid);
            BOOL preeditTestEaten = FALSE;
            BOOL duplicatePreeditTestEaten = FALSE;
            BOOL preeditEaten = FALSE;
            const HRESULT preeditTest =
                keySink->OnTestKeyDown(&context, 'N', 0, &preeditTestEaten);
            const HRESULT duplicatePreeditTest =
                keySink->OnTestKeyDown(&context, 'N', 0, &duplicatePreeditTestEaten);
            const HRESULT preeditResult =
                keySink->OnKeyDown(&context, 'N', 0, &preeditEaten);
            const bool hiddenUiElementAfterPreedit =
                threadManager.hiddenCandidateSemanticsAvailable();
            const HRESULT focusLost = threadManager.simulateDocumentFocusLost(&context);
            const bool focusLossClearedComposition =
                SUCCEEDED(focusLost) && context.compositionEnded() && context.text().empty();
            BOOL resumedPreeditTestEaten = FALSE;
            BOOL resumedPreeditEaten = FALSE;
            const HRESULT resumedPreeditTest =
                keySink->OnTestKeyDown(&context, 'N', 0, &resumedPreeditTestEaten);
            const HRESULT resumedPreeditResult =
                keySink->OnKeyDown(&context, 'N', 0, &resumedPreeditEaten);
            BOOL commitTestEaten = FALSE;
            BOOL duplicateCommitTestEaten = FALSE;
            BOOL commitEaten = FALSE;
            const HRESULT commitTest =
                keySink->OnTestKeyDown(&context, VK_SPACE, 0, &commitTestEaten);
            const HRESULT duplicateCommitTest =
                keySink->OnTestKeyDown(&context, VK_SPACE, 0, &duplicateCommitTestEaten);
            const HRESULT commitResult =
                keySink->OnKeyDown(&context, VK_SPACE, 0, &commitEaten);
            const bool committedChinese = context.text() == L"\x4f60";
            // Idle editor/navigation keys belong to the host application. Fcitx
            // only receives keys that start or continue real IME state, plus
            // explicit IME hotkeys.
            BOOL punctuationTestEaten = FALSE;
            BOOL enterTestEaten = TRUE;
            BOOL leftTestEaten = TRUE;
            BOOL backspaceTestEaten = TRUE;
            BOOL modifierTestEaten = FALSE;
            BOOL liveKeyUpEaten = TRUE;
            const HRESULT punctuationTest =
                keySink->OnTestKeyDown(&context, VK_OEM_COMMA, 0, &punctuationTestEaten);
            const HRESULT enterTest =
                keySink->OnTestKeyDown(&context, VK_RETURN, 0, &enterTestEaten);
            const HRESULT leftTest =
                keySink->OnTestKeyDown(&context, VK_LEFT, 0, &leftTestEaten);
            const HRESULT backspaceTest =
                keySink->OnTestKeyDown(&context, VK_BACK, 0, &backspaceTestEaten);
            const HRESULT modifierTest =
                keySink->OnTestKeyDown(&context, VK_SHIFT, 0, &modifierTestEaten);
            const HRESULT liveKeyUp =
                keySink->OnKeyUp(&context, VK_SHIFT, 0, &liveKeyUpEaten);
            BOOL orphanTestEaten = FALSE;
            BOOL orphanKeyUpEaten = TRUE;
            const HRESULT orphanTest =
                keySink->OnTestKeyDown(&context, 'A', 0, &orphanTestEaten);
            const bool engineAlreadyStopped =
                WaitForSingleObject(engineProcess, 0) == WAIT_OBJECT_0;
            if (!engineAlreadyStopped) (void)TerminateProcess(engineProcess, 0);
            const bool engineStopped =
                engineAlreadyStopped || WaitForSingleObject(engineProcess, 2000) == WAIT_OBJECT_0;
            BOOL fallbackEaten = FALSE;
            const HRESULT fallbackResult =
                keySink->OnKeyDown(&context, 'A', 0, &fallbackEaten);
            const HRESULT orphanKeyUp =
                keySink->OnKeyUp(&context, 'A', 0, &orphanKeyUpEaten);
            if (SUCCEEDED(activeProfile) &&
                SUCCEEDED(preeditTest) && SUCCEEDED(preeditResult) &&
                SUCCEEDED(duplicatePreeditTest) && SUCCEEDED(commitTest) &&
                SUCCEEDED(punctuationTest) && SUCCEEDED(enterTest) &&
                SUCCEEDED(leftTest) && SUCCEEDED(backspaceTest) &&
                SUCCEEDED(modifierTest) && SUCCEEDED(liveKeyUp) &&
                SUCCEEDED(resumedPreeditTest) && SUCCEEDED(resumedPreeditResult) &&
                SUCCEEDED(duplicateCommitTest) && SUCCEEDED(commitResult) &&
                SUCCEEDED(orphanTest) && SUCCEEDED(fallbackResult) &&
                SUCCEEDED(orphanKeyUp) && engineStopped && committedChinese &&
                preeditTestEaten && duplicatePreeditTestEaten && preeditEaten &&
                resumedPreeditTestEaten && resumedPreeditEaten &&
                commitTestEaten && duplicateCommitTestEaten && commitEaten &&
                !punctuationTestEaten && !enterTestEaten && !leftTestEaten &&
                !backspaceTestEaten && !modifierTestEaten && !liveKeyUpEaten &&
                orphanTestEaten && fallbackEaten && !orphanKeyUpEaten &&
                context.text() == L"a" && context.compositionEnded() &&
                lifecycleSinksAdvised && focusLossClearedComposition &&
                hiddenUiElementAfterPreedit && threadManager.uiBeginCount() >= 2 &&
                threadManager.uiEndCount() >= 1) {
                result = 0;
            } else {
                std::cerr << "composition path failed: preedit=0x" << std::hex
                          << preeditResult << ", commit=0x" << commitResult
                          << ", activeProfile=0x" << activeProfile
                          << ", preeditTest=0x" << preeditTest
                          << ", duplicatePreeditTest=0x" << duplicatePreeditTest
                          << ", resumedPreeditTest=0x" << resumedPreeditTest
                          << ", resumedPreedit=0x" << resumedPreeditResult
                          << ", commitTest=0x" << commitTest
                          << ", duplicateCommitTest=0x" << duplicateCommitTest
                          << ", punctuationTest=0x" << punctuationTest
                          << ", enterTest=0x" << enterTest
                          << ", leftTest=0x" << leftTest
                          << ", backspaceTest=0x" << backspaceTest
                          << ", modifierTest=0x" << modifierTest
                          << ", liveKeyUp=0x" << liveKeyUp
                          << ", orphanTest=0x" << orphanTest
                          << ", fallback=0x" << fallbackResult
                          << ", orphanKeyUp=0x" << orphanKeyUp
                          << ", preeditEaten=" << std::dec << preeditEaten
                          << ", preeditTestEaten=" << preeditTestEaten
                          << ", duplicatePreeditTestEaten=" << duplicatePreeditTestEaten
                          << ", resumedPreeditTestEaten=" << resumedPreeditTestEaten
                          << ", resumedPreeditEaten=" << resumedPreeditEaten
                          << ", commitEaten=" << commitEaten
                          << ", commitTestEaten=" << commitTestEaten
                          << ", duplicateCommitTestEaten=" << duplicateCommitTestEaten
                          << ", punctuationTestEaten=" << punctuationTestEaten
                          << ", enterTestEaten=" << enterTestEaten
                          << ", leftTestEaten=" << leftTestEaten
                          << ", backspaceTestEaten=" << backspaceTestEaten
                          << ", modifierTestEaten=" << modifierTestEaten
                          << ", liveKeyUpEaten=" << liveKeyUpEaten
                          << ", orphanTestEaten=" << orphanTestEaten
                          << ", orphanKeyUpEaten=" << orphanKeyUpEaten
                          << ", fallbackEaten=" << fallbackEaten
                          << ", engineStopped=" << engineStopped
                          << ", committedChinese=" << committedChinese
                          << ", focusLossCleared=" << focusLossClearedComposition
                          << ", lifecycleSinksAdvised=" << lifecycleSinksAdvised
                          << ", hiddenUiElement=" << hiddenUiElementAfterPreedit
                          << ", uiBegin=" << threadManager.uiBeginCount()
                          << ", uiEnd=" << threadManager.uiEndCount()
                          << ", started=" << context.compositionStarted()
                          << ", ended=" << context.compositionEnded()
                          << ", text=0x" << std::hex
                          << (context.text().empty()
                                  ? 0U
                                  : static_cast<unsigned>(context.text()[0]))
                          << ", textLength=" << context.text().size() << '\n';
            }
        }
    }
    if (activated) {
        const HRESULT deactivated = service->Deactivate();
        if (FAILED(deactivated) || !threadManager.lifecycleSinksReleased()) result = 1;
    }
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
    const std::wstring testNamespace = L"tsf-" + std::to_wstring(GetCurrentProcessId());
    if (!SetEnvironmentVariableW(L"FCITX5_TEST_NAMESPACE", testNamespace.c_str())) return 1;
    if (!SetEnvironmentVariableW(L"FCITX5_TEST_ENGINE_PATH", argv[2])) return 1;
    const HRESULT initialized = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    if (FAILED(initialized)) return 1;
    EngineProcess engine = startEngine(argv[2]);
    if (!engine.process) std::cerr << "mock engine process unavailable\n";
    int result = engine.process && engine.ready ? exercise(argv[1], engine.process) : 1;
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
