#pragma once

#include <algorithm>
#include <cstddef>
#include <optional>

namespace fcitx::windows::engine {

struct RowNavigationTarget {
    std::size_t index{};
    bool beforeStart{};
    bool afterEnd{};
};

[[nodiscard]] constexpr std::optional<std::size_t>
rowSelectionTarget(std::size_t focus, std::size_t column, std::size_t columns,
                   std::size_t candidateCount) noexcept {
    if (columns == 0U || column >= columns || focus >= candidateCount)
        return std::nullopt;
    const std::size_t target = focus - focus % columns + column;
    if (target >= candidateCount)
        return std::nullopt;
    return target;
}

[[nodiscard]] constexpr std::optional<std::size_t>
rowSelectionColumn(std::size_t focus, std::size_t candidate, std::size_t columns,
                   std::size_t candidateCount) noexcept {
    if (columns == 0U || focus >= candidateCount || candidate >= candidateCount ||
        focus / columns != candidate / columns)
        return std::nullopt;
    return candidate % columns;
}

[[nodiscard]] constexpr RowNavigationTarget
rowNavigationTarget(std::size_t focus, std::size_t columns,
                    std::size_t candidateCount, bool forward,
                    bool preserveColumn) noexcept {
    if (columns == 0U || focus >= candidateCount)
        return {focus};
    const std::size_t base = preserveColumn ? focus : focus - focus % columns;
    if (forward) {
        if (base > candidateCount || columns >= candidateCount - base)
            return {focus, false, true};
        return {base + columns};
    }
    if (base < columns)
        return {focus, true, false};
    return {base - columns};
}

[[nodiscard]] constexpr RowNavigationTarget
columnNavigationTarget(std::size_t focus, std::size_t rows,
                       std::size_t candidateCount, bool forward,
                       bool preserveRow) noexcept {
    if (rows == 0U || focus >= candidateCount)
        return {focus};
    const std::size_t row = preserveRow ? focus % rows : 0U;
    const std::size_t columnStart = focus - focus % rows;
    if (forward) {
        const std::size_t nextColumnStart = columnStart + rows;
        if (nextColumnStart >= candidateCount)
            return {focus, false, true};
        return {std::min(nextColumnStart + row, candidateCount - 1U)};
    }
    if (columnStart < rows)
        return {focus, true, false};
    const std::size_t previousColumnStart = columnStart - rows;
    return {std::min(previousColumnStart + row, candidateCount - 1U)};
}

[[nodiscard]] constexpr RowNavigationTarget
sameColumnNavigationTarget(std::size_t focus, std::size_t rows,
                           std::size_t candidateCount, bool forward) noexcept {
    if (rows == 0U || focus >= candidateCount)
        return {focus};
    const std::size_t columnStart = focus - focus % rows;
    const std::size_t columnEnd = std::min(columnStart + rows, candidateCount);
    if (forward) {
        if (focus + 1U >= columnEnd)
            return {focus, false, true};
        return {focus + 1U};
    }
    if (focus == columnStart)
        return {focus, true, false};
    return {focus - 1U};
}

} // namespace fcitx::windows::engine
