#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>

namespace fcitx::windows::ui {

namespace detail {

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

struct Fcitx5CandidateRenderItemInput {
    Fcitx5CandidateLayoutRect bounds{};
    float labelWidth{};
    float textWidth{};
    float commentWidth{};
    std::uint8_t hasLabel{};
};

struct Fcitx5CandidateRenderItemOutput {
    Fcitx5CandidateLayoutRect label{};
    Fcitx5CandidateLayoutRect text{};
    Fcitx5CandidateLayoutRect comment{};
    std::uint8_t drawComment{};
};

extern "C" int fcitx5_candidate_layout_run(const Fcitx5CandidateLayoutInput* input,
                                            const Fcitx5CandidateLayoutSize* items,
                                            std::size_t itemCount,
                                            Fcitx5CandidateLayoutRect* outItems,
                                            std::size_t* outItemIndices,
                                            std::size_t outCapacity,
                                            Fcitx5CandidateLayoutOutput* output);
extern "C" int fcitx5_candidate_render_segments(const Fcitx5CandidateRenderItemInput* items,
                                                 std::size_t itemCount,
                                                 std::uint8_t horizontal,
                                                 std::uint8_t scrollMode,
                                                 Fcitx5CandidateRenderItemOutput* outItems,
                                                 float* outLabelColumnWidth);

} // namespace detail

struct Point {
    float x{};
    float y{};
};
struct Size {
    float width{};
    float height{};
};
struct Rect {
    float left{};
    float top{};
    float right{};
    float bottom{};
};
enum class Orientation { vertical, horizontal };
enum class Placement { unlocked, below, above };

struct LayoutInput {
    Orientation orientation{Orientation::vertical};
    std::vector<Size> items;
    Point caret;
    float caretHeight{};
    Rect workArea;
    float maxWidth{720};
    float paddingX{8};
    float paddingY{6};
    float rowGap{2};
    float columnGap{8};
    Placement placement{Placement::unlocked};
    bool scrollMode{};
    std::size_t scrollColumns{6};
    std::size_t scrollVisibleRows{6};
    std::size_t selected{};
    float scrollCellWidth{96};
};

struct LayoutResult {
    Rect window;
    std::vector<Rect> items;
    std::vector<std::size_t> itemIndices;
    Rect scrollbarTrack{};
    Rect scrollbarThumb{};
    bool hasScrollbar{};
    std::size_t firstVisible{};
    Placement placement{Placement::below};
};

struct RenderItemInput {
    Rect bounds{};
    float labelWidth{};
    float textWidth{};
    float commentWidth{};
    bool hasLabel{};
};

struct RenderItemSegments {
    Rect label{};
    Rect text{};
    Rect comment{};
    bool drawComment{};
};

[[nodiscard]] inline std::uint32_t toRust(Orientation value) noexcept {
    return value == Orientation::horizontal ? 1U : 0U;
}

[[nodiscard]] inline std::uint32_t toRust(Placement value) noexcept {
    switch (value) {
    case Placement::unlocked:
        return 0U;
    case Placement::below:
        return 1U;
    case Placement::above:
        return 2U;
    }
    return 0U;
}

[[nodiscard]] inline Placement placementFromRust(std::uint32_t value) noexcept {
    switch (value) {
    case 1U:
        return Placement::below;
    case 2U:
        return Placement::above;
    default:
        return Placement::unlocked;
    }
}

[[nodiscard]] inline Rect rectFromRust(
    const detail::Fcitx5CandidateLayoutRect& value) noexcept {
    return {value.left, value.top, value.right, value.bottom};
}

[[nodiscard]] inline LayoutResult layout(const LayoutInput& input) {
    std::vector<detail::Fcitx5CandidateLayoutSize> rustItems;
    rustItems.reserve(input.items.size());
    for (const auto& item : input.items)
        rustItems.push_back({item.width, item.height});

    std::vector<detail::Fcitx5CandidateLayoutRect> outItems(input.items.size());
    std::vector<std::size_t> outItemIndices(input.items.size());
    const detail::Fcitx5CandidateLayoutInput rustInput{
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
    detail::Fcitx5CandidateLayoutOutput rustOutput{};
    if (detail::fcitx5_candidate_layout_run(&rustInput, rustItems.data(), rustItems.size(),
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

[[nodiscard]] inline std::vector<RenderItemSegments> renderSegments(
    Orientation orientation, bool scrollMode, const std::vector<RenderItemInput>& items) {
    std::vector<detail::Fcitx5CandidateRenderItemInput> rustInputs;
    rustInputs.reserve(items.size());
    for (const auto& item : items) {
        rustInputs.push_back({
            {item.bounds.left, item.bounds.top, item.bounds.right, item.bounds.bottom},
            item.labelWidth,
            item.textWidth,
            item.commentWidth,
            static_cast<std::uint8_t>(item.hasLabel ? 1U : 0U),
        });
    }

    std::vector<detail::Fcitx5CandidateRenderItemOutput> rustOutputs(items.size());
    float labelColumnWidth = 0.0F;
    if (!items.empty() &&
        detail::fcitx5_candidate_render_segments(
            rustInputs.data(), rustInputs.size(),
            static_cast<std::uint8_t>(orientation == Orientation::horizontal ? 1U : 0U),
            static_cast<std::uint8_t>(scrollMode ? 1U : 0U), rustOutputs.data(),
            &labelColumnWidth) != 0) {
        return {};
    }

    std::vector<RenderItemSegments> result;
    result.reserve(rustOutputs.size());
    for (const auto& output : rustOutputs) {
        result.push_back({
            rectFromRust(output.label),
            rectFromRust(output.text),
            rectFromRust(output.comment),
            output.drawComment != 0,
        });
    }
    return result;
}

} // namespace fcitx::windows::ui
