#pragma once

#include <cstdint>
#include <deque>
#include <optional>
#include <string>
#include <unordered_map>
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

class CandidateModel final {
public:
    [[nodiscard]] ApplyResult apply(Snapshot snapshot);
    void reset() noexcept;
    [[nodiscard]] const std::optional<Snapshot>& current() const noexcept;

private:
    struct Freshness {
        std::uint64_t compositionId{};
        std::uint64_t latestCompositionId{};
        std::uint64_t revision{};
    };

    void rememberContext(std::uint64_t contextId, const Snapshot& snapshot);

    std::optional<Snapshot> current_;
    std::uint64_t engineEpoch_{};
    std::unordered_map<std::uint64_t, Freshness> freshness_;
    std::deque<std::uint64_t> freshnessOrder_;
};

[[nodiscard]] bool validate(const Snapshot& snapshot) noexcept;

} // namespace fcitx::windows::candidate
