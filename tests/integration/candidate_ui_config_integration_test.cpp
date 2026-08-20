#include "config_model.h"

#include <windows.h>

#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace {

namespace fs = std::filesystem;

class TemporaryDirectory final {
 public:
  TemporaryDirectory() {
    path_ = fs::current_path() /
            (L"candidate-config-integration-" + std::to_wstring(GetCurrentProcessId()));
    std::error_code ignored;
    fs::remove_all(path_, ignored);
    fs::create_directories(path_);
  }

  ~TemporaryDirectory() {
    std::error_code ignored;
    fs::remove_all(path_, ignored);
  }

  [[nodiscard]] const fs::path& path() const noexcept { return path_; }

 private:
  fs::path path_;
};

class ChildProcess final {
 public:
  explicit ChildProcess(const fs::path& executable,
                        std::vector<std::wstring> arguments = {L"--demo"});

  ~ChildProcess() {
    if (process_.hProcess != nullptr) {
      if (WaitForSingleObject(process_.hProcess, 0) == WAIT_TIMEOUT) {
        TerminateProcess(process_.hProcess, ERROR_CANCELLED);
        WaitForSingleObject(process_.hProcess, 5000U);
      }
      CloseHandle(process_.hProcess);
    }
    if (process_.hThread != nullptr) {
      CloseHandle(process_.hThread);
    }
  }

  ChildProcess(const ChildProcess&) = delete;
  ChildProcess& operator=(const ChildProcess&) = delete;

  [[nodiscard]] DWORD id() const noexcept { return process_.dwProcessId; }

  [[nodiscard]] bool running() const noexcept {
    return WaitForSingleObject(process_.hProcess, 0) == WAIT_TIMEOUT;
  }

  void close_window(HWND window) {
    PostMessageW(window, WM_CLOSE, 0, 0);
    if (WaitForSingleObject(process_.hProcess, 5000U) != WAIT_OBJECT_0) {
      throw std::runtime_error("candidate UI did not close after WM_CLOSE");
    }
  }

 private:
  PROCESS_INFORMATION process_{};
};

void expect(bool condition, std::string_view message) {
  if (!condition) {
    throw std::runtime_error(std::string(message));
  }
}

std::wstring quote_argument(std::wstring_view value) {
  std::wstring result = L"\"";
  unsigned slashes = 0;
  for (const auto character : value) {
    if (character == L'\\') {
      ++slashes;
      continue;
    }
    if (character == L'\"') {
      result.append(slashes + 1U, L'\\');
    } else {
      result.append(slashes, L'\\');
    }
    slashes = 0;
    result.push_back(character);
  }
  result.append(slashes * 2U, L'\\');
  result.push_back(L'\"');
  return result;
}

std::wstring command_line(const fs::path& executable,
                          const std::vector<std::wstring>& arguments) {
  std::wstring command = quote_argument(executable.wstring());
  for (const auto& argument : arguments) {
    command.push_back(L' ');
    command += quote_argument(argument);
  }
  return command;
}

ChildProcess::ChildProcess(const fs::path& executable,
                           std::vector<std::wstring> arguments) {
  std::wstring command = command_line(executable, arguments);
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  if (!CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, FALSE, 0, nullptr,
                      executable.parent_path().c_str(), &startup, &process_)) {
    throw std::runtime_error("candidate UI process creation failed");
  }
}

DWORD run_process(const fs::path& executable, const std::vector<std::wstring>& arguments) {
  std::wstring command = command_line(executable, arguments);
  STARTUPINFOW startup{};
  startup.cb = sizeof(startup);
  PROCESS_INFORMATION process{};
  if (!CreateProcessW(executable.c_str(), command.data(), nullptr, nullptr, FALSE,
                      CREATE_NO_WINDOW, nullptr, executable.parent_path().c_str(), &startup,
                      &process)) {
    throw std::runtime_error("control process creation failed");
  }
  const DWORD wait = WaitForSingleObject(process.hProcess, 20000U);
  if (wait != WAIT_OBJECT_0) {
    TerminateProcess(process.hProcess, ERROR_TIMEOUT);
  }
  DWORD exit_code = ERROR_TIMEOUT;
  if (wait == WAIT_OBJECT_0) {
    GetExitCodeProcess(process.hProcess, &exit_code);
  }
  CloseHandle(process.hThread);
  CloseHandle(process.hProcess);
  return exit_code;
}

struct WindowSearch {
  DWORD process_id{};
  HWND window{};
};

