#include "candidate_layout.h"

#include <algorithm>

namespace fcitx::windows::ui {

LayoutResult layout(const LayoutInput& input) {
    LayoutResult result;
    float contentWidth = 0;
    float contentHeight = 0;
    if (input.orientation == Orientation::vertical) {
        for (const auto item : input.items) {
            contentWidth = std::max(contentWidth, item.width);
            if (contentHeight > 0) contentHeight += input.rowGap;
            contentHeight += item.height;
        }
    } else {
        for (const auto item : input.items) {
            if (contentWidth > 0) contentWidth += input.columnGap;
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
    const float left = std::clamp(input.caret.x, input.workArea.left,
                                  input.workArea.right - width);
    result.window = {left, top, left + width, top + height};
    result.placement = placement;
    float x = left + input.paddingX;
    float y = top + input.paddingY;
    for (const auto item : input.items) {
        const float itemWidth = std::min(item.width,
                                         std::max(0.0F, width - input.paddingX * 2));
        result.items.push_back({x, y, x + itemWidth, y + item.height});
        if (input.orientation == Orientation::vertical) y += item.height + input.rowGap;
        else x += item.width + input.columnGap;
    }
    return result;
}

} // namespace fcitx::windows::ui
