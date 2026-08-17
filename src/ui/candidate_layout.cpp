#include "candidate_layout.h"

#include <algorithm>

namespace fcitx::windows::ui {

LayoutResult layout(const LayoutInput& input) {
    LayoutResult result;
    if (input.scrollMode && !input.items.empty()) {
        const std::size_t columns = std::clamp<std::size_t>(input.scrollColumns, 1U, 6U);
        const std::size_t visibleRows = std::clamp<std::size_t>(input.scrollVisibleRows, 1U, 6U);
        const std::size_t rows = (input.items.size() + columns - 1U) / columns;
        const std::size_t selectedRow = std::min(input.selected, input.items.size() - 1U) / columns;
        // Keep the active page at the top like the native macOS scroll panel. Near the
        // end, backfill earlier rows so the viewport does not leave a large empty area.
        const std::size_t firstRow =
            rows > visibleRows ? std::min(selectedRow, rows - visibleRows) : 0U;
        const std::size_t shownRows = std::min(visibleRows, rows - firstRow);
        float rowHeight = 0.0F;
        for (const auto& item : input.items)
            rowHeight = std::max(rowHeight, item.height);
        const float workWidth = std::max(0.0F, input.workArea.right - input.workArea.left);
        const float workHeight = std::max(0.0F, input.workArea.bottom - input.workArea.top);
        const float desiredWidth = std::min(input.maxWidth, workWidth);
        const float width = std::max(0.0F, desiredWidth);
        const float height =
            std::min(input.paddingY * 2.0F + rowHeight * static_cast<float>(shownRows) +
                         input.rowGap * static_cast<float>(shownRows - 1U),
                     workHeight);
        const float below = input.caret.y + input.caretHeight;
        auto placement = input.placement;
        if (placement == Placement::unlocked)
            placement =
                below + height <= input.workArea.bottom ? Placement::below : Placement::above;
        const float top = std::clamp(placement == Placement::below ? below : input.caret.y - height,
                                     input.workArea.top, input.workArea.bottom - height);
        const float left =
            std::clamp(input.caret.x, input.workArea.left, input.workArea.right - width);
        result.window = {left, top, left + width, top + height};
        result.placement = placement;
        result.firstVisible = firstRow * columns;
        const float scrollbarWidth = rows > shownRows ? 8.0F : 0.0F;
        const float usableWidth =
            std::max(0.0F, width - input.paddingX * 2.0F - scrollbarWidth -
                               input.columnGap * static_cast<float>(columns - 1U));
        const float cellWidth = usableWidth / static_cast<float>(columns);
        const std::size_t end = std::min(input.items.size(), (firstRow + shownRows) * columns);
        for (std::size_t index = result.firstVisible; index < end; ++index) {
            const std::size_t local = index - result.firstVisible;
            const std::size_t row = local / columns;
            const std::size_t column = local % columns;
            const float x =
                left + input.paddingX + static_cast<float>(column) * (cellWidth + input.columnGap);
            const float y =
                top + input.paddingY + static_cast<float>(row) * (rowHeight + input.rowGap);
            result.items.push_back({x, y, x + cellWidth, y + rowHeight});
            result.itemIndices.push_back(index);
        }
        if (rows > shownRows) {
            result.hasScrollbar = true;
            result.scrollbarTrack = {left + width - 6.0F, top + input.paddingY, left + width - 2.0F,
                                     top + height - input.paddingY};
            const float trackHeight = result.scrollbarTrack.bottom - result.scrollbarTrack.top;
            const float thumbHeight = std::max(18.0F, trackHeight * static_cast<float>(shownRows) /
                                                          static_cast<float>(rows));
            const float progress = rows == shownRows ? 0.0F
                                                     : static_cast<float>(firstRow) /
                                                           static_cast<float>(rows - shownRows);
            const float thumbTop =
                result.scrollbarTrack.top + (trackHeight - thumbHeight) * progress;
            result.scrollbarThumb = {result.scrollbarTrack.left, thumbTop,
                                     result.scrollbarTrack.right, thumbTop + thumbHeight};
        }
        return result;
    }
    float contentWidth = 0;
    float contentHeight = 0;
    if (input.orientation == Orientation::vertical) {
        for (const auto item : input.items) {
            contentWidth = std::max(contentWidth, item.width);
            if (contentHeight > 0)
                contentHeight += input.rowGap;
            contentHeight += item.height;
        }
    } else {
        for (const auto item : input.items) {
            if (contentWidth > 0)
                contentWidth += input.columnGap;
            contentWidth += item.width;
            contentHeight = std::max(contentHeight, item.height);
        }
    }
    const float workWidth = std::max(0.0F, input.workArea.right - input.workArea.left);
    const float workHeight = std::max(0.0F, input.workArea.bottom - input.workArea.top);
    const float width = std::min({contentWidth + input.paddingX * 2, input.maxWidth, workWidth});
    const float height = std::min(contentHeight + input.paddingY * 2, workHeight);
    const float below = input.caret.y + input.caretHeight;
    Placement placement = input.placement;
    if (placement == Placement::unlocked) {
        placement = below + height <= input.workArea.bottom ? Placement::below : Placement::above;
    }
    float top = placement == Placement::below ? below : input.caret.y - height;
    top = std::clamp(top, input.workArea.top, input.workArea.bottom - height);
    const float left = std::clamp(input.caret.x, input.workArea.left, input.workArea.right - width);
    result.window = {left, top, left + width, top + height};
    result.placement = placement;
    float x = left + input.paddingX;
    float y = top + input.paddingY;
    for (const auto item : input.items) {
        const float itemWidth = std::min(item.width, std::max(0.0F, width - input.paddingX * 2));
        result.items.push_back({x, y, x + itemWidth, y + item.height});
        result.itemIndices.push_back(result.itemIndices.size());
        if (input.orientation == Orientation::vertical)
            y += item.height + input.rowGap;
        else
            x += item.width + input.columnGap;
    }
    return result;
}

} // namespace fcitx::windows::ui
