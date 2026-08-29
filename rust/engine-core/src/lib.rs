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

pub use fcitx5_protocol_core as protocol;

mod presentation;

#[cfg(windows)]
mod presentation_publisher;

pub use presentation::{PresentationPublicationAction, PresentationPublicationQueue};

/// Decodes and validates one presentation KeyResponse frame.
#[must_use]
pub fn decode_presentation_frame(bytes: &[u8]) -> Option<protocol::KeyResponse> {
    let frame = protocol::decode_frame(bytes)?;
    protocol::decode_key_response(&frame)
}

/// A validated request accepted by the Engine product plane.
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Request {
    Hello(protocol::HelloRequest),
    Key(protocol::KeyRequest),
    Launcher(protocol::LauncherRequest),
    CandidateSelect(protocol::CandidateSelectRequest),
    State(protocol::StateRequest),
    EngineStatus(protocol::EngineStatusRequest),
}

/// Decodes and validates one Engine request frame.
#[must_use]
pub fn decode_request(bytes: &[u8]) -> Option<Request> {
    let frame = protocol::decode_frame(bytes)?;
    match frame.message_type {
        protocol::MessageType::HelloRequest => {
            protocol::decode_hello_request(&frame).map(Request::Hello)
        }
        protocol::MessageType::KeyRequest => protocol::decode_key_request(&frame).map(Request::Key),
        protocol::MessageType::LauncherRequest => {
            protocol::decode_launcher_request(&frame).map(Request::Launcher)
        }
        protocol::MessageType::CandidateSelectRequest => {
            protocol::decode_candidate_select_request(&frame).map(Request::CandidateSelect)
        }
        protocol::MessageType::StateRequest => {
            protocol::decode_state_request(&frame).map(Request::State)
        }
        protocol::MessageType::EngineStatusRequest => {
            protocol::decode_engine_status_request(&frame).map(Request::EngineStatus)
        }
        _ => None,
    }
}

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
/// `selectedOverride`, and `inputMethodOverridden`. The E5 `pendingStates`
/// snapshot store is Rust-owned (`snapshot::SnapshotStore`).
pub struct ContextLedger {
    next_composition_id: u64,
    compositions: HashMap<ContextKey, u64>,
    revisions: HashMap<ContextKey, u64>,
    carets: HashMap<ContextKey, CaretRect>,
    popup_allowed: HashMap<ContextKey, bool>,
    selected_override: HashMap<ContextKey, Option<u32>>,
    input_method_overridden: HashMap<ContextKey, bool>,
    snapshot_store: snapshot::SnapshotStore,
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
            snapshot_store: snapshot::SnapshotStore::new(),
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
        self.snapshot_store.forget(key);
    }

    /// Stores a pending snapshot for `key` (mirrors
    /// `impl_->pendingStates[key] = output` in `selectCandidate`).
    pub fn snapshot_put(
        &mut self,
        key: ContextKey,
        revision: u64,
        snapshot: snapshot::EngineSnapshot,
    ) {
        self.snapshot_store.put(key, revision, snapshot);
    }

    /// Takes the pending snapshot when the request revision is strictly older
    /// than the stored revision, removing the entry (mirrors
    /// `FcitxRuntime::takePendingState`).
    pub fn snapshot_take(
        &mut self,
        key: ContextKey,
        request_revision: u64,
    ) -> Option<snapshot::EngineSnapshot> {
        self.snapshot_store.take(key, request_revision)
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

// ---------------------------------------------------------------------------
// E3-3: surrounding-text and input-method-selection decisions
// ---------------------------------------------------------------------------

/// Surrounding-text action decided from request validity + current state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurroundingTextAction {
    Set,
    Invalidate,
}

/// Result of the surrounding-text decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurroundingTextDecision {
    pub action: SurroundingTextAction,
    /// Whether the caller must call `updateSurroundingText`.
    pub update: bool,
}

/// Decides the surrounding-text action, mirroring
/// `EngineInputContext::applySurroundingText` exactly:
///
/// ```cpp
/// if (request.surroundingTextValid) {
///     surroundingText().setText(...); surroundingTextValid_ = true; return true;
/// }
/// if (surroundingTextValid_) {
///     surroundingText().invalidate(); surroundingTextValid_ = false; return true;
/// }
/// surroundingText().invalidate(); return false;
/// ```
pub fn decide_surrounding_text(
    request_valid: bool,
    current_valid: bool,
) -> SurroundingTextDecision {
    if request_valid {
        SurroundingTextDecision {
            action: SurroundingTextAction::Set,
            update: true,
        }
    } else if current_valid {
        SurroundingTextDecision {
            action: SurroundingTextAction::Invalidate,
            update: true,
        }
    } else {
        SurroundingTextDecision {
            action: SurroundingTextAction::Invalidate,
            update: false,
        }
    }
}

