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
    scroll.selected = 42;
    scroll.caret = {100, 100};
    scroll.caretHeight = 24;
    scroll.workArea = {0, 0, 1920, 1080};
    scroll.maxWidth = 860;
    scroll.items.assign(60, Size{120, 34});
    const auto grid = layout(scroll);
    if (grid.items.size() != 36U || grid.itemIndices.front() != 24U ||
        grid.itemIndices.back() != 59U || !grid.hasScrollbar ||
        grid.scrollbarThumb.bottom > grid.scrollbarTrack.bottom) {
        std::cerr << "scroll grid did not keep the selected row in its six-row viewport\n";
        return 1;
    }
    return 0;
}
