//! Engine product-state core: context/composition/revision ledger.
//!
//! This crate is the Rust-authoritative implementation of the per-context
//! Engine ledger previously owned by `src/engine/fcitx_runtime.cpp`:
//!
//! - `nextCompositionId` (composition id allocation, starting at 1, never 0)
//! - `compositions` (per-context current composition id; 0 = none active)
//! - `revisions` (per-context monotonically increasing revision; starts at 0)
//!
//! The semantics below mirror the C++ `FcitxRuntime::Impl` exactly so the
//! ledger can be cut over without changing Engine wire behavior (E2 in
//! `docs/engine-boundary.md`). `ContextKey` matches
//! `ClientContextKey { processId, connectionId, contextId }`.

#![deny(unsafe_op_in_unsafe_fn)]

use std::collections::HashMap;

/// Per-context identity used by the Engine ledger.
///
/// Layout matches `fcitx::windows::engine::ClientContextKey` (and the C ABI
/// `FcitxEngineContextKeyC`). Values are plain data; the `repr(C)` form is
/// used across the FFI boundary.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ContextKey {
    pub process_id: u32,
    pub connection_id: u64,
    pub context_id: u64,
}

impl ContextKey {
    pub fn new(process_id: u32, connection_id: u64, context_id: u64) -> Self {
        Self {
            process_id,
            connection_id,
            context_id,
        }
    }
}

/// Ledger rejection reasons, mirroring the C++ stale-state throws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LedgerError {
    /// Request metadata does not match the current revision/composition state.
    StaleState,
    /// Candidate id is 0 or does not belong to the current composition.
    InvalidCandidate,
}

/// Per-context composition/revision ledger.
///
/// Replaces the C++ `nextCompositionId`/`compositions`/`revisions` members of
/// `FcitxRuntime::Impl`. Contexts that were never seen report revision 0 and
/// composition 0, matching the C++ `unordered_map::operator[]` default.
pub struct ContextLedger {
    next_composition_id: u64,
    compositions: HashMap<ContextKey, u64>,
    revisions: HashMap<ContextKey, u64>,
}

impl Default for ContextLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextLedger {
    /// Creates an empty ledger. The first allocated composition id is 1;
    /// 0 is reserved to mean "no active composition".
    pub fn new() -> Self {
        Self {
            next_composition_id: 1,
            compositions: HashMap::new(),
            revisions: HashMap::new(),
        }
    }

    /// Current revision for `key` (0 if the context was never seen).
    pub fn revision_of(&self, key: ContextKey) -> u64 {
        self.revisions.get(&key).copied().unwrap_or(0)
    }

    /// Current composition id for `key` (0 if none active).
    pub fn composition_of(&self, key: ContextKey) -> u64 {
        self.compositions.get(&key).copied().unwrap_or(0)
    }

    /// Validates a key-request against the current ledger state.
    ///
    /// Mirrors the C++ `processKey` stale check: a request is rejected when
    /// its `revision`/`compositionId` metadata does not match the current
    /// per-context state. Unknown contexts match revision 0 / composition 0.
    pub fn begin_key(
        &self,
        key: ContextKey,
        revision: u64,
        composition_id: u64,
    ) -> Result<(), LedgerError> {
        if self.revision_of(key) != revision || self.composition_of(key) != composition_id {
            return Err(LedgerError::StaleState);
        }
        Ok(())
    }

    /// Validates a candidate-selection request.
    ///
    /// Mirrors the C++ `selectCandidate` stale check: revision/composition
    /// mismatch, `candidateId == 0`, or a candidate id whose high bits do not
    /// encode the current composition id are all rejected.
    pub fn select_candidate(
        &self,
        key: ContextKey,
        revision: u64,
        composition_id: u64,
        candidate_id: u64,
    ) -> Result<(), LedgerError> {
        if self.revision_of(key) != revision || self.composition_of(key) != composition_id {
            return Err(LedgerError::StaleState);
        }
        if candidate_id == 0 || (candidate_id >> 8) != composition_id {
            return Err(LedgerError::InvalidCandidate);
        }
        Ok(())
    }

    /// Applies the end-of-result composition lifecycle and revision bump.
    ///
    /// Mirrors `FcitxRuntime::Impl::collectResult`: while a context has
    /// preedit text or candidates a non-zero composition id is allocated on
    /// first use; when it has neither the composition resets to 0; the
    /// per-context revision is incremented on every result. Returns
    /// `(composition_id, revision)`.
    pub fn end_result(&mut self, key: ContextKey, has_content: bool) -> (u64, u64) {
        let current = self.composition_of(key);
        let composition = if has_content {
            if current == 0 {
                self.allocate_composition_id()
            } else {
                current
            }
        } else {
            0
        };
        self.compositions.insert(key, composition);
        let revision = self.revisions.entry(key).or_insert(0);
        *revision = revision.wrapping_add(1);
        (composition, *revision)
    }

    /// Drops all ledger state for `key` (context erased).
    pub fn forget(&mut self, key: ContextKey) {
        self.compositions.remove(&key);
        self.revisions.remove(&key);
    }

    fn allocate_composition_id(&mut self) -> u64 {
        // Mirrors `composition = nextCompositionId++; if (composition == 0)
        // composition = nextCompositionId++;` — the allocation wraps and
        // skips the reserved id 0.
        let mut id = self.next_composition_id;
        self.next_composition_id = self.next_composition_id.wrapping_add(1);
        if id == 0 {
            id = self.next_composition_id;
            self.next_composition_id = self.next_composition_id.wrapping_add(1);
        }
        id
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub mod capi;
