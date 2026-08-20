#include "guids.h"
#include "input_profiles.h"
#include "module.h"

#include <Windows.h>
#include <msctf.h>
#include <objbase.h>

#include <array>
#include <cwchar>
#include <iterator>
#include <new>
#include <string>

namespace fcitx::windows::tsf {
namespace {

std::wstring guidString(REFGUID guid) {
    std::array<wchar_t, 40> buffer{};
    if (StringFromGUID2(guid, buffer.data(), static_cast<int>(buffer.size())) == 0) {
        return {};
    }
    return buffer.data();
}

HRESULT setStringValue(HKEY root, const std::wstring& path, const wchar_t* name,
                       const std::wstring& value) {
    HKEY key = nullptr;
    const LSTATUS createResult =
        RegCreateKeyExW(root, path.c_str(), 0, nullptr, 0, KEY_WRITE, nullptr, &key, nullptr);
    if (createResult != ERROR_SUCCESS) {
        return HRESULT_FROM_WIN32(createResult);
    }
    const LSTATUS setResult =
        RegSetValueExW(key, name, 0, REG_SZ, reinterpret_cast<const BYTE*>(value.c_str()),
                       static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t)));
    RegCloseKey(key);
    return HRESULT_FROM_WIN32(setResult);
}

std::wstring currentModulePath() {
    std::wstring modulePath(32768, L'\0');
    const DWORD length = GetModuleFileNameW(moduleHandle(), modulePath.data(),
                                            static_cast<DWORD>(modulePath.size()));
    if (length == 0 || length == modulePath.size()) {
        return {};
    }
    modulePath.resize(length);
    return modulePath;
}

HRESULT registerComServer() {
    const std::wstring modulePath = currentModulePath();
    if (modulePath.empty()) {
        return HRESULT_FROM_WIN32(GetLastError());
    }
    const std::wstring classPath = L"Software\\Classes\\CLSID\\" + guidString(kTextServiceClsid);
    HRESULT result = setStringValue(HKEY_LOCAL_MACHINE, classPath, nullptr, kServiceDescription);
    if (FAILED(result)) {
        return result;
    }
    const std::wstring serverPath = classPath + L"\\InprocServer32";
    result = setStringValue(HKEY_LOCAL_MACHINE, serverPath, nullptr, modulePath);
    if (SUCCEEDED(result)) {
        result = setStringValue(HKEY_LOCAL_MACHINE, serverPath, L"ThreadingModel", L"Apartment");
    }
    return result;
}

HRESULT unregisterProfileIfPresent(ITfInputProcessorProfileMgr* profiles, LANGID language,
                                   REFGUID profileGuid) {
    TF_INPUTPROCESSORPROFILE profile{};
    const HRESULT queryResult = profiles->GetProfile(
        TF_PROFILETYPE_INPUTPROCESSOR, language, kTextServiceClsid, profileGuid, nullptr,
        &profile);
    if (FAILED(queryResult)) {
        return S_OK;
    }

    const HRESULT unregisterResult =
        profiles->UnregisterProfile(kTextServiceClsid, language, profileGuid, 0);
    if (FAILED(unregisterResult)) {
        return unregisterResult;
    }

    TF_INPUTPROCESSORPROFILE remaining{};
    return SUCCEEDED(profiles->GetProfile(TF_PROFILETYPE_INPUTPROCESSOR, language,
                                          kTextServiceClsid, profileGuid, nullptr, &remaining))
               ? E_FAIL
               : S_OK;
}

HRESULT unregisterComServer() {
    const std::wstring classPath = L"Software\\Classes\\CLSID\\" + guidString(kTextServiceClsid);
    const LSTATUS result = RegDeleteTreeW(HKEY_LOCAL_MACHINE, classPath.c_str());
    return result == ERROR_SUCCESS || result == ERROR_FILE_NOT_FOUND ? S_OK
                                                                     : HRESULT_FROM_WIN32(result);
}

