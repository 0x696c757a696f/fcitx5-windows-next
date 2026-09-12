#![forbid(unsafe_code)]

//! Canonical engine snapshot DTO, flat ABI projection, and per-context
//! pending store (E5).
//!
//! The `pendingStates` cache (a full `RuntimeResult` published by
//! `selectCandidate` for `stateRequest` replay) is Rust-owned: snapshots are
//! stored per context as decoded [`EngineSnapshot`] values with their
//! revision. `take` succeeds only when the request revision is strictly
//! older than the stored revision, and removes the entry (mirrors
//! `FcitxRuntime::takePendingState`).
//!
//! 083: the previous hand-rolled C++ blob codec on the engine side was
//! deleted; the C++ adapter now marshals a flat self-contained
//! [`Fcitx5EngineSnapshotFlatC`] projection instead of a wire blob, so no
//! serialization format crosses the FFI edge at all.

use crate::ContextKey;
use std::collections::HashMap;

/// A candidate record in a canonical engine snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidate {
    pub id: u64,
    pub label: Vec<u8>,
    pub text: Vec<u8>,
    pub comment: Vec<u8>,
}

/// Canonical engine snapshot (mirrors `RuntimeResult`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineSnapshot {
    pub handled: bool,
    pub preedit_caret_utf8: u32,
    pub composition_id: u64,
    pub revision: u64,
    pub selected_candidate: u32,
    pub candidate_page: u32,
    pub candidate_total: u32,
    pub candidate_visibility: u8,
    pub candidate_page_size: u32,
    pub candidate_bulk: bool,
    pub candidate_end: bool,
    pub delete_surrounding_text: bool,
    pub delete_surrounding_offset: i32,
    pub delete_surrounding_size: u32,
    pub forward_key: bool,
    pub forward_key_sym: u32,
    pub forward_key_states: u32,
    pub forward_key_code: i32,
    pub forward_key_release: bool,
    pub caret_valid: bool,
    pub caret_left: i32,
    pub caret_top: i32,
    pub caret_right: i32,
    pub caret_bottom: i32,
    pub caret_dpi: u32,
    pub popup_allowed: bool,
    pub commit_utf8: Vec<u8>,
    pub preedit_utf8: Vec<u8>,
    pub content_locale_utf8: Vec<u8>,
    pub candidates: Vec<Candidate>,
}

#[derive(Default)]
/// Flat self-contained candidate record for the engine snapshot ABI.
/// String pointers reference caller-owned (arena) storage valid until the
/// documented contract expires ("until the next take-flat call or ledger
/// destroy" for take; "valid for the duration of the put call" for put).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Fcitx5EngineSnapshotRecordC {
    pub id: u64,
    pub label: *const u8,
    pub label_len: usize,
    pub text: *const u8,
    pub text_len: usize,
    pub comment: *const u8,
    pub comment_len: usize,
}

/// Flat self-contained engine snapshot ABI projection of
/// [`EngineSnapshot`] (mirrors the deleted C++ `RuntimeResult` wire
/// struct). Field order is a frozen ABI contract.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Fcitx5EngineSnapshotFlatC {
    pub handled: u8,
    pub preedit_caret_utf8: u32,
    pub composition_id: u64,
    pub revision: u64,
    pub selected_candidate: u32,
    pub candidate_page: u32,
    pub candidate_total: u32,
    pub candidate_visibility: u8,
    pub candidate_page_size: u32,
    pub candidate_bulk: u8,
    pub candidate_end: u8,
    pub delete_surrounding_text: u8,
    pub delete_surrounding_offset: i32,
    pub delete_surrounding_size: u32,
    pub forward_key: u8,
    pub forward_key_sym: u32,
    pub forward_key_states: u32,
    pub forward_key_code: i32,
    pub forward_key_release: u8,
    pub caret_valid: u8,
    pub caret_left: i32,
    pub caret_top: i32,
    pub caret_right: i32,
    pub caret_bottom: i32,
    pub caret_dpi: u32,
    pub popup_allowed: u8,
    pub commit: *const u8,
    pub commit_len: usize,
    pub preedit: *const u8,
    pub preedit_len: usize,
    pub content_locale: *const u8,
    pub content_locale_len: usize,
    pub candidates: *const Fcitx5EngineSnapshotRecordC,
    pub candidate_count: usize,
}

/// Owns the byte/record storage referenced by a [`Fcitx5EngineSnapshotFlatC`]
/// projection. The Rust ledger keeps one alive per `take_flat` response so
/// the returned pointers stay valid until the next `take_flat` call or the
/// ledger is destroyed.
#[derive(Default)]
pub struct FlatSnapshotArena {
    buffers: Vec<Vec<u8>>,
    records: Vec<Fcitx5EngineSnapshotRecordC>,
}

impl FlatSnapshotArena {
    /// Stored byte buffers, in insertion order (test observation surface).
    pub fn stored_buffers(&self) -> impl Iterator<Item = &[u8]> {
        self.buffers.iter().map(|buffer| buffer.as_slice())
    }

