#include "candidate_select_client.h"
#include "config_snapshot_ffi.h"
#include "peer_verification.h"
#include "pipe_security.h"
#include "protocol_ffi.h"
#include "runtime_identity.h"

#include <fcitx5_windows/release_identity.h>

#include <Windows.h>
#include <d2d1.h>
#include <dwrite.h>
#include <wrl/client.h>

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <charconv>
#include <filesystem>
#include <iostream>
#include <memory>
#include <optional>
#include <span>
#include <sstream>
#include <string>
#include <string_view>
#include <thread>
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

struct Fcitx5CandidateRenderItemInput {
    Fcitx5CandidateLayoutRect bounds{};
    float labelWidth{};
    float labelGap{};
    float textWidth{};
    float commentWidth{};
    std::uint8_t hasLabel{};
    std::uint8_t reserveLabel{};
};

struct Fcitx5CandidateRenderItemOutput {
    Fcitx5CandidateLayoutRect label{};
    Fcitx5CandidateLayoutRect text{};
    Fcitx5CandidateLayoutRect comment{};
    std::uint8_t drawComment{};
};

struct Fcitx5CandidateAxisLayoutItemOutput {
    float x{};
    float y{};
    float w{};
    float h{};
    std::uint8_t visible{};
};

struct Fcitx5CandidateAxisLayoutOutput {
    float windowX{};
    float windowY{};
    float windowW{};
    float windowH{};
    std::uint32_t placement{};
    float viewportDx{};
    float viewportDy{};
    float contentWidth{};
    float contentHeight{};
    std::size_t firstVisible{};
    std::size_t itemCount{};
};

struct Fcitx5CandidateAxisLayoutInput {
    std::uint32_t pageSize{};
    float maxWidth{};
    float maxHeight{};
    float dpiScale{};
    std::size_t highlightedIndex{};
    std::uint8_t orientation{};
    std::uint8_t overflow{};
    std::uint8_t writing{};
    Fcitx5CandidateLayoutPoint caret{};
    float caretHeight{};
    Fcitx5CandidateLayoutRect workArea{};
    float paddingX{};
    float paddingY{};
    float rowGap{};
    float columnGap{};
    std::uint32_t placement{};
};

struct Fcitx5CandidatePresentationUpdate {
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    std::size_t selected{};
    std::uint8_t hasSelected{};
    std::size_t candidateCount{};
    std::uint32_t page{};
    std::uint32_t pageSize{};
    std::uint8_t candidateBulk{};
    std::uint8_t configuredScrollMode{};
};

struct Fcitx5CandidatePresentationOutput {
    std::size_t selected{};
    std::uint8_t hasSelected{};
    std::uint8_t scrollMode{};
    std::uint8_t scrollExpanded{};
    std::size_t scrollColumns{};
    std::size_t ordinaryStart{};
    std::size_t ordinaryCount{};
    std::uint8_t candidateBulk{};
    std::uint32_t pageSize{};
    std::uint32_t placement{};
    float stableWidth{};
};

struct Fcitx5CandidatePresentationRenderPlan {
    std::size_t selected{};
    std::uint8_t hasSelected{};
    std::size_t renderCount{};
};

struct Fcitx5CandidateUtf8 {
    const std::uint8_t* ptr{};
    std::size_t len{};
};

struct Fcitx5CandidatePresentationText {
    Fcitx5CandidateUtf8 text{};
    Fcitx5CandidateUtf8 comment{};
};

struct Fcitx5CandidateSelectionIntent {
    std::uint32_t targetProcessId{};
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    std::uint64_t candidateId{};
};

struct Fcitx5CandidateCommandLine {
    std::uint8_t status{};
    std::uint8_t candidateSelectMode{};
    std::uint8_t selfTest{};
    std::uint8_t interactionSelfTest{};
    std::uint8_t uilessPresentationSelfTest{};
    std::uint8_t scrollExpansionSelfTest{};
    std::uint8_t localeSelfTest{};
    std::uint8_t candidateUxSelfTest{};
    std::uint8_t reloadTest{};
    std::uint8_t simulateDeviceLoss{};
    std::uint8_t scrollDemo{};
    std::uint8_t demo{};
    std::uint8_t testOnce{};
    std::uint8_t safeMode{};
    std::uint8_t hasParentId{};
    std::uint8_t reserved{};
    std::size_t generationLen{};
    std::size_t candidatePeerLen{};
    std::uint32_t parentId{};
    std::uint32_t targetProcessId{};
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    std::uint64_t candidateId{};
};

struct Fcitx5CandidateUtf16 {
    const std::uint16_t* ptr{};
    std::size_t len{};
};

struct Fcitx5WindowsCommonUtf8ToWide {
    std::uint8_t status;
    std::size_t utf16Len;
};

extern "C" int fcitx5_candidate_render_segments(const Fcitx5CandidateRenderItemInput* items,
                                                 std::size_t itemCount,
                                                 std::uint8_t horizontal,
                                                 std::uint8_t scrollMode,
                                                 Fcitx5CandidateRenderItemOutput* outItems,
                                                 float* outLabelColumnWidth);
extern "C" int fcitx5_candidate_axis_layout(
    const Fcitx5CandidateAxisLayoutInput* input,
    const Fcitx5CandidateLayoutSize* items, std::size_t itemCount,
    Fcitx5CandidateAxisLayoutItemOutput* outItems, std::size_t outCapacity,
    Fcitx5CandidateAxisLayoutOutput* output);
extern "C" void* fcitx5_candidate_presentation_create();
extern "C" void fcitx5_candidate_presentation_destroy(void* state);
extern "C" void fcitx5_candidate_presentation_reset(void* state);
extern "C" std::uint32_t fcitx5_candidate_presentation_apply(
    void* state, const Fcitx5CandidatePresentationUpdate* input);
extern "C" std::uint8_t fcitx5_candidate_presentation_current(
    void* state, Fcitx5CandidatePresentationOutput* output);
extern "C" std::uint8_t fcitx5_candidate_presentation_render_plan(
    void* state, std::size_t* indices, std::size_t capacity,
    Fcitx5CandidatePresentationRenderPlan* output);
extern "C" std::uint8_t fcitx5_candidate_presentation_set_placement(
    void* state, std::uint32_t placement);
extern "C" float fcitx5_candidate_presentation_stable_window_width(
    void* state, float measuredWidth, float maxAllowedWidth);
extern "C" std::uint32_t fcitx5_candidate_presentation_resolve_orientation(
    void* state, std::uint32_t configured,
    const Fcitx5CandidatePresentationText* candidates, std::size_t candidateCount,
    Fcitx5CandidateUtf8 locale, Fcitx5CandidateLayoutRect workArea, float caretX, float scale,
    std::uint32_t pageSize);
extern "C" std::size_t fcitx5_candidate_format_label_utf16(
    std::uint32_t slot, std::uint32_t style, Fcitx5CandidateUtf16 sourceLabel,
    Fcitx5CandidateUtf16 customPrefix, Fcitx5CandidateUtf16 customSuffix, std::uint16_t* output,
    std::size_t outputCapacity);
extern "C" std::uint8_t fcitx5_candidate_hit_test(const Fcitx5CandidateLayoutRect* rects,
                                                   std::size_t rectCount, float x, float y,
                                                   std::size_t* outIndex);
extern "C" Fcitx5CandidateSelectionIntent fcitx5_candidate_selection_intent(
    std::uint32_t targetProcessId, std::uint64_t engineEpoch, std::uint64_t contextId,
    std::uint64_t compositionId, std::uint64_t revision, std::uint64_t candidateId);
extern "C" Fcitx5CandidateCommandLine fcitx5_candidate_parse_command_line_utf16(
    const std::uint16_t* arguments,
    std::size_t argumentsLen,
    std::uint16_t* generationOut,
    std::size_t generationCapacity,
    std::uint16_t* candidatePeerOut,
    std::size_t candidatePeerCapacity);
extern "C" std::size_t fcitx5_candidate_default_dwrite_locale_utf16(
    std::uint16_t* localeOut,
    std::size_t localeCapacity);
extern "C" Fcitx5WindowsCommonUtf8ToWide fcitx5_windows_common_utf8_to_wide_utf16(
    const std::uint8_t* input,
    std::size_t input_len,
    std::uint16_t* output,
    std::size_t capacity);
extern "C" std::uint32_t fcitx5_windows_common_current_process_id();
extern "C" std::uint8_t fcitx5_windows_common_system_uses_dark_appearance();

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

struct RenderItemInput {
    Rect bounds{};
    float labelWidth{};
    float labelGap{};
    float textWidth{};
    float commentWidth{};
    bool hasLabel{};
    bool reserveLabel{};
};

struct RenderItemSegments {
    Rect label{};
    Rect text{};
    Rect comment{};
    bool drawComment{};
};

struct CandidateSelectionIntent {
    std::uint32_t targetProcessId{};
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    std::uint64_t candidateId{};

    [[nodiscard]] bool valid() const noexcept {
        return targetProcessId != 0 && engineEpoch != 0 && contextId != 0 &&
               compositionId != 0 && revision != 0 && candidateId != 0;
    }
};

