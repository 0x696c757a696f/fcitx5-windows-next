#include "guids.h"
#include "fcitx5_windows/version.h"

#include <Windows.h>
#include <objbase.h>

#include <array>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <iostream>
#include <string>
#include <string_view>

namespace {

namespace fs = std::filesystem;

extern "C" {
struct Fcitx5RegisterUtf16 {
    const wchar_t* ptr;
    std::size_t len;
};

std::uint32_t fcitx5_register_validate_artifact(Fcitx5RegisterUtf16 helper,
                                                Fcitx5RegisterUtf16 dll,
                                                std::uint32_t architectureBits) noexcept;
std::uint32_t fcitx5_register_parse_operation(Fcitx5RegisterUtf16 operation) noexcept;
std::uint32_t fcitx5_register_validate_dll_argument(Fcitx5RegisterUtf16 dll) noexcept;
std::uint32_t fcitx5_register_operation_requires_admin(std::uint32_t operation) noexcept;
std::uint32_t fcitx5_register_operation_export(std::uint32_t operation) noexcept;
std::uint32_t fcitx5_register_registration_status_for_dll(Fcitx5RegisterUtf16 dll) noexcept;
std::uint32_t fcitx5_register_is_elevated() noexcept;
HRESULT fcitx5_register_invoke_registration_export(Fcitx5RegisterUtf16 dll,
                                                   std::uint32_t exportKind) noexcept;
}

enum class RegisterArtifactStatus : std::uint32_t {
    ok = 0,
    invalidArgument = 1,
    helperLocation = 2,
    currentDllMissing = 3,
    pairedDllMissing = 4,
    dllOutsideProduct = 5,
};

enum class RegisterOperation : std::uint32_t {
    unknown = 0,
    registerServer = 1,
    repair = 2,
    unregisterServer = 3,
    status = 4,
    validateArtifact = 5,
};

enum class RegisterDllArgumentStatus : std::uint32_t {
    ok = 0,
    invalid = 1,
};

enum class RegisterExport : std::uint32_t {
    none = 0,
    registerServer = 1,
    unregisterServer = 2,
};

enum class RegisterStatus : std::uint32_t {
    registered = 0,
    notRegistered = 1,
    pathMismatch = 2,
    invalidArgument = 3,
};

fs::path executablePath() {
    std::wstring value(32'768, L'\0');
    const DWORD size = GetModuleFileNameW(nullptr, value.data(), static_cast<DWORD>(value.size()));
    if (size == 0 || size >= value.size())
        return {};
    value.resize(size);
    return value;
}

std::wstring guidString(REFGUID guid) {
    std::array<wchar_t, 40> buffer{};
    return StringFromGUID2(guid, buffer.data(), static_cast<int>(buffer.size())) == 0
               ? std::wstring{}
               : std::wstring(buffer.data());
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

constexpr std::uint32_t currentArchitectureBits() noexcept {
#if defined(_WIN64)
    return 64;
#else
    return 32;
#endif
}

bool validateProductArtifact(const fs::path& dll, std::wstring& error) {
    error.clear();
    const fs::path executable = executablePath();
    if (executable.empty()) {
        error = L"register helper path could not be resolved";
        return false;
    }
    const auto helperNative = executable.native();
    const auto dllNative = dll.native();
    const auto status = static_cast<RegisterArtifactStatus>(
        fcitx5_register_validate_artifact(
            {helperNative.data(), helperNative.size()},
            {dllNative.data(), dllNative.size()},
            currentArchitectureBits()));
    switch (status) {
    case RegisterArtifactStatus::ok:
        return true;
    case RegisterArtifactStatus::helperLocation:
        error = L"register helper is not running from the product bin directory";
        return false;
    case RegisterArtifactStatus::currentDllMissing:
        error = L"current architecture TSF DLL is missing from the product artifact";
        return false;
    case RegisterArtifactStatus::pairedDllMissing:
        error = L"paired architecture TSF DLL is missing from the product artifact";
        return false;
    case RegisterArtifactStatus::dllOutsideProduct:
        error = L"TSF DLL path does not belong to this product artifact";
        return false;
    case RegisterArtifactStatus::invalidArgument:
    default:
        error = L"register artifact validation failed";
        return false;
    }
}

RegisterOperation parseOperation(std::wstring_view value) noexcept {
    return static_cast<RegisterOperation>(
        fcitx5_register_parse_operation({value.data(), value.size()}));
}

bool validateDllArgument(const fs::path& dll) noexcept {
    const auto native = dll.native();
    return static_cast<RegisterDllArgumentStatus>(
               fcitx5_register_validate_dll_argument({native.data(), native.size()})) ==
           RegisterDllArgumentStatus::ok;
}

bool operationRequiresAdmin(RegisterOperation operation) noexcept {
    return fcitx5_register_operation_requires_admin(static_cast<std::uint32_t>(operation)) != 0;
}

RegisterExport operationExport(RegisterOperation operation) noexcept {
    return static_cast<RegisterExport>(
        fcitx5_register_operation_export(static_cast<std::uint32_t>(operation)));
}

RegisterStatus registrationStatusForDll(const fs::path& dll) noexcept {
    const auto native = dll.native();
    return static_cast<RegisterStatus>(
        fcitx5_register_registration_status_for_dll({native.data(), native.size()}));
}

void usage() {
    std::wcerr << L"Usage: fcitx5-register "
                  L"--register|--unregister|--repair|--status|--validate-artifact "
                  L"--dll ABSOLUTE_PATH\n";
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
    const auto operation = parseOperation(argv[1]);
    const fs::path dll(argv[3]);
    if (!validateDllArgument(dll)) {
        std::wcerr << L"The TSF DLL must be an absolute path ending in fcitx5-tsf.dll.\n";
        return 2;
    }
    std::wstring validationError;
    if (!validateProductArtifact(dll, validationError)) {
        std::wcerr << validationError << L'\n';
        return 2;
    }
    if (operation == RegisterOperation::validateArtifact) {
        std::cout << "artifact_valid\n";
        return 0;
    }
    if (operation == RegisterOperation::status) {
        const auto status = registrationStatusForDll(dll);
        if (status == RegisterStatus::registered) {
            std::cout << "registered\n";
            return 0;
        }
        if (status == RegisterStatus::notRegistered) {
            std::cout << "not_registered\n";
            return 3;
        }
        const std::wstring actual = registeredPath();
        std::wcout << L"path_mismatch " << actual << L'\n';
        return 3;
    }
    if (operation != RegisterOperation::registerServer && operation != RegisterOperation::repair &&
        operation != RegisterOperation::unregisterServer) {
        usage();
        return 2;
    }
    if (operationRequiresAdmin(operation) && fcitx5_register_is_elevated() == 0) {
        std::wcerr << L"Registration changes require an elevated administrator token.\n";
        return 5;
    }
    if (!fs::exists(dll) || !fs::is_regular_file(dll)) {
        std::wcerr << L"TSF DLL does not exist: " << dll.c_str() << L'\n';
        return 2;
    }
    const auto exportKind = operationExport(operation);
    const auto native = dll.native();
    const HRESULT result = fcitx5_register_invoke_registration_export(
        {native.data(), native.size()}, static_cast<std::uint32_t>(exportKind));
    if (FAILED(result)) {
        std::wcerr << L"Registration operation failed: 0x" << std::hex
                   << static_cast<unsigned long>(result) << L'\n';
        return 6;
    }
    if (operation != RegisterOperation::unregisterServer &&
        registrationStatusForDll(dll) != RegisterStatus::registered) {
        std::wcerr << L"Registration completed but the registered path does not match.\n";
        return 6;
    }
    return 0;
}
