#include "guids.h"

#include <Windows.h>
#include <msctf.h>
#include <objbase.h>
#include <wrl/client.h>

#include <cstdint>
#include <iostream>

namespace {

struct Resources {
    DWORD handles{};
    DWORD gdi{};
    DWORD user{};
};

Resources resources() {
    Resources result;
    (void)GetProcessHandleCount(GetCurrentProcess(), &result.handles);
    result.gdi = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
    result.user = GetGuiResources(GetCurrentProcess(), GR_USEROBJECTS);
    return result;
}

bool exercise(const wchar_t* path) {
    HMODULE module = LoadLibraryExW(path, nullptr, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR |
                                                      LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!module) return false;
    using GetClassObject = HRESULT(STDAPICALLTYPE*)(REFCLSID, REFIID, void**);
    using CanUnloadNow = HRESULT(STDAPICALLTYPE*)();
    const auto getClassObject = reinterpret_cast<GetClassObject>(
        GetProcAddress(module, "DllGetClassObject"));
    const auto canUnload = reinterpret_cast<CanUnloadNow>(
        GetProcAddress(module, "DllCanUnloadNow"));
    Microsoft::WRL::ComPtr<IClassFactory> factory;
    Microsoft::WRL::ComPtr<ITfTextInputProcessorEx> service;
    bool okay = getClassObject && canUnload &&
                SUCCEEDED(getClassObject(fcitx::windows::tsf::kTextServiceClsid,
                                         IID_PPV_ARGS(&factory))) &&
                SUCCEEDED(factory->CreateInstance(nullptr, IID_PPV_ARGS(&service)));
    service.Reset();
    factory.Reset();
    okay = okay && canUnload() == S_OK;
    return FreeLibrary(module) != FALSE && okay;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) return 1;
    const HRESULT initialized = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    if (FAILED(initialized)) return 1;
    constexpr std::uint64_t iterations = 10'000;
    if (!exercise(argv[1])) {
        CoUninitialize();
        return 1;
    }
    const Resources before = resources();
    for (std::uint64_t iteration = 0; iteration < iterations; ++iteration) {
        if (!exercise(argv[1])) {
            CoUninitialize();
            return 1;
        }
    }
    const Resources after = resources();
    CoUninitialize();
    const auto handleDelta = static_cast<std::int64_t>(after.handles) - before.handles;
    const auto gdiDelta = static_cast<std::int64_t>(after.gdi) - before.gdi;
    const auto userDelta = static_cast<std::int64_t>(after.user) - before.user;
    std::cout << "handle-leak-soak iterations=" << iterations
              << " handle-delta=" << handleDelta << " gdi-delta=" << gdiDelta
              << " user-delta=" << userDelta << '\n';
    return handleDelta > 2 || gdiDelta > 2 || userDelta > 2 ? 1 : 0;
}
