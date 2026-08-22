#include "candidate_model.h"

#include <cstdint>
#include <utility>

namespace {

struct Fcitx5CandidateUtf8 {
    const std::uint8_t* ptr{};
    std::size_t len{};
};

struct Fcitx5CandidateModelItem {
    std::uint64_t id{};
    Fcitx5CandidateUtf8 label{};
    Fcitx5CandidateUtf8 text{};
    Fcitx5CandidateUtf8 comment{};
};

struct Fcitx5CandidateModelSnapshot {
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    Fcitx5CandidateUtf8 preedit{};
    Fcitx5CandidateUtf8 auxiliaryUp{};
    Fcitx5CandidateUtf8 auxiliaryDown{};
    const Fcitx5CandidateModelItem* candidates{};
    std::size_t candidateCount{};
    std::size_t selected{};
    std::uint8_t hasSelected{};
    std::uint32_t page{};
    std::uint32_t total{};
    std::uint8_t visibility{};
    std::uint8_t popupAllowed{};
};

extern "C" void* fcitx5_candidate_model_create();
extern "C" void fcitx5_candidate_model_destroy(void* model);
extern "C" void fcitx5_candidate_model_reset(void* model);
extern "C" std::uint8_t fcitx5_candidate_model_validate(
    const Fcitx5CandidateModelSnapshot* snapshot);
extern "C" std::uint32_t fcitx5_candidate_model_apply(
    void* model, const Fcitx5CandidateModelSnapshot* snapshot);

[[nodiscard]] Fcitx5CandidateUtf8 toRust(std::string_view value) noexcept {
    return {reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

[[nodiscard]] std::uint8_t toRust(fcitx::windows::candidate::Visibility visibility) noexcept {
    switch (visibility) {
    case fcitx::windows::candidate::Visibility::hidden:
        return 0U;
    case fcitx::windows::candidate::Visibility::composition:
        return 1U;
    case fcitx::windows::candidate::Visibility::prediction:
        return 2U;
    }
    return 0U;
}

[[nodiscard]] Fcitx5CandidateModelSnapshot toRust(
    const fcitx::windows::candidate::Snapshot& snapshot,
    const std::vector<Fcitx5CandidateModelItem>& candidates) noexcept {
    return {
        snapshot.engineEpoch,
        snapshot.contextId,
        snapshot.compositionId,
        snapshot.revision,
        toRust(snapshot.preedit),
        toRust(snapshot.auxiliaryUp),
        toRust(snapshot.auxiliaryDown),
        candidates.data(),
        candidates.size(),
        snapshot.selected.value_or(0U),
        static_cast<std::uint8_t>(snapshot.selected ? 1U : 0U),
        snapshot.page,
        snapshot.total,
        toRust(snapshot.visibility),
        static_cast<std::uint8_t>(snapshot.popupAllowed ? 1U : 0U),
    };
}

[[nodiscard]] std::vector<Fcitx5CandidateModelItem> itemsToRust(
    const fcitx::windows::candidate::Snapshot& snapshot) {
    std::vector<Fcitx5CandidateModelItem> result;
    result.reserve(snapshot.candidates.size());
    for (const auto& item : snapshot.candidates) {
        result.push_back({
            item.id,
            toRust(item.label),
            toRust(item.text),
            toRust(item.comment),
        });
    }
    return result;
}

} // namespace

namespace fcitx::windows::candidate {

CandidateModel::CandidateModel() : rustModel_(fcitx5_candidate_model_create()) {}

CandidateModel::~CandidateModel() { fcitx5_candidate_model_destroy(rustModel_); }

bool validate(const Snapshot& snapshot) noexcept {
    const auto candidates = itemsToRust(snapshot);
    const auto rustSnapshot = toRust(snapshot, candidates);
    return fcitx5_candidate_model_validate(&rustSnapshot) != 0;
}

ApplyResult CandidateModel::apply(Snapshot snapshot) {
    const auto candidates = itemsToRust(snapshot);
    const auto rustSnapshot = toRust(snapshot, candidates);
    switch (fcitx5_candidate_model_apply(rustModel_, &rustSnapshot)) {
    case 0U:
        current_ = std::move(snapshot);
        return ApplyResult::applied;
    case 1U:
        return ApplyResult::duplicate;
    case 2U:
        return ApplyResult::stale;
    default:
        return ApplyResult::invalid;
    }
}

void CandidateModel::reset() noexcept {
    fcitx5_candidate_model_reset(rustModel_);
    current_.reset();
}

const std::optional<Snapshot>& CandidateModel::current() const noexcept { return current_; }

} // namespace fcitx::windows::candidate
