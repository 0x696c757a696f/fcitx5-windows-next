#pragma once

#include <cstdint>
#include <deque>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <utility>
#include <vector>

namespace fcitx::windows::candidate {

inline constexpr std::size_t kMaxCandidates = 128;
inline constexpr std::size_t kMaxCandidateTextUtf8 = 4096;
inline constexpr std::size_t kMaxTrackedContexts = 64;

enum class Visibility : std::uint8_t { hidden, composition, prediction };

struct Item {
    std::uint64_t id{};
    std::string label;
    std::string text;
    std::string comment;

    bool operator==(const Item&) const = default;
};

struct Snapshot {
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    std::string preedit;
    std::string auxiliaryUp;
    std::string auxiliaryDown;
    std::vector<Item> candidates;
    std::optional<std::size_t> selected;
    std::uint32_t page{};
    std::uint32_t total{};
    Visibility visibility{Visibility::hidden};
    bool popupAllowed{true};

    bool operator==(const Snapshot&) const = default;
};

enum class ApplyResult { applied, duplicate, stale, invalid };

namespace detail {

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

[[nodiscard]] inline Fcitx5CandidateUtf8 toRust(std::string_view value) noexcept {
    return {reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

[[nodiscard]] inline std::uint8_t toRust(Visibility visibility) noexcept {
    switch (visibility) {
    case Visibility::hidden:
        return 0U;
    case Visibility::composition:
        return 1U;
    case Visibility::prediction:
        return 2U;
    }
    return 0U;
}

[[nodiscard]] inline std::vector<Fcitx5CandidateModelItem> itemsToRust(
    const Snapshot& snapshot) {
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

[[nodiscard]] inline Fcitx5CandidateModelSnapshot toRust(
    const Snapshot& snapshot,
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

} // namespace detail

class CandidateModel final {
public:
    CandidateModel() : rustModel_(detail::fcitx5_candidate_model_create()) {}
    ~CandidateModel() { detail::fcitx5_candidate_model_destroy(rustModel_); }
    CandidateModel(const CandidateModel&) = delete;
    CandidateModel& operator=(const CandidateModel&) = delete;
    CandidateModel(CandidateModel&&) = delete;
    CandidateModel& operator=(CandidateModel&&) = delete;

    [[nodiscard]] ApplyResult apply(Snapshot snapshot) {
        const auto candidates = detail::itemsToRust(snapshot);
        const auto rustSnapshot = detail::toRust(snapshot, candidates);
        switch (detail::fcitx5_candidate_model_apply(rustModel_, &rustSnapshot)) {
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
    void reset() noexcept {
        detail::fcitx5_candidate_model_reset(rustModel_);
        current_.reset();
    }
    [[nodiscard]] const std::optional<Snapshot>& current() const noexcept { return current_; }

private:
    std::optional<Snapshot> current_;
    void* rustModel_{};
};

[[nodiscard]] inline bool validate(const Snapshot& snapshot) noexcept {
    const auto candidates = detail::itemsToRust(snapshot);
    const auto rustSnapshot = detail::toRust(snapshot, candidates);
    return detail::fcitx5_candidate_model_validate(&rustSnapshot) != 0;
}

} // namespace fcitx::windows::candidate
