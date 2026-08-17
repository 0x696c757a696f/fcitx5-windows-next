#include "candidate_model.h"

#include <iostream>

namespace {

fcitx::windows::candidate::Snapshot snapshot(std::uint64_t revision) {
    using namespace fcitx::windows::candidate;
    return Snapshot{10, 20, 30, revision, "ni", {}, {},
                    {{1, "1", "\xe4\xbd\xa0", "n\xc7\x90"},
                     {2, "2", "\xe5\x91\xa2", {}}},
                    0, 0, 2, Visibility::composition};
}

} // namespace

int main() {
    using namespace fcitx::windows::candidate;
    CandidateModel model;
    if (model.apply(snapshot(1)) != ApplyResult::applied ||
        model.apply(snapshot(1)) != ApplyResult::duplicate ||
        model.apply(snapshot(0)) != ApplyResult::invalid ||
        model.apply(snapshot(2)) != ApplyResult::applied) {
        std::cerr << "candidate revision contract failed\n";
        return 1;
    }
    auto stale = snapshot(3);
    stale.engineEpoch = 9;
    if (model.apply(std::move(stale)) != ApplyResult::stale) return 1;
    auto prediction = snapshot(3);
    prediction.preedit.clear();
    prediction.visibility = Visibility::prediction;
    if (model.apply(std::move(prediction)) != ApplyResult::applied) {
        std::cerr << "empty-preedit prediction was hidden\n";
        return 1;
    }
    auto invalid = snapshot(4);
    invalid.selected = 5;
    if (model.apply(std::move(invalid)) != ApplyResult::invalid) return 1;
    auto switched = snapshot(1);
    switched.contextId = 21;
    if (model.apply(std::move(switched)) != ApplyResult::applied ||
        !model.current() || model.current()->contextId != 21) {
        std::cerr << "active context switch was rejected\n";
        return 1;
    }
    auto preeditOnly = snapshot(2);
    preeditOnly.contextId = 21;
    preeditOnly.candidates.clear();
    preeditOnly.selected.reset();
    preeditOnly.total = 0;
    preeditOnly.visibility = Visibility::hidden;
    if (model.apply(std::move(preeditOnly)) != ApplyResult::applied) {
        std::cerr << "preedit-only snapshot was rejected\n";
        return 1;
    }
    return 0;
}
