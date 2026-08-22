#include "candidate_interaction.h"

#include <cstdint>

namespace {

struct Fcitx5CandidateLayoutRect {
    float left{};
    float top{};
    float right{};
    float bottom{};
};

struct Fcitx5CandidateSelectionIntent {
    std::uint32_t targetProcessId{};
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    std::uint64_t candidateId{};
};

extern "C" std::uint8_t fcitx5_candidate_hit_test(const Fcitx5CandidateLayoutRect* rects,
                                                   std::size_t rectCount, float x, float y,
                                                   std::size_t* outIndex);
extern "C" Fcitx5CandidateSelectionIntent fcitx5_candidate_selection_intent(
    std::uint32_t targetProcessId, std::uint64_t engineEpoch, std::uint64_t contextId,
    std::uint64_t compositionId, std::uint64_t revision, std::uint64_t candidateId);

} // namespace

namespace fcitx::windows::ui {

std::optional<std::size_t> hitTestCandidate(const std::vector<D2D1_RECT_F>& itemRects,
                                            float x, float y) noexcept {
    std::vector<Fcitx5CandidateLayoutRect> rustRects;
    rustRects.reserve(itemRects.size());
    for (const auto& rectangle : itemRects)
        rustRects.push_back({rectangle.left, rectangle.top, rectangle.right, rectangle.bottom});
    std::size_t index = 0;
    if (fcitx5_candidate_hit_test(rustRects.data(), rustRects.size(), x, y, &index) == 0)
        return std::nullopt;
    return index;
}

CandidateSelectionIntent makeCandidateSelectionIntent(
    std::uint32_t targetProcessId, std::uint64_t engineEpoch,
    std::uint64_t contextId, std::uint64_t compositionId,
    std::uint64_t revision, std::uint64_t candidateId) noexcept {
    const auto intent = fcitx5_candidate_selection_intent(
        targetProcessId, engineEpoch, contextId, compositionId, revision, candidateId);
    return {intent.targetProcessId, intent.engineEpoch, intent.contextId,
            intent.compositionId, intent.revision, intent.candidateId};
}

} // namespace fcitx::windows::ui
