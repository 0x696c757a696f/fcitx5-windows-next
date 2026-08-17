#include <Windows.h>

#include <iterator>
#include <string>
#include <string_view>

int wmain(int argc, wchar_t** argv) {
    std::wstring_view readyEventName;
    std::wstring_view stopEventName;
    bool safeMode = false;
    for (int index = 1; index < argc; ++index) {
        const std::wstring_view argument(argv[index]);
        if (argument == L"--ready-event" && index + 1 < argc) {
            readyEventName = argv[++index];
        } else if (argument == L"--stop-event" && index + 1 < argc) {
            stopEventName = argv[++index];
        } else if (argument == L"--safe-mode") {
            safeMode = true;
        } else {
            return 1;
        }
    }
    HANDLE ready = readyEventName.empty()
                       ? nullptr
                       : OpenEventW(EVENT_MODIFY_STATE, FALSE,
                                    std::wstring(readyEventName).c_str());
    if (!ready) return 2;
    SetEvent(ready);
    CloseHandle(ready);
    if (!safeMode) return 23;

    wchar_t safeEventName[512]{};
    const DWORD safeNameSize = GetEnvironmentVariableW(
        L"FCITX_TEST_SAFE_MODE_EVENT", safeEventName,
        static_cast<DWORD>(std::size(safeEventName)));
    if (safeNameSize == 0 || safeNameSize >= std::size(safeEventName)) return 3;
    HANDLE safe = OpenEventW(EVENT_MODIFY_STATE, FALSE, safeEventName);
    HANDLE stop = stopEventName.empty()
                      ? nullptr
                      : OpenEventW(SYNCHRONIZE, FALSE,
                                   std::wstring(stopEventName).c_str());
    if (!safe || !stop) {
        if (safe) CloseHandle(safe);
        if (stop) CloseHandle(stop);
        return 4;
    }
    SetEvent(safe);
    CloseHandle(safe);
    const DWORD wait = WaitForSingleObject(stop, 10'000);
    CloseHandle(stop);
    return wait == WAIT_OBJECT_0 ? 0 : 5;
}
