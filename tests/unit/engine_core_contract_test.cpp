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
