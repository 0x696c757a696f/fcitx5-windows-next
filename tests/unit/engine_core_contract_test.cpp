// Engine context/composition/revision ledger contract (E2 corpus freeze).
//
// The per-context ledger semantics below are the frozen C++ behavior of
// `src/engine/fcitx_runtime.cpp` (`FcitxRuntime::Impl`:
// `nextCompositionId`/`compositions`/`revisions`):
//
//   * an unknown context has revision 0 and composition 0;
//   * `processKey` rejects any request whose metadata revision/compositionId
//     does not match the current per-context state ("stale input context");
//   * `selectCandidate` additionally rejects candidateId 0 or a candidate id
//     whose high bits do not encode the current composition id;
//   * `collectResult` allocates a non-zero composition id (starting at 1)
//     while a context has preedit text or candidates, resets it to 0 when it
//     has neither, and increments the per-context revision on every result;
//   * erasing a context forgets all of its ledger state.
//
// This test pins that corpus against the Rust `fcitx5-engine-core` C ABI
// (side-by-side phase of E2; the C++ runtime is not cut over yet).

#include "engine_core_ffi.h"

#include <cstdint>
#include <cstring>
#include <iostream>

namespace {

bool expect(bool condition, const char* message) {
    if (!condition) {
        std::cerr << message << '\n';
    }
    return condition;
}

FcitxEngineContextKeyC keyA() {
    return FcitxEngineContextKeyC{100, 1, 7};
}

FcitxEngineContextKeyC keyB() {
    return FcitxEngineContextKeyC{200, 2, 9};
}

struct LedgerGuard {
    void* handle{fcitx5_engine_core_ledger_new()};
    ~LedgerGuard() { fcitx5_engine_core_ledger_free(handle); }
};

int runCorpus() {
    int failures = 0;
    LedgerGuard guard;
    void* ledger = guard.handle;
    if (!expect(ledger != nullptr, "ledger_new returned null")) {
        return 1;
    }

    // Unknown context: revision 0 / composition 0 accepted.
    const auto a = keyA();
    failures += !expect(
        fcitx5_engine_core_ledger_begin_key(ledger, &a, 0, 0) == FCITX_ENGINE_CORE_OK,
        "begin_key(0,0) on unknown context must be OK");

    // Any other metadata on an unknown context is stale.
    failures += !expect(
        fcitx5_engine_core_ledger_begin_key(ledger, &a, 1, 0) == FCITX_ENGINE_CORE_STALE,
        "begin_key(1,0) on unknown context must be STALE");
    failures += !expect(
        fcitx5_engine_core_ledger_begin_key(ledger, &a, 0, 1) == FCITX_ENGINE_CORE_STALE,
        "begin_key(0,1) on unknown context must be STALE");

    // First content-bearing result allocates composition 1 and revision 1.
    std::uint64_t composition = 0;
    std::uint64_t revision = 0;
    failures += !expect(
        fcitx5_engine_core_ledger_end_result(ledger, &a, 1, &composition, &revision) ==
            FCITX_ENGINE_CORE_OK,
        "end_result must succeed");
    failures += !expect(composition == 1 && revision == 1,
                        "first content-bearing result must be (composition=1, revision=1)");

    // The produced metadata is the only acceptable key metadata now.
    failures += !expect(
        fcitx5_engine_core_ledger_begin_key(ledger, &a, revision, composition) ==
            FCITX_ENGINE_CORE_OK,
        "begin_key with current metadata must be OK");
    failures += !expect(
        fcitx5_engine_core_ledger_begin_key(ledger, &a, 0, 0) == FCITX_ENGINE_CORE_STALE,
        "begin_key with pre-result metadata must be STALE");

    // Content stays active: composition id is kept, revision bumps.
    failures += !expect(
        fcitx5_engine_core_ledger_end_result(ledger, &a, 1, &composition, &revision) ==
            FCITX_ENGINE_CORE_OK,
        "second end_result must succeed");
    failures += !expect(composition == 1 && revision == 2,
                        "second content-bearing result must keep composition=1, revision=2");

    // No content: composition resets to 0, revision still bumps.
    failures += !expect(
        fcitx5_engine_core_ledger_end_result(ledger, &a, 0, &composition, &revision) ==
            FCITX_ENGINE_CORE_OK,
        "empty end_result must succeed");
    failures += !expect(composition == 0 && revision == 3,
                        "empty result must be (composition=0, revision=3)");

    // Next content allocates a fresh composition id.
    failures += !expect(
        fcitx5_engine_core_ledger_end_result(ledger, &a, 1, &composition, &revision) ==
            FCITX_ENGINE_CORE_OK,
        "end_result after reset must succeed");
    failures += !expect(composition == 2 && revision == 4,
                        "result after reset must allocate composition=2, revision=4");

    // Candidate selection on the current composition.
    const std::uint64_t candidateId = (composition << 8) | 1U;
    failures += !expect(
        fcitx5_engine_core_ledger_select_candidate(ledger, &a, revision, composition,
                                                   candidateId) == FCITX_ENGINE_CORE_OK,
        "select_candidate with encoded candidate must be OK");
    failures += !expect(
        fcitx5_engine_core_ledger_select_candidate(ledger, &a, revision, composition, 0) ==
            FCITX_ENGINE_CORE_INVALID_CANDIDATE,
        "select_candidate with candidateId 0 must be INVALID_CANDIDATE");
    failures += !expect(
        fcitx5_engine_core_ledger_select_candidate(ledger, &a, revision, composition,
                                                   ((composition + 1) << 8) | 1U) ==
            FCITX_ENGINE_CORE_INVALID_CANDIDATE,
        "select_candidate with foreign composition bits must be INVALID_CANDIDATE");
    failures += !expect(
        fcitx5_engine_core_ledger_select_candidate(ledger, &a, 0, composition, candidateId) ==
            FCITX_ENGINE_CORE_STALE,
        "select_candidate with stale revision must be STALE");

    // Contexts are isolated.
    const auto b = keyB();
    failures += !expect(
        fcitx5_engine_core_ledger_begin_key(ledger, &b, 0, 0) == FCITX_ENGINE_CORE_OK,
        "second context must start at revision 0 / composition 0");
    std::uint64_t compositionB = 0;
    std::uint64_t revisionB = 0;
    failures += !expect(
        fcitx5_engine_core_ledger_end_result(ledger, &b, 1, &compositionB, &revisionB) ==
            FCITX_ENGINE_CORE_OK,
        "second context end_result must succeed");
    failures += !expect(compositionB == 3 && revisionB == 1,
                        "second context must allocate composition=3, revision=1");

    // Forgetting a context resets it to the unknown state.
    fcitx5_engine_core_ledger_forget(ledger, &a);
    failures += !expect(
        fcitx5_engine_core_ledger_begin_key(ledger, &a, 0, 0) == FCITX_ENGINE_CORE_OK,
        "forgotten context must accept begin_key(0,0) again");

    // Null guard: null key / null outputs fail closed.
    failures += !expect(
        fcitx5_engine_core_ledger_begin_key(ledger, nullptr, 0, 0) ==
            FCITX_ENGINE_CORE_STALE,
        "begin_key with null key must fail closed");
    failures += !expect(
        fcitx5_engine_core_ledger_end_result(ledger, &b, 1, nullptr, nullptr) ==
            FCITX_ENGINE_CORE_STALE,
        "end_result with null outputs must fail closed");

    // E2 extension: per-context product state maps.
    // caret: absent until set; set/get roundtrip; forget clears.
    FcitxEngineCaretC caret{};
    failures += !expect(fcitx5_engine_core_caret(ledger, &a, &caret) == 0,
                        "caret must be absent before set");
    const FcitxEngineCaretC expectedCaret{1, -100, 200, -98, 222, 144};
    failures += !expect(
        fcitx5_engine_core_set_caret(ledger, &a, &expectedCaret) == FCITX_ENGINE_CORE_OK,
        "set_caret must succeed");
    failures += !expect(fcitx5_engine_core_caret(ledger, &a, &caret) == 1 &&
                            caret.valid == expectedCaret.valid &&
                            caret.left == expectedCaret.left &&
                            caret.dpi == expectedCaret.dpi,
                        "caret roundtrip must match");
    fcitx5_engine_core_ledger_forget(ledger, &a);
    failures += !expect(fcitx5_engine_core_caret(ledger, &a, &caret) == 0,
                        "caret must clear on forget");

    // popupAllowed: set false/true; absent until set.
    int allowed = 1;
    failures += !expect(fcitx5_engine_core_popup_allowed(ledger, &b, &allowed) == 0,
                        "popupAllowed must be absent before set");
    failures += !expect(
        fcitx5_engine_core_set_popup_allowed(ledger, &b, 0) == FCITX_ENGINE_CORE_OK,
        "set_popup_allowed(0) must succeed");
    failures += !expect(fcitx5_engine_core_popup_allowed(ledger, &b, &allowed) == 1 &&
                            allowed == 0,
                        "popupAllowed roundtrip (false) must match");
    failures += !expect(
        fcitx5_engine_core_set_popup_allowed(ledger, &b, 1) == FCITX_ENGINE_CORE_OK,
        "set_popup_allowed(1) must succeed");
    failures += !expect(fcitx5_engine_core_popup_allowed(ledger, &b, &allowed) == 1 &&
                            allowed == 1,
                        "popupAllowed roundtrip (true) must match");

    // selectedOverride: set/query/clear; Some(0) is still reported as set.
    std::uint32_t overrideValue = 0;
    failures += !expect(
        fcitx5_engine_core_selected_override(ledger, &b, &overrideValue) == 0,
        "selectedOverride must be absent before set");
    failures += !expect(
        fcitx5_engine_core_set_selected_override(ledger, &b, 4) == FCITX_ENGINE_CORE_OK,
        "set_selected_override must succeed");
    failures += !expect(
        fcitx5_engine_core_selected_override(ledger, &b, &overrideValue) == 1 &&
            overrideValue == 4,
        "selectedOverride roundtrip must match");
    failures += !expect(
        fcitx5_engine_core_set_selected_override(ledger, &b, 0) == FCITX_ENGINE_CORE_OK,
        "set_selected_override(0) must succeed");
    failures += !expect(
        fcitx5_engine_core_selected_override(ledger, &b, &overrideValue) == 1 &&
            overrideValue == 0,
        "selectedOverride Some(0) must still be reported as set");
    failures += !expect(
        fcitx5_engine_core_clear_selected_override(ledger, &b) == FCITX_ENGINE_CORE_OK,
        "clear_selected_override must succeed");
    failures += !expect(
        fcitx5_engine_core_selected_override(ledger, &b, &overrideValue) == 0,
        "selectedOverride must be absent after clear");

    // inputMethodOverridden: unknown context reports absent; set 1 then 0.
    int overridden = 1;
    failures += !expect(
        fcitx5_engine_core_input_method_overridden(ledger, &a, &overridden) == 0,
        "inputMethodOverridden must be absent for unknown context");
    failures += !expect(
        fcitx5_engine_core_set_input_method_overridden(ledger, &a, 1) ==
            FCITX_ENGINE_CORE_OK,
        "set_input_method_overridden(1) must succeed");
    failures += !expect(
        fcitx5_engine_core_input_method_overridden(ledger, &a, &overridden) == 1 &&
            overridden == 1,
        "inputMethodOverridden must report set");
    failures += !expect(
        fcitx5_engine_core_set_input_method_overridden(ledger, &a, 0) ==
            FCITX_ENGINE_CORE_OK,
        "set_input_method_overridden(0) must succeed");
    failures += !expect(
        fcitx5_engine_core_input_method_overridden(ledger, &a, &overridden) == 0,
        "inputMethodOverridden(0) must report absent (C++ default semantics)");

    // E3 Event → Action: input-method switch hotkey decision (mirrors the
    // frozen C++ `matches()` semantics that the Rust decision replaced).
    {
        const char* toggle = "Ctrl+Space";
        const char* next = "Ctrl+Shift";
        int action = FCITX_ENGINE_CORE_IM_ACTION_NONE;
        failures += !expect(
            fcitx5_engine_core_classify_input_method_switch(1, 0, 0, 0x20, toggle, next,
                                                            &action) == 1 &&
                action == FCITX_ENGINE_CORE_IM_ACTION_TOGGLE,
            "Ctrl+Space must decide TOGGLE");
        failures += !expect(
            fcitx5_engine_core_classify_input_method_switch(1, 1, 0, 0xffe1, toggle, next,
                                                            &action) == 1 &&
                action == FCITX_ENGINE_CORE_IM_ACTION_NEXT,
            "Ctrl+Shift on Shift_L must decide NEXT");
        failures += !expect(
            fcitx5_engine_core_classify_input_method_switch(1, 1, 0, 0x20,
                                                            "Ctrl+Shift+Space", next,
                                                            &action) == 1 &&
                action == FCITX_ENGINE_CORE_IM_ACTION_TOGGLE,
            "Ctrl+Shift+Space must decide TOGGLE");
        failures += !expect(
            fcitx5_engine_core_classify_input_method_switch(0, 1, 1, 0xffe1, "Alt+Shift",
                                                            next, &action) == 1 &&
                action == FCITX_ENGINE_CORE_IM_ACTION_TOGGLE,
            "Alt+Shift must decide TOGGLE");
        failures += !expect(
            fcitx5_engine_core_classify_input_method_switch(0, 1, 0, 0x20, toggle, next,
                                                            &action) == 0,
            "Shift+Space must not match any hotkey");
        failures += !expect(
            fcitx5_engine_core_classify_input_method_switch(1, 0, 0, 0x41, toggle, next,
                                                            &action) == 0,
            "ordinary key must not match any hotkey");
        failures += !expect(
            fcitx5_engine_core_classify_input_method_switch(1, 0, 0, 0x20, nullptr, nullptr,
                                                            &action) == 0,
            "unconfigured hotkeys must not match");
        failures += !expect(
            fcitx5_engine_core_classify_input_method_switch(1, 0, 0, 0, toggle, next,
                                                            &action) == 0,
            "FcitxKey_None must never match");
    }

    // E3-2 Event → Action: candidate navigation decision corpus (mirrors the
    // frozen C++ branch semantics the Rust decision replaced).
    {
        FcitxCandidateViewC view{};
        view.count = 10;
        view.listSize = 10;
        view.cursor = 0;
        view.bulkCursor = -1;
        view.hasBulkCursor = 0;
        view.hasBulk = 0;
        view.pageable = 1;
        view.hasPrev = 0;
        view.hasNext = 0;
        FcitxCandidateConfigC viewConfig{};
        viewConfig.scrollMode = 0;
        viewConfig.vertical = 0;
        viewConfig.candidatePageSize = -1;
        FcitxCandidateDecisionC decision{};
        // Ordinary paging ';' selects the second candidate.
        failures += !expect(
            fcitx5_engine_core_decide_candidate_action(0x3b, 1, &view, &viewConfig, 0, 0,
                                                       &decision) == FCITX_ENGINE_CORE_OK &&
                decision.consume == 1 &&
                decision.action == FCITX_ENGINE_CORE_CANDIDATE_ACTION_SELECT_AND_CLEAR &&
                decision.value == 1,
            "ordinary ';' must select candidate 1 and clear override");
        // Space with a highlight override commits that candidate.
        failures += !expect(
            fcitx5_engine_core_decide_candidate_action(0x20, 1, &view, &viewConfig, 1, 3,
                                                       &decision) == FCITX_ENGINE_CORE_OK &&
                decision.consume == 1 &&
                decision.action == FCITX_ENGINE_CORE_CANDIDATE_ACTION_SELECT_AND_CLEAR &&
                decision.value == 3,
            "Space must commit the overridden candidate");
        // Space without override is not consumed.
        failures += !expect(
            fcitx5_engine_core_decide_candidate_action(0x20, 1, &view, &viewConfig, 0, 0,
                                                       &decision) == FCITX_ENGINE_CORE_OK &&
                decision.consume == 0 &&
                decision.action == FCITX_ENGINE_CORE_CANDIDATE_ACTION_NONE,
            "Space without override must not be consumed");
        // Right moves the highlight.
        failures += !expect(
            fcitx5_engine_core_decide_candidate_action(0xff53, 1, &view, &viewConfig, 1, 2,
                                                       &decision) == FCITX_ENGINE_CORE_OK &&
                decision.consume == 1 &&
                decision.action == FCITX_ENGINE_CORE_CANDIDATE_ACTION_SET_OVERRIDE &&
                decision.value == 3,
            "Right must move the highlight override");
        // '=' with a next page turns the page and resets the override.
        {
            FcitxCandidateViewC paged = view;
            paged.hasNext = 1;
            failures += !expect(
                fcitx5_engine_core_decide_candidate_action(0x3d, 1, &paged, &viewConfig, 0, 0,
                                                           &decision) == FCITX_ENGINE_CORE_OK &&
                    decision.consume == 1 &&
                    decision.action ==
                        FCITX_ENGINE_CORE_CANDIDATE_ACTION_PAGE_NEXT_AND_SET_OVERRIDE &&
                    decision.value == 0,
                "next-page key must turn the page and reset the override");
        }
        // Ordinary key is not consumed.
        failures += !expect(
            fcitx5_engine_core_decide_candidate_action(0x61, 1, &view, &viewConfig, 0, 0,
                                                       &decision) == FCITX_ENGINE_CORE_OK &&
                decision.consume == 0,
            "ordinary key must not be consumed");
    }

    // E3-3 Event → Action: surrounding-text and input-method-selection corpus.
    {
        FcitxSurroundingTextDecisionC st{};
        failures += !expect(
            fcitx5_engine_core_decide_surrounding_text(1, 0, &st) == FCITX_ENGINE_CORE_OK &&
                st.action == FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_SET && st.update == 1,
            "valid request must decide SET with update");
        failures += !expect(
            fcitx5_engine_core_decide_surrounding_text(0, 1, &st) == FCITX_ENGINE_CORE_OK &&
                st.action == FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_INVALIDATE &&
                st.update == 1,
            "invalid request with valid state must decide INVALIDATE with update");
        failures += !expect(
            fcitx5_engine_core_decide_surrounding_text(0, 0, &st) == FCITX_ENGINE_CORE_OK &&
                st.action == FCITX_ENGINE_CORE_SURROUNDING_TEXT_ACTION_INVALIDATE &&
                st.update == 0,
            "invalid request with invalid state must decide INVALIDATE without update");
        int selection = -1;
        failures += !expect(
            fcitx5_engine_core_decide_input_method_selection(1, 1, 1, 1, 0, 0, 0, &selection) ==
                    FCITX_ENGINE_CORE_OK &&
                selection == FCITX_ENGINE_CORE_IM_SELECTION_REQUEST,
            "valid request input method must be selected when current differs");
        failures += !expect(
            fcitx5_engine_core_decide_input_method_selection(0, 0, 1, 1, 0, 1, 0, &selection) ==
                    FCITX_ENGINE_CORE_OK &&
                selection == FCITX_ENGINE_CORE_IM_SELECTION_NONE,
            "no switch when current already equals the default");
        failures += !expect(
            fcitx5_engine_core_decide_input_method_selection(1, 1, 1, 1, 0, 0, 1, &selection) ==
                    FCITX_ENGINE_CORE_OK &&
                selection == FCITX_ENGINE_CORE_IM_SELECTION_NONE,
            "override marker must suppress the input-method switch");
    }

    // E4: engine-process session epoch + request-sequence/deadline policy.
    {
        failures += !expect(
            fcitx5_engine_core_validate_engine_epoch(42, 42) == 1 &&
                fcitx5_engine_core_validate_engine_epoch(41, 42) == 0,
            "engine epoch validation must reject mismatched frames");
        failures += !expect(
            fcitx5_engine_core_accept_frame_sequence(1, 0) == 1 &&
                fcitx5_engine_core_accept_frame_sequence(0, 0) == 0 &&
                fcitx5_engine_core_accept_frame_sequence(5, 5) == 0 &&
                fcitx5_engine_core_accept_frame_sequence(4, 5) == 0,
            "request ordering must accept only strictly newer ids");
        failures += !expect(
            fcitx5_engine_core_key_request_timeout_ms(0) == 2500 &&
                fcitx5_engine_core_key_request_timeout_ms(1) == 75 &&
                fcitx5_engine_core_key_request_timeout_ms(42) == 75,
            "key request deadline policy must be 2500 ms cold / 75 ms warm");
    }

    // E5-1: snapshot/status canonicalization.
    {
        failures += !expect(
            fcitx5_engine_core_content_locale_for_input_method("mozc") ==
                    FCITX_ENGINE_CORE_CONTENT_LOCALE_JA_JP &&
                fcitx5_engine_core_content_locale_for_input_method("hangul") ==
                    FCITX_ENGINE_CORE_CONTENT_LOCALE_KO_KR &&
                fcitx5_engine_core_content_locale_for_input_method("keyboard-us") ==
                    FCITX_ENGINE_CORE_CONTENT_LOCALE_EN_US &&
                fcitx5_engine_core_content_locale_for_input_method("pinyin") ==
                    FCITX_ENGINE_CORE_CONTENT_LOCALE_ZH_CN &&
                fcitx5_engine_core_content_locale_for_input_method("unknown") ==
                    FCITX_ENGINE_CORE_CONTENT_LOCALE_NONE,
            "content-locale canonicalization must map input-method families");
        std::uint8_t label[8]{};
        failures += !expect(
            fcitx5_engine_core_status_short_label(
                reinterpret_cast<const std::uint8_t*>("abc"), 3, label, sizeof(label)) == 2 &&
                label[0] == 'a' && label[1] == 'b',
            "short label must take two ASCII bytes");
        const char* chinese = "\xe4\xbd\xa0\xe5\xa5\xbd"; // 你好
        failures += !expect(
            fcitx5_engine_core_status_short_label(
                reinterpret_cast<const std::uint8_t*>(chinese), 6, label, sizeof(label)) == 3 &&
                label[0] == 0xe4 && label[1] == 0xbd && label[2] == 0xa0,
            "short label must take the first code point for non-ASCII");
    }

    // E5-2: snapshot DTO limits validation.
    {
        FcitxEngineSnapshotC snapshot{};
        snapshot.commitUtf8Len = 3;
        snapshot.preeditUtf8Len = 2;
        snapshot.candidateCount = 5;
        snapshot.candidateLabelLenMax = 1;
        snapshot.candidateTextLenMax = 4;
        snapshot.candidateCommentLenMax = 8;
        snapshot.contentLocaleUtf8Len = 5;
        failures += !expect(
            fcitx5_engine_core_validate_snapshot(&snapshot) == 1,
            "normal snapshot facts must validate");
        snapshot.candidateCount = 129;
        failures += !expect(
            fcitx5_engine_core_validate_snapshot(&snapshot) == 0,
            "oversized candidate count must be rejected");
        snapshot.candidateCount = 5;
        snapshot.commitUtf8Len = 16 * 1024 + 1;
        failures += !expect(
            fcitx5_engine_core_validate_snapshot(&snapshot) == 0,
            "oversized commit must be rejected");
        failures += !expect(
            fcitx5_engine_core_validate_snapshot(nullptr) == 0,
            "null snapshot must fail closed");
    }

    // E5-3: pending snapshot store error paths.
    {
        failures += !expect(
            fcitx5_engine_core_snapshot_store_put(ledger, &a, 1, nullptr, 0) ==
                FCITX_ENGINE_CORE_STALE,
            "malformed snapshot blob must be rejected");
        failures += !expect(
            fcitx5_engine_core_snapshot_store_required_size(ledger, &a) == 0,
            "absent pending snapshot reports size 0");
        std::size_t blobLength = 0;
        failures += !expect(
            fcitx5_engine_core_snapshot_store_take(ledger, &a, 0, nullptr, 0, &blobLength) ==
                0,
            "absent pending snapshot must not be taken");
    }

    // E4-3: per-connection session state (handshake + frame validation +
    // request completion; mirrors the frozen C++ `handleRequest` behavior).
    {
        void* session = fcitx5_engine_core_session_create();
        failures += !expect(session != nullptr, "session create must succeed");
        // Null destroy is a no-op; null session operations fail closed.
        fcitx5_engine_core_session_destroy(nullptr);
        failures += !expect(
            fcitx5_engine_core_session_begin_hello(nullptr, 1, 77, 77, 100, 100) == 0,
            "null session hello must fail closed");
        failures += !expect(
            fcitx5_engine_core_session_accept_frame(nullptr, 1, 77, 77, 42, 42) == 0,
            "null session frame must fail closed");
        failures += !expect(
            fcitx5_engine_core_session_complete_request(nullptr, 1) == 0,
            "null session complete must fail closed");
        // Hello handshake: session/process mismatch and stale ids rejected.
        failures += !expect(
            fcitx5_engine_core_session_begin_hello(session, 1, 78, 77, 100, 100) == 0,
            "hello with mismatched frame session id must be rejected");
        failures += !expect(
            fcitx5_engine_core_session_begin_hello(session, 1, 77, 77, 101, 100) == 0,
            "hello with mismatched process id must be rejected");
        failures += !expect(
            fcitx5_engine_core_session_begin_hello(session, 1, 77, 77, 100, 100) == 1,
            "valid hello must be accepted");
        failures += !expect(
            fcitx5_engine_core_session_begin_hello(session, 2, 77, 77, 100, 100) == 0,
            "repeat handshake must be rejected");
        // Non-hello frames require handshake + epoch + session + ordering.
        failures += !expect(
            fcitx5_engine_core_session_accept_frame(session, 2, 77, 77, 43, 42) == 0,
            "frame with mismatched epoch must be rejected");
        failures += !expect(
            fcitx5_engine_core_session_accept_frame(session, 2, 78, 77, 42, 42) == 0,
            "frame with mismatched session id must be rejected");
        failures += !expect(
            fcitx5_engine_core_session_accept_frame(session, 2, 77, 77, 42, 42) == 1,
            "valid frame must be accepted");
        // Accepted but not completed: retryable (mirrors a dropped request).
        failures += !expect(
            fcitx5_engine_core_session_accept_frame(session, 2, 77, 77, 42, 42) == 1,
            "uncompleted frame id must remain retryable");
        failures += !expect(fcitx5_engine_core_session_complete_request(session, 2) == 1,
                            "completed request must be recorded");
        failures += !expect(
            fcitx5_engine_core_session_accept_frame(session, 2, 77, 77, 42, 42) == 0,
            "completed request id must be stale");
        failures += !expect(
            fcitx5_engine_core_session_accept_frame(session, 1, 77, 77, 42, 42) == 0,
            "stale request id must be rejected");
        failures += !expect(
            fcitx5_engine_core_session_accept_frame(session, 3, 77, 77, 42, 42) == 1,
            "newer request id must be accepted");
        // A fresh session starts unhandshaken.
        void* fresh = fcitx5_engine_core_session_create();
        failures += !expect(
            fcitx5_engine_core_session_accept_frame(fresh, 1, 77, 77, 42, 42) == 0,
            "unhandshaken session must reject frames");
        fcitx5_engine_core_session_destroy(fresh);
        fcitx5_engine_core_session_destroy(session);
    }

    return failures;
}

} // namespace

int main() {
    const int failures = runCorpus();
    if (failures == 0) {
        std::cout << "engine-core-contract: all checks passed\n";
        return 0;
    }
    std::cerr << "engine-core-contract: " << failures << " check(s) failed\n";
    return 1;
}
