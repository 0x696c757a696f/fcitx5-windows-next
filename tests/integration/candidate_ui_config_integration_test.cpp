#include "config_model.h"

#include <windows.h>

#include <cstdint>
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

struct CaptureEvidence {
  std::uint64_t checksum{};
  std::size_t bytes{};
  std::size_t non_background_pixels{};
};

CaptureEvidence capture_window(HWND window, const fs::path& path) {
  const RECT rectangle = window_rectangle(window);
  const int width = rectangle.right - rectangle.left;
  const int height = rectangle.bottom - rectangle.top;
  expect(width > 0 && height > 0, "cannot capture an empty candidate UI window");

  HDC window_dc = GetWindowDC(window);
  if (window_dc == nullptr) {
    throw std::runtime_error("GetWindowDC failed for candidate UI screenshot");
  }
  HDC memory_dc = CreateCompatibleDC(window_dc);
  if (memory_dc == nullptr) {
    ReleaseDC(window, window_dc);
    throw std::runtime_error("CreateCompatibleDC failed for candidate UI screenshot");
  }
  HBITMAP bitmap = CreateCompatibleBitmap(window_dc, width, height);
  if (bitmap == nullptr) {
    DeleteDC(memory_dc);
    ReleaseDC(window, window_dc);
    throw std::runtime_error("CreateCompatibleBitmap failed for candidate UI screenshot");
  }
  HGDIOBJ old_object = SelectObject(memory_dc, bitmap);
  if (!BitBlt(memory_dc, 0, 0, width, height, window_dc, 0, 0, SRCCOPY)) {
    SelectObject(memory_dc, old_object);
    DeleteObject(bitmap);
    DeleteDC(memory_dc);
    ReleaseDC(window, window_dc);
    throw std::runtime_error("BitBlt failed for candidate UI screenshot");
  }

  BITMAPINFO info{};
  info.bmiHeader.biSize = sizeof(BITMAPINFOHEADER);
  info.bmiHeader.biWidth = width;
  info.bmiHeader.biHeight = -height;
  info.bmiHeader.biPlanes = 1;
  info.bmiHeader.biBitCount = 32;
  info.bmiHeader.biCompression = BI_RGB;
  std::vector<std::uint32_t> pixels(static_cast<std::size_t>(width) *
                                    static_cast<std::size_t>(height));
  if (GetDIBits(memory_dc, bitmap, 0, static_cast<UINT>(height), pixels.data(), &info,
                DIB_RGB_COLORS) == 0) {
    SelectObject(memory_dc, old_object);
    DeleteObject(bitmap);
    DeleteDC(memory_dc);
    ReleaseDC(window, window_dc);
    throw std::runtime_error("GetDIBits failed for candidate UI screenshot");
  }

  SelectObject(memory_dc, old_object);
  DeleteObject(bitmap);
  DeleteDC(memory_dc);
  ReleaseDC(window, window_dc);

  const std::uint32_t background = pixels.empty() ? 0U : pixels.front();
  std::size_t non_background = 0;
  std::uint64_t checksum = 1469598103934665603ULL;
  for (const auto pixel : pixels) {
    if (pixel != background) {
      ++non_background;
    }
    checksum ^= pixel;
    checksum *= 1099511628211ULL;
  }
  expect(non_background > 0, "candidate UI screenshot did not contain visible content");

  BITMAPFILEHEADER file_header{};
  file_header.bfType = 0x4D42;
  file_header.bfOffBits = sizeof(BITMAPFILEHEADER) + sizeof(BITMAPINFOHEADER);
  file_header.bfSize =
      file_header.bfOffBits + static_cast<DWORD>(pixels.size() * sizeof(std::uint32_t));

  fs::create_directories(path.parent_path());
  std::ofstream output(path, std::ios::binary);
  if (!output) {
    throw std::runtime_error("failed to open candidate UI screenshot artifact");
  }
  output.write(reinterpret_cast<const char*>(&file_header), sizeof(file_header));
  output.write(reinterpret_cast<const char*>(&info.bmiHeader), sizeof(info.bmiHeader));
  output.write(reinterpret_cast<const char*>(pixels.data()),
               static_cast<std::streamsize>(pixels.size() * sizeof(std::uint32_t)));
  if (!output) {
    throw std::runtime_error("failed to write candidate UI screenshot artifact");
  }

  return CaptureEvidence{
      checksum,
      static_cast<std::size_t>(file_header.bfSize),
      non_background,
  };
}

