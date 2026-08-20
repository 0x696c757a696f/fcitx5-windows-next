#include "candidate_layout.h"

#include <cmath>
#include <iostream>

int main() {
    using namespace fcitx::windows::ui;
    LayoutInput input{Orientation::vertical,
                      {{100, 24}, {140, 24}},
                      {1900, 1060},
                      20,
                      {0, 0, 1920, 1080},
                      720,
                      8,
                      6,
                      2,
                      8,
                      Placement::unlocked};
    const auto first = layout(input);
    if (first.placement != Placement::above || first.window.right > 1920 ||
        first.window.bottom > 1080 || first.items.size() != 2)
        return 1;
    input.placement = first.placement;
    input.items[1].width = 300;
    const auto stable = layout(input);
    if (stable.placement != Placement::above) {
        std::cerr << "composition placement lock changed\n";
        return 1;
    }
    input.orientation = Orientation::horizontal;
    input.placement = Placement::below;
    input.caret = {10, 10};
    const auto horizontal = layout(input);
    if (horizontal.items[1].left <= horizontal.items[0].right)
        return 1;
    for (const float scale : {1.25F, 1.5F, 2.0F}) {
        input.orientation = Orientation::vertical;
        input.placement = Placement::unlocked;
        input.caret = {-1900.0F, 900.0F};
        input.caretHeight = 20.0F * scale;
        input.workArea = {-1920.0F, 0.0F, 0.0F, 1080.0F};
        input.maxWidth = 720.0F * scale;
        input.paddingX = 8.0F * scale;
        input.paddingY = 6.0F * scale;
        input.items = {{500.0F * scale, 24.0F * scale}, {900.0F * scale, 24.0F * scale}};
        const auto scaled = layout(input);
        if (scaled.window.left < input.workArea.left ||
            scaled.window.right > input.workArea.right || scaled.window.top < input.workArea.top ||
            scaled.window.bottom > input.workArea.bottom) {
            std::cerr << "scaled negative-coordinate monitor was not clamped\n";
            return 1;
        }
    }
    LayoutInput scroll;
    scroll.scrollMode = true;
    scroll.scrollColumns = 6;
    scroll.scrollVisibleRows = 6;
    scroll.caret = {100, 100};
    scroll.caretHeight = 24;
    scroll.workArea = {0, 0, 1920, 1080};
    scroll.maxWidth = 860;
    scroll.items.assign(60, Size{120, 34});
    scroll.orientation = Orientation::horizontal;
    scroll.selected = 6;
    const auto sameViewport = layout(scroll);
    if (sameViewport.firstVisible != 0U || sameViewport.itemIndices.front() != 0U ||
        sameViewport.itemIndices.back() != 35U) {
        std::cerr << "moving within the first six rows shifted the scroll viewport\n";
        return 1;
    }
    LayoutInput horizontalBaseline = scroll;
    horizontalBaseline.scrollMode = false;
    horizontalBaseline.orientation = Orientation::horizontal;
    horizontalBaseline.items.resize(6);
    const auto horizontalBaselineLayout = layout(horizontalBaseline);
    const float baselineWidth =
        horizontalBaselineLayout.window.right - horizontalBaselineLayout.window.left;
    if (std::abs((sameViewport.window.right - sameViewport.window.left) -
                 baselineWidth) > 0.01F) {
        std::cerr << "scroll layout width did not match the ordinary candidate width\n";
        return 1;
    }
    scroll.selected = 42;
    const auto nextViewport = layout(scroll);
    if (nextViewport.items.size() != 36U || nextViewport.itemIndices.front() != 24U ||
        nextViewport.itemIndices.back() != 59U || !nextViewport.hasScrollbar ||
        nextViewport.scrollbarThumb.bottom > nextViewport.scrollbarTrack.bottom) {
        std::cerr << "scroll grid did not advance at its six-row boundary\n";
        return 1;
    }
    LayoutInput verticalScroll = scroll;
    verticalScroll.orientation = Orientation::vertical;
    verticalScroll.scrollColumns = 6;
    verticalScroll.maxWidth = 720;
    verticalScroll.scrollCellWidth = 96;
    verticalScroll.selected = 6;
    verticalScroll.items.assign(60, Size{48, 34});
    const auto firstColumns = layout(verticalScroll);
    const float fixedCellWidth = 96.0F * 6.0F + 8.0F * 2.0F + 8.0F * 5.0F;
    if (firstColumns.items.size() != 36U || firstColumns.itemIndices.front() != 0U ||
        firstColumns.itemIndices.back() != 35U ||
        firstColumns.window.right - firstColumns.window.left >= fixedCellWidth - 0.01F) {
        std::cerr << "vertical scroll layout treated the bounded cell width as a fixed width\n";
        return 1;
    }
    for (const auto& item : firstColumns.items) {
        if (std::abs((item.right - item.left) - 48.0F) > 0.01F) {
            std::cerr << "vertical scroll cells did not preserve compact natural width\n";
            return 1;
        }
    }
    verticalScroll.items.assign(60, Size{420, 34});
    const auto longCandidateColumns = layout(verticalScroll);
    if (longCandidateColumns.items.size() != 36U ||
        longCandidateColumns.window.right - longCandidateColumns.window.left >
            verticalScroll.maxWidth + 0.01F) {
        std::cerr << "long vertical scroll candidates widened the grid instead of ellipsizing\n";
        return 1;
    }
    verticalScroll.selected = 58;
    const auto finalColumn = layout(verticalScroll);
    if (finalColumn.items.size() != 36U || finalColumn.itemIndices.front() != 24U ||
        finalColumn.itemIndices.back() != 59U) {
        std::cerr << "vertical scroll layout did not keep the selected column viewport\n";
        return 1;
    }
    return 0;
}