/// Input-method selection decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputMethodSelection {
    /// Keep the current input method.
    NoChange,
    /// Activate `request.inputMethodUtf8` (it is a valid entry).
    SelectRequest,
    /// Activate the group default input method.
    SelectDefault,
}

/// Decides whether to switch the per-context input method, mirroring
/// `FcitxRuntime::processKey`:
///
/// ```cpp
/// const std::string selected =
///     !request.inputMethodUtf8.empty() && entry(request.inputMethodUtf8)
///         ? request.inputMethodUtf8 : group.defaultInputMethod();
/// if ((!overridden) && !selected.empty() && entry(selected) &&
///     inputMethod(&context) != selected) { switch to selected; }
/// ```
///
/// The C++ adapter passes the string-comparison and `entry()` facts so the
/// decision stays pure. `overridden` is the ledger input-method-override
/// marker (a user hotkey switch must survive the next keystroke).
pub fn decide_input_method_selection(
    has_request_im: bool,
    request_im_valid: bool,
    default_im_valid: bool,
    default_im_nonempty: bool,
    current_eq_request: bool,
    current_eq_default: bool,
    overridden: bool,
) -> InputMethodSelection {
    let use_request = has_request_im && request_im_valid;
    let selected_valid = if use_request {
        request_im_valid
    } else {
        default_im_valid
    };
    let selected_nonempty = if use_request {
        true
    } else {
        default_im_nonempty
    };
    let current_eq_selected = if use_request {
        current_eq_request
    } else {
        current_eq_default
    };
    if !overridden && selected_nonempty && selected_valid && !current_eq_selected {
        if use_request {
            InputMethodSelection::SelectRequest
        } else {
            InputMethodSelection::SelectDefault
        }
    } else {
        InputMethodSelection::NoChange
    }
}

// ---------------------------------------------------------------------------
// E3 event-shape consolidation: single EngineEvent -> EngineKeyDecision entry
//
// `handle_key_event` is the unified Event→Action entry for a key request. It
// composes the four product decisions (surrounding-text, input-method
// selection, input-method switch hotkey, candidate navigation) in the exact
// `FcitxRuntime::processKey` order and returns one `EngineKeyDecision` that
// the C++ adapter executes. The individual decision functions remain public
// and their C ABI stays available (corpus compatibility).
// ---------------------------------------------------------------------------

/// Flattened key-event facts for the unified decision.
pub struct EngineKeyEvent<'a> {
    pub key_sym: u32,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub plain_shortcut: bool,
    pub is_release: bool,
    pub hotkey_toggle: Option<&'a str>,
    pub hotkey_next: Option<&'a str>,
    pub surrounding_text_valid: bool,
    pub current_surrounding_valid: bool,
    pub has_request_im: bool,
    pub request_im_valid: bool,
    pub default_im_valid: bool,
    pub default_im_nonempty: bool,
    pub current_eq_request: bool,
    pub current_eq_default: bool,
    pub im_overridden: bool,
    pub has_candidates: bool,
    pub candidate_count: i32,
    pub candidate_list_size: i32,
    pub candidate_cursor: i32,
    pub candidate_bulk_cursor: i32,
    pub candidate_has_bulk_cursor: bool,
    pub candidate_has_bulk: bool,
    pub candidate_pageable: bool,
    pub candidate_has_prev: bool,
    pub candidate_has_next: bool,
    pub scroll_mode: bool,
    pub vertical: bool,
    pub candidate_page_size: Option<i32>,
    pub has_override: bool,
    pub override_value: u32,
}

/// Unified decision output for a key event. The adapter executes the
/// surrounding-text and input-method-selection actions first, then exactly one
/// main path: input-method switch, candidate navigation, or forward-to-Fcitx.
pub struct EngineKeyDecision {
    pub surrounding: SurroundingTextDecision,
    pub im_selection: InputMethodSelection,
    pub im_switch: Option<ImSwitchAction>,
    pub candidate: Option<navigation::CandidateDecision>,
    pub clear_override: bool,
    pub forward_key: bool,
}