BOOL CALLBACK find_window_callback(HWND window, LPARAM parameter) {
  auto& search = *reinterpret_cast<WindowSearch*>(parameter);
  DWORD process_id = 0;
  GetWindowThreadProcessId(window, &process_id);
  if (process_id == search.process_id && IsWindowVisible(window)) {
    search.window = window;
    return FALSE;
  }
  return TRUE;
}

HWND wait_for_window(const ChildProcess& process) {
  for (unsigned attempt = 0; attempt < 200U; ++attempt) {
    if (!process.running()) {
      throw std::runtime_error("candidate UI exited before showing its preview");
    }
    WindowSearch search{process.id(), nullptr};
    EnumWindows(find_window_callback, reinterpret_cast<LPARAM>(&search));
    if (search.window != nullptr) {
      return search.window;
    }
    Sleep(25U);
  }
  throw std::runtime_error("candidate UI preview window was not shown");
}

RECT window_rectangle(HWND window) {
  RECT rectangle{};
  if (!GetWindowRect(window, &rectangle)) {
    throw std::runtime_error("candidate UI rectangle query failed");
  }
  return rectangle;
}

bool same_size(const RECT& left, const RECT& right, LONG tolerance = 3) {
  const LONG left_width = left.right - left.left;
  const LONG left_height = left.bottom - left.top;
  const LONG right_width = right.right - right.left;
  const LONG right_height = right.bottom - right.top;
  return std::abs(left_width - right_width) <= tolerance &&
         std::abs(left_height - right_height) <= tolerance;
}

RECT wait_for_size_change(HWND window, const RECT& original) {
  for (unsigned attempt = 0; attempt < 200U; ++attempt) {
    const RECT current = window_rectangle(window);
    if (!same_size(current, original)) {
      return current;
    }
    Sleep(25U);
  }
  throw std::runtime_error("live presentation update did not reflow the candidate window");
}

RECT wait_for_stable_size(HWND window) {
  RECT previous = window_rectangle(window);
  unsigned stable_samples = 0;
  for (unsigned attempt = 0; attempt < 200U; ++attempt) {
    const RECT current = window_rectangle(window);
    if (same_size(current, previous, 0)) {
      ++stable_samples;
    } else {
      stable_samples = 0;
      previous = current;
    }
    if (stable_samples >= 5U) {
      return current;
    }
    Sleep(25U);
  }
  throw std::runtime_error("candidate UI size did not stabilize");
}

RECT wait_for_matching_size(HWND window, const RECT& expected) {
  RECT current = window_rectangle(window);
  for (unsigned attempt = 0; attempt < 200U; ++attempt) {
    current = window_rectangle(window);
    if (same_size(current, expected)) {
      return wait_for_stable_size(window);
    }
    Sleep(25U);
  }
  return current;
}

std::string size_text(const RECT& rectangle) {
  return std::to_string(rectangle.right - rectangle.left) + "x" +
         std::to_string(rectangle.bottom - rectangle.top);
}

