#include "process_execution.h"

#include <cstdint>
#include <filesystem>
#include <string>
#include <vector>

namespace {

struct Fcitx5ProcessUtf16 {
    const char16_t* ptr;
    std::size_t len;
};

struct Fcitx5ProcessRunResult {
    std::uint8_t success;
    std::uint8_t reserved[7];
    char16_t* outputPtr;
    std::size_t outputLen;
};

extern "C" {
int fcitx5_process_run_utf16(Fcitx5ProcessUtf16 executable,
                             const Fcitx5ProcessUtf16* arguments,
                             std::size_t argumentCount, std::uint32_t timeoutMilliseconds,
                             std::size_t maxOutputBytes, Fcitx5ProcessRunResult* result);
void fcitx5_process_output_free(char16_t* ptr, std::size_t len);
}

Fcitx5ProcessUtf16 view(const std::wstring& value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(char16_t));
    return {reinterpret_cast<const char16_t*>(value.data()), value.size()};
}

} // namespace

namespace fcitx::windows::config {

bool runExecutable(const std::filesystem::path& executable,
                   const std::vector<std::wstring>& arguments,
                   std::wstring& output, unsigned timeoutMilliseconds,
                   std::size_t maxOutputBytes) {
    output.clear();
    const std::wstring executableText = executable.wstring();
    std::vector<Fcitx5ProcessUtf16> argumentViews;
    argumentViews.reserve(arguments.size());
    for (const auto& argument : arguments) {
        argumentViews.push_back(view(argument));
    }
    Fcitx5ProcessRunResult result{};
    const int status =
        fcitx5_process_run_utf16(view(executableText), argumentViews.data(),
                                 argumentViews.size(), timeoutMilliseconds, maxOutputBytes,
                                 &result);
    if (status != 0) {
        return false;
    }
    if (result.outputPtr && result.outputLen > 0) {
        output.assign(reinterpret_cast<const wchar_t*>(result.outputPtr), result.outputLen);
    }
    fcitx5_process_output_free(result.outputPtr, result.outputLen);
    return result.success != 0;
}

} // namespace fcitx::windows::config
