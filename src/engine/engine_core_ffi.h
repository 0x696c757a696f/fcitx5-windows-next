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

} // extern "C"
