#pragma once

#include <cstddef>
#include <vector>

namespace fcitx::windows::ui {

struct Point { float x{}; float y{}; };
struct Size { float width{}; float height{}; };
struct Rect { float left{}; float top{}; float right{}; float bottom{}; };
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
};

struct LayoutResult {
    Rect window;
    std::vector<Rect> items;
    Placement placement{Placement::below};
};

[[nodiscard]] LayoutResult layout(const LayoutInput& input);

} // namespace fcitx::windows::ui
