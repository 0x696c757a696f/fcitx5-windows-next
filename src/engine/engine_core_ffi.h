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

void* fcitx5_engine_core_presentation_publisher_create(const std::uint16_t* pipeName,
                                                       std::size_t pipeNameLength,
                                                       const std::uint16_t* uiExecutable,
                                                       std::size_t uiExecutableLength);
int fcitx5_engine_core_presentation_publisher_publish(void* publisher,
                                                      const std::uint8_t* frame,
                                                      std::size_t frameLength);
void fcitx5_engine_core_presentation_publisher_destroy(void* publisher);

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
// `key_request_timeout_ms` returns the dispatcher deadline (7500 ms for a
// cold context with revision 0, 250 ms for warm keys).
int fcitx5_engine_core_accept_frame_sequence(std::uint64_t requestId,
                                             std::uint64_t lastRequestId);
std::uint32_t fcitx5_engine_core_key_request_timeout_ms(std::uint64_t revision);

// E4-3: per-connection session state. The engine server creates one opaque
// session per connection (`session_create`, freed with `session_destroy`);
// `begin_hello` performs the hello handshake (rejects repeat handshake,
// session/process mismatch and stale request ids, then marks the session
// complete), `accept_frame` validates every non-hello frame (handshake
// complete, epoch match, session id match, strictly-newer request id), and
// `complete_request` records a successfully processed request id so its id
// cannot be retried. All functions return 1 on acceptance and 0 on rejection
// or a null session; the C++ `handshakeComplete`/`lastRequestId` locals are
// deleted.
void* fcitx5_engine_core_session_create(void);
void fcitx5_engine_core_session_destroy(void* session);
int fcitx5_engine_core_session_begin_hello(void* session, std::uint64_t requestId,
                                           std::uint64_t frameSessionId,
                                           std::uint64_t clientSessionId,
                                           std::uint32_t requestProcessId,
                                           std::uint32_t clientProcessId);
int fcitx5_engine_core_session_accept_frame(void* session, std::uint64_t requestId,
                                            std::uint64_t frameSessionId,
                                            std::uint64_t clientSessionId,
                                            std::uint64_t frameEpoch,
                                            std::uint64_t engineEpoch);
int fcitx5_engine_core_session_complete_request(void* session,
                                                std::uint64_t requestId);

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

// E5-2: typed EngineSnapshot DTO limits validation. `validate_snapshot`
// checks the payload budgets (commit/preedit 16 KiB, candidates 128, each
// candidate field 4 KiB, locale 35 bytes); the engine fails closed on
// rejection at snapshot construction.
struct FcitxEngineSnapshotC {
    std::uint8_t handled;
    std::size_t commitUtf8Len;
    std::size_t preeditUtf8Len;
    std::uint32_t preeditCaretUtf8;
    std::uint64_t compositionId;
    std::uint64_t revision;
    std::uint32_t candidateCount;
    std::size_t candidateLabelLenMax;
    std::size_t candidateTextLenMax;
    std::size_t candidateCommentLenMax;
    std::size_t contentLocaleUtf8Len;
    std::uint32_t selectedCandidate;
    std::uint32_t candidatePage;
    std::uint32_t candidateTotal;
    std::uint8_t candidateVisibility;
    std::uint32_t candidatePageSize;
    std::uint8_t candidateBulk;
    std::uint8_t candidateEnd;
    std::uint8_t deleteSurroundingText;
    std::int32_t deleteSurroundingOffset;
    std::uint32_t deleteSurroundingSize;
    std::uint8_t forwardKey;
    std::uint32_t forwardKeySym;
    std::uint32_t forwardKeyStates;
    std::int32_t forwardKeyCode;
    std::uint8_t forwardKeyRelease;
    std::uint8_t caretValid;
    std::uint8_t popupAllowed;
};

int fcitx5_engine_core_validate_snapshot(const FcitxEngineSnapshotC* snapshot);

// E5-3: pending snapshot store. `put` stores a canonical snapshot blob with
// its revision; `take` removes it only when the request revision is strictly
// older than the stored revision and writes the canonical blob back (0 when
// absent/stale/buffer-too-small, with the required size in `*outLength`);
// `required_size` returns the stored blob size (0 when absent) so the caller
// can size the take buffer without consuming the entry.
int fcitx5_engine_core_snapshot_store_put(void* ledger, const FcitxEngineContextKeyC* key,
                                          std::uint64_t revision, const std::uint8_t* blob,
                                          std::size_t blobLength);
int fcitx5_engine_core_snapshot_store_take(void* ledger, const FcitxEngineContextKeyC* key,
                                           std::uint64_t requestRevision, std::uint8_t* out,
                                           std::size_t outCapacity, std::size_t* outLength);
std::size_t fcitx5_engine_core_snapshot_store_required_size(
    void* ledger, const FcitxEngineContextKeyC* key);

// E6: scroll-mode candidate label offset (mirrors the C++
// `columnSelectionRow`/`rowSelectionColumn` choice). Returns 1 and writes
// `outOffset` when the candidate index shares the cursor row/column.
int fcitx5_engine_core_scroll_label_offset(int vertical, std::uint32_t cursor,
                                           std::uint32_t index,
                                           std::uint32_t dimension,
                                           std::uint32_t size,
                                           std::uint32_t* outOffset);

} // extern "C"