std::string read_text(const fs::path& path) {
  std::ifstream input(path, std::ios::binary);
  if (!input) {
    throw std::runtime_error("saved presentation config is missing");
  }
  return {std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>()};
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
  try {
    expect(argc == 4, "expected UI, Control, and renderer resource paths");
    const fs::path ui_source = argv[1];
    const fs::path control_source = argv[2];
    const fs::path resources_source = argv[3];
    expect(fs::is_regular_file(ui_source), "candidate UI executable is missing");
    expect(fs::is_regular_file(control_source), "Control executable is missing");
    expect(fs::is_directory(resources_source), "candidate renderer resources are missing");

    TemporaryDirectory temporary;
    const auto root = temporary.path() / L"Fcitx5";
    const auto bin = root / L"bin";
    fs::create_directories(bin);
    std::ofstream(root / L"portable.flag", std::ios::binary).put('\n');
    const auto ui = bin / L"fcitx5-ui.exe";
    const auto control = bin / L"fcitx5-control.exe";
    fs::copy_file(ui_source, ui, fs::copy_options::overwrite_existing);
    fs::copy_file(control_source, control, fs::copy_options::overwrite_existing);
    fs::copy(resources_source, bin / L"resources",
             fs::copy_options::recursive | fs::copy_options::overwrite_existing);

    expect(run_process(control, {L"--set-presentation", L"light", L"builtin:default",
                                 L"vertical", L"disabled", L"5", L"Segoe UI"}) == 0,
           "initial vertical presentation save failed");
    ChildProcess candidate(ui);
    const HWND window = wait_for_window(candidate);
    // The demo window is made visible before its synthetic CandidateModel is applied.
    // Wait until that startup reflow has reached the renderer before taking the baseline.
    Sleep(300U);
    const RECT vertical = wait_for_stable_size(window);
    const LONG vertical_width = vertical.right - vertical.left;
    const LONG vertical_height = vertical.bottom - vertical.top;
    expect(vertical_width > 0 && vertical_height > 0,
           "vertical candidate preview has an invalid size");

    expect(run_process(control, {L"--set-presentation", L"dark", L"builtin:default",
                                 L"horizontal", L"enabled", L"6", L"Microsoft YaHei", L"720",
                                 L"96", L"18", L"12", L"enabled", L"0.95", L"panel"}) == 0,
           "live horizontal presentation save failed");
    static_cast<void>(wait_for_size_change(window, vertical));
    const RECT horizontal = wait_for_stable_size(window);
    const LONG horizontal_width = horizontal.right - horizontal.left;
    const LONG horizontal_height = horizontal.bottom - horizontal.top;
    expect(horizontal_width > vertical_width && horizontal_height < vertical_height,
           "horizontal setting did not produce the expected candidate reflow");
    expect(horizontal_height < 96,
           "ordinary horizontal candidate preview unexpectedly used multiple rows");

    candidate.close_window(window);

    expect(run_process(control, {L"--set-presentation", L"dark", L"builtin:default",
                                 L"horizontal", L"enabled", L"6", L"Microsoft YaHei", L"720",
                                 L"96", L"18", L"12", L"enabled", L"0.95", L"panel"}) == 0,
           "scroll-demo horizontal presentation save failed");
    ChildProcess scroll_candidate(ui, {L"--demo", L"--scroll-demo"});
    const HWND scroll_window = wait_for_window(scroll_candidate);
    Sleep(300U);
    const RECT horizontal_scroll = wait_for_stable_size(scroll_window);
    const LONG horizontal_scroll_width = horizontal_scroll.right - horizontal_scroll.left;
    const LONG horizontal_scroll_height = horizontal_scroll.bottom - horizontal_scroll.top;
    expect(horizontal_scroll_width > 0 && horizontal_scroll_height > horizontal_height * 2,
           "horizontal scroll mode did not expand into multiple candidate rows: ordinary " +
               size_text(horizontal) + ", scroll " + size_text(horizontal_scroll));
    expect(horizontal_scroll_width <= horizontal_width + 6,
           "horizontal scroll mode became wider than the ordinary candidate row: ordinary " +
               size_text(horizontal) + ", scroll " + size_text(horizontal_scroll));
    scroll_candidate.close_window(scroll_window);

    const auto saved = read_text(root / L"data/config.toml");
    fcitx::windows::config::Config saved_config;
    fcitx::windows::config::ParseError parse_error;
    expect(fcitx::windows::config::parseConfig(saved, saved_config, parse_error),
           "saved presentation config did not pass the product parser");
    expect(saved_config.appearanceMode == fcitx::windows::config::AppearanceMode::dark,
           "appearance mode was not persisted");
    expect(saved_config.orientation == fcitx::windows::config::Orientation::horizontal,
           "candidate orientation was not persisted");
    expect(saved_config.scrollMode == true, "scroll mode was not persisted");
    expect(saved_config.candidatePageSize && *saved_config.candidatePageSize == 6,
           "candidate page size was not persisted");
    expect(saved_config.opacity == 0.95, "candidate opacity was not persisted");
    expect(saved_config.preeditMode == fcitx::windows::config::PreeditMode::panel,
           "candidate preedit mode was not persisted");
    expect(saved_config.candidateFont.families &&
               !saved_config.candidateFont.families->empty() &&
               saved_config.candidateFont.families->front() == "Microsoft YaHei",
           "candidate font was not persisted");

    expect(run_process(control, {L"--set-presentation", L"light", L"builtin:default",
                                 L"vertical", L"disabled", L"5", L"Segoe UI"}) == 0,
           "reversible vertical presentation save failed");
    ChildProcess restored_candidate(ui);
    const HWND restored_window = wait_for_window(restored_candidate);
    const RECT restored = wait_for_matching_size(restored_window, vertical);
    expect(same_size(restored, vertical),
           "reversible presentation did not restore candidate size: initial " +
               size_text(vertical) + ", horizontal " + size_text(horizontal) + ", restored " +
               size_text(restored));
    restored_candidate.close_window(restored_window);

    std::cout << "candidate UI live presentation contract passed: " << vertical_width << 'x'
              << vertical_height << " -> " << horizontal_width << 'x' << horizontal_height
              << " -> restored\n";
    return 0;
  } catch (const std::exception& error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
