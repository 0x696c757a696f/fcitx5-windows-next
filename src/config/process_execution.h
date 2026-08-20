#pragma once

#include <filesystem>
#include <cstddef>
#include <string>
#include <vector>

namespace fcitx::windows::config {

// Runs an executable with redirected stdout/stderr pipes and returns its
// combined output. The pipes are drained concurrently while the process runs,
// so a child that emits more output than the pipe buffer holds cannot deadlock
// against WaitForSingleObject (the classic read-after-wait pipe deadlock).
// Returns false if the process does not exit within the timeout.
[[nodiscard]] bool runExecutable(const std::filesystem::path& executable,
                                 const std::vector<std::wstring>& arguments,
                                 std::wstring& output,
                                 unsigned timeoutMilliseconds = 120'000,
                                 std::size_t maxOutputBytes = 2U * 1024U * 1024U);

} // namespace fcitx::windows::config