std::string read_text(const fs::path& path) {
  std::ifstream input(path, std::ios::binary);
  if (!input) {
    throw std::runtime_error("expected text artifact is missing: " + path.string());
  }
  return {std::istreambuf_iterator<char>(input), std::istreambuf_iterator<char>()};
}

int json_int_field(const std::string& text, std::string_view field) {
  const std::string needle = "\"" + std::string(field) + "\":";
  const auto marker = text.find(needle);
  if (marker == std::string::npos) {
    throw std::runtime_error("missing JSON field: " + std::string(field));
  }
  const auto start = marker + needle.size();
  const auto end = text.find_first_not_of("-0123456789", start);
  return std::stoi(text.substr(start, end == std::string::npos ? end : end - start));
}

std::uint64_t json_u64_field(const std::string& text, std::string_view field) {
  const std::string needle = "\"" + std::string(field) + "\":";
  const auto marker = text.find(needle);
  if (marker == std::string::npos) {
    throw std::runtime_error("missing JSON field: " + std::string(field));
  }
  const auto start = marker + needle.size();
  const auto end = text.find_first_not_of("0123456789", start);
  return std::stoull(text.substr(start, end == std::string::npos ? end : end - start));
}

void expect_contains(const std::string& text, std::string_view needle,
                     std::string_view message) {
  if (text.find(needle) == std::string::npos) {
    throw std::runtime_error(std::string(message));
  }
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
  try {
    expect(argc == 5, "expected UI, Control, renderer resource, and Rust PoC paths");
    const fs::path ui_source = argv[1];
    const fs::path control_source = argv[2];
    const fs::path resources_source = argv[3];
    const fs::path rust_candidate_poc = argv[4];
    expect(fs::is_regular_file(ui_source), "candidate UI executable is missing");
    expect(fs::is_regular_file(control_source), "Control executable is missing");
    expect(fs::is_directory(resources_source), "candidate renderer resources are missing");
    expect(fs::is_regular_file(rust_candidate_poc), "Rust Candidate PoC executable is missing");

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
    const auto cpp_demo_screenshot = temporary.path() / L"cpp-candidate-demo.bmp";
    const CaptureEvidence cpp_demo = capture_window(window, cpp_demo_screenshot);
    expect(cpp_demo.bytes > 0 && cpp_demo.non_background_pixels > 0 && cpp_demo.checksum != 0,
           "C++ candidate demo screenshot evidence is invalid");

    const auto rust_demo_report = temporary.path() / L"rust-candidate-demo.json";
    const auto rust_demo_screenshot = temporary.path() / L"rust-candidate-demo.bmp";
    expect(run_process(rust_candidate_poc,
                       {L"--window-smoke", L"--demo-snapshot", L"--report",
                        rust_demo_report.wstring(), L"--screenshot",
                        rust_demo_screenshot.wstring()}) == 0,
           "Rust candidate demo snapshot smoke failed");
    const auto rust_demo = read_text(rust_demo_report);
    expect_contains(rust_demo, "\"snapshot_name\":\"demo-snapshot\"",
                    "Rust candidate demo snapshot report used the wrong snapshot");
    expect_contains(rust_demo, "\"orientation\":\"vertical\"",
                    "Rust candidate demo snapshot did not use vertical orientation");
    expect_contains(rust_demo, "\"candidate_count\":3",
                    "Rust candidate demo snapshot did not use the C++ demo candidate count");
    expect_contains(rust_demo, "\"screenshot_written\":true",
                    "Rust candidate demo snapshot did not write screenshot evidence");
    expect_contains(rust_demo, "\"msaa_accessible_name_readable\":true",
                    "Rust candidate demo snapshot did not prove accessibility name");
    expect_contains(rust_demo, "\"uia_name_readable\":true",
                    "Rust candidate demo snapshot did not prove UIA name");
    expect(json_u64_field(rust_demo, "visual_non_background_pixels") > 0,
           "Rust candidate demo snapshot screenshot did not contain visible content");
    expect(json_u64_field(rust_demo, "visual_checksum") != 0,
           "Rust candidate demo snapshot checksum is invalid");
    const int rust_demo_width =
        json_int_field(rust_demo, "window_right") - json_int_field(rust_demo, "window_left");
    const int rust_demo_height =
        json_int_field(rust_demo, "window_bottom") - json_int_field(rust_demo, "window_top");
    expect(rust_demo_width > 0 && rust_demo_height > 0,
           "Rust candidate demo snapshot has invalid geometry");
    expect(rust_demo_width <= vertical_width * 3 && rust_demo_width * 3 >= vertical_width,
           "Rust/C++ candidate demo width diverged beyond allowed PoC tolerance");
    expect(rust_demo_height <= vertical_height * 3 && rust_demo_height * 3 >= vertical_height,
           "Rust/C++ candidate demo height diverged beyond allowed PoC tolerance");

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
    const auto cpp_scroll_screenshot = temporary.path() / L"cpp-candidate-scroll-demo.bmp";
    const CaptureEvidence cpp_scroll = capture_window(scroll_window, cpp_scroll_screenshot);
    expect(cpp_scroll.bytes > 0 && cpp_scroll.non_background_pixels > 0 &&
               cpp_scroll.checksum != 0,
           "C++ candidate scroll-demo screenshot evidence is invalid");
    scroll_candidate.close_window(scroll_window);

    const auto rust_scroll_report = temporary.path() / L"rust-candidate-scroll-demo.json";
    const auto rust_scroll_screenshot = temporary.path() / L"rust-candidate-scroll-demo.bmp";
    expect(run_process(rust_candidate_poc,
                       {L"--window-smoke", L"--scroll-demo-snapshot", L"--report",
                        rust_scroll_report.wstring(), L"--screenshot",
                        rust_scroll_screenshot.wstring()}) == 0,
           "Rust candidate scroll-demo snapshot smoke failed");
    const auto rust_scroll = read_text(rust_scroll_report);
    expect_contains(rust_scroll, "\"snapshot_name\":\"scroll-demo-snapshot\"",
                    "Rust candidate scroll-demo report used the wrong snapshot");
    expect_contains(rust_scroll, "\"orientation\":\"horizontal\"",
                    "Rust candidate scroll-demo snapshot did not use horizontal orientation");
    expect_contains(rust_scroll, "\"scroll_mode\":true",
                    "Rust candidate scroll-demo snapshot did not enable scroll mode");
    expect_contains(rust_scroll, "\"candidate_count\":60",
                    "Rust candidate scroll-demo snapshot did not use the C++ scroll candidate count");
    expect_contains(rust_scroll, "\"screenshot_written\":true",
                    "Rust candidate scroll-demo snapshot did not write screenshot evidence");
    expect_contains(rust_scroll, "\"msaa_accessible_name_readable\":true",
                    "Rust candidate scroll-demo snapshot did not prove accessibility name");
    expect_contains(rust_scroll, "\"uia_name_readable\":true",
                    "Rust candidate scroll-demo snapshot did not prove UIA name");
    expect(json_u64_field(rust_scroll, "visual_non_background_pixels") > 0,
           "Rust candidate scroll-demo screenshot did not contain visible content");
    expect(json_u64_field(rust_scroll, "visual_checksum") != 0,
           "Rust candidate scroll-demo checksum is invalid");
    const int rust_scroll_width =
        json_int_field(rust_scroll, "window_right") - json_int_field(rust_scroll, "window_left");
    const int rust_scroll_height =
        json_int_field(rust_scroll, "window_bottom") - json_int_field(rust_scroll, "window_top");
    expect(rust_scroll_width > 0 && rust_scroll_height > rust_demo_height,
           "Rust candidate scroll-demo snapshot did not expand beyond the vertical demo height");
    expect(rust_scroll_width <= horizontal_scroll_width * 3 &&
               rust_scroll_width * 3 >= horizontal_scroll_width,
           "Rust/C++ candidate scroll-demo width diverged beyond allowed PoC tolerance");
    expect(rust_scroll_height <= horizontal_scroll_height * 3 &&
               rust_scroll_height * 3 >= horizontal_scroll_height,
           "Rust/C++ candidate scroll-demo height diverged beyond allowed PoC tolerance");

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