/// Decides the full action set for a key request, mirroring
/// `FcitxRuntime::processKey` order:
///
/// 1. surrounding-text action (always);
/// 2. input-method selection (always);
/// 3. input-method switch hotkey (non-release) — if matched, that is the
///    main path and nothing else is decided;
/// 4. candidate navigation (non-release, candidates visible) — if it
///    consumes the event, that is the main path;
/// 5. otherwise forward to Fcitx, clearing the highlight override on an
///    ordinary non-modifier key.
pub fn handle_key_event(event: &EngineKeyEvent) -> EngineKeyDecision {
    let surrounding = decide_surrounding_text(
        event.surrounding_text_valid,
        event.current_surrounding_valid,
    );
    let im_selection = decide_input_method_selection(
        event.has_request_im,
        event.request_im_valid,
        event.default_im_valid,
        event.default_im_nonempty,
        event.current_eq_request,
        event.current_eq_default,
        event.im_overridden,
    );
    if !event.is_release {
        if let Some(im_switch) = classify_input_method_switch(
            event.ctrl,
            event.shift,
            event.alt,
            event.key_sym,
            event.hotkey_toggle,
            event.hotkey_next,
        ) {
            return EngineKeyDecision {
                surrounding,
                im_selection,
                im_switch: Some(im_switch),
                candidate: None,
                clear_override: false,
                forward_key: false,
            };
        }
        if event.has_candidates {
            let candidate = navigation::decide_candidate_action(
                event.key_sym,
                event.plain_shortcut,
                event.candidate_count,
                event.candidate_list_size,
                event.candidate_cursor,
                event.candidate_bulk_cursor,
                event.candidate_has_bulk_cursor,
                event.candidate_has_bulk,
                event.candidate_pageable,
                event.candidate_has_prev,
                event.candidate_has_next,
                event.scroll_mode,
                event.vertical,
                event.candidate_page_size,
                if event.has_override {
                    Some(event.override_value)
                } else {
                    None
                },
            );
            if candidate.consume {
                return EngineKeyDecision {
                    surrounding,
                    im_selection,
                    im_switch: None,
                    candidate: Some(candidate),
                    clear_override: false,
                    forward_key: false,
                };
            }
        }
    }
    let clear_override = !event.is_release
        && event.key_sym != KEY_SYM_SHIFT_L
        && event.key_sym != KEY_SYM_CONTROL_L
        && event.key_sym != KEY_SYM_ALT_L;
    EngineKeyDecision {
        surrounding,
        im_selection,
        im_switch: None,
        candidate: None,
        clear_override,
        forward_key: true,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

pub mod capi;
pub mod navigation;
pub mod session;
pub mod snapshot;

// ---------------------------------------------------------------------------
// E4: engine-process session epoch (start)
//
// The engine-process handshake epoch (`EngineEpoch`) is generated and
// validated by Rust; the C++ process shell only holds the value and compares
// it per frame. Generation (`FCITX5_RELEASE_GENERATION`) is a release
// platform attribute that stays in `windows-common-core`/platform scope.
// ---------------------------------------------------------------------------

/// Seconds between 1601-01-01 (FILETIME epoch) and 1970-01-01 (Unix epoch).
const FILETIME_UNIX_EPOCH_DELTA_SECONDS: u64 = 11_644_473_600;

/// Generates the engine-process session epoch.
///
/// Mirrors the C++ `GetSystemTimeAsFileTime`-derived value: a
/// 100-nanosecond-interval count since 1601-01-01 UTC. The engine process
/// generates this once at startup and rejects frames whose metadata epoch
/// does not match.
pub fn generate_engine_epoch() -> u64 {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    duration
        .as_secs()
        .saturating_add(FILETIME_UNIX_EPOCH_DELTA_SECONDS)
        .saturating_mul(10_000_000)
        .saturating_add(u64::from(duration.subsec_nanos()) / 100)
}

/// Validates a frame's engine epoch against the process epoch (mirrors
/// `frame.metadata.engineEpoch != engineEpoch` rejection).
pub fn validate_engine_epoch(frame_epoch: u64, process_epoch: u64) -> bool {
    frame_epoch == process_epoch
}

// ---------------------------------------------------------------------------
// E4-2: request-sequence and deadline policy
//
// The engine server's per-connection request ordering and key-request
// deadline policy are Rust-owned; `fcitx_engine_main.cpp` (`handleRequest`)
// only applies them. Client-side response-scalar validation already lives in
// `windows-common-core` (`fcitx5_windows_common_apply_*_response_scalars`).
// ---------------------------------------------------------------------------

/// Accepts a request frame when its id is strictly newer than the last
/// accepted id on the connection (mirrors
/// `frame.metadata.requestId <= lastRequestId` rejection).
pub fn accept_frame_sequence(request_id: u64, last_request_id: u64) -> bool {
    request_id > last_request_id
}

/// Key-request dispatcher deadline in milliseconds (mirrors the
/// `FcitxRuntime::processKey` timeout policy in `handleRequest`): a cold
/// context (revision 0) gets the widened first-context deadline, warm keys
/// the tight input deadline. The warm deadline intentionally matches the IPC
/// client hot-path bound so the engine dispatcher does not drop a valid input
/// request before the caller's bounded wait expires.
pub fn key_request_timeout_ms(revision: u64) -> u32 {
    if revision == 0 {
        7500
    } else {
        250
    }
}

// ---------------------------------------------------------------------------
// E5-1: snapshot/status canonicalization
//
// The content-locale and short-label canonicalization for engine snapshots
// and status are Rust-owned; `FcitxRuntime::Impl` (`collectResult`,
// `currentInputMethod`) applies them. Both mirror the frozen C++ helpers
// exactly (byte-level, tolerating arbitrary input).
// ---------------------------------------------------------------------------

/// Canonical content locale for an input-method id (mirrors
/// `contentLocaleForInputMethod`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentLocale {
    None,
    ZhCn,
    JaJp,
    KoKr,
    EnUs,
}

/// Maps an input-method id to the canonical content locale, mirroring
/// `contentLocaleForInputMethod` (substring checks in C++ order; the zh-CN
/// family is checked last).
pub fn content_locale_for_input_method(id: &str) -> ContentLocale {
    if id.contains("mozc") {
        ContentLocale::JaJp
    } else if id.contains("hangul") {
        ContentLocale::KoKr
    } else if id.contains("keyboard-us") {
        ContentLocale::EnUs
    } else if id.contains("rime") || id.contains("pinyin") || id.contains("libime") {
        ContentLocale::ZhCn
    } else {
        ContentLocale::None
    }
}

/// Byte length of the UTF-8 code point starting at `offset` (mirrors
/// `utf8CharacterEnd`; tolerates overlong encodings).
fn utf8_character_end(text: &[u8], offset: usize) -> usize {
    if offset >= text.len() {
        return text.len();
    }
    let byte = text[offset];
    let length = if byte & 0xe0 == 0xc0 {
        2
    } else if byte & 0xf0 == 0xe0 {
        3
    } else if byte & 0xf8 == 0xf0 {
        4
    } else {
        1
    };
    text.len().min(offset + length)
}

/// Canonical short label: two ASCII bytes when the text starts with two
/// ASCII bytes, otherwise the first code point (mirrors `statusShortLabel`).
pub fn status_short_label(text: &[u8]) -> &[u8] {
    if text.is_empty() {
        return &[];
    }
    if text.len() >= 2 && text[0] < 0x80 && text[1] < 0x80 {
        &text[..2]
    } else {
        &text[..utf8_character_end(text, 0)]
    }
}

// ---------------------------------------------------------------------------
// E5-2: typed EngineSnapshot DTO + limits validation
//
// The canonical engine snapshot DTO mirrors `RuntimeResult` (handled, commit,
// preedit, composition/revision, candidates, selection/page/bulk metadata,
// delete-surrounding, forward-key, caret, popup policy, content locale).
// `validate_snapshot` is the Rust-authoritative limits check (payload
// budgets); the C++ adapter feeds the DTO facts and fails closed on
// rejection. The `pendingStates` per-context snapshot store builds on this
// DTO next.
// ---------------------------------------------------------------------------

/// `protocol::kMaxCommitUtf8` / `kMaxPreeditUtf8`.
pub const MAX_SNAPSHOT_TEXT_UTF8: usize = 16 * 1024;
/// `protocol::kMaxCandidates`.
pub const MAX_SNAPSHOT_CANDIDATES: u32 = 128;
/// `protocol::kMaxCandidateFieldUtf8`.
pub const MAX_SNAPSHOT_CANDIDATE_FIELD_UTF8: usize = 4096;
/// `protocol::kMaxLocaleUtf8`.
pub const MAX_SNAPSHOT_LOCALE_UTF8: usize = 35;

/// Length facts of a canonical engine snapshot (payload budgets).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SnapshotFacts {
    pub commit_utf8_len: usize,
    pub preedit_utf8_len: usize,
    pub candidate_count: u32,
    pub candidate_label_len_max: usize,
    pub candidate_text_len_max: usize,
    pub candidate_comment_len_max: usize,
    pub content_locale_utf8_len: usize,
}

/// Validates snapshot payload budgets (mirrors the protocol limits the codec
/// enforces; the engine validates at snapshot construction so the store and
/// the wire share one authority).
pub fn validate_snapshot(facts: &SnapshotFacts) -> bool {
    facts.commit_utf8_len <= MAX_SNAPSHOT_TEXT_UTF8
        && facts.preedit_utf8_len <= MAX_SNAPSHOT_TEXT_UTF8
        && facts.candidate_count <= MAX_SNAPSHOT_CANDIDATES
        && facts.candidate_label_len_max <= MAX_SNAPSHOT_CANDIDATE_FIELD_UTF8
        && facts.candidate_text_len_max <= MAX_SNAPSHOT_CANDIDATE_FIELD_UTF8
        && facts.candidate_comment_len_max <= MAX_SNAPSHOT_CANDIDATE_FIELD_UTF8
        && facts.content_locale_utf8_len <= MAX_SNAPSHOT_LOCALE_UTF8
}