    fn push_bytes(&mut self, bytes: &[u8]) -> (*const u8, usize) {
        self.buffers.push(bytes.to_vec());
        let buffer = self.buffers.last().expect("just pushed");
        (buffer.as_ptr(), buffer.len())
    }

    /// Builds an arena plus the flat projection of `snapshot`. All returned
    /// pointers reference `arena`.
    pub fn build(snapshot: &EngineSnapshot) -> (Self, Fcitx5EngineSnapshotFlatC) {
        let mut arena = Self::default();
        let (commit, commit_len) = arena.push_bytes(&snapshot.commit_utf8);
        let (preedit, preedit_len) = arena.push_bytes(&snapshot.preedit_utf8);
        let (content_locale, content_locale_len) = arena.push_bytes(&snapshot.content_locale_utf8);
        arena.records = snapshot
            .candidates
            .iter()
            .map(|candidate| {
                let (label, label_len) = arena.push_bytes(&candidate.label);
                let (text, text_len) = arena.push_bytes(&candidate.text);
                let (comment, comment_len) = arena.push_bytes(&candidate.comment);
                Fcitx5EngineSnapshotRecordC {
                    id: candidate.id,
                    label,
                    label_len,
                    text,
                    text_len,
                    comment,
                    comment_len,
                }
            })
            .collect();
        let flat = Fcitx5EngineSnapshotFlatC {
            handled: u8::from(snapshot.handled),
            preedit_caret_utf8: snapshot.preedit_caret_utf8,
            composition_id: snapshot.composition_id,
            revision: snapshot.revision,
            selected_candidate: snapshot.selected_candidate,
            candidate_page: snapshot.candidate_page,
            candidate_total: snapshot.candidate_total,
            candidate_visibility: snapshot.candidate_visibility,
            candidate_page_size: snapshot.candidate_page_size,
            candidate_bulk: u8::from(snapshot.candidate_bulk),
            candidate_end: u8::from(snapshot.candidate_end),
            delete_surrounding_text: u8::from(snapshot.delete_surrounding_text),
            delete_surrounding_offset: snapshot.delete_surrounding_offset,
            delete_surrounding_size: snapshot.delete_surrounding_size,
            forward_key: u8::from(snapshot.forward_key),
            forward_key_sym: snapshot.forward_key_sym,
            forward_key_states: snapshot.forward_key_states,
            forward_key_code: snapshot.forward_key_code,
            forward_key_release: u8::from(snapshot.forward_key_release),
            caret_valid: u8::from(snapshot.caret_valid),
            caret_left: snapshot.caret_left,
            caret_top: snapshot.caret_top,
            caret_right: snapshot.caret_right,
            caret_bottom: snapshot.caret_bottom,
            caret_dpi: snapshot.caret_dpi,
            popup_allowed: u8::from(snapshot.popup_allowed),
            commit,
            commit_len,
            preedit,
            preedit_len,
            content_locale,
            content_locale_len,
            candidates: arena.records.as_ptr(),
            candidate_count: arena.records.len(),
        };
        (arena, flat)
    }
}

/// A flat take response plus the arena that owns its string storage. Held in
/// a box by the ledger so the returned pointers stay stable.
pub struct FlatTake {
    pub arena: FlatSnapshotArena,
    pub flat: Fcitx5EngineSnapshotFlatC,
}

/// One pending snapshot entry retained until a strictly newer `stateRequest`
/// replay consumes it.
#[derive(Clone, Debug)]
pub struct PendingSnapshot {
    pub revision: u64,
    pub snapshot: EngineSnapshot,
}

/// Per-context pending snapshot store. The authoritative representation is
/// the decoded [`EngineSnapshot`]; no wire format crosses the FFI edge.
#[derive(Default)]
pub struct SnapshotStore {
    entries: HashMap<ContextKey, PendingSnapshot>,
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Stores a snapshot under `key` (mirrors
    /// `impl_->pendingStates[key] = output`).
    pub fn put(&mut self, key: ContextKey, revision: u64, snapshot: EngineSnapshot) {
        self.entries
            .insert(key, PendingSnapshot { revision, snapshot });
    }

    /// Takes the pending snapshot when the request revision is strictly older
    /// than the stored revision, removing the entry (mirrors
    /// `FcitxRuntime::takePendingState`). Returns `None` when absent or stale.
    pub fn take(&mut self, key: ContextKey, request_revision: u64) -> Option<EngineSnapshot> {
        let entry = self.entries.get(&key)?;
        if request_revision >= entry.revision {
            return None;
        }
        self.entries.remove(&key).map(|entry| entry.snapshot)
    }

    /// Drops the pending snapshot for `key` (context erased).
    pub fn forget(&mut self, key: ContextKey) {
        self.entries.remove(&key);
    }

    /// Returns the stored snapshot for `key`, if any (non-destructive peek).
    pub fn entry_size(&self, key: ContextKey) -> Option<&EngineSnapshot> {
        self.entries.get(&key).map(|entry| &entry.snapshot)
    }
}
