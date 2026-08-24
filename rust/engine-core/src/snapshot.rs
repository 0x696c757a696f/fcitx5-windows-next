//! Canonical engine snapshot blob codec and per-context pending store (E5).
//!
//! The `pendingStates` cache (a full `RuntimeResult` published by
//! `selectCandidate` for `stateRequest` replay) is now Rust-owned: the
//! canonical snapshot is serialized through the Rust blob codec below and
//! stored per context with its revision. `take` succeeds only when the
//! request revision is strictly older than the stored revision, and removes
//! the entry (mirrors `FcitxRuntime::takePendingState`).
//!
//! The blob is a transport/serialization format only; the authoritative
//! representation inside the store is the decoded `EngineSnapshot`.

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

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_i32(out: &mut Vec<u8>, value: i32) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

fn push_bytes(out: &mut Vec<u8>, bytes: &[u8]) {
    push_u32(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

/// Encodes a canonical snapshot into the Rust-authoritative blob format.
pub fn encode_snapshot(snapshot: &EngineSnapshot) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(snapshot.handled as u8);
    push_u32(&mut out, snapshot.preedit_caret_utf8);
    push_u64(&mut out, snapshot.composition_id);
    push_u64(&mut out, snapshot.revision);
    push_u32(&mut out, snapshot.selected_candidate);
    push_u32(&mut out, snapshot.candidate_page);
    push_u32(&mut out, snapshot.candidate_total);
    out.push(snapshot.candidate_visibility);
    push_u32(&mut out, snapshot.candidate_page_size);
    out.push(snapshot.candidate_bulk as u8);
    out.push(snapshot.candidate_end as u8);
    out.push(snapshot.delete_surrounding_text as u8);
    push_i32(&mut out, snapshot.delete_surrounding_offset);
    push_u32(&mut out, snapshot.delete_surrounding_size);
    out.push(snapshot.forward_key as u8);
    push_u32(&mut out, snapshot.forward_key_sym);
    push_u32(&mut out, snapshot.forward_key_states);
    push_i32(&mut out, snapshot.forward_key_code);
    out.push(snapshot.forward_key_release as u8);
    out.push(snapshot.caret_valid as u8);
    push_i32(&mut out, snapshot.caret_left);
    push_i32(&mut out, snapshot.caret_top);
    push_i32(&mut out, snapshot.caret_right);
    push_i32(&mut out, snapshot.caret_bottom);
    push_u32(&mut out, snapshot.caret_dpi);
    out.push(snapshot.popup_allowed as u8);
    push_bytes(&mut out, &snapshot.commit_utf8);
    push_bytes(&mut out, &snapshot.preedit_utf8);
    push_bytes(&mut out, &snapshot.content_locale_utf8);
    push_u32(&mut out, snapshot.candidates.len() as u32);
    for candidate in &snapshot.candidates {
        push_u64(&mut out, candidate.id);
        push_bytes(&mut out, &candidate.label);
        push_bytes(&mut out, &candidate.text);
        push_bytes(&mut out, &candidate.comment);
    }
    out
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let end = self.offset.checked_add(count)?;
        if end > self.bytes.len() {
            return None;
        }
        let slice = &self.bytes[self.offset..end];
        self.offset = end;
        Some(slice)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn bytes_len_prefixed(&mut self) -> Option<Vec<u8>> {
        let length = self.u32()? as usize;
        Some(self.take(length)?.to_vec())
    }
}

/// Decodes a canonical snapshot blob. Returns `None` on malformed input
/// (truncated, oversized lengths, candidate count over the limit).
pub fn decode_snapshot(bytes: &[u8]) -> Option<EngineSnapshot> {
    let mut reader = Reader { bytes, offset: 0 };
    let handled = reader.u8()?;
    let preedit_caret_utf8 = reader.u32()?;
    let composition_id = reader.u64()?;
    let revision = reader.u64()?;
    let selected_candidate = reader.u32()?;
    let candidate_page = reader.u32()?;
    let candidate_total = reader.u32()?;
    let candidate_visibility = reader.u8()?;
    let candidate_page_size = reader.u32()?;
    let candidate_bulk = reader.u8()?;
    let candidate_end = reader.u8()?;
    let delete_surrounding_text = reader.u8()?;
    let delete_surrounding_offset = reader.i32()?;
    let delete_surrounding_size = reader.u32()?;
    let forward_key = reader.u8()?;
    let forward_key_sym = reader.u32()?;
    let forward_key_states = reader.u32()?;
    let forward_key_code = reader.i32()?;
    let forward_key_release = reader.u8()?;
    let caret_valid = reader.u8()?;
    let caret_left = reader.i32()?;
    let caret_top = reader.i32()?;
    let caret_right = reader.i32()?;
    let caret_bottom = reader.i32()?;
    let caret_dpi = reader.u32()?;
    let popup_allowed = reader.u8()?;
    let commit_utf8 = reader.bytes_len_prefixed()?;
    let preedit_utf8 = reader.bytes_len_prefixed()?;
    let content_locale_utf8 = reader.bytes_len_prefixed()?;
    let candidate_count = reader.u32()?;
    if candidate_count > crate::MAX_SNAPSHOT_CANDIDATES {
        return None;
    }
    let mut candidates = Vec::with_capacity(candidate_count as usize);
    for _ in 0..candidate_count {
        let id = reader.u64()?;
        let label = reader.bytes_len_prefixed()?;
        let text = reader.bytes_len_prefixed()?;
        let comment = reader.bytes_len_prefixed()?;
        candidates.push(Candidate {
            id,
            label,
            text,
            comment,
        });
    }
    if reader.offset != bytes.len() {
        return None;
    }
    Some(EngineSnapshot {
        handled: handled != 0,
        preedit_caret_utf8,
        composition_id,
        revision,
        selected_candidate,
        candidate_page,
        candidate_total,
        candidate_visibility,
        candidate_page_size,
        candidate_bulk: candidate_bulk != 0,
        candidate_end: candidate_end != 0,
        delete_surrounding_text: delete_surrounding_text != 0,
        delete_surrounding_offset,
        delete_surrounding_size,
        forward_key: forward_key != 0,
        forward_key_sym,
        forward_key_states,
        forward_key_code,
        forward_key_release: forward_key_release != 0,
        caret_valid: caret_valid != 0,
        caret_left,
        caret_top,
        caret_right,
        caret_bottom,
        caret_dpi,
        popup_allowed: popup_allowed != 0,
        commit_utf8,
        preedit_utf8,
        content_locale_utf8,
        candidates,
    })
}

/// A pending snapshot entry (mirrors `impl_->pendingStates[key]`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingSnapshot {
    pub revision: u64,
    pub snapshot: EngineSnapshot,
}

/// Per-context pending snapshot store (the `pendingStates` cache).
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
