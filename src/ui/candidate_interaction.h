#pragma once

#include <d2d1.h>

#include <cstddef>
#include <cstdint>
#include <optional>
#include <vector>

namespace fcitx::windows::ui {

struct CandidateSelectionIntent {
    std::uint32_t targetProcessId{};
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    std::uint64_t candidateId{};

    [[nodiscard]] bool valid() const noexcept {
        return targetProcessId != 0 && engineEpoch != 0 && contextId != 0 &&
               compositionId != 0 && revision != 0 && candidateId != 0;
    }
};

[[nodiscard]] std::optional<std::size_t> hitTestCandidate(
    const std::vector<D2D1_RECT_F>& itemRects, float x, float y) noexcept;

[[nodiscard]] CandidateSelectionIntent makeCandidateSelectionIntent(
    std::uint32_t targetProcessId, std::uint64_t engineEpoch,
    std::uint64_t contextId, std::uint64_t compositionId,
    std::uint64_t revision, std::uint64_t candidateId) noexcept;

} // namespace fcitx::windows::ui
