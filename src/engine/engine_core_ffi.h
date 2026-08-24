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

} // extern "C"
