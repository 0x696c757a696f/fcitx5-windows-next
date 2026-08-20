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

fcitx::windows::candidate::Snapshot snapshot(std::uint64_t engineEpoch,
                                             std::uint64_t contextId,
                                             std::uint64_t compositionId,
                                             std::uint64_t revision) {
    auto result = snapshot(revision);
    result.engineEpoch = engineEpoch;
    result.contextId = contextId;
    result.compositionId = compositionId;
    return result;
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
    auto returned = snapshot(5);
    returned.contextId = 20;
    returned.compositionId = 30;
    returned.popupAllowed = false;
    if (model.apply(std::move(returned)) != ApplyResult::applied ||
        !model.current() || model.current()->contextId != 20 ||
        model.current()->revision != 5 || model.current()->popupAllowed) {
        std::cerr << "A->B->A candidate snapshot was rejected by global composition ordering\n";
        return 1;
    }
    auto duplicateInactiveA = snapshot(10, 20, 30, 5);
    duplicateInactiveA.popupAllowed = false;
    if (model.apply(std::move(duplicateInactiveA)) != ApplyResult::duplicate) {
        std::cerr << "current-context duplicate was not recognized\n";
        return 1;
    }
    auto newerA = snapshot(10, 20, 30, 6);
    newerA.selected = 1;
    newerA.popupAllowed = false;
    if (model.apply(std::move(newerA)) != ApplyResult::applied ||
        !model.current() || model.current()->contextId != 20 ||
        model.current()->revision != 6 ||
        model.current()->selected != std::optional<std::size_t>{1U} ||
        model.current()->popupAllowed) {
        std::cerr << "newer returned context corrupted candidate metadata\n";
        return 1;
    }
    auto staleReturnedA = snapshot(10, 20, 30, 5);
    if (model.apply(std::move(staleReturnedA)) != ApplyResult::stale ||
        !model.current() || model.current()->revision != 6) {
        std::cerr << "older A revision overwrote newer A state\n";
        return 1;
    }
    auto smallerB = snapshot(10, 22, 2, 1);
    smallerB.preedit = "bo";
    smallerB.candidates = {{3, "1", "bo", ""}};
    smallerB.total = 1;
    if (model.apply(std::move(smallerB)) != ApplyResult::applied ||
        !model.current() || model.current()->contextId != 22 ||
        model.current()->compositionId != 2 || model.current()->candidates.size() != 1) {
        std::cerr << "different-context smaller counters were rejected\n";
        return 1;
    }
    auto finalA = snapshot(10, 20, 30, 7);
    finalA.auxiliaryDown = "visible";
    if (model.apply(std::move(finalA)) != ApplyResult::applied ||
        !model.current() || model.current()->contextId != 20 ||
        model.current()->revision != 7 || model.current()->auxiliaryDown != "visible") {
        std::cerr << "REG-CTX-002 A->B->A newer snapshot failed\n";
        return 1;
    }
    auto oldEpoch = snapshot(9, 20, 30, 6);
    if (model.apply(std::move(oldEpoch)) != ApplyResult::stale ||
        !model.current() || model.current()->engineEpoch != 10) {
        std::cerr << "old engine epoch overwrote current candidate state\n";
        return 1;
    }
    auto newComposition = snapshot(10, 20, 31, 1);
    newComposition.preedit = "xin";
    if (model.apply(std::move(newComposition)) != ApplyResult::applied ||
        !model.current() || model.current()->compositionId != 31 ||
        model.current()->revision != 1) {
        std::cerr << "new same-context composition was rejected by old revision\n";
        return 1;
    }
    auto previousComposition = snapshot(10, 20, 30, 6);
    if (model.apply(std::move(previousComposition)) != ApplyResult::stale ||
        !model.current() || model.current()->compositionId != 31) {
        std::cerr << "previous composition overwrote newer composition\n";
        return 1;
    }
    auto preeditOnly = snapshot(2);
    preeditOnly.contextId = 23;
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
