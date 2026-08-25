#include "tsf_test_identity.h"

#include <Windows.h>
#include <msctf.h>
#include <objbase.h>
#include <tlhelp32.h>

#include <array>
#include <chrono>
#include <iostream>
#include <string>
#include <string_view>
#include <thread>

namespace {

struct ProcessOwner {
    HANDLE process{};
    ~ProcessOwner() {
        if (!process)
            return;
        if (WaitForSingleObject(process, 0) != WAIT_OBJECT_0) {
            TerminateProcess(process, 0);
            WaitForSingleObject(process, 2000);
        }
        CloseHandle(process);
    }
};

struct WindowSearch {
    DWORD processId{};
    HWND window{};
};

struct TsfSessionActivation {
    bool comInitialized{};
    ITfThreadMgr* threadManager{};
    ITfInputProcessorProfileMgr* profileManager{};
    ITfInputProcessorProfiles* profiles{};
    TfClientId clientId{TF_CLIENTID_NULL};

    ~TsfSessionActivation() {
        if (profiles)
            profiles->Release();
        if (profileManager)
            profileManager->Release();
        if (threadManager) {
            if (clientId != TF_CLIENTID_NULL)
                threadManager->Deactivate();
            threadManager->Release();
        }
        if (comInitialized)
            CoUninitialize();
    }

    bool activate() {
        const HRESULT initialize = CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
        if (FAILED(initialize))
            return false;
        comInitialized = true;
        if (FAILED(CoCreateInstance(CLSID_TF_ThreadMgr, nullptr, CLSCTX_INPROC_SERVER,
                                    IID_ITfThreadMgr,
                                    reinterpret_cast<void**>(&threadManager))) ||
            FAILED(threadManager->Activate(&clientId)))
            return false;
        if (FAILED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr,
                                    CLSCTX_INPROC_SERVER, IID_ITfInputProcessorProfileMgr,
                                    reinterpret_cast<void**>(&profileManager))) ||
            FAILED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr,
                                    CLSCTX_INPROC_SERVER, IID_ITfInputProcessorProfiles,
                                    reinterpret_cast<void**>(&profiles))))
            return false;

        const auto registrableProfiles = fcitx::windows::tsf::loadRegistrableInputProfiles();
        if (registrableProfiles.empty()) {
            std::cerr << "no TSF profiles are enabled for registration\n";
            return false;
        }
        for (const auto& expectedProfile : registrableProfiles) {
            TF_INPUTPROCESSORPROFILE registered{};
            if (FAILED(profileManager->GetProfile(
                    TF_PROFILETYPE_INPUTPROCESSOR, expectedProfile.language,
                    fcitx::windows::tsf::kTextServiceClsid, expectedProfile.guid,
                    nullptr, &registered))) {
                std::cerr << "registered TSF profile was not enumerated\n";
                return false;
            }
        }
        const auto& expected = registrableProfiles[0];
        // ActivateProfile with DONTCARECURRENTINPUTLANGUAGE only queues the profile until its
        // language is selected. A deterministic host test first switches the desktop TSF
        // language, then activates and verifies the exact keyboard profile.
        if (FAILED(profiles->ChangeCurrentLanguage(expected.language))) {
            std::cerr << "TSF current language could not switch to zh-CN\n";
            return false;
        }
        const HRESULT result = profileManager->ActivateProfile(
            TF_PROFILETYPE_INPUTPROCESSOR, expected.language,
            fcitx::windows::tsf::kTextServiceClsid, expected.guid, nullptr,
            TF_IPPMF_FORSESSION | TF_IPPMF_ENABLEPROFILE);
        if (result != S_OK) {
            std::cerr << "TSF profile activation failed: 0x" << std::hex << result << '\n';
            return false;
        }
        TF_INPUTPROCESSORPROFILE active{};
        if (FAILED(profileManager->GetActiveProfile(GUID_TFCAT_TIP_KEYBOARD, &active)) ||
            active.dwProfileType != TF_PROFILETYPE_INPUTPROCESSOR ||
            active.langid != expected.language ||
            !IsEqualGUID(active.clsid, fcitx::windows::tsf::kTextServiceClsid) ||
            !IsEqualGUID(active.guidProfile, expected.guid)) {
            std::cerr << "Fcitx5 did not become the active desktop keyboard profile\n";
            return false;
        }
        return true;
    }
};

