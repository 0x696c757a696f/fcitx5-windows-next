#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace fcitx::windows::candidate {

inline constexpr std::size_t kMaxCandidates = 128;
inline constexpr std::size_t kMaxCandidateTextUtf8 = 4096;

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
    std::optional<Snapshot> current_;
};

[[nodiscard]] bool validate(const Snapshot& snapshot) noexcept;

} // namespace fcitx::windows::candidate
