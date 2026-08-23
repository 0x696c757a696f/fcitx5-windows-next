#pragma once

#include <cstddef>
#include <cstdint>

namespace fcitx::windows::launcher::rust_abi {

struct Fcitx5LauncherSnapshot {
    std::uint32_t state;
    std::uint32_t consecutiveStartupCrashes;
    std::uint64_t nextStartAllowedMilliseconds;
};

struct Fcitx5LauncherMachine {
    Fcitx5LauncherSnapshot snapshot;
    std::uint32_t engineState;
};

struct Fcitx5LauncherStartDecision {
    std::uint32_t disposition;
    std::uint8_t safeMode;
    std::uint8_t reserved[7];
    std::uint64_t retryAfterMilliseconds;
};

} // namespace fcitx::windows::launcher::rust_abi

extern "C" {
int fcitx5_launcher_state_init(
    std::uint64_t now,
    fcitx::windows::launcher::rust_abi::Fcitx5LauncherSnapshot snapshot,
    fcitx::windows::launcher::rust_abi::Fcitx5LauncherMachine* output);
std::uint8_t fcitx5_launcher_state_can_apply(std::uint32_t state, std::uint32_t command);
std::uint32_t fcitx5_launcher_state_after(std::uint32_t state, std::uint32_t command);
std::uint8_t fcitx5_launcher_state_apply(
    fcitx::windows::launcher::rust_abi::Fcitx5LauncherMachine* machine, std::uint32_t command);
int fcitx5_launcher_state_request_start(
    fcitx::windows::launcher::rust_abi::Fcitx5LauncherMachine* machine, std::uint64_t now,
    fcitx::windows::launcher::rust_abi::Fcitx5LauncherStartDecision* output);
void fcitx5_launcher_state_engine_ready(
    fcitx::windows::launcher::rust_abi::Fcitx5LauncherMachine* machine);
void fcitx5_launcher_state_engine_exited(
    fcitx::windows::launcher::rust_abi::Fcitx5LauncherMachine* machine,
    std::uint64_t runtimeMilliseconds, std::uint64_t now);
void fcitx5_launcher_state_engine_stopped_intentionally(
    fcitx::windows::launcher::rust_abi::Fcitx5LauncherMachine* machine);
std::uint8_t fcitx5_launcher_state_is_persistent(std::uint32_t state);
std::uint32_t fcitx5_launcher_state_store_load_utf16(
    const std::uint16_t* path, std::size_t len,
    fcitx::windows::launcher::rust_abi::Fcitx5LauncherSnapshot* snapshot);
std::uint8_t fcitx5_launcher_state_store_save_utf16(
    const std::uint16_t* path, std::size_t len,
    fcitx::windows::launcher::rust_abi::Fcitx5LauncherSnapshot snapshot);
std::size_t fcitx5_launcher_default_state_store_path_utf16(std::uint16_t* output,
                                                           std::size_t capacity);
std::uint8_t fcitx5_launcher_absolute_windows_path_utf16(const std::uint16_t* path,
                                                         std::size_t len);
std::uint8_t fcitx5_launcher_resolve_default_process_paths_utf16(
    const std::uint16_t* executable_directory, std::size_t executable_directory_len,
    const std::uint16_t* generation, std::size_t generation_len, std::uint16_t* engine_output,
    std::size_t engine_capacity, std::uint16_t* ui_output, std::size_t ui_capacity,
    std::size_t* required_engine_len, std::size_t* required_ui_len);
std::size_t fcitx5_launcher_tray_status_text_utf16(std::uint32_t launcher_state,
                                                   std::uint32_t engine_state,
                                                   std::uint8_t chinese, std::uint16_t* output,
                                                   std::size_t capacity);
std::size_t fcitx5_launcher_tray_input_method_display_utf16(
    const std::uint8_t* native_name, std::size_t native_name_len, const std::uint8_t* name,
    std::size_t name_len, const std::uint8_t* id, std::size_t id_len, std::uint16_t* output,
    std::size_t capacity);
std::size_t fcitx5_launcher_tray_tooltip_utf16(
    const std::uint16_t* product_name, std::size_t product_name_len,
    std::uint32_t launcher_state, std::uint32_t engine_state, std::uint8_t chinese,
    const std::uint8_t* native_name, std::size_t native_name_len, const std::uint8_t* name,
    std::size_t name_len, const std::uint8_t* id, std::size_t id_len, std::uint16_t* output,
    std::size_t capacity);
std::size_t fcitx5_launcher_engine_command_utf16(
    const std::uint16_t* engine_path, std::size_t engine_path_len,
    const std::uint16_t* ready_event, std::size_t ready_event_len,
    const std::uint16_t* stop_event, std::size_t stop_event_len,
    const std::uint16_t* generation, std::size_t generation_len, std::uint8_t safe_mode,
    std::uint16_t* output, std::size_t capacity);
std::size_t fcitx5_launcher_ui_command_utf16(
    const std::uint16_t* ui_path, std::size_t ui_path_len, std::uint32_t parent_pid,
    const std::uint16_t* generation, std::size_t generation_len, std::uint8_t safe_mode,
    std::uint16_t* output, std::size_t capacity);
std::size_t fcitx5_launcher_config_command_utf16(
    const std::uint16_t* config_path, std::size_t config_path_len, const std::uint16_t* arguments,
    std::size_t arguments_len, std::uint16_t* output, std::size_t capacity);
}