BOOL CALLBACK findWindow(HWND window, LPARAM parameter) {
    auto& search = *reinterpret_cast<WindowSearch*>(parameter);
    DWORD processId = 0;
    GetWindowThreadProcessId(window, &processId);
    if (processId == search.processId && IsWindowVisible(window) && GetWindow(window, GW_OWNER) == nullptr) {
        search.window = window;
        return FALSE;
    }
    return TRUE;
}

bool sendVirtualKey(WORD key) {
    std::array<INPUT, 2> input{};
    input[0].type = INPUT_KEYBOARD;
    input[0].ki.wVk = key;
    input[1] = input[0];
    input[1].ki.dwFlags = KEYEVENTF_KEYUP;
    return SendInput(static_cast<UINT>(input.size()), input.data(), sizeof(INPUT)) == input.size();
}

bool sendControlKey(WORD key) {
    std::array<INPUT, 4> input{};
    for (auto& item : input)
        item.type = INPUT_KEYBOARD;
    input[0].ki.wVk = VK_CONTROL;
    input[1].ki.wVk = key;
    input[2].ki.wVk = key;
    input[2].ki.dwFlags = KEYEVENTF_KEYUP;
    input[3].ki.wVk = VK_CONTROL;
    input[3].ki.dwFlags = KEYEVENTF_KEYUP;
    return SendInput(static_cast<UINT>(input.size()), input.data(), sizeof(INPUT)) == input.size();
}

bool sendAsciiLetters(std::wstring_view text) {
    for (wchar_t ch : text) {
        if (ch >= L'a' && ch <= L'z') {
            ch = static_cast<wchar_t>(ch - L'a' + L'A');
        }
        if (ch < L'A' || ch > L'Z')
            return false;
        if (!sendVirtualKey(static_cast<WORD>(ch)))
            return false;
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }
    return true;
}

void unlockForegroundActivation() {
    std::array<INPUT, 2> input{};
    input[0].type = INPUT_KEYBOARD;
    input[0].ki.wVk = VK_MENU;
    input[1] = input[0];
    input[1].ki.dwFlags = KEYEVENTF_KEYUP;
    SendInput(static_cast<UINT>(input.size()), input.data(), sizeof(INPUT));
}

