#include "candidate_interaction.h"

#include <iostream>

int main() {
    using namespace fcitx::windows::ui;

    const std::vector<D2D1_RECT_F> rectangles{
        D2D1::RectF(8.0F, 8.0F, 120.0F, 36.0F),
        D2D1::RectF(8.0F, 38.0F, 120.0F, 66.0F),
    };
    if (hitTestCandidate(rectangles, 20.0F, 50.0F) != 1U ||
        hitTestCandidate(rectangles, 4.0F, 50.0F).has_value() ||
        hitTestCandidate(rectangles, 20.0F, 37.0F).has_value()) {
        std::cerr << "candidate hit testing contract failed\n";
        return 1;
    }

    const auto intent = makeCandidateSelectionIntent(41U, 9U, 10U, 11U, 12U, 13U);
    if (!intent.valid() || intent.targetProcessId != 41U || intent.engineEpoch != 9U ||
        intent.contextId != 10U || intent.compositionId != 11U || intent.revision != 12U ||
        intent.candidateId != 13U) {
        std::cerr << "semantic candidate identity was not preserved\n";
        return 1;
    }

    if (makeCandidateSelectionIntent(0U, 9U, 10U, 11U, 12U, 13U).valid() ||
        makeCandidateSelectionIntent(41U, 0U, 10U, 11U, 12U, 13U).valid() ||
        makeCandidateSelectionIntent(41U, 9U, 10U, 0U, 12U, 13U).valid() ||
        makeCandidateSelectionIntent(41U, 9U, 10U, 11U, 12U, 0U).valid()) {
        std::cerr << "incomplete candidate identity was accepted\n";
        return 1;
    }
    return 0;
}
