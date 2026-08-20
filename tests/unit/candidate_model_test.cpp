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
    auto uiless = snapshot(3);
    uiless.popupAllowed = false;
    uiless.selected = 1;
    if (model.apply(std::move(uiless)) != ApplyResult::applied ||
        !model.current() || model.current()->popupAllowed ||
        model.current()->candidates.size() != 2 ||
        model.current()->selected != std::optional<std::size_t>{1U}) {
        std::cerr << "UILess policy suppressed candidate semantics\n";
        return 1;
    }
    auto stale = snapshot(4);
    stale.engineEpoch = 9;
    if (model.apply(std::move(stale)) != ApplyResult::stale) return 1;
    auto prediction = snapshot(4);
    prediction.preedit.clear();
    prediction.visibility = Visibility::prediction;
    if (model.apply(std::move(prediction)) != ApplyResult::applied) {
        std::cerr << "empty-preedit prediction was hidden\n";
        return 1;
    }
    auto invalid = snapshot(5);
    invalid.selected = 5;
    if (model.apply(std::move(invalid)) != ApplyResult::invalid) return 1;
    auto switched = snapshot(1);
    switched.contextId = 21;
    switched.compositionId = 40;
    if (model.apply(std::move(switched)) != ApplyResult::applied ||
        !model.current() || model.current()->contextId != 21 ||
        !model.current()->popupAllowed) {
        std::cerr << "active context switch was rejected\n";
        return 1;
    }
    auto returned = snapshot(3);
    returned.contextId = 20;
    returned.compositionId = 30;
    returned.popupAllowed = false;
    if (model.apply(std::move(returned)) != ApplyResult::applied ||
        !model.current() || model.current()->contextId != 20 ||
        model.current()->revision != 3 || model.current()->popupAllowed) {
        std::cerr << "A->B->A candidate snapshot was rejected by global composition ordering\n";
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
    model.reset();
    if (model.current()) {
        std::cerr << "context end retained presentation policy\n";
        return 1;
    }
    auto reconnected = snapshot(1);
    reconnected.popupAllowed = false;
    if (model.apply(std::move(reconnected)) != ApplyResult::applied ||
        !model.current() || model.current()->popupAllowed ||
        model.current()->candidates.size() != 2) {
        std::cerr << "reconnect did not derive policy from authoritative snapshot\n";
        return 1;
    }
    return 0;
}
