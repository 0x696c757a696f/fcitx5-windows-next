#include "candidate_interaction.h"

namespace fcitx::windows::ui {

std::optional<std::size_t> hitTestCandidate(const std::vector<D2D1_RECT_F>& itemRects,
                                            float x, float y) noexcept {
    for (std::size_t index = 0; index < itemRects.size(); ++index) {
        const auto& rectangle = itemRects[index];
        if (x >= rectangle.left && x < rectangle.right && y >= rectangle.top &&
            y < rectangle.bottom) {
            return index;
        }
    }
    return std::nullopt;
}

CandidateSelectionIntent makeCandidateSelectionIntent(
    std::uint32_t targetProcessId, std::uint64_t engineEpoch,
    std::uint64_t contextId, std::uint64_t compositionId,
    std::uint64_t revision, std::uint64_t candidateId) noexcept {
    CandidateSelectionIntent result{targetProcessId, engineEpoch, contextId,
                                    compositionId, revision, candidateId};
    return result.valid() ? result : CandidateSelectionIntent{};
}

} // namespace fcitx::windows::ui
