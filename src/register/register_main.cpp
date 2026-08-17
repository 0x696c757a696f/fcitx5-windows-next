#include "guids.h"
#include "fcitx5_windows/version.h"

#include <Windows.h>
#include <objbase.h>

#include <array>
#include <filesystem>
#include <iostream>
#include <string>
#include <string_view>

namespace {

namespace fs = std::filesystem;
using RegisterFunction = HRESULT(STDAPICALLTYPE*)();

std::wstring guidString(REFGUID guid) {
    std::array<wchar_t, 40> buffer{};
    return StringFromGUID2(guid, buffer.data(), static_cast<int>(buffer.size())) == 0
               ? std::wstring{}
               : std::wstring(buffer.data());
}

bool isElevated() noexcept {
    BOOL administrator = FALSE;
    SID_IDENTIFIER_AUTHORITY authority = SECURITY_NT_AUTHORITY;
    PSID sid = nullptr;
    if (!AllocateAndInitializeSid(&authority, 2, SECURITY_BUILTIN_DOMAIN_RID,
                                  DOMAIN_ALIAS_RID_ADMINS, 0, 0, 0, 0, 0, 0, &sid)) {
        return false;
    }
    const BOOL checked = CheckTokenMembership(nullptr, sid, &administrator);
    FreeSid(sid);
    return checked && administrator;
}

std::wstring registeredPath() {
    const std::wstring keyPath = L"Software\\Classes\\CLSID\\" +
                                 guidString(fcitx::windows::tsf::kTextServiceClsid) +
                                 L"\\InprocServer32";
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_LOCAL_MACHINE, keyPath.c_str(), 0, KEY_QUERY_VALUE, &key) !=
        ERROR_SUCCESS) return {};
    DWORD type = 0;
    DWORD bytes = 0;
    if (RegQueryValueExW(key, nullptr, nullptr, &type, nullptr, &bytes) != ERROR_SUCCESS ||
        type != REG_SZ || bytes < sizeof(wchar_t)) {
        RegCloseKey(key);
        return {};
    }
    std::wstring value(bytes / sizeof(wchar_t), L'\0');
    const LSTATUS result = RegQueryValueExW(key, nullptr, nullptr, &type,
                                            reinterpret_cast<BYTE*>(value.data()), &bytes);
    RegCloseKey(key);
    if (result != ERROR_SUCCESS) return {};
    while (!value.empty() && value.back() == L'\0') value.pop_back();
    return value;
}

bool samePath(const fs::path& first, const fs::path& second) {
    std::error_code error;
    const fs::path normalizedFirst = fs::weakly_canonical(first, error);
    if (error) return false;
    const fs::path normalizedSecond = fs::weakly_canonical(second, error);
    return !error && CompareStringOrdinal(normalizedFirst.c_str(), -1,
                                          normalizedSecond.c_str(), -1, TRUE) == CSTR_EQUAL;
}

HRESULT invokeRegistration(const fs::path& dll, const char* exportName) {
    SetDllDirectoryW(L"");
    HMODULE module = LoadLibraryExW(dll.c_str(), nullptr, LOAD_WITH_ALTERED_SEARCH_PATH);
    if (!module) return HRESULT_FROM_WIN32(GetLastError());
    const auto function = reinterpret_cast<RegisterFunction>(GetProcAddress(module, exportName));
    const HRESULT result = function ? function() : HRESULT_FROM_WIN32(ERROR_PROC_NOT_FOUND);
    FreeLibrary(module);
    return result;
}

void usage() {
    std::wcerr << L"Usage: fcitx5-register "
                  L"--register|--unregister|--repair|--status --dll ABSOLUTE_PATH\n";
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc == 2 && std::wstring_view(argv[1]) == L"--version") {
        std::cout << fcitx::windows::version() << '\n';
        return 0;
    }
    if (argc != 4 || std::wstring_view(argv[2]) != L"--dll") {
        usage();
        return 2;
    }
    const std::wstring_view operation(argv[1]);
    const fs::path dll(argv[3]);
    if (!dll.is_absolute() || dll.filename() != L"fcitx5-tsf.dll") {
        std::wcerr << L"The TSF DLL must be an absolute path ending in fcitx5-tsf.dll.\n";
        return 2;
    }
    if (operation == L"--status") {
        const std::wstring actual = registeredPath();
        if (actual.empty()) {
            std::cout << "not_registered\n";
            return 3;
        }
        if (!samePath(actual, dll)) {
            std::wcout << L"path_mismatch " << actual << L'\n';
            return 3;
        }
        std::cout << "registered\n";
        return 0;
    }
    if (operation != L"--register" && operation != L"--repair" &&
        operation != L"--unregister") {
        usage();
        return 2;
    }
    if (!isElevated()) {
        std::wcerr << L"Registration changes require an elevated administrator token.\n";
        return 5;
    }
    if (!fs::exists(dll) || !fs::is_regular_file(dll)) {
        std::wcerr << L"TSF DLL does not exist: " << dll.c_str() << L'\n';
        return 2;
    }
    const char* function = operation == L"--unregister" ? "DllUnregisterServer"
                                                         : "DllRegisterServer";
    const HRESULT result = invokeRegistration(dll, function);
    if (FAILED(result)) {
        std::wcerr << L"Registration operation failed: 0x" << std::hex
                   << static_cast<unsigned long>(result) << L'\n';
        return 6;
    }
    if (operation != L"--unregister" && !samePath(registeredPath(), dll)) {
        std::wcerr << L"Registration completed but the registered path does not match.\n";
        return 6;
    }
    return 0;
}
