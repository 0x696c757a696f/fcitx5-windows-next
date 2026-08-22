#include "candidate_layout.h"

#include <cstdint>

namespace {

struct Fcitx5CandidateLayoutPoint {
    float x{};
    float y{};
};

struct Fcitx5CandidateLayoutSize {
    float width{};
    float height{};
};

struct Fcitx5CandidateLayoutRect {
    float left{};
    float top{};
    float right{};
    float bottom{};
};

struct Fcitx5CandidateLayoutInput {
    std::uint32_t orientation{};
    Fcitx5CandidateLayoutPoint caret{};
    float caretHeight{};
    Fcitx5CandidateLayoutRect workArea{};
    float maxWidth{};
    float paddingX{};
    float paddingY{};
    float rowGap{};
    float columnGap{};
    std::uint32_t placement{};
    std::uint8_t scrollMode{};
    std::size_t scrollColumns{};
    std::size_t scrollVisibleRows{};
    std::size_t selected{};
    float scrollCellWidth{};
};

struct Fcitx5CandidateLayoutOutput {
    Fcitx5CandidateLayoutRect window{};
    Fcitx5CandidateLayoutRect scrollbarTrack{};
    Fcitx5CandidateLayoutRect scrollbarThumb{};
    std::uint8_t hasScrollbar{};
    std::size_t firstVisible{};
    std::uint32_t placement{};
    std::size_t itemCount{};
};

extern "C" int fcitx5_candidate_layout_run(const Fcitx5CandidateLayoutInput* input,
                                            const Fcitx5CandidateLayoutSize* items,
                                            std::size_t itemCount,
                                            Fcitx5CandidateLayoutRect* outItems,
                                            std::size_t* outItemIndices,
                                            std::size_t outCapacity,
                                            Fcitx5CandidateLayoutOutput* output);

[[nodiscard]] std::uint32_t toRust(fcitx::windows::ui::Orientation value) noexcept {
    return value == fcitx::windows::ui::Orientation::horizontal ? 1U : 0U;
}

[[nodiscard]] std::uint32_t toRust(fcitx::windows::ui::Placement value) noexcept {
    switch (value) {
    case fcitx::windows::ui::Placement::unlocked:
        return 0U;
    case fcitx::windows::ui::Placement::below:
        return 1U;
    case fcitx::windows::ui::Placement::above:
        return 2U;
    }
    return 0U;
}

[[nodiscard]] fcitx::windows::ui::Placement placementFromRust(std::uint32_t value) noexcept {
    switch (value) {
    case 1U:
        return fcitx::windows::ui::Placement::below;
    case 2U:
        return fcitx::windows::ui::Placement::above;
    default:
        return fcitx::windows::ui::Placement::unlocked;
    }
}

[[nodiscard]] fcitx::windows::ui::Rect rectFromRust(
    const Fcitx5CandidateLayoutRect& value) noexcept {
    return {value.left, value.top, value.right, value.bottom};
}

} // namespace

namespace fcitx::windows::ui {

LayoutResult layout(const LayoutInput& input) {
    std::vector<Fcitx5CandidateLayoutSize> rustItems;
    rustItems.reserve(input.items.size());
    for (const auto& item : input.items)
        rustItems.push_back({item.width, item.height});

    std::vector<Fcitx5CandidateLayoutRect> outItems(input.items.size());
    std::vector<std::size_t> outItemIndices(input.items.size());
    const Fcitx5CandidateLayoutInput rustInput{
        toRust(input.orientation),
        {input.caret.x, input.caret.y},
        input.caretHeight,
        {input.workArea.left, input.workArea.top, input.workArea.right, input.workArea.bottom},
        input.maxWidth,
        input.paddingX,
        input.paddingY,
        input.rowGap,
        input.columnGap,
        toRust(input.placement),
        static_cast<std::uint8_t>(input.scrollMode ? 1U : 0U),
        input.scrollColumns,
        input.scrollVisibleRows,
        input.selected,
        input.scrollCellWidth,
    };
    Fcitx5CandidateLayoutOutput rustOutput{};
    if (fcitx5_candidate_layout_run(&rustInput, rustItems.data(), rustItems.size(),
                                    outItems.data(), outItemIndices.data(), outItems.size(),
                                    &rustOutput) != 0) {
        return {};
    }

    LayoutResult result;
    result.window = rectFromRust(rustOutput.window);
    result.scrollbarTrack = rectFromRust(rustOutput.scrollbarTrack);
    result.scrollbarThumb = rectFromRust(rustOutput.scrollbarThumb);
    result.hasScrollbar = rustOutput.hasScrollbar != 0;
    result.firstVisible = rustOutput.firstVisible;
    result.placement = placementFromRust(rustOutput.placement);
    result.items.reserve(rustOutput.itemCount);
    result.itemIndices.reserve(rustOutput.itemCount);
    for (std::size_t index = 0; index < rustOutput.itemCount; ++index) {
        result.items.push_back(rectFromRust(outItems[index]));
        result.itemIndices.push_back(outItemIndices[index]);
    }
    return result;
}

} // namespace fcitx::windows::ui