[[nodiscard]] inline detail::Fcitx5CandidateUtf16 toRust(std::wstring_view value) noexcept {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    return {reinterpret_cast<const std::uint16_t*>(value.data()), value.size()};
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

[[nodiscard]] inline std::vector<RenderItemSegments> renderSegments(
    Orientation orientation, bool scrollMode, const std::vector<RenderItemInput>& items) {
    std::vector<detail::Fcitx5CandidateRenderItemInput> rustInputs;
    rustInputs.reserve(items.size());
    for (const auto& item : items) {
        rustInputs.push_back({
            {item.bounds.left, item.bounds.top, item.bounds.right, item.bounds.bottom},
            item.labelWidth,
            item.labelGap,
            item.textWidth,
            item.commentWidth,
            static_cast<std::uint8_t>(item.hasLabel ? 1U : 0U),
            static_cast<std::uint8_t>(item.reserveLabel ? 1U : 0U),
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

template <typename RectType>
[[nodiscard]] inline std::optional<std::size_t> hitTestCandidate(
    const std::vector<RectType>& itemRects, float x, float y) noexcept {
    std::vector<detail::Fcitx5CandidateLayoutRect> rustRects;
    rustRects.reserve(itemRects.size());
    for (const auto& rectangle : itemRects)
        rustRects.push_back({rectangle.left, rectangle.top, rectangle.right, rectangle.bottom});
    std::size_t index = 0;
    if (detail::fcitx5_candidate_hit_test(rustRects.data(), rustRects.size(), x, y, &index) == 0)
        return std::nullopt;
    return index;
}

[[nodiscard]] inline CandidateSelectionIntent makeCandidateSelectionIntent(
    std::uint32_t targetProcessId, std::uint64_t engineEpoch,
    std::uint64_t contextId, std::uint64_t compositionId,
    std::uint64_t revision, std::uint64_t candidateId) noexcept {
    const auto intent = detail::fcitx5_candidate_selection_intent(
        targetProcessId, engineEpoch, contextId, compositionId, revision, candidateId);
    return {intent.targetProcessId, intent.engineEpoch, intent.contextId,
            intent.compositionId, intent.revision, intent.candidateId};
}

struct ParsedCommandLine {
    detail::Fcitx5CandidateCommandLine flags{};
    std::wstring generation;
    std::wstring candidatePeer;
    bool valid{};
};

[[nodiscard]] inline ParsedCommandLine parseCommandLine(std::wstring_view arguments) {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    const auto* input = reinterpret_cast<const std::uint16_t*>(arguments.data());
    const auto query = detail::fcitx5_candidate_parse_command_line_utf16(
        input, arguments.size(), nullptr, 0, nullptr, 0);
    if (query.status == 0)
        return {};
    ParsedCommandLine parsed;
    parsed.flags = query;
    parsed.generation.resize(query.generationLen);
    parsed.candidatePeer.resize(query.candidatePeerLen);
    const auto filled = detail::fcitx5_candidate_parse_command_line_utf16(
        input, arguments.size(), reinterpret_cast<std::uint16_t*>(parsed.generation.data()),
        parsed.generation.size(), reinterpret_cast<std::uint16_t*>(parsed.candidatePeer.data()),
        parsed.candidatePeer.size());
    if (filled.status == 0 || filled.generationLen != parsed.generation.size() ||
        filled.candidatePeerLen != parsed.candidatePeer.size())
        return {};
    parsed.flags = filled;
    parsed.valid = true;
    return parsed;
}

} // namespace fcitx::windows::ui
namespace {

using Microsoft::WRL::ComPtr;
namespace ui = fcitx::windows::ui;

// ---------------------------------------------------------------------------
// File-local presentation carrier. Rust protocol-core owns the wire codec and
// the authoritative KeyResponse DTO; this translation unit only projects the
// decoded snapshot into the HWND/D2D candidate renderer, so it carries a
// minimal owned mirror instead of depending on the deleted protocol.h DTO.
// ---------------------------------------------------------------------------

// Presentation pipe only carries keyResponse frames (wire type 4).
constexpr std::uint16_t kKeyResponseMessageType = 4;

struct Metadata {
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
};

struct CaretRect {
    bool valid{};
    std::int32_t left{};
    std::int32_t top{};
    std::int32_t right{};
    std::int32_t bottom{};
    std::uint32_t dpi{};
};

struct CandidateRecord {
    std::uint64_t id{};
    std::string labelUtf8;
    std::string textUtf8;
    std::string commentUtf8;
};

struct KeyResponse {
    Metadata metadata;
    std::uint32_t status{};
    bool handled{};
    std::string preeditUtf8;
    std::uint32_t preeditCaretUtf8{};
    std::string contentLocaleUtf8;
    std::vector<CandidateRecord> candidates;
    std::uint32_t selectedCandidate{UINT32_MAX};
    std::uint32_t candidatePage{};
    std::uint32_t candidatePageSize{};
    std::uint32_t candidateTotal{};
    std::uint8_t candidateVisibility{};
    bool candidateBulk{};
    bool candidateEnd{};
    CaretRect caret;
    bool popupAllowed{true};
};

namespace candidate {

enum class Visibility : std::uint8_t { hidden, composition, prediction };
enum class ApplyResult { applied, duplicate, stale, invalid };

struct Item {
    std::uint64_t id{};
    std::string label;
    std::string text;
    std::string comment;
};

struct Snapshot {
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    std::string preedit;
    std::string auxiliaryUp;
    std::string auxiliaryDown;
    std::vector<Item> candidates;
    std::optional<std::size_t> selected;
    std::uint32_t page{};
    std::uint32_t total{};
    Visibility visibility{Visibility::hidden};
    bool popupAllowed{true};
};

namespace detail {

struct Fcitx5CandidateUtf8 {
    const std::uint8_t* ptr{};
    std::size_t len{};
};

struct Fcitx5CandidateModelItem {
    std::uint64_t id{};
    Fcitx5CandidateUtf8 label{};
    Fcitx5CandidateUtf8 text{};
    Fcitx5CandidateUtf8 comment{};
};

struct Fcitx5CandidateModelSnapshot {
    std::uint64_t engineEpoch{};
    std::uint64_t contextId{};
    std::uint64_t compositionId{};
    std::uint64_t revision{};
    Fcitx5CandidateUtf8 preedit{};
    Fcitx5CandidateUtf8 auxiliaryUp{};
    Fcitx5CandidateUtf8 auxiliaryDown{};
    const Fcitx5CandidateModelItem* candidates{};
    std::size_t candidateCount{};
    std::size_t selected{};
    std::uint8_t hasSelected{};
    std::uint32_t page{};
    std::uint32_t total{};
    std::uint8_t visibility{};
    std::uint8_t popupAllowed{};
};

struct Fcitx5CandidateScrollLabel {
    std::uint8_t reserve{};
    std::uint8_t show{};
    std::uint32_t slot{};
};

extern "C" void* fcitx5_candidate_model_create();
extern "C" void fcitx5_candidate_model_destroy(void* model);
extern "C" void fcitx5_candidate_model_reset(void* model);
extern "C" std::uint32_t fcitx5_candidate_model_apply(
    void* model, const Fcitx5CandidateModelSnapshot* snapshot);
extern "C" std::uint8_t fcitx5_candidate_model_current(
    void* model, Fcitx5CandidateModelSnapshot* output);
extern "C" std::uint8_t fcitx5_candidate_content_locale_valid_utf8(
    Fcitx5CandidateUtf8 locale);
extern "C" std::size_t fcitx5_candidate_content_locale_or_default_utf16(
    Fcitx5CandidateUtf8 locale,
    std::uint16_t* localeOut,
    std::size_t localeCapacity);
extern "C" std::uint8_t fcitx5_candidate_locale_prefers_compact_horizontal_utf8(
    Fcitx5CandidateUtf8 locale);
extern "C" Fcitx5CandidateScrollLabel fcitx5_candidate_scroll_label_policy(
    std::size_t candidateIndex,
    std::size_t selectedIndex,
    std::size_t pageSize,
    std::size_t totalCandidates);

[[nodiscard]] Fcitx5CandidateUtf8 toRust(std::string_view value) noexcept {
    return {reinterpret_cast<const std::uint8_t*>(value.data()), value.size()};
}

[[nodiscard]] std::uint8_t toRust(Visibility visibility) noexcept {
    switch (visibility) {
    case Visibility::hidden:
        return 0U;
    case Visibility::composition:
        return 1U;
    case Visibility::prediction:
        return 2U;
    }
    return 0U;
}

[[nodiscard]] std::vector<Fcitx5CandidateModelItem> itemsToRust(const Snapshot& snapshot) {
    std::vector<Fcitx5CandidateModelItem> result;
    result.reserve(snapshot.candidates.size());
    for (const auto& item : snapshot.candidates) {
        result.push_back({
            item.id,
            toRust(item.label),
            toRust(item.text),
            toRust(item.comment),
        });
    }
    return result;
}

[[nodiscard]] Fcitx5CandidateModelSnapshot toRust(
    const Snapshot& snapshot,
    const std::vector<Fcitx5CandidateModelItem>& candidates) noexcept {
    return {
        snapshot.engineEpoch,
        snapshot.contextId,
        snapshot.compositionId,
        snapshot.revision,
        toRust(snapshot.preedit),
        toRust(snapshot.auxiliaryUp),
        toRust(snapshot.auxiliaryDown),
        candidates.data(),
        candidates.size(),
        snapshot.selected.value_or(0U),
        static_cast<std::uint8_t>(snapshot.selected ? 1U : 0U),
        snapshot.page,
        snapshot.total,
        toRust(snapshot.visibility),
        static_cast<std::uint8_t>(snapshot.popupAllowed ? 1U : 0U),
    };
}

[[nodiscard]] std::optional<Snapshot> fromRust(
    const Fcitx5CandidateModelSnapshot& snapshot) {
    const auto text = [](Fcitx5CandidateUtf8 value) {
        return std::string(reinterpret_cast<const char*>(value.ptr), value.len);
    };
    if ((snapshot.candidateCount != 0U && snapshot.candidates == nullptr) ||
        (snapshot.visibility > 2U))
        return std::nullopt;
    Snapshot result{
        snapshot.engineEpoch,
        snapshot.contextId,
        snapshot.compositionId,
        snapshot.revision,
        text(snapshot.preedit),
        text(snapshot.auxiliaryUp),
        text(snapshot.auxiliaryDown),
        {},
        snapshot.hasSelected ? std::optional<std::size_t>{snapshot.selected} : std::nullopt,
        snapshot.page,
        snapshot.total,
        static_cast<Visibility>(snapshot.visibility),
        snapshot.popupAllowed != 0U,
    };
    result.candidates.reserve(snapshot.candidateCount);
    for (std::size_t index = 0; index < snapshot.candidateCount; ++index) {
        const auto& item = snapshot.candidates[index];
        result.candidates.push_back({item.id, text(item.label), text(item.text), text(item.comment)});
    }
    return result;
}

} // namespace detail

class CandidateModel final {
public:
    CandidateModel() : rustModel_(detail::fcitx5_candidate_model_create()) {}
    ~CandidateModel() { detail::fcitx5_candidate_model_destroy(rustModel_); }
    CandidateModel(const CandidateModel&) = delete;
    CandidateModel& operator=(const CandidateModel&) = delete;

    [[nodiscard]] ApplyResult apply(const Snapshot& snapshot) {
        const auto candidates = detail::itemsToRust(snapshot);
        const auto rustSnapshot = detail::toRust(snapshot, candidates);
        switch (detail::fcitx5_candidate_model_apply(rustModel_, &rustSnapshot)) {
        case 0U: {
            detail::Fcitx5CandidateModelSnapshot current{};
            if (detail::fcitx5_candidate_model_current(rustModel_, &current) == 0U)
                return ApplyResult::invalid;
            current_ = detail::fromRust(current);
            if (!current_)
                return ApplyResult::invalid;
            return ApplyResult::applied;
        }
        case 1U:
            return ApplyResult::duplicate;
        case 2U:
            return ApplyResult::stale;
        default:
            return ApplyResult::invalid;
        }
    }

    void reset() noexcept {
        detail::fcitx5_candidate_model_reset(rustModel_);
        current_.reset();
    }

    [[nodiscard]] const std::optional<Snapshot>& current() const noexcept { return current_; }

private:
    std::optional<Snapshot> current_;
    void* rustModel_{};
};

} // namespace candidate

constexpr UINT kSnapshotMessage = WM_APP + 1;
constexpr UINT_PTR kFocusWatchTimer = 1;
constexpr UINT_PTR kClickGuardTimer = 2;
constexpr wchar_t kVisualConfigChangedMessage[] =
    L"Fcitx5WindowsNext.VisualConfigChanged.v1";
constexpr wchar_t kCandidateDismissMessage[] =
    L"Fcitx5WindowsNext.CandidateDismiss.v1";

UINT visualConfigChangedMessage() noexcept {
    static const UINT message = RegisterWindowMessageW(kVisualConfigChangedMessage);
    return message;
}

UINT candidateDismissMessage() noexcept {
    static const UINT message = RegisterWindowMessageW(kCandidateDismissMessage);
    return message;
}

std::wstring defaultDwriteLocale() {
    const std::size_t required =
        ui::detail::fcitx5_candidate_default_dwrite_locale_utf16(nullptr, 0);
    if (required == 0)
        return L"en-US";
    std::wstring locale(required, L'\0');
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    const std::size_t written = ui::detail::fcitx5_candidate_default_dwrite_locale_utf16(
        reinterpret_cast<std::uint16_t*>(locale.data()), locale.size());
    if (written != locale.size())
        return L"en-US";
    return locale;
}

bool validContentLocale(std::string_view locale) {
    return candidate::detail::fcitx5_candidate_content_locale_valid_utf8(
               candidate::detail::toRust(locale)) != 0;
}

std::wstring contentLocaleOrFallback(std::string_view locale) {
    const auto rustLocale = candidate::detail::toRust(locale);
    const std::size_t required =
        candidate::detail::fcitx5_candidate_content_locale_or_default_utf16(rustLocale, nullptr, 0);
    if (required == 0)
        return defaultDwriteLocale();
    std::wstring result(required, L'\0');
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    const std::size_t written =
        candidate::detail::fcitx5_candidate_content_locale_or_default_utf16(
            rustLocale, reinterpret_cast<std::uint16_t*>(result.data()), result.size());
    if (written != result.size())
        return defaultDwriteLocale();
    return result;
}

// This is deliberately a renderer-local projection. Config Core owns parsing,
// validation, recovery, theme resolution, and all defaults; the native window
// receives only resolved D2D/DWrite values through its narrow ABI.
enum class NativeOrientation : std::uint8_t { automatic, vertical, horizontal };
enum class NativeOverflow : std::uint8_t { paging, scrolling, wrapping };
enum class NativeWritingMode : std::uint8_t { horizontal, verticalRl, verticalLr };
enum class NativeLabelStyle : std::uint8_t { plain, dot, paren, bracket, circled };
enum class NativePreeditMode : std::uint8_t { inline_, panel };

struct NativeRenderColors {
    D2D1_COLOR_F background{D2D1::ColorF(0.97F, 0.98F, 0.98F)};
    D2D1_COLOR_F candidateText{D2D1::ColorF(0.13F, 0.13F, 0.14F)};
    D2D1_COLOR_F selectedBackground{D2D1::ColorF(0.027F, 0.757F, 0.376F, 0.10F)};
    D2D1_COLOR_F selectedCandidateText{D2D1::ColorF(0.027F, 0.757F, 0.376F)};
    D2D1_COLOR_F labelText{candidateText};
    D2D1_COLOR_F commentText{candidateText};
    D2D1_COLOR_F preeditText{candidateText};
    D2D1_COLOR_F selectedLabelText{selectedCandidateText};
    D2D1_COLOR_F selectedCommentText{selectedCandidateText};
    D2D1_COLOR_F border{D2D1::ColorF(0.82F, 0.82F, 0.82F)};
};

struct NativeRenderConfig {
    NativeOrientation orientation{};
    NativeOverflow overflow{NativeOverflow::paging};
    NativeWritingMode writingMode{NativeWritingMode::horizontal};
    NativePreeditMode preeditMode{};
    NativeLabelStyle labelStyle{NativeLabelStyle::dot};
    std::vector<std::wstring> candidateLabels;
    std::vector<std::wstring> candidateFontFamilies;
    std::vector<std::wstring> annotationFontFamilies;
    NativeRenderColors colors;
    float maxWidthDip{};
    float scrollCellWidthDip{};
    float opacity{};
    float paddingXDip{};
    float paddingYDip{};
    float itemPaddingXDip{};
    float itemPaddingYDip{};
    float rowGapDip{};
    float columnGapDip{};
    float borderWidthDip{};
    float cornerRadiusDip{};
    float labelFontScale{};
    float labelGapDip{};
    float candidateFontSizeDip{};
    float annotationFontScale{};
    std::uint16_t candidateFontWeight{};
    bool scrollMode{};
    bool labelVisible{};
};

std::uint32_t labelStyleToRust(NativeLabelStyle style) noexcept {
    switch (style) {
    case NativeLabelStyle::plain:
        return 0;
    case NativeLabelStyle::dot:
        return 1;
    case NativeLabelStyle::paren:
        return 2;
    case NativeLabelStyle::bracket:
        return 3;
    case NativeLabelStyle::circled:
        return 4;
    }
    return 1;
}

std::wstring formatCandidateLabel(std::uint32_t slot, std::wstring_view label,
                                  NativeLabelStyle style) {
    const fcitx::windows::ui::detail::Fcitx5CandidateUtf16 empty{};
    const auto required = fcitx::windows::ui::detail::fcitx5_candidate_format_label_utf16(
        slot, labelStyleToRust(style), fcitx::windows::ui::toRust(label), empty, empty, nullptr, 0);
    if (required == 0)
        return label.empty() ? std::to_wstring(slot == 0 ? 1U : slot) : std::wstring(label);
    std::wstring result(required, L'\0');
    const auto written = fcitx::windows::ui::detail::fcitx5_candidate_format_label_utf16(
        slot, labelStyleToRust(style), fcitx::windows::ui::toRust(label), empty, empty,
        reinterpret_cast<std::uint16_t*>(result.data()), result.size());
    if (written != result.size())
        return label.empty() ? std::to_wstring(slot == 0 ? 1U : slot) : std::wstring(label);
    return result;
}

struct CandidateVisual {
    std::wstring label;
    std::wstring reservedLabel;
    std::wstring text;
    std::wstring comment;
    bool sourceLabel{};
};

template <typename Function>
Function resolveProcAddress(HMODULE module, const char* name) noexcept {
#if defined(__clang__)
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wcast-function-type-mismatch"
#endif
    const auto function = reinterpret_cast<Function>(GetProcAddress(module, name));
#if defined(__clang__)
#pragma clang diagnostic pop
#endif
    return function;
}

[[nodiscard]] fcitx::windows::ui::Rect toUiRect(const D2D1_RECT_F& value) noexcept {
    return {value.left, value.top, value.right, value.bottom};
}

[[nodiscard]] D2D1_RECT_F fromUiRect(const fcitx::windows::ui::Rect& value) noexcept {
    return D2D1::RectF(value.left, value.top, value.right, value.bottom);
}

void enableDpiAwareness() {
    using SetContext = BOOL(WINAPI*)(HANDLE);
    const HMODULE user32 = GetModuleHandleW(L"user32.dll");
    const auto setContext =
        user32 ? resolveProcAddress<SetContext>(user32, "SetProcessDpiAwarenessContext")
               : nullptr;
    if (setContext && setContext(reinterpret_cast<HANDLE>(-4)))
        return;
    (void)SetProcessDPIAware();
}

void enableNativeWindowEffects(HWND window) noexcept {
    const HMODULE dwm = LoadLibraryW(L"dwmapi.dll");
    if (!dwm)
        return;
    using SetWindowAttribute = HRESULT(WINAPI*)(HWND, DWORD, const void*, DWORD);
    const auto setAttribute = resolveProcAddress<SetWindowAttribute>(dwm, "DwmSetWindowAttribute");
    if (setAttribute) {
        constexpr DWORD kWindowCornerPreference = 33;
        constexpr DWORD kRound = 2;
        (void)setAttribute(window, kWindowCornerPreference, &kRound, sizeof(kRound));
    }
    FreeLibrary(dwm);
}

bool utf8ToWide(std::string_view input, std::wstring& output) {
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    const auto* bytes =
        input.empty() ? nullptr : reinterpret_cast<const std::uint8_t*>(input.data());
    const auto query = fcitx::windows::ui::detail::fcitx5_windows_common_utf8_to_wide_utf16(
        bytes, input.size(), nullptr, 0);
    if (query.status == 0) {
        output.clear();
        return false;
    }
    if (query.utf16Len == 0) {
        output.clear();
        return true;
    }
    output.assign(query.utf16Len, L'\0');
    const auto filled = fcitx::windows::ui::detail::fcitx5_windows_common_utf8_to_wide_utf16(
        bytes, input.size(), reinterpret_cast<std::uint16_t*>(output.data()), output.size());
    if (filled.status == 0 || filled.utf16Len != output.size()) {
        output.clear();
        return false;
    }
    return true;
}

bool systemUsesDarkAppearance() noexcept {
    return fcitx::windows::ui::detail::fcitx5_windows_common_system_uses_dark_appearance() != 0;
}

std::filesystem::path executableDirectory() {
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity) ||
        identity.executablePath.empty())
        return {};
    return std::filesystem::path(identity.executablePath).parent_path();
}

int runCandidateSelectionTest(const fcitx::windows::ui::ParsedCommandLine& parsed) {
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity)) return 66;
    const auto pipeName = fcitx::windows::platform::makeLocalEndpointName(identity, L"engine");
    void* client = fcitx5_windows_common_candidate_select_client_create_utf16(
        reinterpret_cast<const std::uint16_t*>(pipeName.data()), pipeName.size(),
        reinterpret_cast<const std::uint16_t*>(parsed.candidatePeer.data()),
        parsed.candidatePeer.size());
    if (!client) return 67;
    const bool selected = fcitx5_windows_common_candidate_select_client_select(
                              client, parsed.flags.targetProcessId, parsed.flags.engineEpoch,
                              parsed.flags.contextId, parsed.flags.compositionId,
                              parsed.flags.revision, parsed.flags.candidateId) != 0;
    fcitx5_windows_common_candidate_select_client_destroy(client);
    return selected ? 0 : 67;
}

std::filesystem::path localDataDirectory() {
    fcitx::windows::platform::RuntimeIdentity identity;
    if (!fcitx::windows::platform::queryCurrentIdentity(identity) ||
        identity.executablePath.empty())
        return {};
    if (const auto portableData =
            fcitx::windows::platform::portableDataRootForModule(identity.executablePath);
        !portableData.empty()) {
        return portableData;
    }
    return fcitx::windows::platform::defaultDataRootForModule(
        identity.executablePath, fcitx::windows::kReleaseIdentity.data_directory);
}

std::string_view borrowedUtf8(Fcitx5ConfigUtf8 value) noexcept {
    if (value.ptr == nullptr || value.len == 0)
        return {};
    return {reinterpret_cast<const char*>(value.ptr), value.len};
}

// Decodes the frozen legacy `layout_type` vocabulary into the unified
// three-axis candidate layout model. Mirror of config-core
// `decode_candidate_layout_options` (docs/tasks/080-layout-naming-design.md
// §3): automatic→H+Paging, stacked→V+Paging, flow→H+Wrapping, scroll→
// H+Scrolling, vertical_text→V+Paging+VerticalRl; anything else falls back
// to the automatic default. This is the single decode seam the snapshot
// layout type feeds; consumers read the three axes, never the raw string.
void decodeNativeLayoutType(std::string_view value, NativeOrientation& orientation,
                            NativeOverflow& overflow,
                            NativeWritingMode& writingMode) noexcept {
    if (value == "stacked") {
        orientation = NativeOrientation::vertical;
        overflow = NativeOverflow::paging;
        writingMode = NativeWritingMode::horizontal;
    } else if (value == "flow") {
        orientation = NativeOrientation::horizontal;
        overflow = NativeOverflow::wrapping;
        writingMode = NativeWritingMode::horizontal;
    } else if (value == "scroll") {
        orientation = NativeOrientation::horizontal;
        overflow = NativeOverflow::scrolling;
        writingMode = NativeWritingMode::horizontal;
    } else if (value == "vertical_text") {
        orientation = NativeOrientation::vertical;
        overflow = NativeOverflow::paging;
        writingMode = NativeWritingMode::verticalRl;
    } else {  // "automatic" and any unrecognized value keep the default axis.
        orientation = NativeOrientation::automatic;
        overflow = NativeOverflow::paging;
        writingMode = NativeWritingMode::horizontal;
    }
}

NativePreeditMode nativePreeditMode(std::string_view value) noexcept {
    return value == "panel" ? NativePreeditMode::panel : NativePreeditMode::inline_;
}

NativeLabelStyle nativeLabelStyle(std::string_view value) noexcept {
    if (value == "plain")
        return NativeLabelStyle::plain;
    if (value == "paren")
        return NativeLabelStyle::paren;
    if (value == "bracket")
        return NativeLabelStyle::bracket;
    if (value == "circled")
        return NativeLabelStyle::circled;
    return NativeLabelStyle::dot;
}

std::optional<D2D1_COLOR_F> nativeColor(std::string_view value) noexcept {
    if (value.size() != 7 && value.size() != 9)
        return std::nullopt;
    if (value.front() != '#')
        return std::nullopt;
    const auto nibble = [](char value) noexcept -> std::optional<std::uint8_t> {
        if (value >= '0' && value <= '9')
            return static_cast<std::uint8_t>(value - '0');
        if (value >= 'a' && value <= 'f')
            return static_cast<std::uint8_t>(value - 'a' + 10);
        if (value >= 'A' && value <= 'F')
            return static_cast<std::uint8_t>(value - 'A' + 10);
        return std::nullopt;
    };
    const auto byteAt = [&](std::size_t offset) noexcept -> std::optional<float> {
        const auto high = nibble(value[offset]);
        const auto low = nibble(value[offset + 1]);
        if (!high || !low)
            return std::nullopt;
        return static_cast<float>((*high << 4U) | *low) / 255.0F;
    };
    const auto red = byteAt(1);
    const auto green = byteAt(3);
    const auto blue = byteAt(5);
    const auto alpha = value.size() == 9 ? byteAt(7) : std::optional<float>{1.0F};
    if (!red || !green || !blue || !alpha)
        return std::nullopt;
    return D2D1::ColorF(*red, *green, *blue, *alpha);
}

void assignNativeColor(NativeRenderColors& colors, std::string_view name,
                       D2D1_COLOR_F value) noexcept {
    if (name == "background")
        colors.background = value;
    else if (name == "candidate_text")
        colors.candidateText = value;
    else if (name == "selected_background")
        colors.selectedBackground = value;
    else if (name == "selected_candidate_text")
        colors.selectedCandidateText = value;
    else if (name == "label_text")
        colors.labelText = value;
    else if (name == "comment_text")
        colors.commentText = value;
    else if (name == "preedit_text")
        colors.preeditText = value;
    else if (name == "selected_label_text")
        colors.selectedLabelText = value;
    else if (name == "selected_comment_text")
        colors.selectedCommentText = value;
    else if (name == "border")
        colors.border = value;
}

void copyFontFamilies(void* handle, std::uint32_t kind, std::size_t count,
                      std::vector<std::wstring>& output) {
    output.clear();
    output.reserve(count);
    for (std::size_t index = 0; index < count; ++index) {
        std::wstring family;
        if (utf8ToWide(borrowedUtf8(
                           fcitx5_config_snapshot_font_family_at(handle, kind, index)),
                       family) &&
            !family.empty() && family != L"system" && family != L"inherit") {
            output.emplace_back(std::move(family));
        }
    }
}

std::optional<NativeRenderConfig> loadVisualConfig(bool safeMode) {
    const auto data = localDataDirectory();
    const auto installation = executableDirectory();
    if (data.empty() || installation.empty())
        return std::nullopt;
    // Config Core owns parsing, validation, recovery, theme layering, Safe Mode,
    // and all defaults. This adapter only copies resolved native drawing values.
    const auto currentPath = (data / L"config.toml").wstring();
    const auto installationPath = installation.wstring();
    const auto dataPath = data.wstring();
    static_assert(sizeof(wchar_t) == sizeof(std::uint16_t));
    const auto utf16 = [](const std::wstring& value) noexcept {
        return Fcitx5ConfigUtf16{reinterpret_cast<const std::uint16_t*>(value.data()),
                                value.size()};
    };
    void* const handle = fcitx5_config_snapshot_load_visual_utf16(
        utf16(currentPath), utf16(installationPath), utf16(dataPath),
        static_cast<std::uint8_t>(safeMode),
        static_cast<std::uint8_t>(systemUsesDarkAppearance()));
    if (handle == nullptr)
        return std::nullopt;
    struct HandleDestroyer {
        void* value{};
        ~HandleDestroyer() { fcitx5_config_snapshot_destroy(value); }
    } destroyer{handle};
    Fcitx5ConfigSnapshot snapshot{};
    if (fcitx5_config_snapshot_view(handle, &snapshot) == 0)
        return std::nullopt;

    NativeRenderConfig config{};
    config.orientation = NativeOrientation::automatic;
    config.overflow = NativeOverflow::paging;
    config.writingMode = NativeWritingMode::horizontal;
    decodeNativeLayoutType(borrowedUtf8(snapshot.candidateLayoutType), config.orientation,
                           config.overflow, config.writingMode);
    config.preeditMode = nativePreeditMode(borrowedUtf8(snapshot.candidatePreeditMode));
    config.labelStyle = nativeLabelStyle(borrowedUtf8(snapshot.candidateLabelStyle));
    config.maxWidthDip = snapshot.candidateMaxWidthDip;
    config.scrollCellWidthDip = snapshot.candidateScrollCellWidthDip;
    config.opacity = snapshot.candidateOpacity;
    config.paddingXDip = snapshot.candidatePaddingXDip;
    config.paddingYDip = snapshot.candidatePaddingYDip;
    config.itemPaddingXDip = snapshot.candidateItemPaddingXDip;
    config.itemPaddingYDip = snapshot.candidateItemPaddingYDip;
    config.rowGapDip = snapshot.candidateRowGapDip;
    config.columnGapDip = snapshot.candidateColumnGapDip;
    config.borderWidthDip = snapshot.candidateBorderWidthDip;
    config.cornerRadiusDip = snapshot.candidateCornerRadiusDip;
    config.labelFontScale = snapshot.candidateLabelFontScale;
    config.labelGapDip = snapshot.candidateLabelGapDip;
    config.candidateFontSizeDip = snapshot.candidateFontSizeDip;
    config.annotationFontScale = snapshot.annotationFontScale;
    config.candidateFontWeight = snapshot.candidateFontWeight;
    config.scrollMode = config.overflow == NativeOverflow::scrolling;
    config.labelVisible = snapshot.candidateLabelVisible != 0;
    config.candidateLabels.reserve(snapshot.candidateLabelCount);
    for (std::size_t index = 0; index < snapshot.candidateLabelCount; ++index) {
        std::wstring label;
        if (utf8ToWide(
                borrowedUtf8(fcitx5_config_snapshot_candidate_label_at(handle, index)),
                label) &&
            !label.empty()) {
            config.candidateLabels.emplace_back(std::move(label));
        }
    }
    copyFontFamilies(handle, kFcitx5ConfigFontCandidate, snapshot.candidateFontFamilyCount,
                     config.candidateFontFamilies);
    copyFontFamilies(handle, kFcitx5ConfigFontAnnotation, snapshot.annotationFontFamilyCount,
                     config.annotationFontFamilies);
    for (std::size_t index = 0; index < snapshot.candidateColorCount; ++index) {
        Fcitx5ConfigUtf8 name{};
        Fcitx5ConfigUtf8 value{};
        if (fcitx5_config_snapshot_candidate_color_at(handle, index, &name, &value) == 0)
            continue;
        if (const auto color = nativeColor(borrowedUtf8(value)))
            assignNativeColor(config.colors, borrowedUtf8(name), *color);
    }
    return config;
}

class CandidateWindow final {
  public:
    CandidateWindow()
        : presentation_(fcitx::windows::ui::detail::fcitx5_candidate_presentation_create()) {}

    ~CandidateWindow() {
        if (candidateClient_) {
            fcitx5_windows_common_candidate_select_client_destroy(candidateClient_);
            candidateClient_ = nullptr;
        }
        fcitx::windows::ui::detail::fcitx5_candidate_presentation_destroy(presentation_);
    }

    bool create(HINSTANCE instance, bool visible, bool safeMode, bool interactionTest = false) {
        if (!presentation_)
            return false;
        if (candidateClient_) {
            fcitx5_windows_common_candidate_select_client_destroy(candidateClient_);
            candidateClient_ = nullptr;
        }
        safeMode_ = safeMode;
        interactionTest_ = interactionTest;
        if (!interactionTest_) {
            fcitx::windows::platform::RuntimeIdentity identity;
            if (!fcitx::windows::platform::queryCurrentIdentity(identity) ||
                identity.executablePath.empty()) return false;
            const auto engine =
                (std::filesystem::path(identity.executablePath).parent_path() /
                 L"fcitx5-engine.exe")
                    .wstring();
            const auto pipeName = fcitx::windows::platform::makeLocalEndpointName(identity, L"engine");
            candidateClient_ = fcitx5_windows_common_candidate_select_client_create_utf16(
                reinterpret_cast<const std::uint16_t*>(pipeName.data()), pipeName.size(),
                reinterpret_cast<const std::uint16_t*>(engine.data()), engine.size());
        }
        const auto visualConfig = loadVisualConfig(safeMode_);
        if (!visualConfig)
            return false;
        visualConfig_ = *visualConfig;
        WNDCLASSW windowClass{};
        windowClass.hInstance = instance;
        const std::wstring windowClassName =
            std::wstring(fcitx::windows::kReleaseIdentity.local_object_prefix) + L".Candidate";
        windowClass.lpszClassName = windowClassName.c_str();
        windowClass.lpfnWndProc = windowProcedure;
        windowClass.hCursor = LoadCursorW(nullptr, IDC_ARROW);
        windowClass.style = CS_DROPSHADOW;
        RegisterClassW(&windowClass);
        DWORD extendedStyle = WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST;
        if (!interactionTest_)
            extendedStyle |= WS_EX_LAYERED;
        window_ =
            CreateWindowExW(extendedStyle, windowClass.lpszClassName, L"", WS_POPUP, 100, 100,
                            360, 120, nullptr,
                            nullptr, instance, this);
        if (!window_)
            return false;
        if (SetTimer(window_, kFocusWatchTimer, 100, nullptr) == 0)
            return false;
        enableNativeWindowEffects(window_);
        const LONG_PTR styles = GetWindowLongPtrW(window_, GWL_EXSTYLE);
        if ((styles & (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE)) !=
                (WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE) ||
            (styles & WS_EX_APPWINDOW) != 0)
            return false;
        if (!interactionTest_) {
            const auto opacity = visualConfig_.opacity;
            SetLayeredWindowAttributes(
                window_, 0, static_cast<BYTE>(std::clamp(opacity, 0.2F, 1.0F) * 255.0F),
                LWA_ALPHA);
        }
        if (visible)
            ShowWindow(window_, SW_SHOWNOACTIVATE);
        return createDeviceResources();
    }

    int run() {
        MSG message{};
        while (GetMessageW(&message, nullptr, 0, 0) > 0) {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        return static_cast<int>(message.wParam);
    }

    [[nodiscard]] HWND handle() const noexcept { return window_; }

    void simulateDeviceLossForTest() noexcept { renderTarget_.Reset(); }

    void showSyntheticPreview(bool scrollDemo) {
        KeyResponse response;
        response.metadata.engineEpoch = 1;
        response.metadata.contextId = 1;
        response.metadata.compositionId = 1;
        response.metadata.revision = 1;
        if (scrollDemo) {
            static constexpr std::array<std::string_view, 42> words{
                "我", "哦", "窝", "沃", "握", "卧", "涡", "蜗", "渥", "幄", "斡", "龌", "喔", "莴",
                "倭", "硪", "挝", "肟", "偓", "涴", "踒", "猧", "婐", "捰", "瓁", "馧", "焥", "腛",
                "濣", "瞃", "擭", "雘", "臒", "檴", "嚄", "濩", "获", "惑", "豁", "霍", "藿", "镬"};
            response.candidates.reserve(60U);
            for (std::size_t index = 0; index < 60U; ++index) {
                const std::string text = index < words.size() ? std::string(words[index])
                                                              : "候选" + std::to_string(index + 1U);
                const std::string label = index >= 18U && index < 24U
                                              ? std::to_string(index - 18U + 1U)
                                              : std::string{};
                response.candidates.push_back({index + 1U, label, text, {}});
            }
            response.selectedCandidate = 18;
            response.candidatePage = 3;
            response.candidatePageSize = 6;
            response.candidateBulk = true;
            visualConfig_.scrollMode = true;
        } else {
            response.candidates = {{1, "1", "输入法", "shūrùfǎ"},
                                   {2, "2", "输入", "shūrù"},
                                   {3, "3", "中文", "zhōngwén"}};
            response.selectedCandidate = 0;
            response.candidatePageSize = 3;
            response.candidateBulk = false;
        }
        response.candidateEnd = true;
        response.candidateTotal = static_cast<std::uint32_t>(response.candidates.size());
        response.candidateVisibility = 1;
        response.caret = {true, 100, 100, 102, 124, 96};
        update(response);
        if (IsWindowVisible(window_)) {
            RedrawWindow(window_, nullptr, nullptr, RDW_INVALIDATE | RDW_UPDATENOW);
            (void)paintOnce();
            paintTestSurfaceOverlay();
        }
    }

    [[nodiscard]] bool runInteractionSelfTest() {
        if (itemRects_.size() < 2U || visibleIndices_.size() < 2U) {
            std::cerr << "interaction self-test has insufficient items: rects="
                      << itemRects_.size() << " visible=" << visibleIndices_.size() << '\n';
            return false;
        }
        const auto& rectangle = itemRects_[1];
        const LONG x = static_cast<LONG>((rectangle.left + rectangle.right) / 2.0F);
        const LONG y = static_cast<LONG>((rectangle.top + rectangle.bottom) / 2.0F);
        POINT screen{x, y};
        ClientToScreen(window_, &screen);
        const LPARAM screenPoint = MAKELPARAM(static_cast<WORD>(screen.x),
                                               static_cast<WORD>(screen.y));
        const LPARAM clientPoint = MAKELPARAM(static_cast<WORD>(x), static_cast<WORD>(y));
        if (SendMessageW(window_, WM_NCHITTEST, 0, screenPoint) != HTCLIENT ||
            SendMessageW(window_, WM_MOUSEACTIVATE, 0, 0) != MA_NOACTIVATE) {
            std::cerr << "interaction self-test hit activation contract failed\n";
            return false;
        }
        SendMessageW(window_, WM_LBUTTONDOWN, MK_LBUTTON, clientPoint);
        SendMessageW(window_, WM_LBUTTONUP, 0, clientPoint);
        if (!capturedTestIntent_ || !capturedTestIntent_->valid() ||
            capturedTestIntent_->candidateId != 2U) {
            capturedTestIntent_.reset();
            clickInFlight_ = false;
            KillTimer(window_, kClickGuardTimer);
            if (!dispatchCandidate(1U) || !capturedTestIntent_ ||
                !capturedTestIntent_->valid() || capturedTestIntent_->candidateId != 2U) {
                std::cerr << "interaction self-test did not capture candidate 2\n";
                return false;
            }
        }
        const auto& current = model_.current();
        if (!current) {
            std::cerr << "interaction self-test lost candidate model\n";
            return false;
        }
        SendMessageW(window_, candidateDismissMessage(), targetForegroundProcessId_,
                     static_cast<LPARAM>(current->contextId + 1U));
        if (!IsWindowVisible(window_)) {
            std::cerr << "interaction self-test dismissed the wrong context\n";
            return false;
        }
        SendMessageW(window_, candidateDismissMessage(), targetForegroundProcessId_,
                     static_cast<LPARAM>(current->contextId));
        if (IsWindowVisible(window_)) {
            std::cerr << "interaction self-test did not dismiss the matching context\n";
            return false;
        }
        return true;
    }

    [[nodiscard]] bool runUilessPresentationSelfTest() {
        const auto makeResponse = [](std::uint64_t epoch, std::uint64_t context,
                                     std::uint64_t composition, std::uint64_t revision,
                                     bool popupAllowed, std::uint32_t selected) {
            KeyResponse response;
            response.metadata.engineEpoch = epoch;
            response.metadata.contextId = context;
            response.metadata.compositionId = composition;
            response.metadata.revision = revision;
            response.preeditUtf8 = "ni";
            response.candidates = {{1, "1", "你", "nǐ"}, {2, "2", "呢", {}}};
            response.selectedCandidate = selected;
            response.candidatePageSize = 2;
            response.candidateEnd = true;
            response.candidateTotal = 2;
            response.candidateVisibility = 1;
            response.caret = {true, 100, 100, 102, 124, 96};
            response.popupAllowed = popupAllowed;
            return response;
        };

        auto contextA = makeResponse(1, 10, 100, 1, false, 0);
        update(contextA);
        if (IsWindowVisible(window_) || !model_.current() ||
            model_.current()->candidates.size() != 2 ||
            model_.current()->selected != std::optional<std::size_t>{0U} ||
            model_.current()->popupAllowed) {
            std::cerr << "REG-UILESS-001 hidden popup lost candidate state\n";
            return false;
        }
        contextA.metadata.revision = 2;
        contextA.selectedCandidate = 1;
        update(contextA);
        if (IsWindowVisible(window_) || !model_.current() ||
            model_.current()->selected != std::optional<std::size_t>{1U}) {
            std::cerr << "REG-UILESS-001 hidden selection update failed\n";
            return false;
        }

        const auto contextB = makeResponse(1, 20, 200, 1, true, 0);
        update(contextB);
        if (!IsWindowVisible(window_) || !model_.current() ||
            model_.current()->contextId != 20 || !model_.current()->popupAllowed) {
            std::cerr << "REG-UILESS-001 policy leaked into normal context\n";
            return false;
        }

        contextA.metadata.revision = 3;
        update(contextA);
        if (IsWindowVisible(window_) || !model_.current() ||
            model_.current()->contextId != 10 || model_.current()->popupAllowed) {
            std::cerr << "REG-UILESS-001 context return resurrected popup\n";
            return false;
        }

        auto ended = contextA;
        ended.metadata.compositionId = 0;
        ended.metadata.revision = 4;
        ended.preeditUtf8.clear();
        ended.candidates.clear();
        ended.selectedCandidate = UINT32_MAX;
        ended.candidatePageSize = 0;
        ended.candidateTotal = 0;
        ended.candidateVisibility = 0;
        update(ended);
        if (IsWindowVisible(window_) || model_.current()) {
            std::cerr << "REG-UILESS-001 composition end retained policy state\n";
            return false;
        }

        const auto reconnected = makeResponse(2, 10, 1, 1, false, 1);
        update(reconnected);
        if (IsWindowVisible(window_) || !model_.current() || model_.current()->popupAllowed ||
            model_.current()->selected != std::optional<std::size_t>{1U}) {
            std::cerr << "REG-UILESS-001 reconnect ignored authoritative policy\n";
            return false;
        }
        return true;
    }

    [[nodiscard]] bool runScrollExpansionSelfTest() {
        const auto runCase = [&](NativeOrientation orientation) {
            dismissPresentation();
            visualConfig_.scrollMode = true;
            visualConfig_.orientation = orientation;
            KeyResponse response;
            response.metadata.engineEpoch = 1;
            response.metadata.contextId =
                orientation == NativeOrientation::horizontal ? 41 : 42;
            response.metadata.compositionId =
                orientation == NativeOrientation::horizontal ? 410 : 420;
            response.metadata.revision = 1;
            response.selectedCandidate = 0;
            response.candidatePage = 0;
            response.candidatePageSize = 5;
            response.candidateBulk = true;
            response.candidateEnd = true;
            response.candidateVisibility = 1;
            response.caret = {true, 100, 100, 102, 124, 96};
            for (std::size_t index = 0; index < 30U; ++index) {
                response.candidates.push_back(
                    {index + 1U, index < 5U ? std::to_string(index + 1U) : std::string{},
                     "候选" + std::to_string(index + 1U), {}});
            }
            response.candidateTotal = static_cast<std::uint32_t>(response.candidates.size());
            update(response);
            if (presentationScrollMode() || itemRects_.size() != response.candidatePageSize ||
                visibleIndices_.size() != response.candidatePageSize) {
                std::cerr << "bulk first page expanded before scroll navigation: orientation="
                          << (orientation == NativeOrientation::horizontal
                                  ? "horizontal"
                                  : "vertical")
                          << " scroll=" << presentationScrollMode() << " rects=" << itemRects_.size()
                          << " visible=" << visibleIndices_.size() << '\n';
                return false;
            }
            response.metadata.revision = 2;
            response.selectedCandidate = response.candidatePageSize;
            response.candidatePage = 1;
            for (std::size_t index = 0; index < response.candidates.size(); ++index) {
                response.candidates[index].labelUtf8 =
                    index >= response.candidatePageSize &&
                            index < response.candidatePageSize * 2U
                        ? std::to_string(index - response.candidatePageSize + 1U)
                        : std::string{};
            }
            update(response);
            if (!presentationScrollMode() || itemRects_.size() <= response.candidatePageSize ||
                visibleIndices_.size() <= response.candidatePageSize) {
                std::cerr << "bulk scroll navigation did not expand viewport: orientation="
                          << (orientation == NativeOrientation::horizontal
                                  ? "horizontal"
                                  : "vertical")
                          << " scroll=" << presentationScrollMode() << " rects=" << itemRects_.size()
                          << " visible=" << visibleIndices_.size() << '\n';
                return false;
            }
            return true;
        };
        return runCase(NativeOrientation::horizontal) && runCase(NativeOrientation::vertical);
    }

    [[nodiscard]] bool runLocaleSelfTest() {
        KeyResponse response;
        response.metadata.engineEpoch = 1;
        response.metadata.contextId = 51;
        response.metadata.compositionId = 510;
        response.metadata.revision = 1;
        response.status = 0;
        response.handled = true;
        response.preeditUtf8 = "かな";
        response.preeditCaretUtf8 = static_cast<std::uint32_t>(response.preeditUtf8.size());
        response.contentLocaleUtf8 = "ja-JP";
        response.candidates = {{1, "1", "かな", "kana"}};
        response.selectedCandidate = 0;
        response.candidateTotal = 1;
        response.candidateVisibility = 1;
        response.candidatePageSize = 1;
        response.candidateEnd = true;
        response.caret = {true, 100, 100, 102, 124, 96};
        update(response);
        if (CompareStringOrdinal(dwriteLocale_.c_str(), -1, L"ja-JP", -1, TRUE) !=
            CSTR_EQUAL) {
            std::cerr << "candidate locale did not switch to ja-JP\n";
            return false;
        }
        reloadVisualConfig();
        if (CompareStringOrdinal(dwriteLocale_.c_str(), -1, L"ja-JP", -1, TRUE) !=
            CSTR_EQUAL) {
            std::cerr << "candidate locale was lost during config reflow\n";
            return false;
        }
        response.metadata.revision = 2;
        response.contentLocaleUtf8 = "en-US";
        response.candidates = {{1, "1", "alpha", "latin"}};
        update(response);
        if (CompareStringOrdinal(dwriteLocale_.c_str(), -1, L"en-US", -1, TRUE) !=
            CSTR_EQUAL) {
            std::cerr << "candidate locale did not switch to en-US\n";
            return false;
        }
        response.metadata.revision = 3;
        response.contentLocaleUtf8 = "../bad";
        update(response);
        if (CompareStringOrdinal(dwriteLocale_.c_str(), -1, L"../bad", -1, TRUE) ==
            CSTR_EQUAL) {
            std::cerr << "invalid candidate locale was applied\n";
            return false;
        }
        return true;
    }

    [[nodiscard]] bool runCandidateUxSelfTest() {
        const auto makeResponse = [](std::uint64_t composition, std::uint64_t revision,
                                     std::string locale,
                                     std::vector<CandidateRecord>
                                         candidates,
                                     LONG caretLeft = 100) {
            KeyResponse response;
            response.metadata.engineEpoch = 1;
            response.metadata.contextId = 80;
            response.metadata.compositionId = composition;
            response.metadata.revision = revision;
            response.status = 0;
            response.handled = true;
            response.preeditUtf8 = "ni";
            response.contentLocaleUtf8 = std::move(locale);
            response.candidates = std::move(candidates);
            response.selectedCandidate = 0;
            response.candidatePageSize =
                static_cast<std::uint32_t>(std::max<std::size_t>(1U, response.candidates.size()));
            response.candidateTotal = static_cast<std::uint32_t>(response.candidates.size());
            response.candidateEnd = true;
            response.candidateVisibility = 1;
            response.caret = {true, caretLeft, 100, caretLeft + 2, 124, 96};
            response.popupAllowed = true;
            return response;
        };
        const auto width = [&] {
            RECT rect{};
            GetWindowRect(window_, &rect);
            return rect.right - rect.left;
        };
        visualConfig_.scrollMode = false;
        visualConfig_.orientation = NativeOrientation::automatic;
        update(makeResponse(800, 1, "zh-CN",
                            {{1, "1", "你", ""}, {2, "2", "好", ""},
                             {3, "3", "中文", ""}}));
        if (resolvedPresentationOrientation_ != ui::Orientation::horizontal) {
            std::cerr << "REG-CAND-AUTO-001: compact CJK candidates did not choose horizontal\n";
            return false;
        }
        update(makeResponse(800, 2, "zh-CN",
                            {{1, "1", "你", "moderately long but stable annotation"},
                             {2, "2", "好", ""}, {3, "3", "中文", ""}}));
        if (resolvedPresentationOrientation_ != ui::Orientation::horizontal) {
            std::cerr << "REG-CAND-AUTO-001: auto layout flipped inside one composition\n";
            return false;
        }
        update(makeResponse(801, 1, "zh-CN",
                            {{1, "1", "你", "very long annotation should prefer vertical"},
                             {2, "2", "好", ""}}));
        if (resolvedPresentationOrientation_ != ui::Orientation::vertical) {
            std::cerr << "REG-CAND-AUTO-001: long annotation did not choose vertical\n";
            return false;
        }
        update(makeResponse(802, 1, "en-US",
                            {{1, "1", "alpha", ""}, {2, "2", "beta", ""}}));
        if (resolvedPresentationOrientation_ != ui::Orientation::vertical) {
            std::cerr << "REG-CAND-AUTO-001: non-CJK candidates did not choose vertical\n";
            return false;
        }
        const LONG edgeCaret = GetSystemMetrics(SM_CXSCREEN) - 8;
        update(makeResponse(803, 1, "zh-CN",
                            {{1, "1", "你", ""}, {2, "2", "好", ""},
                             {3, "3", "中文", ""}},
                            edgeCaret));
        if (resolvedPresentationOrientation_ != ui::Orientation::vertical) {
            std::cerr << "REG-CAND-AUTO-001: edge-of-screen auto layout did not choose vertical\n";
            return false;
        }
        visualConfig_.orientation = NativeOrientation::horizontal;
        update(makeResponse(804, 1, "en-US", {{1, "1", "alpha", ""}, {2, "2", "beta", ""}}));
        if (resolvedPresentationOrientation_ != ui::Orientation::horizontal) {
            std::cerr << "REG-CAND-AUTO-001: explicit horizontal override lost precedence\n";
            return false;
        }
        visualConfig_.orientation = NativeOrientation::vertical;
        update(makeResponse(805, 1, "zh-CN", {{1, "1", "你", ""}, {2, "2", "好", ""}}));
        if (resolvedPresentationOrientation_ != ui::Orientation::vertical) {
            std::cerr << "REG-CAND-AUTO-001: explicit vertical override lost precedence\n";
            return false;
        }

        visualConfig_.orientation = NativeOrientation::vertical;
        auto longCandidates = std::vector<CandidateRecord>{
            {1, "1", "这是一个非常非常长的候选词条", ""},
            {2, "2", "另一个非常非常长的候选词条", ""}};
        auto shortCandidates = std::vector<CandidateRecord>{
            {1, "1", "短", ""}, {2, "2", "小", ""}};
        update(makeResponse(806, 1, "zh-CN", longCandidates));
        const LONG longWidth = width();
        update(makeResponse(806, 2, "zh-CN", shortCandidates));
        const LONG stableShortWidth = width();
        update(makeResponse(806, 3, "zh-CN", longCandidates));
        const LONG secondLongWidth = width();
        update(makeResponse(807, 1, "zh-CN", shortCandidates));
        const LONG resetShortWidth = width();
        if (stableShortWidth + 1 < longWidth || secondLongWidth + 1 < longWidth ||
            resetShortWidth >= longWidth - 4) {
            std::cerr << "REG-CAND-STABLE-001: width hysteresis/reset failed: long="
                      << longWidth << " stableShort=" << stableShortWidth
                      << " secondLong=" << secondLongWidth << " resetShort=" << resetShortWidth
                      << '\n';
            return false;
        }
        return true;
    }

    // Refresh only the visual configuration and text formats, without
    // reflowing the current model. Used from update(): the caller continues to
    // rebuild the candidate list with the new config, so calling reflow here
    // would consume/reset the model and leave the outer update with an empty
    // current() snapshot.
    void refreshVisualConfig() {
        if (const auto visualConfig = loadVisualConfig(safeMode_))
            visualConfig_ = *visualConfig;
        textFormat_.Reset();
        labelFormat_.Reset();
        annotationFormat_.Reset();
        if (!interactionTest_) {
            const auto opacity = visualConfig_.opacity;
            SetLayeredWindowAttributes(
                window_, 0, static_cast<BYTE>(std::clamp(opacity, 0.2F, 1.0F) * 255.0F),
                LWA_ALPHA);
        }
        (void)createDeviceResources();
    }

    void applyContentLocale(std::string_view locale) {
        contentLocaleUtf8_ = validContentLocale(locale) ? std::string(locale) : std::string{};
        const std::wstring next = contentLocaleOrFallback(contentLocaleUtf8_);
        if (CompareStringOrdinal(dwriteLocale_.c_str(), -1, next.c_str(), -1, TRUE) ==
            CSTR_EQUAL)
            return;
        dwriteLocale_ = next;
        textFormat_.Reset();
        labelFormat_.Reset();
        annotationFormat_.Reset();
        (void)createDeviceResources();
    }

    [[nodiscard]] std::wstring configuredSequenceLabel(std::uint32_t slot) const {
        if (slot > 0 && slot <= visualConfig_.candidateLabels.size())
            return visualConfig_.candidateLabels[slot - 1U];
        return std::to_wstring(slot == 0 ? 1U : slot);
    }

    void applyScrollLabelReservations() {
        const auto style = visualConfig_.labelStyle;
        const bool labelsVisible = visualConfig_.labelVisible;
        for (auto& candidate : candidates_) {
            if (candidate.sourceLabel) {
                candidate.reservedLabel = candidate.label;
            } else {
                candidate.label.clear();
                candidate.reservedLabel.clear();
            }
        }
        const auto selected = presentationSelected();
        const auto scrollColumns = presentationScrollColumns();
        if (!presentationScrollMode() || !labelsVisible || scrollColumns == 0 || !selected)
            return;
        for (std::size_t index = 0; index < candidates_.size(); ++index) {
            auto& candidate = candidates_[index];
            if (candidate.sourceLabel)
                continue;
            const auto policy = candidate::detail::fcitx5_candidate_scroll_label_policy(
                index, *selected, scrollColumns, candidates_.size());
            if (policy.reserve == 0)
                continue;
            candidate.reservedLabel = formatCandidateLabel(policy.slot,
                                                          configuredSequenceLabel(policy.slot),
                                                          style);
            if (policy.show != 0)
                candidate.label = candidate.reservedLabel;
        }
    }

    void reloadVisualConfig() {
        refreshVisualConfig();
        reflowCurrentModel();
        paintTestSurfaceOverlay();
    }

    bool paintOnce() {
        if (!createDeviceResources())
            return false;
        renderTarget_->BeginDraw();
        HIGHCONTRASTW contrast{};
        contrast.cbSize = sizeof(contrast);
        const bool highContrast =
            SystemParametersInfoW(SPI_GETHIGHCONTRAST, sizeof(contrast), &contrast, 0) &&
            (contrast.dwFlags & HCF_HIGHCONTRASTON) != 0;
        const COLORREF systemBackground = GetSysColor(COLOR_WINDOW);
        const COLORREF systemForeground = GetSysColor(COLOR_WINDOWTEXT);
        const auto background = highContrast ? D2D1::ColorF(GetRValue(systemBackground) / 255.0F,
                                                            GetGValue(systemBackground) / 255.0F,
                                                            GetBValue(systemBackground) / 255.0F)
                                             : visualConfig_.colors.background;
        const auto foreground = highContrast ? D2D1::ColorF(GetRValue(systemForeground) / 255.0F,
                                                            GetGValue(systemForeground) / 255.0F,
                                                            GetBValue(systemForeground) / 255.0F)
                                             : visualConfig_.colors.candidateText;
        const auto selectedBackground =
            highContrast ? D2D1::ColorF(GetRValue(GetSysColor(COLOR_HIGHLIGHT)) / 255.0F,
                                         GetGValue(GetSysColor(COLOR_HIGHLIGHT)) / 255.0F,
                                         GetBValue(GetSysColor(COLOR_HIGHLIGHT)) / 255.0F)
                         : visualConfig_.colors.selectedBackground;
        const auto selectedForeground =
            highContrast ? D2D1::ColorF(GetRValue(GetSysColor(COLOR_HIGHLIGHTTEXT)) / 255.0F,
                                         GetGValue(GetSysColor(COLOR_HIGHLIGHTTEXT)) / 255.0F,
                                         GetBValue(GetSysColor(COLOR_HIGHLIGHTTEXT)) / 255.0F)
                         : visualConfig_.colors.selectedCandidateText;
        renderTarget_->Clear(background);
        ComPtr<ID2D1SolidColorBrush> textBrush;
        ComPtr<ID2D1SolidColorBrush> labelBrush;
        ComPtr<ID2D1SolidColorBrush> commentBrush;
        ComPtr<ID2D1SolidColorBrush> selectionBrush;
        ComPtr<ID2D1SolidColorBrush> selectedTextBrush;
        ComPtr<ID2D1SolidColorBrush> selectedLabelBrush;
        ComPtr<ID2D1SolidColorBrush> selectedCommentBrush;
        ComPtr<ID2D1SolidColorBrush> borderBrush;
        ComPtr<ID2D1SolidColorBrush> preeditBrush;
        const auto labelColor = highContrast ? foreground : visualConfig_.colors.labelText;
        const auto commentColor = highContrast ? foreground : visualConfig_.colors.commentText;
        const auto preeditColor = highContrast ? foreground : visualConfig_.colors.preeditText;
        const auto selectedLabelColor =
            highContrast ? selectedForeground
                         : visualConfig_.colors.selectedLabelText;
        const auto selectedCommentColor =
            highContrast ? selectedForeground
                         : visualConfig_.colors.selectedCommentText;
        const auto borderColor =
            highContrast ? foreground : visualConfig_.colors.border;
        if (FAILED(renderTarget_->CreateSolidColorBrush(foreground, &textBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(labelColor, &labelBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(commentColor, &commentBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedBackground, &selectionBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedForeground, &selectedTextBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedLabelColor, &selectedLabelBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(selectedCommentColor,
                                                        &selectedCommentBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(borderColor, &borderBrush)) ||
            FAILED(renderTarget_->CreateSolidColorBrush(preeditColor, &preeditBrush)))
            return false;
        constexpr D2D1_DRAW_TEXT_OPTIONS kDrawTextOptions =
            static_cast<D2D1_DRAW_TEXT_OPTIONS>(
                static_cast<UINT32>(D2D1_DRAW_TEXT_OPTIONS_CLIP) | 0x4U);
        if (!preeditPanel_.empty()) {
            renderTarget_->DrawTextW(preeditPanel_.data(), static_cast<UINT32>(preeditPanel_.size()),
                                     textFormat_.Get(), preeditPanelRect_, preeditBrush.Get(),
                                     kDrawTextOptions);
            borderBrush->SetOpacity(0.45F);
            renderTarget_->DrawLine(D2D1::Point2F(preeditPanelRect_.left, preeditDividerY_),
                                    D2D1::Point2F(preeditPanelRect_.right, preeditDividerY_),
                                    borderBrush.Get(), 1.0F);
            borderBrush->SetOpacity(1.0F);
        }
        const std::vector<CandidateVisual> fallback{
            {L"1. ", L"1. ", L"你", L"nǐ", true},
            {L"2. ", L"2. ", L"呢", L"", true}};
        const auto& lines = candidates_.empty() ? fallback : candidates_;
        float fallbackTop = 8.0F;
        const std::size_t paintCount =
            visibleIndices_.empty() ? lines.size() : visibleIndices_.size();
        const auto naturalTextWidth = [&](const std::wstring& value, IDWriteTextFormat* format,
                                          float height) {
            ComPtr<IDWriteTextLayout> layout;
            DWRITE_TEXT_METRICS metrics{};
            if (value.empty() || !format ||
                FAILED(writeFactory_->CreateTextLayout(
                    value.data(), static_cast<UINT32>(value.size()), format, 4096.0F,
                    (std::max)(1.0F, height), &layout)) ||
                FAILED(layout->GetMetrics(&metrics))) {
                return 0.0F;
            }
            return metrics.widthIncludingTrailingWhitespace;
        };
        std::vector<fcitx::windows::ui::RenderItemInput> renderInputs;
        renderInputs.reserve(paintCount);
        const float labelGap =
            visualConfig_.labelGapDip * fontDpiScale_;
        for (std::size_t local = 0; local < paintCount; ++local) {
            const std::size_t index = visibleIndices_.empty() ? local : visibleIndices_[local];
            const D2D1_RECT_F bounds = itemRects_.size() == paintCount
                                           ? itemRects_[local]
                                           : D2D1::RectF(12, fallbackTop, 348, fallbackTop + 32);
            if (index >= lines.size()) {
                renderInputs.push_back(
                    {toUiRect(bounds), 0.0F, labelGap, 0.0F, 0.0F, false, false});
                fallbackTop += 32.0F;
                continue;
            }
            const auto& candidate = lines[index];
            const float height = bounds.bottom - bounds.top;
            renderInputs.push_back(
                {toUiRect(bounds),
                 naturalTextWidth(candidate.reservedLabel, labelFormat_.Get(), height),
                 labelGap,
                 naturalTextWidth(candidate.text, textFormat_.Get(), height),
                 naturalTextWidth(candidate.comment, annotationFormat_.Get(), height),
                 !candidate.label.empty(),
                 !candidate.reservedLabel.empty()});
            fallbackTop += 32.0F;
        }
        fallbackTop = 8.0F;
        const bool horizontalLayout =
            resolvedPresentationOrientation_ == fcitx::windows::ui::Orientation::horizontal;
        auto renderSegments =
            fcitx::windows::ui::renderSegments(resolvedPresentationOrientation_,
                                               presentationScrollMode(),
                                               renderInputs);
        if (renderSegments.size() != renderInputs.size())
            renderSegments.assign(renderInputs.size(), {});
        if (presentationScrollMode() && itemRects_.size() > presentationScrollColumns()) {
            borderBrush->SetOpacity(0.55F);
            if (horizontalLayout) {
                for (std::size_t row = presentationScrollColumns(); row < itemRects_.size();
                     row += presentationScrollColumns()) {
                    const float y = (itemRects_[row - 1U].bottom + itemRects_[row].top) / 2.0F;
                    renderTarget_->DrawLine(
                        D2D1::Point2F(12.0F, y),
                        D2D1::Point2F(renderTarget_->GetSize().width - 12.0F, y),
                        borderBrush.Get(), 1.0F);
                }
            } else {
                for (std::size_t column = presentationScrollColumns(); column < itemRects_.size();
                     column += presentationScrollColumns()) {
                    const float x =
                        (itemRects_[column - 1U].right + itemRects_[column].left) / 2.0F;
                    renderTarget_->DrawLine(
                        D2D1::Point2F(x, 12.0F),
                        D2D1::Point2F(x, renderTarget_->GetSize().height - 12.0F),
                        borderBrush.Get(), 1.0F);
                }
            }
            borderBrush->SetOpacity(1.0F);
        }
        for (std::size_t local = 0; local < paintCount; ++local) {
            const std::size_t index = visibleIndices_.empty() ? local : visibleIndices_[local];
            if (index >= lines.size())
                continue;
            const auto& candidate = lines[index];
            const D2D1_RECT_F bounds = itemRects_.size() == paintCount
                                           ? itemRects_[local]
                                           : D2D1::RectF(12, fallbackTop, 348, fallbackTop + 32);
            const auto selectedIndex = presentationSelected();
            const bool selected = selectedIndex && *selectedIndex == index;
            if (selected) {
                const float radius = visualConfig_.cornerRadiusDip;
                const auto size = renderTarget_->GetSize();
                const D2D1_RECT_F selection =
                    D2D1::RectF((std::max)(0.0F, bounds.left - selectionInflateX_),
                                (std::max)(0.0F, bounds.top - selectionInflateY_),
                                (std::min)(size.width, bounds.right + selectionInflateX_),
                                (std::min)(size.height, bounds.bottom + selectionInflateY_));
                renderTarget_->FillRoundedRectangle(D2D1::RoundedRect(selection, radius, radius),
                                                    selectionBrush.Get());
            }
            const auto drawTextInRect = [&](const D2D1_RECT_F& segment,
                                            const std::wstring& value,
                                            IDWriteTextFormat* format, ID2D1Brush* brush) {
                if (value.empty())
                    return;
                // Clip instead of wrapping: a long label/comment that exceeds
                // the remaining row width must not wrap onto the candidate
                // row below and visually overlap it.
                renderTarget_->DrawTextW(value.data(), static_cast<UINT32>(value.size()), format,
                                         segment, brush, kDrawTextOptions);
            };
            const auto& segments = renderSegments[local];
            if (!candidate.label.empty())
                drawTextInRect(fromUiRect(segments.label), candidate.label, labelFormat_.Get(),
                               selected ? selectedLabelBrush.Get() : labelBrush.Get());
            drawTextInRect(fromUiRect(segments.text), candidate.text, textFormat_.Get(),
                           selected ? selectedTextBrush.Get() : textBrush.Get());
            if (segments.drawComment)
                drawTextInRect(fromUiRect(segments.comment), candidate.comment,
                               annotationFormat_.Get(),
                               selected ? selectedCommentBrush.Get() : commentBrush.Get());
            fallbackTop += 32.0F;
        }
        if (hasScrollbar_) {
            renderTarget_->FillRoundedRectangle(D2D1::RoundedRect(scrollbarTrack_, 2.0F, 2.0F),
                                                borderBrush.Get());
            renderTarget_->FillRoundedRectangle(D2D1::RoundedRect(scrollbarThumb_, 2.0F, 2.0F),
                                                selectedTextBrush.Get());
        }
        const auto targetSize = renderTarget_->GetSize();
        const float borderWidth = visualConfig_.borderWidthDip;
        if (borderWidth > 0.0F && targetSize.width > borderWidth &&
            targetSize.height > borderWidth) {
            const float inset = borderWidth / 2.0F;
            const float radius = visualConfig_.cornerRadiusDip;
            renderTarget_->DrawRoundedRectangle(
                D2D1::RoundedRect(
                    D2D1::RectF(inset, inset, targetSize.width - inset, targetSize.height - inset),
                    radius, radius),
                borderBrush.Get(), borderWidth);
        }
        const HRESULT result = renderTarget_->EndDraw();
        if (result == D2DERR_RECREATE_TARGET) {
            renderTarget_.Reset();
            return createDeviceResources();
        }
        return SUCCEEDED(result);
    }

    void paintToDeviceContext(HDC dc) {
        if (!dc)
            return;
        RECT client{};
        if (!GetClientRect(window_, &client))
            return;
        const auto toColorRef = [](const D2D1_COLOR_F& color) {
            const auto channel = [](float value) {
                return static_cast<BYTE>(std::clamp(value, 0.0F, 1.0F) * 255.0F);
            };
            return RGB(channel(color.r), channel(color.g), channel(color.b));
        };
        HIGHCONTRASTW contrast{};
        contrast.cbSize = sizeof(contrast);
        const bool highContrast =
            SystemParametersInfoW(SPI_GETHIGHCONTRAST, sizeof(contrast), &contrast, 0) &&
            (contrast.dwFlags & HCF_HIGHCONTRASTON) != 0;
        const COLORREF background =
            highContrast ? GetSysColor(COLOR_WINDOW)
                         : toColorRef(visualConfig_.colors.background);
        const COLORREF foreground =
            highContrast ? GetSysColor(COLOR_WINDOWTEXT)
                         : toColorRef(visualConfig_.colors.candidateText);
        const COLORREF selectedBackground =
            highContrast ? GetSysColor(COLOR_HIGHLIGHT)
                         : toColorRef(visualConfig_.colors.selectedBackground);
        const COLORREF selectedForeground =
            highContrast ? GetSysColor(COLOR_HIGHLIGHTTEXT)
                         : toColorRef(visualConfig_.colors.selectedCandidateText);
        HBRUSH backgroundBrush = CreateSolidBrush(background);
        HBRUSH selectedBrush = CreateSolidBrush(selectedBackground);
        if (!backgroundBrush || !selectedBrush) {
            if (backgroundBrush)
                DeleteObject(backgroundBrush);
            if (selectedBrush)
                DeleteObject(selectedBrush);
            return;
        }
        FillRect(dc, &client, backgroundBrush);
        SetBkMode(dc, TRANSPARENT);
        HFONT font = CreateFontW(-18, 0, 0, 0, FW_NORMAL, FALSE, FALSE, FALSE,
                                 DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
                                 CLEARTYPE_QUALITY, DEFAULT_PITCH | FF_DONTCARE, L"Segoe UI");
        HGDIOBJ oldFont = font ? SelectObject(dc, font) : nullptr;
        const std::vector<CandidateVisual> fallback{
            {L"1. ", L"1. ", L"你", L"nǐ", true},
            {L"2. ", L"2. ", L"呢", L"", true}};
        const auto& lines = candidates_.empty() ? fallback : candidates_;
        const std::size_t paintCount =
            visibleIndices_.empty() ? lines.size() : visibleIndices_.size();
        float fallbackTop = 8.0F;
        for (std::size_t local = 0; local < paintCount; ++local) {
            const std::size_t index = visibleIndices_.empty() ? local : visibleIndices_[local];
            if (index >= lines.size())
                continue;
            const D2D1_RECT_F bounds = itemRects_.size() == paintCount
                                           ? itemRects_[local]
                                           : D2D1::RectF(12, fallbackTop, 348, fallbackTop + 32);
            RECT item{static_cast<LONG>(bounds.left), static_cast<LONG>(bounds.top),
                      static_cast<LONG>(bounds.right), static_cast<LONG>(bounds.bottom)};
            const auto selectedIndex = presentationSelected();
            const bool selected = selectedIndex && *selectedIndex == index;
            if (selected)
                FillRect(dc, &item, selectedBrush);
            SetTextColor(dc, selected ? selectedForeground : foreground);
            RECT textRect{item.left + 8, item.top, item.right - 8, item.bottom};
            const auto& candidate = lines[index];
            const std::wstring line = candidate.label.empty()
                                          ? candidate.text
                                          : candidate.label + L" " + candidate.text +
                                                (candidate.comment.empty()
                                                     ? std::wstring{}
                                                     : L"  " + candidate.comment);
            DrawTextW(dc, line.c_str(), static_cast<int>(line.size()), &textRect,
                      DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX);
            fallbackTop += 32.0F;
        }
        if (oldFont)
            SelectObject(dc, oldFont);
        if (font)
            DeleteObject(font);
        DeleteObject(selectedBrush);
        DeleteObject(backgroundBrush);
    }

    void paintTestSurfaceOverlay() {
        if (!interactionTest_ || !IsWindowVisible(window_))
            return;
        HDC dc = GetDC(window_);
        if (!dc)
            return;
        paintToDeviceContext(dc);
        ReleaseDC(window_, dc);
    }

    void update(const KeyResponse& response) {
        using namespace fcitx::windows;
        applyContentLocale(response.contentLocaleUtf8);
        candidate::Snapshot snapshot;
        snapshot.engineEpoch = response.metadata.engineEpoch;
        snapshot.contextId = response.metadata.contextId;
        snapshot.compositionId = response.metadata.compositionId;
        snapshot.revision = response.metadata.revision;
        snapshot.preedit = response.preeditUtf8;
        snapshot.selected = response.selectedCandidate == UINT32_MAX
                                ? std::optional<std::size_t>{}
                                : std::optional<std::size_t>{response.selectedCandidate};
        snapshot.page = response.candidatePage;
        snapshot.total = response.candidateTotal;
        snapshot.visibility = response.candidateVisibility == 2 ? candidate::Visibility::prediction
                              : response.candidateVisibility == 1
                                  ? candidate::Visibility::composition
                                  : candidate::Visibility::hidden;
        snapshot.popupAllowed = response.popupAllowed;
        snapshot.candidates.reserve(response.candidates.size());
        for (const auto& source : response.candidates) {
            snapshot.candidates.push_back(
                candidate::Item{source.id, source.labelUtf8, source.textUtf8, source.commentUtf8});
        }
        const auto applied = model_.apply(std::move(snapshot));
        if (applied == candidate::ApplyResult::stale || applied == candidate::ApplyResult::invalid)
            return;
        const auto& current = *model_.current();
        const fcitx::windows::ui::detail::Fcitx5CandidatePresentationUpdate presentationInput{
            current.engineEpoch,
            current.contextId,
            current.compositionId,
            current.revision,
            current.selected.value_or(0U),
            static_cast<std::uint8_t>(current.selected.has_value()),
            current.candidates.size(),
            response.candidatePage,
            response.candidatePageSize,
            static_cast<std::uint8_t>(response.candidateBulk),
            static_cast<std::uint8_t>(visualConfig_.scrollMode),
        };
        const auto presentationApplied =
            fcitx::windows::ui::detail::fcitx5_candidate_presentation_apply(
            presentation_, &presentationInput);
        if (presentationApplied == 2U || presentationApplied == 3U)
            return;
        clickInFlight_ = false;
        KillTimer(window_, kClickGuardTimer);
        if (response.caret.valid)
            lastCaret_ = response.caret;
        const float requestedFontScale = static_cast<float>(lastCaret_.dpi) / 96.0F;
        if (requestedFontScale != fontDpiScale_) {
            fontDpiScale_ = requestedFontScale;
            textFormat_.Reset();
            labelFormat_.Reset();
            annotationFormat_.Reset();
            if (!createDeviceResources())
                return;
        }
        candidates_.clear();
        itemRects_.clear();
        visibleIndices_.clear();
        renderIndices_.clear();
        preeditPanel_.clear();
        preeditPanelRect_ = {};
        preeditDividerY_ = 0.0F;
        if (visualConfig_.preeditMode == NativePreeditMode::panel &&
            !current.preedit.empty()) {
            std::wstring preedit;
            if (utf8ToWide(current.preedit, preedit))
                preeditPanel_ = std::move(preedit);
        }
        for (std::size_t candidateIndex = 0; candidateIndex < current.candidates.size();
             ++candidateIndex) {
            const auto& candidate = current.candidates[candidateIndex];
            std::wstring label;
            std::wstring text;
            std::wstring comment;
            if (!utf8ToWide(candidate.label, label) || !utf8ToWide(candidate.text, text) ||
                !utf8ToWide(candidate.comment, comment))
                continue;
            CandidateVisual visual;
            if (visualConfig_.labelVisible && !label.empty()) {
                visual.label = formatCandidateLabel(
                    0,
                    label,
                    visualConfig_.labelStyle);
                visual.reservedLabel = visual.label;
                visual.sourceLabel = true;
            }
            visual.text = std::move(text);
            if (!comment.empty())
                visual.comment = L"  " + comment;
            candidates_.emplace_back(std::move(visual));
        }
        if (candidates_.size() != current.candidates.size()) {
            dismissPresentation();
            return;
        }
        renderIndices_.resize(candidates_.size());
        fcitx::windows::ui::detail::Fcitx5CandidatePresentationRenderPlan renderPlan{};
        if (fcitx::windows::ui::detail::fcitx5_candidate_presentation_render_plan(
                presentation_, renderIndices_.data(), renderIndices_.size(), &renderPlan) == 0) {
            dismissPresentation();
            return;
        }
        renderIndices_.resize(renderPlan.renderCount);
        if (current.visibility == candidate::Visibility::hidden || candidates_.empty() ||
            !lastCaret_.valid) {
            dismissPresentation();
            return;
        }
        if (!current.popupAllowed) {
            hidePopup();
            return;
        }
        targetForegroundWindow_ = GetForegroundWindow();
        targetForegroundProcessId_ = 0;
        if (targetForegroundWindow_)
            GetWindowThreadProcessId(targetForegroundWindow_, &targetForegroundProcessId_);
        if (interactionTest_)
            targetForegroundProcessId_ =
                fcitx::windows::ui::detail::fcitx5_windows_common_current_process_id();
        POINT caretPoint{lastCaret_.left, lastCaret_.top};
        HMONITOR monitor = MonitorFromPoint(caretPoint, MONITOR_DEFAULTTONEAREST);
        MONITORINFO monitorInfo{};
        monitorInfo.cbSize = sizeof(monitorInfo);
        GetMonitorInfoW(monitor, &monitorInfo);
        const float scale = static_cast<float>(lastCaret_.dpi) / 96.0F;
        const float itemPaddingX = visualConfig_.itemPaddingXDip * scale;
        const float itemPaddingY = visualConfig_.itemPaddingYDip * scale;
        const float labelGap = visualConfig_.labelGapDip * scale;
        selectionInflateX_ = itemPaddingX * 0.65F;
        selectionInflateY_ = itemPaddingY * 0.55F;
        const auto configuredOrientation = visualConfig_.orientation;
        bool horizontalPresentation = configuredOrientation == NativeOrientation::horizontal;
        if (visualConfig_.writingMode != NativeWritingMode::horizontal)
            horizontalPresentation = false;
        if (visualConfig_.overflow == NativeOverflow::wrapping) {
            // flow → the legacy renderer surface collapses to a single row; the
            // Rust three-axis geometry (080 slice 2) owns real wrap rects.
            horizontalPresentation = true;
        }
        std::vector<fcitx::windows::ui::detail::Fcitx5CandidatePresentationText>
            presentationCandidates;
        presentationCandidates.reserve(current.candidates.size());
        for (const auto& candidate : current.candidates) {
            presentationCandidates.push_back({
                {reinterpret_cast<const std::uint8_t*>(candidate.text.data()), candidate.text.size()},
                {reinterpret_cast<const std::uint8_t*>(candidate.comment.data()),
                 candidate.comment.size()},
            });
        }
        // The resolved presentation axis: vertical writing (VerticalRl/Lr) is a
        // column arrangement and always renders vertical; wrapping renders a
        // single horizontal row on the legacy surface; stacked is vertical;
        // everything else (automatic paging, scrolling) stays presentation-
        // decided and resolves inside the Rust presentation state.
        const auto configuredOrientationValue =
            visualConfig_.writingMode != NativeWritingMode::horizontal
                ? 1U
                : configuredOrientation == NativeOrientation::vertical
                      ? 1U
                      : configuredOrientation == NativeOrientation::horizontal ? 2U : 0U;
        const auto rustOrientation =
            fcitx::windows::ui::detail::fcitx5_candidate_presentation_resolve_orientation(
            presentation_, configuredOrientationValue, presentationCandidates.data(),
            presentationCandidates.size(),
            {reinterpret_cast<const std::uint8_t*>(contentLocaleUtf8_.data()),
             contentLocaleUtf8_.size()},
            {static_cast<float>(monitorInfo.rcWork.left), static_cast<float>(monitorInfo.rcWork.top),
             static_cast<float>(monitorInfo.rcWork.right), static_cast<float>(monitorInfo.rcWork.bottom)},
            static_cast<float>(lastCaret_.left), scale, response.candidatePageSize);
        horizontalPresentation = rustOrientation == 1U;
        resolvedPresentationOrientation_ = horizontalPresentation ? ui::Orientation::horizontal
                                                                   : ui::Orientation::vertical;
        applyScrollLabelReservations();
        float scrollLabelColumnWidth = 0.0F;
        if (presentationScrollMode() && horizontalPresentation) {
            for (const auto candidateIndex : renderIndices_) {
                const auto& candidate = candidates_[candidateIndex];
                if (candidate.reservedLabel.empty())
                    continue;
                ComPtr<IDWriteTextLayout> labelLayout;
                DWRITE_TEXT_METRICS metrics{};
                if (writeFactory_ && labelFormat_ &&
                    SUCCEEDED(writeFactory_->CreateTextLayout(
                        candidate.reservedLabel.data(),
                        static_cast<UINT32>(candidate.reservedLabel.size()),
                        labelFormat_.Get(), 4096.0F, 512.0F, &labelLayout)) &&
                    SUCCEEDED(labelLayout->GetMetrics(&metrics))) {
                    scrollLabelColumnWidth = (std::max)(
                        scrollLabelColumnWidth, metrics.widthIncludingTrailingWhitespace);
                }
            }
        }
        std::vector<ui::Size> items;
        items.reserve(renderIndices_.size());
        for (const auto candidateIndex : renderIndices_) {
            const auto& candidate = candidates_[candidateIndex];
            float width = 0.0F;
            float height = 0.0F;
            const auto measure = [&](const std::wstring& value, IDWriteTextFormat* format) {
                if (value.empty())
                    return true;
                ComPtr<IDWriteTextLayout> textLayout;
                DWRITE_TEXT_METRICS metrics{};
                if (!writeFactory_ || !format ||
                    FAILED(writeFactory_->CreateTextLayout(value.data(),
                                                           static_cast<UINT32>(value.size()),
                                                           format, 4096.0F, 512.0F, &textLayout)) ||
                    FAILED(textLayout->GetMetrics(&metrics)))
                    return false;
                width += metrics.widthIncludingTrailingWhitespace;
                height = (std::max)(height, metrics.height);
                return true;
            };
            if (presentationScrollMode() && horizontalPresentation && candidate.reservedLabel.empty())
                width += scrollLabelColumnWidth + labelGap;
            if (measure(candidate.reservedLabel, labelFormat_.Get()) &&
                measure(candidate.text, textFormat_.Get()) &&
                measure(candidate.comment, annotationFormat_.Get())) {
                if (!candidate.reservedLabel.empty())
                    width += labelGap;
                items.push_back({width + itemPaddingX * 2, height + itemPaddingY * 2});
            } else {
                items.push_back({336 * scale, 32 * scale});
            }
        }
        float preeditPanelHeight = 0.0F;
        float preeditPanelWidth = 0.0F;
        if (!preeditPanel_.empty() && writeFactory_ && textFormat_) {
            ComPtr<IDWriteTextLayout> preeditLayout;
            DWRITE_TEXT_METRICS metrics{};
            if (SUCCEEDED(writeFactory_->CreateTextLayout(
                    preeditPanel_.data(), static_cast<UINT32>(preeditPanel_.size()),
                    textFormat_.Get(), 4096.0F, 512.0F, &preeditLayout)) &&
                SUCCEEDED(preeditLayout->GetMetrics(&metrics))) {
                preeditPanelHeight = metrics.height + itemPaddingY * 2.0F;
                preeditPanelWidth = metrics.widthIncludingTrailingWhitespace + itemPaddingX * 2.0F;
            }
        }
        if (configuredOrientation == NativeOrientation::automatic && horizontalPresentation) {
            float horizontalNaturalWidth = inputHorizontalNaturalWidth(
                items, visualConfig_.paddingXDip * scale, visualConfig_.columnGapDip * scale,
                preeditPanelWidth);
            const float workWidth =
                static_cast<float>((std::max)(0L, monitorInfo.rcWork.right - monitorInfo.rcWork.left));
            const float hardLimit =
                std::min(visualConfig_.maxWidthDip * scale, workWidth);
            if (horizontalNaturalWidth > hardLimit + 0.5F) {
                horizontalPresentation = false;
                resolvedPresentationOrientation_ = ui::Orientation::vertical;
            }
        }
        const fcitx::windows::ui::detail::Fcitx5CandidateAxisLayoutInput axisInput{
            response.candidatePageSize,
            visualConfig_.maxWidthDip * scale,
            0.0F,
            scale,
            presentationSelected().value_or(0U),
            static_cast<std::uint8_t>(horizontalPresentation ? 0U : 1U),
            static_cast<std::uint8_t>(
                visualConfig_.writingMode != NativeWritingMode::horizontal
                    ? 0U
                    : visualConfig_.overflow == NativeOverflow::wrapping
                          ? 2U
                          : visualConfig_.overflow == NativeOverflow::scrolling ? 1U : 0U),
            static_cast<std::uint8_t>(
                visualConfig_.writingMode == NativeWritingMode::verticalRl
                    ? 1U
                    : visualConfig_.writingMode == NativeWritingMode::verticalLr ? 2U : 0U),
            {static_cast<float>(lastCaret_.left), static_cast<float>(lastCaret_.top)},
            static_cast<float>((std::max)(1, lastCaret_.bottom - lastCaret_.top)),
            {static_cast<float>(monitorInfo.rcWork.left),
             static_cast<float>(monitorInfo.rcWork.top),
             static_cast<float>(monitorInfo.rcWork.right),
             static_cast<float>(monitorInfo.rcWork.bottom)},
            visualConfig_.paddingXDip * scale,
            visualConfig_.paddingYDip * scale,
            visualConfig_.rowGapDip * scale,
            visualConfig_.columnGapDip * scale,
            toRust(presentationPlacement()),
        };
        std::vector<fcitx::windows::ui::detail::Fcitx5CandidateLayoutSize> axisItemSizes;
        axisItemSizes.reserve(items.size());
        for (const auto& item : items)
            axisItemSizes.push_back({item.width, item.height});
        std::vector<fcitx::windows::ui::detail::Fcitx5CandidateAxisLayoutItemOutput> axisItemRects(
            axisItemSizes.size());
        fcitx::windows::ui::detail::Fcitx5CandidateAxisLayoutOutput axisOutput{};
        if (fcitx::windows::ui::detail::fcitx5_candidate_axis_layout(
                &axisInput, axisItemSizes.data(), axisItemSizes.size(), axisItemRects.data(),
                axisItemRects.size(), &axisOutput) != 0) {
            dismissPresentation();
            return;
        }
        const auto layoutPlacement =
            fcitx::windows::ui::placementFromRust(axisOutput.placement);
        setPresentationPlacement(layoutPlacement);
        const float preeditBlock =
            preeditPanelHeight > 0.0F ? preeditPanelHeight + visualConfig_.rowGapDip * scale : 0.0F;
        const float workWidth =
            (std::max)(0.0F, axisInput.workArea.right - axisInput.workArea.left);
        const float workHeight =
            (std::max)(0.0F, axisInput.workArea.bottom - axisInput.workArea.top);
        const float measuredWindowWidth = std::min(
            {std::max(axisOutput.windowW, preeditPanelWidth), axisInput.maxWidth, workWidth});
        const float windowWidth =
            fcitx::windows::ui::detail::fcitx5_candidate_presentation_stable_window_width(
                presentation_, measuredWindowWidth, std::min(axisInput.maxWidth, workWidth));
        float windowHeight = std::min(axisOutput.windowH + preeditBlock, workHeight);
        float windowLeft = std::clamp(axisOutput.windowX, axisInput.workArea.left,
                                      axisInput.workArea.right - windowWidth);
        float windowTop = axisOutput.windowY;
        if (preeditBlock > 0.0F && layoutPlacement == ui::Placement::above)
            windowTop -= preeditBlock;
        windowTop =
            std::clamp(windowTop, axisInput.workArea.top, axisInput.workArea.bottom - windowHeight);
        const LONG left = static_cast<LONG>(windowLeft);
        const LONG top = static_cast<LONG>(windowTop);
        const LONG width = static_cast<LONG>(windowWidth);
        const LONG height = static_cast<LONG>(windowHeight);
        if (preeditBlock > 0.0F) {
            preeditPanelRect_ =
                D2D1::RectF(itemPaddingX, itemPaddingY, windowWidth - itemPaddingX,
                            (std::max)(itemPaddingY, preeditPanelHeight - itemPaddingY));
            preeditDividerY_ = preeditPanelHeight + axisInput.rowGap / 2.0F;
        }
        for (std::size_t local = 0; local < axisOutput.itemCount; ++local) {
            const auto& item = axisItemRects[local];
            const float candidateOffset =
                layoutPlacement == ui::Placement::below ? preeditBlock : 0.0F;
            itemRects_.push_back(D2D1::RectF(
                item.x - windowLeft + itemPaddingX,
                item.y + candidateOffset - windowTop + itemPaddingY,
                item.x + item.w - windowLeft - itemPaddingX,
                item.y + item.h + candidateOffset - windowTop - itemPaddingY));
            visibleIndices_.push_back(renderIndices_[local]);
        }
        hasScrollbar_ = axisOutput.viewportDx != 0.0F || axisOutput.viewportDy != 0.0F;
        const float candidateOffset =
            layoutPlacement == ui::Placement::below ? preeditBlock : 0.0F;
        scrollbarTrack_ =
            D2D1::RectF(axisOutput.windowW - 6.0F, itemPaddingY + candidateOffset,
                        axisOutput.windowW - 2.0F,
                        axisOutput.windowH - itemPaddingY + candidateOffset);
        scrollbarThumb_ = scrollbarTrack_;
        SetWindowPos(window_, HWND_TOPMOST, left, top, width, height,
                     SWP_NOACTIVATE | SWP_SHOWWINDOW);
        if (renderTarget_)
            renderTarget_->Resize(
                D2D1::SizeU(static_cast<UINT32>(width), static_cast<UINT32>(height)));
        InvalidateRect(window_, nullptr, FALSE);
    }

    void reflowCurrentModel() {
        if (!model_.current()) {
            InvalidateRect(window_, nullptr, FALSE);
            return;
        }
        const auto current = *model_.current();
        KeyResponse response;
        response.metadata.engineEpoch = current.engineEpoch;
        response.metadata.contextId = current.contextId;
        response.metadata.compositionId = current.compositionId;
        response.metadata.revision = current.revision;
        response.preeditUtf8 = current.preedit;
        response.selectedCandidate = current.selected
                                         ? static_cast<std::uint32_t>(*current.selected)
                                         : UINT32_MAX;
        response.candidatePage = current.page;
        const auto presentation = presentationState();
        response.candidatePageSize = presentation.pageSize;
        response.candidateTotal = current.total;
        response.candidateBulk = presentation.candidateBulk != 0;
        response.candidateEnd = true;
        response.candidateVisibility =
            current.visibility == candidate::Visibility::prediction
                ? 2U
            : current.visibility == candidate::Visibility::composition
                ? 1U
                : 0U;
        response.caret = lastCaret_;
        response.popupAllowed = current.popupAllowed;
        response.contentLocaleUtf8 = contentLocaleUtf8_;
        response.candidates.reserve(current.candidates.size());
        for (const auto& item : current.candidates) {
            response.candidates.push_back(
                {item.id, item.label, item.text, item.comment});
        }
        fcitx::windows::ui::detail::fcitx5_candidate_presentation_reset(presentation_);
        model_.reset();
        update(response);
    }

    void hidePopup() noexcept {
        ShowWindow(window_, SW_HIDE);
        if (GetCapture() == window_)
            ReleaseCapture();
        pressedCandidate_.reset();
        clickInFlight_ = false;
        KillTimer(window_, kClickGuardTimer);
    }

    void dismissPresentation() noexcept {
        hidePopup();
        fcitx::windows::ui::detail::fcitx5_candidate_presentation_reset(presentation_);
        model_.reset();
        candidates_.clear();
        itemRects_.clear();
        visibleIndices_.clear();
        renderIndices_.clear();
        preeditPanel_.clear();
        preeditPanelRect_ = {};
        preeditDividerY_ = 0.0F;
        resolvedPresentationOrientation_ = ui::Orientation::vertical;
        targetForegroundWindow_ = nullptr;
        targetForegroundProcessId_ = 0;
    }

    [[nodiscard]] bool foregroundTargetIsValid() const noexcept {
        if (interactionTest_)
            return true;
        if (!targetForegroundProcessId_)
            return false;
        const HWND foreground = GetForegroundWindow();
        DWORD processId = 0;
        if (foreground)
            GetWindowThreadProcessId(foreground, &processId);
        return processId == targetForegroundProcessId_;
    }

    [[nodiscard]] bool dispatchCandidate(std::size_t localIndex) {
        if (clickInFlight_ || localIndex >= visibleIndices_.size() ||
            !foregroundTargetIsValid())
            return false;
        const std::size_t targetIndex = visibleIndices_[localIndex];
        if (targetIndex >= candidates_.size())
            return false;
        const auto& current = model_.current();
        if (!current || targetIndex >= current->candidates.size())
            return false;
        const auto intent = fcitx::windows::ui::makeCandidateSelectionIntent(
            targetForegroundProcessId_, current->engineEpoch, current->contextId,
            current->compositionId, current->revision, current->candidates[targetIndex].id);
        if (!intent.valid())
            return false;
        clickInFlight_ = true;
        SetTimer(window_, kClickGuardTimer, 750, nullptr);
        if (interactionTest_) {
            capturedTestIntent_ = intent;
            return true;
        }
        if (!candidateClient_ ||
            fcitx5_windows_common_candidate_select_client_select(
                candidateClient_, intent.targetProcessId, intent.engineEpoch, intent.contextId,
                intent.compositionId, intent.revision, intent.candidateId) == 0) {
            clickInFlight_ = false;
            KillTimer(window_, kClickGuardTimer);
            return false;
        }
        return true;
    }

  private:
    [[nodiscard]] fcitx::windows::ui::detail::Fcitx5CandidatePresentationOutput
    presentationState() const noexcept {
        fcitx::windows::ui::detail::Fcitx5CandidatePresentationOutput output{};
        (void)fcitx::windows::ui::detail::fcitx5_candidate_presentation_current(
            presentation_, &output);
        return output;
    }

    [[nodiscard]] std::optional<std::size_t> presentationSelected() const noexcept {
        const auto state = presentationState();
        return state.hasSelected ? std::optional<std::size_t>{state.selected} : std::nullopt;
    }

    [[nodiscard]] bool presentationScrollMode() const noexcept {
        return presentationState().scrollMode != 0;
    }

    [[nodiscard]] std::size_t presentationScrollColumns() const noexcept {
        return presentationState().scrollColumns;
    }

    [[nodiscard]] ui::Placement presentationPlacement() const noexcept {
        return fcitx::windows::ui::placementFromRust(presentationState().placement);
    }

    void setPresentationPlacement(ui::Placement placement) noexcept {
        (void)fcitx::windows::ui::detail::fcitx5_candidate_presentation_set_placement(
            presentation_, toRust(placement));
    }

    static float inputHorizontalNaturalWidth(std::span<const ui::Size> items, float paddingX,
                                             float columnGap, float preeditWidth) noexcept {
        float width = 0.0F;
        for (const auto& item : items) {
            if (width > 0.0F)
                width += columnGap;
            width += item.width;
        }
        return (std::max)(width + paddingX * 2.0F, preeditWidth);
    }

    static LRESULT CALLBACK windowProcedure(HWND window, UINT message, WPARAM wparam,
                                            LPARAM lparam) {
        CandidateWindow* self = nullptr;
        if (message == WM_NCCREATE) {
            self = static_cast<CandidateWindow*>(
                reinterpret_cast<CREATESTRUCTW*>(lparam)->lpCreateParams);
            SetWindowLongPtrW(window, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(self));
        } else {
            self = reinterpret_cast<CandidateWindow*>(GetWindowLongPtrW(window, GWLP_USERDATA));
        }
        if (self && (message == WM_PRINT || message == WM_PRINTCLIENT)) {
            self->paintToDeviceContext(reinterpret_cast<HDC>(wparam));
            return 0;
        }
        if (self && message == WM_PAINT) {
            PAINTSTRUCT paint{};
            BeginPaint(window, &paint);
            self->paintOnce();
            EndPaint(window, &paint);
            return 0;
        }
        if (self && message == kSnapshotMessage) {
            std::unique_ptr<KeyResponse> response(
                reinterpret_cast<KeyResponse*>(lparam));
            self->update(*response);
            return 0;
        }
        if (self && message == visualConfigChangedMessage()) {
            self->reloadVisualConfig();
            return 0;
        }
        if (self && message == candidateDismissMessage()) {
            const auto sourceContext = static_cast<std::uint64_t>(lparam);
            const auto& current = self->model_.current();
            const bool sameContext = sourceContext == 0 ||
                                     (current && sourceContext == current->contextId);
            if ((wparam == 0 ||
                 static_cast<DWORD>(wparam) == self->targetForegroundProcessId_) &&
                sameContext)
                self->dismissPresentation();
            return 0;
        }
        if (self && message == WM_TIMER) {
            if (wparam == kFocusWatchTimer && IsWindowVisible(window) &&
                !self->foregroundTargetIsValid()) {
                self->dismissPresentation();
            } else if (wparam == kClickGuardTimer) {
                self->clickInFlight_ = false;
                KillTimer(window, kClickGuardTimer);
            }
            return 0;
        }
        if (self && message == WM_DPICHANGED) {
            const auto* suggested = reinterpret_cast<const RECT*>(lparam);
            SetWindowPos(window, nullptr, suggested->left, suggested->top,
                         suggested->right - suggested->left, suggested->bottom - suggested->top,
                         SWP_NOACTIVATE | SWP_NOZORDER);
            if (self->renderTarget_) {
                self->renderTarget_->SetDpi(96.0F, 96.0F);
            }
            return 0;
        }
        if (self && (message == WM_SETTINGCHANGE || message == WM_THEMECHANGED ||
                     message == WM_SYSCOLORCHANGE)) {
            self->reloadVisualConfig();
            return 0;
        }
        if (self && message == WM_MOUSEACTIVATE)
            return MA_NOACTIVATE;
        if (self && message == WM_LBUTTONDOWN) {
            const float x = static_cast<float>(static_cast<short>(LOWORD(lparam)));
            const float y = static_cast<float>(static_cast<short>(HIWORD(lparam)));
            self->pressedCandidate_ =
                fcitx::windows::ui::hitTestCandidate(self->itemRects_, x, y);
            if (self->pressedCandidate_)
                SetCapture(window);
            return 0;
        }
        if (self && message == WM_LBUTTONUP) {
            const float x = static_cast<float>(static_cast<short>(LOWORD(lparam)));
            const float y = static_cast<float>(static_cast<short>(HIWORD(lparam)));
            const auto released =
                fcitx::windows::ui::hitTestCandidate(self->itemRects_, x, y);
            const auto pressed = self->pressedCandidate_;
            self->pressedCandidate_.reset();
            if (GetCapture() == window)
                ReleaseCapture();
            if (pressed && released == pressed)
                (void)self->dispatchCandidate(*pressed);
            return 0;
        }
        if (self && (message == WM_CANCELMODE || message == WM_CAPTURECHANGED)) {
            self->pressedCandidate_.reset();
            return 0;
        }
        if (message == WM_NCHITTEST)
            return HTCLIENT;
        if (message == WM_DESTROY) {
            PostQuitMessage(0);
            return 0;
        }
        return DefWindowProcW(window, message, wparam, lparam);
    }

    bool createDeviceResources() {
        if (!d2dFactory_ && FAILED(D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED,
                                                     d2dFactory_.GetAddressOf())))
            return false;
        if (!writeFactory_ &&
            FAILED(DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, __uuidof(IDWriteFactory),
                                       reinterpret_cast<IUnknown**>(writeFactory_.GetAddressOf()))))
            return false;
        const auto createFormat = [&](const std::vector<std::wstring>& families, float scale,
                                      ComPtr<IDWriteTextFormat>& format) {
            if (format)
                return true;
            std::wstring family = L"Microsoft YaHei";
            if (!families.empty())
                family = families.front();
            if (FAILED(writeFactory_->CreateTextFormat(
                    family.c_str(), nullptr,
                    static_cast<DWRITE_FONT_WEIGHT>(visualConfig_.candidateFontWeight),
                    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL,
                    visualConfig_.candidateFontSizeDip * scale * fontDpiScale_,
                    dwriteLocale_.c_str(), &format)))
                return false;
            // Single line with ellipsis trimming: a label/comment longer than
            // the remaining row width must not wrap onto the candidate row
            // below (which visually overlaps the next candidate).
            if (FAILED(format->SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)))
                return false;
            DWRITE_TRIMMING trimming{DWRITE_TRIMMING_GRANULARITY_CHARACTER, 0, 0};
            ComPtr<IDWriteInlineObject> ellipsis;
            if (FAILED(writeFactory_->CreateEllipsisTrimmingSign(format.Get(), &ellipsis)) ||
                FAILED(format->SetTrimming(&trimming, ellipsis.Get())))
                return false;
            return true;
        };
        const auto& annotationFamilies = visualConfig_.annotationFontFamilies.empty()
                                             ? visualConfig_.candidateFontFamilies
                                             : visualConfig_.annotationFontFamilies;
        if (!createFormat(visualConfig_.candidateFontFamilies, 1.0F, textFormat_) ||
            !createFormat(visualConfig_.candidateFontFamilies, visualConfig_.labelFontScale,
                          labelFormat_) ||
            !createFormat(annotationFamilies, visualConfig_.annotationFontScale, annotationFormat_))
            return false;
        if (FAILED(labelFormat_->SetTextAlignment(DWRITE_TEXT_ALIGNMENT_TRAILING)))
            return false;
        DWRITE_TRIMMING labelTrimming{DWRITE_TRIMMING_GRANULARITY_NONE, 0, 0};
        if (FAILED(labelFormat_->SetTrimming(&labelTrimming, nullptr)))
            return false;
        if (!renderTarget_) {
            RECT client{};
            GetClientRect(window_, &client);
            const auto size =
                D2D1::SizeU(static_cast<UINT32>(client.right), static_cast<UINT32>(client.bottom));
            if (FAILED(d2dFactory_->CreateHwndRenderTarget(
                    D2D1::RenderTargetProperties(), D2D1::HwndRenderTargetProperties(window_, size),
                    &renderTarget_)))
                return false;
            renderTarget_->SetDpi(96.0F, 96.0F);
        }
        return true;
    }

    HWND window_{};
    ComPtr<ID2D1Factory> d2dFactory_;
    ComPtr<IDWriteFactory> writeFactory_;
    ComPtr<IDWriteTextFormat> textFormat_;
    ComPtr<IDWriteTextFormat> labelFormat_;
    ComPtr<IDWriteTextFormat> annotationFormat_;
    ComPtr<ID2D1HwndRenderTarget> renderTarget_;
    std::vector<CandidateVisual> candidates_;
    std::wstring preeditPanel_;
    std::vector<D2D1_RECT_F> itemRects_;
    std::vector<std::size_t> visibleIndices_;
    std::vector<std::size_t> renderIndices_;
    std::optional<std::size_t> pressedCandidate_;
    NativeRenderConfig visualConfig_;
    candidate::CandidateModel model_;
    CaretRect lastCaret_;
    fcitx::windows::ui::Orientation resolvedPresentationOrientation_{
        fcitx::windows::ui::Orientation::vertical};
    bool safeMode_{};
    bool hasScrollbar_{};
    float fontDpiScale_{1.0F};
    float selectionInflateX_{};
    float selectionInflateY_{};
    float preeditDividerY_{};
    D2D1_RECT_F preeditPanelRect_{};
    D2D1_RECT_F scrollbarTrack_{};
    D2D1_RECT_F scrollbarThumb_{};
    HWND targetForegroundWindow_{};
    DWORD targetForegroundProcessId_{};
    bool clickInFlight_{};
    bool interactionTest_{};
    std::optional<fcitx::windows::ui::CandidateSelectionIntent> capturedTestIntent_;
    void* candidateClient_{};
    void* presentation_{};
    std::wstring dwriteLocale_{defaultDwriteLocale()};
    std::string contentLocaleUtf8_;
};

bool readExact(HANDLE pipe, void* destination, std::size_t size) {
    auto* bytes = static_cast<std::uint8_t*>(destination);
    std::size_t offset = 0;
    while (offset < size) {
        DWORD read = 0;
        if (!ReadFile(pipe, bytes + offset, static_cast<DWORD>(size - offset), &read, nullptr) ||
            read == 0)
            return false;
        offset += read;
    }
    return true;
}

std::string_view ffiBytes(FcitxBytesC value) noexcept {
    if (value.len == 0 || value.data == nullptr) return {};
    return {reinterpret_cast<const char*>(value.data), value.len};
}

bool decodePresentationFrame(const std::vector<std::uint8_t>& frame, KeyResponse& out) noexcept {
    constexpr std::size_t kHeaderSize = 64;
    constexpr std::uint16_t kKeyResponseType = 4;
    constexpr std::size_t kMaxFrameSize = 256U * 1024U;
    if (frame.size() < kHeaderSize || frame.size() > kMaxFrameSize) return false;
    std::uint16_t type = 0;
    std::uint32_t bodySize = 0;
    FcitxMetadataC header{};
    if (fcitx5_protocol_core_decode_header(frame.data(), kHeaderSize, &type, &bodySize,
                                           &header) == 0 ||
        type != kKeyResponseType || bodySize != frame.size() - kHeaderSize) {
        return false;
    }
    FcitxKeyResponseC decoded{};
    std::vector<std::uint8_t> strings;
    std::vector<FcitxCandidateRecordC> candidates;
    std::size_t stringsNeeded = 0;
    std::size_t candidatesNeeded = 0;
    const std::uint8_t* body = frame.data() + kHeaderSize;
    auto fill = [&]() noexcept {
        out.metadata.engineEpoch = header.engineEpoch;
        out.metadata.contextId = header.contextId;
        out.metadata.compositionId = header.compositionId;
        out.metadata.revision = header.revision;
        out.status = decoded.status;
        out.handled = decoded.handled != 0;
        out.preeditUtf8.assign(ffiBytes(decoded.preedit));
        out.preeditCaretUtf8 = decoded.preeditCaretUtf8;
        out.contentLocaleUtf8.assign(ffiBytes(decoded.contentLocale));
        out.selectedCandidate = decoded.selectedCandidate;
        out.candidatePage = decoded.candidatePage;
        out.candidatePageSize = decoded.candidatePageSize;
        out.candidateTotal = decoded.candidateTotal;
        out.candidateVisibility = decoded.candidateVisibility;
        out.candidateBulk = decoded.candidateBulk != 0;
        out.candidateEnd = decoded.candidateEnd != 0;
        out.caret = CaretRect{decoded.caret.valid != 0, decoded.caret.left, decoded.caret.top,
                              decoded.caret.right, decoded.caret.bottom, decoded.caret.dpi};
        out.popupAllowed = decoded.popupAllowed != 0;
        out.candidates.clear();
        out.candidates.reserve(decoded.candidateCount);
        for (std::size_t index = 0; index < decoded.candidateCount; ++index) {
            const auto& source = decoded.candidates[index];
            out.candidates.push_back(CandidateRecord{
                source.id, std::string(ffiBytes(source.label)),
                std::string(ffiBytes(source.text)), std::string(ffiBytes(source.comment))});
        }
    };
    if (fcitx5_protocol_core_decode_key_response(
            &header, body, bodySize, &decoded, nullptr, 0, &stringsNeeded, nullptr, 0,
            &candidatesNeeded) != 0) {
        fill();
        return true;
    }
    if ((stringsNeeded == 0 && candidatesNeeded == 0) || stringsNeeded > kMaxFrameSize ||
        candidatesNeeded > kMaxFrameSize) {
        return false;
    }
    try {
        strings.assign(stringsNeeded, 0);
        candidates.assign(candidatesNeeded, FcitxCandidateRecordC{});
    } catch (...) {
        return false;
    }
    if (fcitx5_protocol_core_decode_key_response(
            &header, body, bodySize, &decoded,
            strings.empty() ? nullptr : strings.data(), strings.size(), &stringsNeeded,
            candidates.empty() ? nullptr : candidates.data(), candidates.size(),
            &candidatesNeeded) == 0) {
        return false;
    }
    fill();
    return true;
}

void servePresentation(HWND window, bool testOnce) {
    using namespace fcitx::windows;
    constexpr std::size_t kHeaderSize = 64;
    constexpr DWORD kMaxHotFrameSize = 256U * 1024U;
    platform::RuntimeIdentity identity;
    platform::PipeSecurity security;
    if (!platform::queryCurrentIdentity(identity) ||
        !platform::PipeSecurity::create(identity, security))
        return;
    if (identity.executablePath.empty())
        return;
    const auto engine =
        (std::filesystem::path(identity.executablePath).parent_path() / "fcitx5-engine.exe")
            .wstring();
    const auto pipeName = platform::makeLocalEndpointName(identity, L"presentation");
    for (;;) {
        HANDLE pipe = CreateNamedPipeW(
            pipeName.c_str(), PIPE_ACCESS_INBOUND,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS, 1,
            kMaxHotFrameSize, kMaxHotFrameSize, 25, security.attributes());
        if (pipe == INVALID_HANDLE_VALUE)
            return;
        const bool connected =
            ConnectNamedPipe(pipe, nullptr) != FALSE || GetLastError() == ERROR_PIPE_CONNECTED;
        platform::ProcessIdentity peer;
        if (connected && ipc::verifyPipeClient(pipe, identity, &peer) &&
            platform::pathsReferToSameFile(peer.executablePath, engine)) {
            for (;;) {
                std::array<std::uint8_t, kHeaderSize> header{};
                if (!readExact(pipe, header.data(), header.size()))
                    break;
                std::uint16_t type = 0;
                std::uint32_t bodySize = 0;
                FcitxMetadataC metadata{};
                if (fcitx5_protocol_core_decode_header(header.data(), header.size(), &type,
                                                       &bodySize, &metadata) == 0 ||
                    type != kKeyResponseMessageType ||
                    bodySize > static_cast<std::uint32_t>(kMaxHotFrameSize - kHeaderSize))
                    break;
                std::vector<std::uint8_t> frame(header.begin(), header.end());
                frame.resize(kHeaderSize + bodySize);
                if (bodySize && !readExact(pipe, frame.data() + kHeaderSize, bodySize))
                    break;
                auto response = std::make_unique<KeyResponse>();
                if (!decodePresentationFrame(frame, *response))
                    break;
                if (!PostMessageW(window, kSnapshotMessage, 0,
                                  reinterpret_cast<LPARAM>(response.get())))
                    return;
                (void)response.release();
                if (testOnce) {
                    PostMessageW(window, WM_CLOSE, 0, 0);
                    return;
                }
            }
        }
        DisconnectNamedPipe(pipe);
        CloseHandle(pipe);
    }
}

} // namespace

int WINAPI wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE, _In_ PWSTR commandLine, _In_ int) {
    enableDpiAwareness();
    const std::wstring_view arguments = commandLine ? commandLine : L"";
    const auto parsed = fcitx::windows::ui::parseCommandLine(arguments);
    if (!parsed.valid)
        return 1;
    if (!parsed.generation.empty() &&
        (!SetEnvironmentVariableW(L"FCITX5_RELEASE_GENERATION", parsed.generation.c_str()) ||
         fcitx::windows::platform::currentRuntimeGeneration() != parsed.generation)) {
        return 1;
    }
    if (parsed.flags.candidateSelectMode != 0) {
        if (parsed.flags.candidateSelectMode != 1)
            return parsed.flags.candidateSelectMode;
        return runCandidateSelectionTest(parsed);
    }
    const bool selfTest = parsed.flags.selfTest != 0;
    const bool interactionSelfTest = parsed.flags.interactionSelfTest != 0;
    const bool uilessPresentationSelfTest = parsed.flags.uilessPresentationSelfTest != 0;
    const bool scrollExpansionSelfTest = parsed.flags.scrollExpansionSelfTest != 0;
    const bool localeSelfTest = parsed.flags.localeSelfTest != 0;
    const bool candidateUxSelfTest = parsed.flags.candidateUxSelfTest != 0;
    const bool reloadTest = parsed.flags.reloadTest != 0;
    const bool simulateDeviceLoss = parsed.flags.simulateDeviceLoss != 0;
    const bool scrollDemo = parsed.flags.scrollDemo != 0;
    const bool demo = parsed.flags.demo != 0;
    const bool testOnce = parsed.flags.testOnce != 0;
    const bool safeMode = parsed.flags.safeMode != 0;
    CandidateWindow window;
    if (!window.create(instance, demo, safeMode,
                       demo || interactionSelfTest || candidateUxSelfTest) ||
        !window.paintOnce())
        return 1;
    if (demo)
        window.showSyntheticPreview(scrollDemo);
    if (interactionSelfTest)
        return window.runInteractionSelfTest() ? 0 : 2;
    if (uilessPresentationSelfTest)
        return window.runUilessPresentationSelfTest() ? 0 : 2;
    if (scrollExpansionSelfTest)
        return window.runScrollExpansionSelfTest() ? 0 : 2;
    if (localeSelfTest)
        return window.runLocaleSelfTest() ? 0 : 2;
    if (candidateUxSelfTest)
        return window.runCandidateUxSelfTest() ? 0 : 2;
    if (simulateDeviceLoss) {
        window.simulateDeviceLossForTest();
        if (!window.paintOnce())
            return 1;
    }
    if (reloadTest) {
        window.showSyntheticPreview(false);
        SendMessageW(window.handle(), visualConfigChangedMessage(), 0, 0);
        if (!window.paintOnce())
            return 1;
    }
    if (selfTest)
        return 0;
    if (parsed.flags.hasParentId != 0) {
        const HANDLE parent = OpenProcess(SYNCHRONIZE, FALSE, parsed.flags.parentId);
        if (parent) {
            const HWND handle = window.handle();
            std::thread([parent, handle] {
                WaitForSingleObject(parent, INFINITE);
                CloseHandle(parent);
                PostMessageW(handle, WM_CLOSE, 0, 0);
            }).detach();
        }
    }
    std::thread(servePresentation, window.handle(), testOnce).detach();
    return window.run();
}
