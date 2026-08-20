#include <Windows.h>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <string>
#include <vector>

namespace {

namespace fs = std::filesystem;

std::wstring quote(std::wstring_view value) { return L"\"" + std::wstring(value) + L"\""; }

void write_file(const fs::path& path, std::string_view text) {
    fs::create_directories(path.parent_path());
    std::ofstream output(path, std::ios::binary);
    output << text;
    if (!output)
        throw std::runtime_error("fixture write failed");
}

DWORD run_register(const fs::path& executable, const fs::path& dll) {
    std::wstring command = quote(executable.wstring()) + L" --validate-artifact --dll " +
                           quote(dll.wstring());
    std::vector<wchar_t> mutableCommand(command.begin(), command.end());
    mutableCommand.push_back(L'\0');
    STARTUPINFOW startup{};
    startup.cb = sizeof(startup);
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(executable.c_str(), mutableCommand.data(), nullptr, nullptr, FALSE,
                        CREATE_NO_WINDOW, nullptr, executable.parent_path().c_str(), &startup,
                        &process))
        throw std::runtime_error("register process creation failed");
    CloseHandle(process.hThread);
    if (WaitForSingleObject(process.hProcess, 10'000) != WAIT_OBJECT_0) {
        TerminateProcess(process.hProcess, ERROR_TIMEOUT);
    }
    DWORD exitCode = ERROR_TIMEOUT;
    GetExitCodeProcess(process.hProcess, &exitCode);
    CloseHandle(process.hProcess);
    return exitCode;
}

std::wstring current_architecture() {
#if defined(_WIN64)
    return L"x64";
#else
    return L"x86";
#endif
}

std::wstring paired_architecture() {
#if defined(_WIN64)
    return L"x86";
#else
    return L"x64";
#endif
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2) {
        std::cerr << "expected register executable path\n";
        return 1;
    }
    const fs::path source = argv[1];
    const fs::path root = fs::temp_directory_path() /
                          (L"fcitx5-register-artifact-" +
                           std::to_wstring(GetCurrentProcessId()));
    std::error_code ignored;
    fs::remove_all(root, ignored);
    try {
        const fs::path bin = root / L"bin";
        fs::create_directories(bin);
        const fs::path helper = bin / source.filename();
        fs::copy_file(source, helper, fs::copy_options::overwrite_existing);
        const fs::path currentDll =
            root / L"tsf" / current_architecture() / L"fcitx5-tsf.dll";
        const fs::path pairedDll =
            root / L"tsf" / paired_architecture() / L"fcitx5-tsf.dll";
        write_file(currentDll, "current architecture fixture\n");
        write_file(pairedDll, "paired architecture fixture\n");
        if (run_register(helper, currentDll) != 0) {
            std::cerr << "valid product artifact was rejected\n";
            fs::remove_all(root, ignored);
            return 1;
        }
        const fs::path outside = root / L"outside" / L"fcitx5-tsf.dll";
        write_file(outside, "outside fixture\n");
        if (run_register(helper, outside) == 0) {
            std::cerr << "outside-root TSF DLL was accepted\n";
            fs::remove_all(root, ignored);
            return 1;
        }
        fs::remove(pairedDll, ignored);
        if (run_register(helper, currentDll) == 0) {
            std::cerr << "artifact with missing paired architecture was accepted\n";
            fs::remove_all(root, ignored);
            return 1;
        }
        fs::remove_all(root, ignored);
        std::cout << "register artifact validation ok\n";
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << '\n';
        fs::remove_all(root, ignored);
        return 1;
    }
}
