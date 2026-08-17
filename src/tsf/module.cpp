#include "guids.h"
#include "module.h"
#include "text_service.h"

#include <Windows.h>
#include <objbase.h>

#include <atomic>
#include <new>

namespace fcitx::windows::tsf {
namespace {

HMODULE globalModule{};
std::atomic<long> globalReferences{};

class ClassFactory final : public IClassFactory {
public:
    ClassFactory() noexcept { moduleAddRef(); }

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID interfaceId, void** object) noexcept override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;
        if (IsEqualIID(interfaceId, IID_IUnknown) ||
            IsEqualIID(interfaceId, IID_IClassFactory)) {
            *object = static_cast<IClassFactory*>(this);
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

    HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown* outer, REFIID interfaceId,
                                             void** object) noexcept override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;
        if (outer) {
            return CLASS_E_NOAGGREGATION;
        }
        try {
            auto* service = new (std::nothrow) TextService();
            if (!service) {
                return E_OUTOFMEMORY;
            }
            const HRESULT result = service->QueryInterface(interfaceId, object);
            service->Release();
            return result;
        } catch (const std::bad_alloc&) {
            return E_OUTOFMEMORY;
        } catch (...) {
            return E_UNEXPECTED;
        }
    }

    HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) noexcept override {
        if (lock) {
            moduleAddRef();
        } else {
            moduleRelease();
        }
        return S_OK;
    }

private:
    ~ClassFactory() { moduleRelease(); }

    std::atomic<ULONG> referenceCount_{1};
};

} // namespace

void moduleAddRef() noexcept { globalReferences.fetch_add(1, std::memory_order_relaxed); }

void moduleRelease() noexcept { globalReferences.fetch_sub(1, std::memory_order_relaxed); }

long moduleReferenceCount() noexcept { return globalReferences.load(std::memory_order_acquire); }

HMODULE moduleHandle() noexcept { return globalModule; }

} // namespace fcitx::windows::tsf

extern "C" BOOL WINAPI DllMain(HINSTANCE instance, DWORD reason, void*) {
    if (reason == DLL_PROCESS_ATTACH) {
        fcitx::windows::tsf::globalModule = instance;
        DisableThreadLibraryCalls(instance);
    }
    return TRUE;
}

_Check_return_ STDAPI DllGetClassObject(_In_ REFCLSID classId, _In_ REFIID interfaceId,
                                        _Outptr_ LPVOID FAR* object) {
    if (!object) {
        return E_POINTER;
    }
    *object = nullptr;
    if (!IsEqualCLSID(classId, fcitx::windows::tsf::kTextServiceClsid)) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    auto* factory = new (std::nothrow) fcitx::windows::tsf::ClassFactory();
    if (!factory) {
        return E_OUTOFMEMORY;
    }
    const HRESULT result = factory->QueryInterface(interfaceId, object);
    factory->Release();
    return result;
}

__control_entrypoint(DllExport) STDAPI DllCanUnloadNow(void) {
    return fcitx::windows::tsf::moduleReferenceCount() == 0 ? S_OK : S_FALSE;
}
