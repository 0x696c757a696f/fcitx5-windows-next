#include "candidate_layout.h"

#include <algorithm>

namespace fcitx::windows::ui {

LayoutResult layout(const LayoutInput& input) {
    LayoutResult result;
    if (input.scrollMode && !input.items.empty()) {
        const float preferredScrollCellWidth = std::max(40.0F, input.scrollCellWidth);
        if (input.orientation == Orientation::vertical) {
            const std::size_t rowsPerColumn =
                std::clamp<std::size_t>(input.scrollColumns, 1U, 9U);
            const std::size_t visibleColumns =
                std::clamp<std::size_t>(input.scrollVisibleRows, 1U, 6U);
            const std::size_t columns = (input.items.size() + rowsPerColumn - 1U) / rowsPerColumn;
            const std::size_t selectedColumn =
                std::min(input.selected, input.items.size() - 1U) / rowsPerColumn;
            const std::size_t viewportStart =
                (selectedColumn / visibleColumns) * visibleColumns;
            const std::size_t firstColumn =
                columns > visibleColumns ? std::min(viewportStart, columns - visibleColumns) : 0U;
            std::size_t shownColumns = std::min(visibleColumns, columns - firstColumn);
            float rowHeight = 0.0F;
            for (const auto& item : input.items)
                rowHeight = std::max(rowHeight, item.height);
            const float workWidth = std::max(0.0F, input.workArea.right - input.workArea.left);
            const float workHeight = std::max(0.0F, input.workArea.bottom - input.workArea.top);
            const float targetWidth =
                std::min(workWidth, input.maxWidth > 0.0F ? input.maxWidth : workWidth);
            const auto buildColumnWidths = [&](std::size_t count) {
                std::vector<float> widths(count, 0.0F);
                for (std::size_t column = 0; column < count; ++column) {
                    for (std::size_t row = 0; row < rowsPerColumn; ++row) {
                        const std::size_t index = (firstColumn + column) * rowsPerColumn + row;
                        if (index >= input.items.size())
                            break;
                        widths[column] = std::max(widths[column], input.items[index].width);
                    }
                    widths[column] =
                        std::clamp(widths[column], 40.0F, preferredScrollCellWidth);
                }
                return widths;
            };
            const auto naturalWidth = [&](const std::vector<float>& widths) {
                float columnsWidth = 0.0F;
                for (const auto width : widths)
                    columnsWidth += width;
                const float columnGaps =
                    input.columnGap *
                    static_cast<float>(widths.empty() ? 0U : widths.size() - 1U);
                return columnsWidth + columnGaps + input.paddingX * 2.0F;
            };
            auto columnWidths = buildColumnWidths(shownColumns);
            // Vertical scroll is column-major. Match fcitx5-macos' dedicated
            // scroll cell policy: cells have a bounded width, long candidates
            // are ellipsized by the renderer, and the grid never grows wider
            // than the configured candidate max width unless the work area is
            // smaller. If the bounded six-column viewport still cannot fit,
            // reduce visible columns before clipping a single column.
            while (shownColumns > 1U && naturalWidth(columnWidths) > targetWidth) {
                --shownColumns;
                columnWidths = buildColumnWidths(shownColumns);
            }
            const std::size_t firstVisible = firstColumn * rowsPerColumn;
            const std::size_t end =
                std::min(input.items.size(), (firstColumn + shownColumns) * rowsPerColumn);
            const float width = std::min(naturalWidth(columnWidths), targetWidth);
            if (shownColumns == 1U && width < naturalWidth(columnWidths)) {
                columnWidths[0] = std::max(1.0F, width - input.paddingX * 2.0F);
            }
            const float height =
                std::min(input.paddingY * 2.0F + rowHeight * static_cast<float>(rowsPerColumn) +
                             input.rowGap * static_cast<float>(rowsPerColumn - 1U),
                         workHeight);
            const float below = input.caret.y + input.caretHeight;
            auto placement = input.placement;
            if (placement == Placement::unlocked)
                placement =
                    below + height <= input.workArea.bottom ? Placement::below : Placement::above;
            const float top = std::clamp(
                placement == Placement::below ? below : input.caret.y - height,
                input.workArea.top, input.workArea.bottom - height);
            const float left =
                std::clamp(input.caret.x, input.workArea.left, input.workArea.right - width);
            result.window = {left, top, left + width, top + height};
            result.placement = placement;
            result.firstVisible = firstVisible;
            float x = left + input.paddingX;
            for (std::size_t column = 0; column < shownColumns; ++column) {
                for (std::size_t row = 0; row < rowsPerColumn; ++row) {
                    const std::size_t index = (firstColumn + column) * rowsPerColumn + row;
                    if (index >= end)
                        break;
                    const float y =
                        top + input.paddingY + static_cast<float>(row) *
                                                   (rowHeight + input.rowGap);
                    result.items.push_back({x, y, x + columnWidths[column], y + rowHeight});
                    result.itemIndices.push_back(index);
                }
                x += columnWidths[column] + input.columnGap;
            }
            if (columns > shownColumns) {
                result.hasScrollbar = true;
                result.scrollbarTrack = {left + width - 6.0F, top + input.paddingY,
                                         left + width - 2.0F, top + height - input.paddingY};
                const float trackHeight = result.scrollbarTrack.bottom - result.scrollbarTrack.top;
                const float thumbHeight =
                    std::max(18.0F, trackHeight * static_cast<float>(shownColumns) /
                                         static_cast<float>(columns));
                const float progress =
                    columns == shownColumns ? 0.0F
                                            : static_cast<float>(firstColumn) /
                                                  static_cast<float>(columns - shownColumns);
                const float thumbTop =
                    result.scrollbarTrack.top + (trackHeight - thumbHeight) * progress;
                result.scrollbarThumb = {result.scrollbarTrack.left, thumbTop,
                                         result.scrollbarTrack.right, thumbTop + thumbHeight};
            }
            return result;
        }
        const std::size_t columns =
            std::clamp<std::size_t>(input.scrollColumns, 1U, 9U);
        const std::size_t visibleRows =
            std::clamp<std::size_t>(input.scrollVisibleRows, 1U, 6U);
        const std::size_t rows = (input.items.size() + columns - 1U) / columns;
        const std::size_t selectedRow = std::min(input.selected, input.items.size() - 1U) / columns;
        // Keep a stable six-row viewport while focus moves inside it. Advancing
        // from row 1 to row 2 must not make row 2 jump to the top; move the
        // viewport only after focus crosses a visible-row boundary.
        const std::size_t viewportStart = (selectedRow / visibleRows) * visibleRows;
        const std::size_t firstRow =
            rows > visibleRows ? std::min(viewportStart, rows - visibleRows) : 0U;
        const std::size_t shownRows = std::min(visibleRows, rows - firstRow);
        const std::size_t firstVisible = firstRow * columns;
        const std::size_t end = std::min(input.items.size(), (firstRow + shownRows) * columns);
        float rowHeight = 0.0F;
        for (const auto& item : input.items)
            rowHeight = std::max(rowHeight, item.height);
        const float workWidth = std::max(0.0F, input.workArea.right - input.workArea.left);
        const float workHeight = std::max(0.0F, input.workArea.bottom - input.workArea.top);
        float contentWidth = 0.0F;
        for (std::size_t row = 0; row < shownRows; ++row) {
            float rowWidth = 0.0F;
            for (std::size_t column = 0; column < columns; ++column) {
                const std::size_t index = firstVisible + row * columns + column;
                if (index >= end)
                    break;
                if (rowWidth > 0.0F)
                    rowWidth += input.columnGap;
                rowWidth += input.items[index].width;
            }
            contentWidth = std::max(contentWidth, rowWidth);
        }
        const float width =
            std::min({contentWidth + input.paddingX * 2.0F, input.maxWidth, workWidth});
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
        result.firstVisible = firstVisible;
        const float usableWidth =
            std::max(0.0F,
                     width - input.paddingX * 2.0F -
                         input.columnGap * static_cast<float>(columns - 1U));
        const float cellWidth = usableWidth / static_cast<float>(columns);
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
