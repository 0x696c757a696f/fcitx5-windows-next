#include "candidate_navigation.h"

#include <iostream>

int main() {
    using fcitx::windows::engine::rowNavigationTarget;
    using fcitx::windows::engine::rowSelectionColumn;
    using fcitx::windows::engine::rowSelectionTarget;
    using fcitx::windows::engine::columnNavigationTarget;
    using fcitx::windows::engine::sameColumnNavigationTarget;

    if (rowSelectionTarget(8U, 0U, 6U, 20U) != 6U ||
        rowSelectionTarget(8U, 5U, 6U, 20U) != 11U) {
        std::cerr << "number shortcuts did not select from the highlighted row\n";
        return 1;
    }
    if (rowSelectionTarget(8U, 6U, 6U, 20U).has_value()) {
        std::cerr << "an unlabeled number shortcut selected a candidate\n";
        return 1;
    }
    if (rowSelectionTarget(19U, 2U, 6U, 20U).has_value()) {
        std::cerr << "a row shortcut selected past the final candidate\n";
        return 1;
    }
    if (rowSelectionColumn(8U, 6U, 6U, 20U) != 0U ||
        rowSelectionColumn(8U, 11U, 6U, 20U) != 5U ||
        rowSelectionColumn(8U, 12U, 6U, 20U).has_value()) {
        std::cerr << "snapshot labels did not describe the highlighted row\n";
        return 1;
    }
    const auto displayedColumn = rowSelectionColumn(8U, 6U, 6U, 20U);
    if (!displayedColumn || rowSelectionTarget(8U, *displayedColumn, 6U, 20U) != 6U) {
        std::cerr << "a displayed label did not map back to its candidate\n";
        return 1;
    }
    const auto beforeFirst = rowNavigationTarget(2U, 6U, 20U, false, false);
    const auto afterLast = rowNavigationTarget(18U, 6U, 20U, true, false);
    const auto firstPlus = rowNavigationTarget(0U, 6U, 20U, true, false);
    const auto nextRowStart = rowNavigationTarget(8U, 6U, 20U, true, false);
    const auto repeatedPlus = rowNavigationTarget(nextRowStart.index, 6U, 20U, true, false);
    const auto nextSameColumn = rowNavigationTarget(8U, 6U, 20U, true, true);
    if (!beforeFirst.beforeStart || beforeFirst.index != 2U || !afterLast.afterEnd ||
        afterLast.index != 18U || firstPlus.index != 6U || nextRowStart.index != 12U ||
        repeatedPlus.index != 18U ||
        nextSameColumn.index != 14U) {
        std::cerr << "row navigation did not stay put at a list boundary\n";
        return 1;
    }
    const auto downSameColumn = sameColumnNavigationTarget(6U, 6U, 20U, true);
    const auto upSameColumn = sameColumnNavigationTarget(7U, 6U, 20U, false);
    const auto downColumnEnd = sameColumnNavigationTarget(11U, 6U, 20U, true);
    const auto rightNextColumn = columnNavigationTarget(8U, 6U, 20U, true, true);
    const auto leftPreviousColumn = columnNavigationTarget(14U, 6U, 20U, false, true);
    const auto rightShortFinalColumn = columnNavigationTarget(17U, 6U, 20U, true, true);
    const auto rightColumnTop = columnNavigationTarget(8U, 6U, 20U, true, false);
    if (downSameColumn.index != 7U || upSameColumn.index != 6U ||
        !downColumnEnd.afterEnd || downColumnEnd.index != 11U ||
        rightNextColumn.index != 14U || leftPreviousColumn.index != 8U ||
        rightShortFinalColumn.index != 19U || rightColumnTop.index != 12U) {
        std::cerr << "vertical scroll navigation did not keep row/column semantics\n";
        return 1;
    }
    return 0;
}
