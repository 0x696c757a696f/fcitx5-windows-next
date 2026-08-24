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

/// Last-known caret rectangle for a context.
///
/// Layout matches `protocol::CaretRect` (dpi defaults to 96 in the C++
/// aggregate default; the ledger stores whatever was set).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct CaretRect {
    pub valid: u8,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub dpi: u32,
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
///
/// E2 extension: the ledger also owns the remaining per-context product state
/// maps that were in `FcitxRuntime::Impl` — `carets`, `popupAllowed`,
/// `selectedOverride`, and `inputMethodOverridden`. `pendingStates` stays a
/// C++-owned derived cache until the E5 snapshot DTO moves to Rust.
pub struct ContextLedger {
    next_composition_id: u64,
    compositions: HashMap<ContextKey, u64>,
    revisions: HashMap<ContextKey, u64>,
    carets: HashMap<ContextKey, CaretRect>,
    popup_allowed: HashMap<ContextKey, bool>,
    selected_override: HashMap<ContextKey, Option<u32>>,
    input_method_overridden: HashMap<ContextKey, bool>,
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
            carets: HashMap::new(),
            popup_allowed: HashMap::new(),
            selected_override: HashMap::new(),
            input_method_overridden: HashMap::new(),
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
        self.carets.remove(&key);
        self.popup_allowed.remove(&key);
        self.selected_override.remove(&key);
        self.input_method_overridden.remove(&key);
    }

    /// Stores the last-known caret rectangle for `key` (mirrors
    /// `impl_->carets[key] = request.caret` in `processKey`).
    pub fn set_caret(&mut self, key: ContextKey, caret: CaretRect) {
        self.carets.insert(key, caret);
    }

    /// Returns the stored caret for `key`, if any (mirrors
    /// `carets.find(key)` in `collectResult`).
    pub fn caret(&self, key: ContextKey) -> Option<CaretRect> {
        self.carets.get(&key).copied()
    }

    /// Stores the popup policy for `key` (mirrors
    /// `impl_->popupAllowed[key] = request.popupAllowed`).
    pub fn set_popup_allowed(&mut self, key: ContextKey, allowed: bool) {
        self.popup_allowed.insert(key, allowed);
    }

    /// Returns the stored popup policy for `key`, if any.
    pub fn popup_allowed(&self, key: ContextKey) -> Option<bool> {
        self.popup_allowed.get(&key).copied()
    }

    /// Sets the candidate-highlight override for `key` (mirrors
    /// `impl_->selectedOverride[key] = value`).
    pub fn set_selected_override(&mut self, key: ContextKey, value: u32) {
        self.selected_override.insert(key, Some(value));
    }

    /// Clears the candidate-highlight override for `key` (mirrors
    /// `impl_->selectedOverride.erase(key)`).
    pub fn clear_selected_override(&mut self, key: ContextKey) {
        self.selected_override.remove(&key);
    }

    /// Returns the candidate-highlight override for `key`, if set. A stored
    /// override of `Some(0)` is reported as present (mirrors the C++ `found
    /// != end && found->second` checks where `Some(0)` still overrides to 0).
    pub fn selected_override(&self, key: ContextKey) -> Option<u32> {
        self.selected_override.get(&key).copied().flatten()
    }

    /// Marks `key` as input-method-overridden (mirrors
    /// `inputMethodOverridden[key] = true` in the toggle/next handlers).
    pub fn set_input_method_overridden(&mut self, key: ContextKey, overridden: bool) {
        self.input_method_overridden.insert(key, overridden);
    }

    /// Returns whether `key` is marked input-method-overridden. Unknown
    /// contexts report `false` (mirrors `impl_->inputMethodOverridden[key]`
    /// default construction).
    pub fn input_method_overridden(&self, key: ContextKey) -> bool {
        self.input_method_overridden
            .get(&key)
            .copied()
            .unwrap_or(false)
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

// ---------------------------------------------------------------------------
// E3: Event → Action (functional core)
//
// The Engine is a Functional Core / Imperative Shell: the C++ Fcitx adapter
// flattens Fcitx events into plain EngineEvents, Rust decides the action
// batch from the authoritative product state, and the C++ adapter executes
// the actions against Fcitx objects. `classify_input_method_switch` is the
// first decision moved to Rust; it mirrors the C++ `matches()` hotkey logic
// in `FcitxRuntime::processKey` exactly (toggle is checked before next).
// ---------------------------------------------------------------------------

/// Keysym constants mirroring `fcitx-utils/keysymgen.h`.
pub const KEY_SYM_NONE: u32 = 0x0;
pub const KEY_SYM_SPACE: u32 = 0x0020;
pub const KEY_SYM_SHIFT_L: u32 = 0xffe1;
pub const KEY_SYM_CONTROL_L: u32 = 0xffe3;
pub const KEY_SYM_ALT_L: u32 = 0xffe9;

/// Input-method switch action decided from a key event + hotkey config.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImSwitchAction {
    Toggle,
    Next,
}

/// Decides whether a non-release key event triggers the configured
/// input-method switch hotkey.
///
/// Mirrors `FcitxRuntime::processKey`:
///
/// ```cpp
/// if (!event.isRelease()) {
///   const auto matches = [&](const std::optional<std::string>& hotkey) {
///     if (!hotkey || keySym == FcitxKey_None) return false;
///     if (*hotkey == "Ctrl+Space") return ctrl && !shift && !alt && keySym == FcitxKey_space;
///     if (*hotkey == "Ctrl+Shift") return ctrl && shift && !alt && keySym == FcitxKey_Shift_L;
///     if (*hotkey == "Ctrl+Shift+Space") return ctrl && shift && !alt && keySym == FcitxKey_space;
///     if (*hotkey == "Alt+Shift") return alt && shift && !ctrl && keySym == FcitxKey_Shift_L;
///     return false;
///   };
///   if (matches(engineConfig.hotkeyToggle)) { toggle... }
///   if (matches(engineConfig.hotkeyNext)) { next... }
/// }
/// ```
///
/// The caller already excluded release events, so `is_release` is not part of
/// the decision. Unknown hotkey strings never match. Toggle wins when both
/// configured hotkeys match the same event.
pub fn classify_input_method_switch(
    ctrl: bool,
    shift: bool,
    alt: bool,
    key_sym: u32,
    hotkey_toggle: Option<&str>,
    hotkey_next: Option<&str>,
) -> Option<ImSwitchAction> {
    if key_sym == KEY_SYM_NONE {
        return None;
    }
    let matches = |hotkey: Option<&str>| -> bool {
        let Some(hotkey) = hotkey else {
            return false;
        };
        match hotkey {
            "Ctrl+Space" => ctrl && !shift && !alt && key_sym == KEY_SYM_SPACE,
            "Ctrl+Shift" => ctrl && shift && !alt && key_sym == KEY_SYM_SHIFT_L,
            "Ctrl+Shift+Space" => ctrl && shift && !alt && key_sym == KEY_SYM_SPACE,
            "Alt+Shift" => alt && shift && !ctrl && key_sym == KEY_SYM_SHIFT_L,
            _ => false,
        }
    };
    if matches(hotkey_toggle) {
        return Some(ImSwitchAction::Toggle);
    }
    if matches(hotkey_next) {
        return Some(ImSwitchAction::Next);
    }
    None
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub mod capi;