bool focusWindow(HWND window) {
    if (!window)
        return false;
    AllowSetForegroundWindow(ASFW_ANY);
    ShowWindow(window, SW_RESTORE);
    const DWORD currentThread = GetCurrentThreadId();
    DWORD targetProcess = 0;
    const DWORD targetThread = GetWindowThreadProcessId(window, &targetProcess);
    for (int attempt = 0; attempt < 20; ++attempt) {
        const HWND foreground = GetForegroundWindow();
        DWORD foregroundProcess = 0;
        const DWORD foregroundThread =
            foreground ? GetWindowThreadProcessId(foreground, &foregroundProcess) : 0;
        const bool attachTarget =
            targetThread != 0 && targetThread != currentThread &&
            AttachThreadInput(currentThread, targetThread, TRUE) != FALSE;
        const bool attachForeground =
            foregroundThread != 0 && foregroundThread != currentThread &&
            foregroundThread != targetThread &&
            AttachThreadInput(currentThread, foregroundThread, TRUE) != FALSE;
        unlockForegroundActivation();
        SetWindowPos(window, HWND_TOPMOST, 0, 0, 0, 0,
                     SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
        SetWindowPos(window, HWND_NOTOPMOST, 0, 0, 0, 0,
                     SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
        BringWindowToTop(window);
        SetForegroundWindow(window);
        SetFocus(window);
        if (attachForeground)
            AttachThreadInput(currentThread, foregroundThread, FALSE);
        if (attachTarget)
            AttachThreadInput(currentThread, targetThread, FALSE);
        if (GetForegroundWindow() == window)
            return true;
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }
    return false;
}

bool candidateWindowVisible() {
    const std::wstring className =
        std::wstring(fcitx::windows::kReleaseIdentity.local_object_prefix) + L".Candidate";
    const HWND window = FindWindowW(className.c_str(), nullptr);
    return window && IsWindowVisible(window);
}

bool waitCandidateWindowVisible() {
    for (int attempt = 0; attempt < 40; ++attempt) {
        if (candidateWindowVisible())
            return true;
        std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }
    return false;
}

std::wstring clipboardText() {
    if (!OpenClipboard(nullptr))
        return {};
    std::wstring result;
    if (HANDLE data = GetClipboardData(CF_UNICODETEXT)) {
        if (const auto* text = static_cast<const wchar_t*>(GlobalLock(data))) {
            result = text;
            GlobalUnlock(data);
        }
    }
    CloseClipboard();
    return result;
}

bool clearClipboardText() {
    for (int attempt = 0; attempt < 20; ++attempt) {
        if (OpenClipboard(nullptr)) {
            const bool cleared = EmptyClipboard() != FALSE;
            CloseClipboard();
            return cleared;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    return false;
}

bool processHasModule(DWORD processId, std::wstring_view moduleName) {
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, processId);
    if (snapshot == INVALID_HANDLE_VALUE) return false;
    MODULEENTRY32W entry{};
    entry.dwSize = sizeof(entry);
    bool found = false;
    if (Module32FirstW(snapshot, &entry)) {
        do {
            if (_wcsicmp(entry.szModule, moduleName.data()) == 0) {
                found = true;
                break;
            }
        } while (Module32NextW(snapshot, &entry));
    }
    CloseHandle(snapshot);
    return found;
}

std::wstring processModulePath(DWORD processId, std::wstring_view moduleName) {
    HANDLE snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, processId);
    if (snapshot == INVALID_HANDLE_VALUE) return {};
    MODULEENTRY32W entry{};
    entry.dwSize = sizeof(entry);
    std::wstring path;
    if (Module32FirstW(snapshot, &entry)) {
        do {
            if (_wcsicmp(entry.szModule, moduleName.data()) == 0) {
                path = entry.szExePath;
                break;
            }
        } while (Module32NextW(snapshot, &entry));
    }
    CloseHandle(snapshot);
    return path;
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    const bool passthrough =
        argc >= 2 && std::wstring_view(argv[1]) == L"--passthrough";
    const bool rawHarness =
        argc >= 2 && std::wstring_view(argv[1]) == L"--raw-harness";
    const bool sampleMode =
        argc >= 2 && std::wstring_view(argv[1]) == L"--sample";
    const bool candidateWindowMode =
        argc >= 2 && std::wstring_view(argv[1]) == L"--candidate-window";
    if (argc < 1 || argc > 5 || (passthrough && argc > 2) ||
        (rawHarness && argc > 2) || (sampleMode && argc != 5) ||
        (candidateWindowMode && argc != 3) ||
        (!passthrough && !rawHarness && !sampleMode && !candidateWindowMode && argc > 2))
        return 1;
    // The engine executable is an optional argument used only in the real-input
    // mode. It is published through FCITX5_TEST_ENGINE_PATH so the development
    // TSF peer verification resolves the exact engine binary instead of guessing
    // from the DLL location; the variable is inherited by the Notepad child
    // process. The passthrough mode deliberately omits it: that mode verifies
    // the fail-open path when no engine is available.
    if (!passthrough && !rawHarness && (sampleMode || candidateWindowMode || argc == 2)) {
        const std::wstring enginePath(sampleMode       ? argv[4]
                                      : candidateWindowMode ? argv[2]
                                                            : argv[1]);
        if (enginePath.empty() || !SetEnvironmentVariableW(L"FCITX5_TEST_ENGINE_PATH",
                                                           enginePath.c_str()))
            return 1;
    }
    TsfSessionActivation activation;
    if (!rawHarness && !activation.activate())
        return 5;

    wchar_t command[] = L"notepad.exe";
    STARTUPINFOW startup{sizeof(startup)};
    PROCESS_INFORMATION process{};
    if (!CreateProcessW(nullptr, command, nullptr, nullptr, FALSE, 0, nullptr, nullptr,
                        &startup, &process)) {
        std::cerr << "Notepad could not start: " << GetLastError() << '\n';
        return 6;
    }
    CloseHandle(process.hThread);
    ProcessOwner owner{process.hProcess};
    WaitForInputIdle(process.hProcess, 10'000);
    WindowSearch search{process.dwProcessId, nullptr};
    for (int attempt = 0; attempt < 100 && !search.window; ++attempt) {
        EnumWindows(findWindow, reinterpret_cast<LPARAM>(&search));
        if (!search.window)
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
    }
    if (!search.window || !focusWindow(search.window)) {
        std::cerr << "Notepad window could not receive foreground input\n";
        return 7;
    }
    // Profile activation is broadcast to GUI threads asynchronously. On the first host process
    // after a fresh registration, Notepad can become foreground before its TSF document manager
    // has loaded the newly active TIP. Give that documented session transition a bounded settle
    // window; subsequent key handling is still measured and must succeed without retries.
    std::this_thread::sleep_for(std::chrono::milliseconds(1500));
    if (!clearClipboardText()) {
        std::cerr << "Clipboard could not be cleared before the typing smoke test\n";
        return 8;
    }
    if (!sendControlKey('A') || !sendVirtualKey(VK_BACK))
        return 8;
    std::this_thread::sleep_for(std::chrono::milliseconds(100));
    if (passthrough || rawHarness) {
        if (!sendVirtualKey('A') || !sendVirtualKey('B') || !sendVirtualKey('C'))
            return 8;
    } else if (sampleMode) {
        if (!sendAsciiLetters(argv[2]))
            return 8;
        std::this_thread::sleep_for(std::chrono::milliseconds(300));
        if (!sendVirtualKey(VK_SPACE))
            return 8;
    } else if (candidateWindowMode) {
        if (!sendVirtualKey('N'))
            return 8;
        if (!waitCandidateWindowVisible()) {
            const bool tsfLoaded = processHasModule(process.dwProcessId, L"fcitx5-tsf.dll");
            std::cerr << "Candidate UI window was not visible after preedit; "
                      << "fcitx5_tsf_loaded=" << (tsfLoaded ? "yes" : "no");
            const std::wstring modulePath =
                processModulePath(process.dwProcessId, L"fcitx5-tsf.dll");
            if (!modulePath.empty())
                std::wcerr << L" fcitx5_tsf_path=" << modulePath;
            std::cerr << '\n';
            return 10;
        }
        std::cout << "Notepad TSF candidate window smoke passed\n";
        return 0;
    } else {
        if (!sendVirtualKey('N') || !sendVirtualKey('I'))
            return 8;
        std::this_thread::sleep_for(std::chrono::milliseconds(300));
        if (!sendVirtualKey(VK_SPACE))
            return 8;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(300));
    if (!sendControlKey('A') || !sendControlKey('C'))
        return 8;
    std::this_thread::sleep_for(std::chrono::milliseconds(200));
    const std::wstring text = clipboardText();
    const std::wstring expected = (passthrough || rawHarness) ? L"abc" :
                                  sampleMode ? std::wstring(argv[3]) :
                                               L"\x4f60";
    if (text != expected) {
        const bool tsfLoaded = processHasModule(process.dwProcessId, L"fcitx5-tsf.dll");
        std::cerr << "Notepad did not receive the exact expected text; length="
                  << text.size() << " first=U+" << std::hex
                  << (text.empty() ? 0U : static_cast<unsigned>(text.front()))
                  << " fcitx5_tsf_loaded=" << (tsfLoaded ? "yes" : "no") << '\n';
        return 9;
    }
    std::cout << (rawHarness
                      ? "Notepad raw keyboard harness passed: abc\n"
                      : passthrough
                      ? "Notepad TSF engine-unavailable fallback passed: abc\n"
                      : sampleMode ? "Notepad TSF sample typing smoke passed\n"
                      : "Notepad TSF typing smoke passed: U+4F60\n");
    return 0;
}