HRESULT registerProfiles() {
    const std::wstring modulePath = currentModulePath();
    if (modulePath.empty()) {
        return HRESULT_FROM_WIN32(GetLastError());
    }
    ITfInputProcessorProfileMgr* profiles = nullptr;
    HRESULT result =
        CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
                         IID_ITfInputProcessorProfileMgr, reinterpret_cast<void**>(&profiles));
    if (FAILED(result)) {
        return result;
    }
    for (const auto& profile : kObsoleteInputProfiles) {
        result = unregisterProfileIfPresent(profiles, profile.language, profile.guid);
        if (FAILED(result)) {
            break;
        }
    }
    for (const auto& profile : kInputProfiles) {
        result = unregisterProfileIfPresent(profiles, profile.language, profile.guid);
        if (FAILED(result)) {
            break;
        }
    }
    const auto ledger = loadDynamicProfileLedger();
    for (const auto& profile : ledger) {
        result = unregisterProfileIfPresent(profiles, profile.language, profile.guid);
        if (FAILED(result)) {
            break;
        }
    }
    const auto inputProfiles = loadRegistrableInputProfiles();
    for (const auto& profile : inputProfiles) {
        if (FAILED(result)) {
            break;
        }
        result = profiles->RegisterProfile(
            kTextServiceClsid, profile.language, profile.guid, profile.description.c_str(),
            static_cast<ULONG>(profile.description.size()), modulePath.c_str(),
            static_cast<ULONG>(modulePath.size()), 0, nullptr, 0, TRUE, 0);
    }
    profiles->Release();
    if (FAILED(result)) {
        return result;
    }
    (void)storeDynamicProfileLedger(inputProfiles);

    ITfCategoryMgr* categories = nullptr;
    result = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER,
                              IID_ITfCategoryMgr, reinterpret_cast<void**>(&categories));
    if (SUCCEEDED(result)) {
        result = categories->RegisterCategory(kTextServiceClsid, GUID_TFCAT_TIP_KEYBOARD,
                                              kTextServiceClsid);
        categories->Release();
    }
    return result;
}

HRESULT unregisterProfiles() {
    ITfCategoryMgr* categories = nullptr;
    if (SUCCEEDED(CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER,
                                   IID_ITfCategoryMgr, reinterpret_cast<void**>(&categories)))) {
        categories->UnregisterCategory(kTextServiceClsid, GUID_TFCAT_TIP_KEYBOARD,
                                       kTextServiceClsid);
        categories->Release();
    }

    ITfInputProcessorProfileMgr* profiles = nullptr;
    HRESULT result =
        CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER,
                         IID_ITfInputProcessorProfileMgr, reinterpret_cast<void**>(&profiles));
    if (FAILED(result)) {
        return result;
    }
    for (const auto& profile : loadRegistrableInputProfiles()) {
        const HRESULT profileResult =
            unregisterProfileIfPresent(profiles, profile.language, profile.guid);
        if (FAILED(profileResult) && SUCCEEDED(result))
            result = profileResult;
    }
    for (const auto& profile : loadDynamicProfileLedger()) {
        const HRESULT profileResult =
            unregisterProfileIfPresent(profiles, profile.language, profile.guid);
        if (FAILED(profileResult) && SUCCEEDED(result))
            result = profileResult;
    }
    for (const auto& profile : kObsoleteInputProfiles) {
        const HRESULT profileResult =
            unregisterProfileIfPresent(profiles, profile.language, profile.guid);
        if (FAILED(profileResult) && SUCCEEDED(result))
            result = profileResult;
    }
    profiles->Release();
    return result;
}

} // namespace

HRESULT registerTextService() noexcept {
    bool uninitialize = false;
    try {
        const HRESULT initializeResult = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
        if (FAILED(initializeResult) && initializeResult != RPC_E_CHANGED_MODE) {
            return initializeResult;
        }
        uninitialize = SUCCEEDED(initializeResult);
        HRESULT result = registerComServer();
        if (SUCCEEDED(result)) {
            result = registerProfiles();
        }
        if (FAILED(result)) {
            unregisterProfiles();
            unregisterComServer();
        }
        if (uninitialize) {
            CoUninitialize();
            uninitialize = false;
        }
        return result;
    } catch (const std::bad_alloc&) {
        try {
            unregisterProfiles();
            unregisterComServer();
        } catch (...) {
        }
        if (uninitialize) {
            CoUninitialize();
        }
        return E_OUTOFMEMORY;
    } catch (...) {
        try {
            unregisterProfiles();
            unregisterComServer();
        } catch (...) {
        }
        if (uninitialize) {
            CoUninitialize();
        }
        return E_UNEXPECTED;
    }
}

HRESULT unregisterTextService() noexcept {
    bool uninitialize = false;
    try {
        const HRESULT initializeResult = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
        if (FAILED(initializeResult) && initializeResult != RPC_E_CHANGED_MODE) {
            return initializeResult;
        }
        uninitialize = SUCCEEDED(initializeResult);
        const HRESULT profileResult = unregisterProfiles();
        const HRESULT comResult = unregisterComServer();
        if (uninitialize) {
            CoUninitialize();
            uninitialize = false;
        }
        return FAILED(profileResult) ? profileResult : comResult;
    } catch (const std::bad_alloc&) {
        if (uninitialize) {
            CoUninitialize();
        }
        return E_OUTOFMEMORY;
    } catch (...) {
        if (uninitialize) {
            CoUninitialize();
        }
        return E_UNEXPECTED;
    }
}

} // namespace fcitx::windows::tsf

STDAPI DllRegisterServer() { return fcitx::windows::tsf::registerTextService(); }

STDAPI DllUnregisterServer() { return fcitx::windows::tsf::unregisterTextService(); }
