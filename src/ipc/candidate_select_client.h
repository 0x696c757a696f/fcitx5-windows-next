#pragma once

#include <cstddef>
#include <cstdint>

// Narrow C ABI for the Rust-owned opaque candidate-select client (078 Stage 1).
// The C++ Candidate renderer host holds only the opaque handle returned by
// create and passes plain scalar arguments to select; all connect/peer-verify/
// handshake/candidate-select state and policy live in Rust
// (`rust/windows-common-core/src/candidate_select_client.rs`).
extern "C" {

void* fcitx5_windows_common_candidate_select_client_create_utf16(
    const std::uint16_t* pipe_name,
    std::size_t pipe_name_len,
    const std::uint16_t* expected_peer_path,
    std::size_t expected_peer_path_len);

std::uint8_t fcitx5_windows_common_candidate_select_client_select(
    void* handle,
    std::uint32_t target_process_id,
    std::uint64_t expected_engine_epoch,
    std::uint64_t context_id,
    std::uint64_t composition_id,
    std::uint64_t revision,
    std::uint64_t candidate_id);

void fcitx5_windows_common_candidate_select_client_destroy(void* handle);

} // extern "C"
