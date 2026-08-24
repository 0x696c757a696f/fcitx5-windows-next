// C ABI surface of the Rust `fcitx5-engine-core` crate (E2 side-by-side).
//
// This header declares the flat structures and functions exported by
// `rust/engine-core/src/capi.rs`. It is the first narrow Engine Rust ABI: it
// passes only plain data (context key + metadata scalars) and an opaque ledger
// handle, and never exposes Fcitx object pointers.
//
// ABI conventions (see capi.rs for the authoritative documentation):
//
// * All functions return an `FcitxEngineCoreErrorC` code:
//   0 (OK) success; 1 (STALE) request metadata does not match current
//   per-context state (or the call failed closed); 2 (INVALID_CANDIDATE) the
//   candidate id is 0 or does not encode the current composition id.
// * `end_result` writes the resulting composition id and revision through the
//   output pointers; a null output pointer fails closed without writing.

#pragma once

#include <cstddef>
#include <cstdint>

extern "C" {

struct FcitxEngineContextKeyC {
    std::uint32_t processId;
    std::uint64_t connectionId;
    std::uint64_t contextId;
};

// Matches `protocol::CaretRect` (and the Rust `CaretRect`).
struct FcitxEngineCaretC {
    std::uint8_t valid;
    std::int32_t left;
    std::int32_t top;
    std::int32_t right;
    std::int32_t bottom;
    std::uint32_t dpi;
};

enum FcitxEngineCoreErrorC {
    FCITX_ENGINE_CORE_OK = 0,
    FCITX_ENGINE_CORE_STALE = 1,
    FCITX_ENGINE_CORE_INVALID_CANDIDATE = 2,
};

// Creates an empty context/composition/revision ledger. Returns an opaque
// handle that must be released with `fcitx5_engine_core_ledger_free`.
void* fcitx5_engine_core_ledger_new(void);

// Releases a ledger created by `fcitx5_engine_core_ledger_new`. Null is a
// no-op.
void fcitx5_engine_core_ledger_free(void* ledger);

// Drops all ledger state for a context key.
void fcitx5_engine_core_ledger_forget(void* ledger, const FcitxEngineContextKeyC* key);

// Validates a key request against current ledger state (mirrors the C++
// `processKey` stale check). Returns OK or STALE.
int fcitx5_engine_core_ledger_begin_key(void* ledger, const FcitxEngineContextKeyC* key,
                                        std::uint64_t revision,
                                        std::uint64_t compositionId);

// Validates a candidate-selection request (mirrors the C++ `selectCandidate`
// stale check). Returns OK, STALE, or INVALID_CANDIDATE.
int fcitx5_engine_core_ledger_select_candidate(void* ledger,
                                               const FcitxEngineContextKeyC* key,
                                               std::uint64_t revision,
                                               std::uint64_t compositionId,
                                               std::uint64_t candidateId);

// Applies the end-of-result composition lifecycle and revision bump. Writes
// the new (compositionId, revision) through the output pointers. Returns OK
// or STALE (failed closed).
int fcitx5_engine_core_ledger_end_result(void* ledger, const FcitxEngineContextKeyC* key,
                                         int hasContent,
                                         std::uint64_t* outCompositionId,
                                         std::uint64_t* outRevision);

// Per-context product state (E2 extension). Setter functions return
// FCITX_ENGINE_CORE_OK/STALE; query functions return 1 when the value is
// present (and write the output) and 0 when absent or on null input.

int fcitx5_engine_core_set_caret(void* ledger, const FcitxEngineContextKeyC* key,
                                 const FcitxEngineCaretC* caret);
int fcitx5_engine_core_caret(void* ledger, const FcitxEngineContextKeyC* key,
                             FcitxEngineCaretC* outCaret);

int fcitx5_engine_core_set_popup_allowed(void* ledger, const FcitxEngineContextKeyC* key,
                                         int allowed);
int fcitx5_engine_core_popup_allowed(void* ledger, const FcitxEngineContextKeyC* key,
                                     int* outAllowed);

int fcitx5_engine_core_set_selected_override(void* ledger, const FcitxEngineContextKeyC* key,
                                             std::uint32_t value);
int fcitx5_engine_core_clear_selected_override(void* ledger, const FcitxEngineContextKeyC* key);
int fcitx5_engine_core_selected_override(void* ledger, const FcitxEngineContextKeyC* key,
                                         std::uint32_t* outValue);

int fcitx5_engine_core_set_input_method_overridden(void* ledger,
                                                   const FcitxEngineContextKeyC* key,
                                                   int overridden);
int fcitx5_engine_core_input_method_overridden(void* ledger,
                                               const FcitxEngineContextKeyC* key,
                                               int* outOverridden);

// E3 Event → Action: input-method switch hotkey decision. Returns 1 and
// writes `outAction` (FCITX_ENGINE_CORE_IM_ACTION_TOGGLE=1 / NEXT=2) when the
// non-release key event matches a configured hotkey; 0 otherwise.
// `hotkeyToggle`/`hotkeyNext` are NUL-terminated UTF-8 hotkey strings
// ("Ctrl+Space" etc.) or null (not configured).
enum FcitxEngineCoreImActionC {
    FCITX_ENGINE_CORE_IM_ACTION_NONE = 0,
    FCITX_ENGINE_CORE_IM_ACTION_TOGGLE = 1,
    FCITX_ENGINE_CORE_IM_ACTION_NEXT = 2,
};

int fcitx5_engine_core_classify_input_method_switch(int ctrl, int shift, int alt,
                                                    std::uint32_t keySym,
                                                    const char* hotkeyToggle,
                                                    const char* hotkeyNext,
                                                    int* outAction);

// E3-2 Event → Action: candidate navigation decision. The C++ adapter
// flattens the Fcitx candidate list + config into `FcitxCandidateViewC` /
// `FcitxCandidateConfigC`, asks Rust what to do for a non-release key, and
// executes the returned `FcitxCandidateDecisionC` against Fcitx.
struct FcitxCandidateViewC {
    std::int32_t count;
    std::int32_t listSize;
    std::int32_t cursor;
    std::int32_t bulkCursor;
    std::uint8_t hasBulkCursor;
    std::uint8_t hasBulk;
    std::uint8_t pageable;
    std::uint8_t hasPrev;
    std::uint8_t hasNext;
};

struct FcitxCandidateConfigC {
    std::uint8_t scrollMode;
    std::uint8_t vertical;
    std::int32_t candidatePageSize; // -1 = not configured
};

enum FcitxEngineCoreCandidateActionC {
    FCITX_ENGINE_CORE_CANDIDATE_ACTION_NONE = 0,
    FCITX_ENGINE_CORE_CANDIDATE_ACTION_CONSUME_ONLY = 1,
    FCITX_ENGINE_CORE_CANDIDATE_ACTION_SELECT_AND_CLEAR = 2,
    FCITX_ENGINE_CORE_CANDIDATE_ACTION_SET_OVERRIDE = 3,
    FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_NEXT_AND_SET_OVERRIDE = 4,
    FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_PREV_AND_SET_OVERRIDE = 5,
};

struct FcitxCandidateDecisionC {
    std::uint8_t consume;
    std::int32_t action;
    std::uint32_t value;
};

int fcitx5_engine_core_decide_candidate_action(std::uint32_t keySym, int plainShortcut,
                                               const FcitxCandidateViewC* view,
                                               const FcitxCandidateConfigC* config,
                                               int hasOverride, std::uint32_t overrideValue,
                                               FcitxCandidateDecisionC* outDecision);

// E3-3 Event → Action: surrounding-text and input-method-selection decisions.
enum FcitxEngineCoreSurroundingTextActionC {
    FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_SET = 0,
    FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_INVALIDATE = 1,
};

struct FcitxSurroundingTextDecisionC {
    std::int32_t action;
    std::uint8_t update;
};

int fcitx5_engine_core_decide_surrounding_text(int requestValid, int currentValid,
                                               FcitxSurroundingTextDecisionC* outDecision);

enum FcitxEngineCoreImSelectionC {
    FCITX_ENGINE_CORE_IM_SELECTION_NONE = 0,
    FCITX_ENGINE_CORE_IM_SELECTION_REQUEST = 1,
    FCITX_ENGINE_CORE_IM_SELECTION_DEFAULT = 2,
};

int fcitx5_engine_core_decide_input_method_selection(int hasRequestIm, int requestImValid,
                                                     int defaultImValid, int defaultImNonempty,
                                                     int currentEqRequest, int currentEqDefault,
                                                     int overridden, int* outSelection);

// E3 event-shape consolidation: unified Event→Action entry for a key request.
// The C++ adapter flattens all key facts into `FcitxEngineKeyEventC`, Rust
// composes the four product decisions in `processKey` order, and the adapter
// executes the returned `FcitxEngineKeyDecisionC`.
struct FcitxEngineKeyEventC {
    std::uint32_t keySym;
    std::uint32_t keyFlags; // protocol kKeyFlag* bits (Shift/Control/Alt/...)
    std::uint8_t isRelease;
    const char* hotkeyToggle; // NUL-terminated or null
    const char* hotkeyNext;   // NUL-terminated or null
    std::uint8_t surroundingTextValid;
    std::uint8_t currentSurroundingValid;
    std::uint8_t hasRequestIm;
    std::uint8_t requestImValid;
    std::uint8_t defaultImValid;
    std::uint8_t defaultImNonempty;
    std::uint8_t currentEqRequest;
    std::uint8_t currentEqDefault;
    std::uint8_t imOverridden;
    std::uint8_t hasCandidates;
    FcitxCandidateViewC view;
    FcitxCandidateConfigC config;
    std::uint8_t hasOverride;
    std::uint32_t overrideValue;
};

struct FcitxEngineKeyDecisionC {
    std::int32_t surroundingAction;
    std::uint8_t surroundingUpdate;
    std::int32_t imSelection;
    std::int32_t imSwitch;
    std::uint8_t candidateConsume;
    std::int32_t candidateAction;
    std::uint32_t candidateValue;
    std::uint8_t clearOverride;
    std::uint8_t forwardKey;
};

int fcitx5_engine_core_handle_key_event(const FcitxEngineKeyEventC* event,
                                        FcitxEngineKeyDecisionC* outDecision);

// E4: engine-process session epoch. `generate_engine_epoch` returns a
// 100ns-since-1601 value (mirrors GetSystemTimeAsFileTime); the engine calls
// it once at startup and `validate_engine_epoch` rejects mismatched frames.
std::uint64_t fcitx5_engine_core_generate_engine_epoch(void);
int fcitx5_engine_core_validate_engine_epoch(std::uint64_t frameEpoch,
                                             std::uint64_t processEpoch);

// E4-2: request-sequence and deadline policy. `accept_frame_sequence`
// rejects stale/duplicate request ids on a connection;
// `key_request_timeout_ms` returns the dispatcher deadline (2500 ms for a
// cold context with revision 0, 75 ms for warm keys).
int fcitx5_engine_core_accept_frame_sequence(std::uint64_t requestId,
                                             std::uint64_t lastRequestId);
std::uint32_t fcitx5_engine_core_key_request_timeout_ms(std::uint64_t revision);

// E5-1: snapshot/status canonicalization. `content_locale_for_input_method`
// maps an input-method id to the canonical content-locale code;
// `status_short_label` writes the canonical short label (two ASCII bytes or
// the first code point) and returns the byte count.
enum FcitxEngineCoreContentLocaleC {
    FCITX_ENGINE_CORE_CONTENT_LOCALE_NONE = 0,
    FCITX_ENGINE_CORE_CONTENT_LOCALE_ZH_CN = 1,
    FCITX_ENGINE_CORE_CONTENT_LOCALE_JA_JP = 2,
    FCITX_ENGINE_CORE_CONTENT_LOCALE_KO_KR = 3,
    FCITX_ENGINE_CORE_CONTENT_LOCALE_EN_US = 4,
};

int fcitx5_engine_core_content_locale_for_input_method(const char* inputMethodId);
std::size_t fcitx5_engine_core_status_short_label(const std::uint8_t* text,
                                                  std::size_t textLength,
                                                  std::uint8_t* out,
                                                  std::size_t outCapacity);

} // extern "C"
