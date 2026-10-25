//! The candidate model: validated semantic snapshots, freshness, privacy policy, accessibility notifications and their C ABI.

#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Visibility {
    Hidden,
    Composition,
    Prediction,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CandidateItem {
    pub(crate) id: u64,
    pub(crate) label: Vec<u8>,
    pub(crate) text: Vec<u8>,
    pub(crate) comment: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CandidateSnapshot {
    pub(crate) engine_epoch: u64,
    pub(crate) context_id: u64,
    pub(crate) composition_id: u64,
    pub(crate) revision: u64,
    pub(crate) preedit: Vec<u8>,
    pub(crate) auxiliary_up: Vec<u8>,
    pub(crate) auxiliary_down: Vec<u8>,
    pub(crate) candidates: Vec<CandidateItem>,
    pub(crate) selected: Option<usize>,
    pub(crate) page: u32,
    pub(crate) total: u32,
    pub(crate) visibility: Visibility,
    pub(crate) popup_allowed: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Freshness {
    pub(crate) composition_id: u64,
    pub(crate) latest_composition_id: u64,
    pub(crate) revision: u64,
}

#[derive(Default)]
pub struct CandidateModel {
    pub(crate) current: Option<CandidateSnapshot>,
    pub(crate) ffi_items: Vec<Fcitx5CandidateModelItem>,
    pub(crate) engine_epoch: u64,
    pub(crate) freshness: HashMap<u64, Freshness>,
    pub(crate) freshness_order: VecDeque<u64>,
}

/// Identity attached to every candidate semantic snapshot and notification.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CandidateSnapshotIdentity {
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
}

/// Independent presentation capabilities. They intentionally compose instead of
/// selecting a mutually-exclusive accessibility mode.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CandidateCapabilities {
    pub keyboard: bool,
    pub uia: bool,
    pub narrator_nvda: bool,
    pub high_contrast: bool,
    pub large_text: bool,
    pub reduced_motion: bool,
    pub reduced_candidates: bool,
    pub stable_layout: bool,
}

/// Context classification used to enforce privacy at the semantic boundary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CandidatePrivacyContext {
    #[default]
    Normal,
    Password,
    Pin,
    Sensitive,
}

impl CandidatePrivacyContext {
    #[must_use]
    pub const fn suppress_text(self) -> bool {
        !matches!(self, Self::Normal)
    }

    #[must_use]
    pub const fn policy(self) -> CandidatePrivacyPolicy {
        if self.suppress_text() {
            CandidatePrivacyPolicy {
                allow_speech: false,
                allow_text_logging: false,
                allow_learning: false,
                allow_network: false,
            }
        } else {
            CandidatePrivacyPolicy {
                allow_speech: true,
                allow_text_logging: true,
                allow_learning: true,
                allow_network: true,
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidatePrivacyPolicy {
    pub(crate) allow_speech: bool,
    pub(crate) allow_text_logging: bool,
    pub(crate) allow_learning: bool,
    pub(crate) allow_network: bool,
}

impl CandidatePrivacyPolicy {
    #[must_use]
    pub const fn allows_speech(self) -> bool {
        self.allow_speech
    }

    #[must_use]
    pub const fn allows_text_logging(self) -> bool {
        self.allow_text_logging
    }

    #[must_use]
    pub const fn allows_learning(self) -> bool {
        self.allow_learning
    }

    #[must_use]
    pub const fn allows_network(self) -> bool {
        self.allow_network
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateSemanticItem {
    pub id: u64,
    pub label: String,
    pub text: String,
    pub comment: String,
}

/// Immutable semantic projection shared by renderer, UIA, and notifications.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateSemanticSnapshot {
    pub identity: CandidateSnapshotIdentity,
    pub preedit: String,
    pub auxiliary_up: String,
    pub auxiliary_down: String,
    pub candidates: Vec<CandidateSemanticItem>,
    pub selected: Option<usize>,
    pub page: u32,
    pub total: u32,
    pub visibility: u8,
    pub popup_allowed: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateNotificationKind {
    Snapshot,
    Selection,
    Count,
    State,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateNotification {
    pub identity: CandidateSnapshotIdentity,
    pub kind: CandidateNotificationKind,
    pub selected: Option<usize>,
    pub count: usize,
    pub visibility: u8,
    pub text: Option<String>,
}

/// Deterministic revision-aware notification buffer.
#[derive(Clone, Debug, Default)]
pub struct CandidateNotificationQueue {
    pending: VecDeque<CandidateNotification>,
    latest: HashMap<(u64, u64), CandidateNotificationState>,
    cancelled: Vec<CandidateSnapshotIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CandidateNotificationState {
    pub(crate) identity: CandidateSnapshotIdentity,
    pub(crate) selected: Option<usize>,
    pub(crate) selected_text: Option<String>,
    pub(crate) count: usize,
    pub(crate) visibility: u8,
}

impl CandidateNotificationQueue {
    pub fn enqueue(
        &mut self,
        snapshot: &CandidateSemanticSnapshot,
        capabilities: CandidateCapabilities,
        privacy: CandidatePrivacyContext,
    ) {
        let identity = snapshot.identity;
        let scope = (identity.engine_epoch, identity.context_id);
        if self.latest.get(&scope).is_some_and(|latest| {
            identity.composition_id < latest.identity.composition_id
                || (identity.composition_id == latest.identity.composition_id
                    && identity.revision <= latest.identity.revision)
        }) {
            return;
        }
        let policy = privacy.policy();
        let selected_text = if policy.allows_speech() {
            snapshot
                .selected
                .and_then(|index| snapshot.candidates.get(index))
                .map(|candidate| candidate.text.clone())
        } else {
            None
        };
        let state = CandidateNotificationState {
            identity,
            selected: snapshot.selected,
            selected_text,
            count: snapshot.total as usize,
            visibility: snapshot.visibility,
        };
        let kinds = match self.latest.get(&scope) {
            None => vec![CandidateNotificationKind::Snapshot],
            Some(previous) => {
                let mut kinds = Vec::with_capacity(3);
                if previous.selected != state.selected
                    || previous.selected_text != state.selected_text
                {
                    kinds.push(CandidateNotificationKind::Selection);
                }
                if previous.count != state.count {
                    kinds.push(CandidateNotificationKind::Count);
                }
                if previous.visibility != state.visibility {
                    kinds.push(CandidateNotificationKind::State);
                }
                kinds
            }
        };
        self.pending.retain(|item| {
            let item_scope = (item.identity.engine_epoch, item.identity.context_id);
            item_scope != scope
        });
        for kind in kinds {
            self.pending.push_back(CandidateNotification {
                identity,
                kind,
                selected: state.selected,
                count: state.count,
                visibility: state.visibility,
                text: (capabilities.narrator_nvda
                    && policy.allows_speech()
                    && matches!(
                        kind,
                        CandidateNotificationKind::Snapshot | CandidateNotificationKind::Selection
                    ))
                .then(|| state.selected_text.clone())
                .flatten(),
            });
        }
        self.cancelled
            .retain(|cancelled| (cancelled.engine_epoch, cancelled.context_id) != scope);
        self.latest.insert(scope, state);
    }

    pub fn cancel(&mut self, identity: CandidateSnapshotIdentity) {
        self.cancelled.push(identity);
        self.pending.retain(|item| item.identity != identity);
    }

    #[must_use]
    pub fn drain_for(&mut self, identity: CandidateSnapshotIdentity) -> Vec<CandidateNotification> {
        if self.cancelled.contains(&identity)
            || self
                .latest
                .get(&(identity.engine_epoch, identity.context_id))
                .is_none_or(|latest| latest.identity != identity)
        {
            return Vec::new();
        }
        let mut notifications = Vec::new();
        self.pending.retain(|item| {
            if item.identity == identity {
                notifications.push(item.clone());
                false
            } else {
                true
            }
        });
        notifications
    }
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_model_create() -> *mut c_void {
    Box::into_raw(Box::<CandidateModel>::default()) as *mut c_void
}

#[no_mangle]
/// # Safety
///
/// `model` must be either null or a pointer returned by
/// `fcitx5_candidate_model_create` that has not already been destroyed.
pub unsafe extern "C" fn fcitx5_candidate_model_destroy(model: *mut c_void) {
    if model.is_null() {
        return;
    }
    // SAFETY: the API contract makes this a unique allocation from the matching create export.
    drop(unsafe { Box::from_raw(model.cast::<CandidateModel>()) });
}

#[no_mangle]
/// # Safety
///
/// `model` must be either null or a valid pointer returned by
/// `fcitx5_candidate_model_create`.
pub unsafe extern "C" fn fcitx5_candidate_model_reset(model: *mut c_void) {
    if model.is_null() {
        return;
    }
    // SAFETY: the API contract supplies a live uniquely borrowed CandidateModel allocation.
    let model = unsafe { &mut *model.cast::<CandidateModel>() };
    model.reset();
}

#[no_mangle]
/// # Safety
///
/// `snapshot` and the candidate/string buffers it references must be valid for
/// the duration of this call. Pointers are not retained.
pub unsafe extern "C" fn fcitx5_candidate_model_validate(
    snapshot: *const Fcitx5CandidateModelSnapshot,
) -> u8 {
    // SAFETY: the API contract keeps the snapshot and all nested spans valid for this call.
    let Some(snapshot) = (unsafe { snapshot_from_ffi(snapshot) }) else {
        return 0;
    };
    u8::from(validate_snapshot(&snapshot))
}

#[no_mangle]
/// # Safety
///
/// `model` must be a valid pointer returned by `fcitx5_candidate_model_create`.
/// `snapshot` and the buffers it references must remain valid for the duration
/// of this call. Pointers are not retained.
pub unsafe extern "C" fn fcitx5_candidate_model_apply(
    model: *mut c_void,
    snapshot: *const Fcitx5CandidateModelSnapshot,
) -> u32 {
    if model.is_null() {
        return 3;
    }
    // SAFETY: the API contract keeps the snapshot and all nested spans valid for this call.
    let Some(snapshot) = (unsafe { snapshot_from_ffi(snapshot) }) else {
        return 3;
    };
    // SAFETY: the API contract supplies a live uniquely borrowed CandidateModel allocation.
    let model = unsafe { &mut *model.cast::<CandidateModel>() };
    model.apply(snapshot)
}

#[no_mangle]
/// # Safety
///
/// `model` must be a valid pointer returned by `fcitx5_candidate_model_create`.
/// `output` must point to writable storage for one snapshot. Its string and item
/// pointers remain valid until the next model mutation or destruction.
pub unsafe extern "C" fn fcitx5_candidate_model_current(
    model: *mut c_void,
    output: *mut Fcitx5CandidateModelSnapshot,
) -> u8 {
    if model.is_null() || output.is_null() {
        return 0;
    }
    // SAFETY: the API contract supplies a live CandidateModel allocation for this read.
    let model = unsafe { &mut *model.cast::<CandidateModel>() };
    let Some(snapshot) = model.current.as_ref() else {
        return 0;
    };
    let visibility = match snapshot.visibility {
        Visibility::Hidden => 0,
        Visibility::Composition => 1,
        Visibility::Prediction => 2,
    };
    // SAFETY: the API contract supplies writable output storage for one snapshot.
    unsafe {
        *output = Fcitx5CandidateModelSnapshot {
            engine_epoch: snapshot.engine_epoch,
            context_id: snapshot.context_id,
            composition_id: snapshot.composition_id,
            revision: snapshot.revision,
            preedit: ffi_utf8(&snapshot.preedit),
            auxiliary_up: ffi_utf8(&snapshot.auxiliary_up),
            auxiliary_down: ffi_utf8(&snapshot.auxiliary_down),
            candidates: model.ffi_items.as_ptr(),
            candidate_count: model.ffi_items.len(),
            selected: snapshot.selected.unwrap_or(0),
            has_selected: u8::from(snapshot.selected.is_some()),
            page: snapshot.page,
            total: snapshot.total,
            visibility,
            popup_allowed: u8::from(snapshot.popup_allowed),
        };
    }
    1
}

pub(crate) fn ffi_utf8(value: &[u8]) -> Fcitx5CandidateUtf8 {
    Fcitx5CandidateUtf8 {
        ptr: value.as_ptr(),
        len: value.len(),
    }
}

impl CandidateModel {
    /// Applies an owned semantic snapshot and returns the stable FFI-compatible
    /// result code: 0 applied, 1 duplicate, 2 stale, 3 invalid.
    pub fn apply_semantic_snapshot(&mut self, snapshot: CandidateSemanticSnapshot) -> u32 {
        let visibility = match snapshot.visibility {
            0 => Visibility::Hidden,
            1 => Visibility::Composition,
            2 => Visibility::Prediction,
            _ => return 3,
        };
        self.apply(CandidateSnapshot {
            engine_epoch: snapshot.identity.engine_epoch,
            context_id: snapshot.identity.context_id,
            composition_id: snapshot.identity.composition_id,
            revision: snapshot.identity.revision,
            preedit: snapshot.preedit.into_bytes(),
            auxiliary_up: snapshot.auxiliary_up.into_bytes(),
            auxiliary_down: snapshot.auxiliary_down.into_bytes(),
            candidates: snapshot
                .candidates
                .into_iter()
                .map(|item| CandidateItem {
                    id: item.id,
                    label: item.label.into_bytes(),
                    text: item.text.into_bytes(),
                    comment: item.comment.into_bytes(),
                })
                .collect(),
            selected: snapshot.selected,
            page: snapshot.page,
            total: snapshot.total,
            visibility,
            popup_allowed: snapshot.popup_allowed,
        })
    }

    #[must_use]
    pub fn semantic_snapshot(&self) -> Option<CandidateSemanticSnapshot> {
        self.current
            .as_ref()
            .map(|snapshot| CandidateSemanticSnapshot {
                identity: CandidateSnapshotIdentity {
                    engine_epoch: snapshot.engine_epoch,
                    context_id: snapshot.context_id,
                    composition_id: snapshot.composition_id,
                    revision: snapshot.revision,
                },
                preedit: String::from_utf8_lossy(&snapshot.preedit).into_owned(),
                auxiliary_up: String::from_utf8_lossy(&snapshot.auxiliary_up).into_owned(),
                auxiliary_down: String::from_utf8_lossy(&snapshot.auxiliary_down).into_owned(),
                candidates: snapshot
                    .candidates
                    .iter()
                    .map(|item| CandidateSemanticItem {
                        id: item.id,
                        label: String::from_utf8_lossy(&item.label).into_owned(),
                        text: String::from_utf8_lossy(&item.text).into_owned(),
                        comment: String::from_utf8_lossy(&item.comment).into_owned(),
                    })
                    .collect(),
                selected: snapshot.selected,
                page: snapshot.page,
                total: snapshot.total,
                visibility: match snapshot.visibility {
                    Visibility::Hidden => 0,
                    Visibility::Composition => 1,
                    Visibility::Prediction => 2,
                },
                popup_allowed: snapshot.popup_allowed,
            })
    }

    pub(crate) fn apply(&mut self, snapshot: CandidateSnapshot) -> u32 {
        if !validate_snapshot(&snapshot) {
            return 3;
        }
        if self.engine_epoch != 0 && snapshot.engine_epoch < self.engine_epoch {
            return 2;
        }
        if self.engine_epoch == 0 || snapshot.engine_epoch > self.engine_epoch {
            self.engine_epoch = snapshot.engine_epoch;
            self.current = None;
            self.freshness.clear();
            self.freshness_order.clear();
        }
        if let Some(freshness) = self.freshness.get(&snapshot.context_id) {
            if snapshot.composition_id == freshness.composition_id
                && snapshot.revision == freshness.revision
            {
                return if self.current.as_ref() == Some(&snapshot) {
                    1
                } else {
                    2
                };
            }
            if snapshot.composition_id == freshness.composition_id || snapshot.composition_id == 0 {
                if snapshot.revision < freshness.revision {
                    return 2;
                }
            } else if snapshot.composition_id <= freshness.latest_composition_id {
                return 2;
            }
        }
        self.remember_context(snapshot.context_id, &snapshot);
        self.current = Some(snapshot);
        self.refresh_ffi_items();
        0
    }

    pub(crate) fn remember_context(&mut self, context_id: u64, snapshot: &CandidateSnapshot) {
        let inserted = !self.freshness.contains_key(&context_id);
        if inserted {
            self.freshness_order.push_back(context_id);
            while self.freshness.len() > MAX_TRACKED_CONTEXTS {
                let Some(evicted) = self.freshness_order.pop_front() else {
                    break;
                };
                if evicted != context_id {
                    self.freshness.remove(&evicted);
                }
            }
        }
        let freshness = self.freshness.entry(context_id).or_default();
        freshness.composition_id = snapshot.composition_id;
        if snapshot.composition_id != 0 && snapshot.composition_id > freshness.latest_composition_id
        {
            freshness.latest_composition_id = snapshot.composition_id;
        }
        freshness.revision = snapshot.revision;
    }

    pub fn reset(&mut self) {
        self.current = None;
        self.ffi_items.clear();
        self.engine_epoch = 0;
        self.freshness.clear();
        self.freshness_order.clear();
    }

    pub(crate) fn refresh_ffi_items(&mut self) {
        self.ffi_items = self
            .current
            .as_ref()
            .map(|snapshot| {
                snapshot
                    .candidates
                    .iter()
                    .map(|item| Fcitx5CandidateModelItem {
                        id: item.id,
                        label: ffi_utf8(&item.label),
                        text: ffi_utf8(&item.text),
                        comment: ffi_utf8(&item.comment),
                    })
                    .collect()
            })
            .unwrap_or_default();
    }
}

pub(crate) fn validate_snapshot(snapshot: &CandidateSnapshot) -> bool {
    if snapshot.engine_epoch == 0
        || snapshot.context_id == 0
        || snapshot.revision == 0
        || snapshot.candidates.len() > MAX_CANDIDATES
        || !valid_text(&snapshot.preedit)
        || !valid_text(&snapshot.auxiliary_up)
        || !valid_text(&snapshot.auxiliary_down)
    {
        return false;
    }
    if snapshot
        .selected
        .is_some_and(|selected| selected >= snapshot.candidates.len())
    {
        return false;
    }
    if snapshot.total < snapshot.candidates.len() as u32 {
        return false;
    }
    if snapshot.visibility == Visibility::Hidden && !snapshot.candidates.is_empty() {
        return false;
    }
    if snapshot.visibility != Visibility::Hidden && snapshot.composition_id == 0 {
        return false;
    }
    snapshot.candidates.iter().all(|item| {
        item.id != 0
            && valid_text(&item.label)
            && valid_text(&item.text)
            && valid_text(&item.comment)
    })
}

pub(crate) fn valid_text(value: &[u8]) -> bool {
    value.len() <= MAX_CANDIDATE_TEXT_UTF8
        && !value.contains(&0)
        && std::str::from_utf8(value).is_ok()
}

// SAFETY: the caller keeps `snapshot`, its candidate array, and every nested FFI string span
// valid and immutable for this conversion; the returned model owns copied bytes only.
pub(crate) unsafe fn snapshot_from_ffi(
    snapshot: *const Fcitx5CandidateModelSnapshot,
) -> Option<CandidateSnapshot> {
    if snapshot.is_null() {
        return None;
    }
    // SAFETY: the non-null `snapshot` pointer is valid and aligned for one readable snapshot.
    let snapshot = unsafe { *snapshot };
    if snapshot.candidate_count > 0 && snapshot.candidates.is_null() {
        return None;
    }
    let candidates = if snapshot.candidate_count == 0 {
        &[]
    } else {
        // SAFETY: the caller supplies `candidate_count` contiguous readable candidate records.
        unsafe { std::slice::from_raw_parts(snapshot.candidates, snapshot.candidate_count) }
    };
    let visibility = match snapshot.visibility {
        0 => Visibility::Hidden,
        1 => Visibility::Composition,
        2 => Visibility::Prediction,
        _ => return None,
    };
    let mut owned_candidates = Vec::with_capacity(candidates.len());
    for item in candidates {
        owned_candidates.push(CandidateItem {
            id: item.id,
            // SAFETY: the outer snapshot contract keeps this label span readable during the copy.
            label: unsafe { bytes_from_ffi(item.label) }?.to_vec(),
            // SAFETY: the outer snapshot contract keeps this text span readable during the copy.
            text: unsafe { bytes_from_ffi(item.text) }?.to_vec(),
            // SAFETY: the outer snapshot contract keeps this comment span readable during the copy.
            comment: unsafe { bytes_from_ffi(item.comment) }?.to_vec(),
        });
    }
    Some(CandidateSnapshot {
        engine_epoch: snapshot.engine_epoch,
        context_id: snapshot.context_id,
        composition_id: snapshot.composition_id,
        revision: snapshot.revision,
        // SAFETY: the outer snapshot contract keeps this preedit span readable during the copy.
        preedit: unsafe { bytes_from_ffi(snapshot.preedit) }?.to_vec(),
        // SAFETY: the outer snapshot contract keeps this upper auxiliary span readable during the copy.
        auxiliary_up: unsafe { bytes_from_ffi(snapshot.auxiliary_up) }?.to_vec(),
        // SAFETY: the outer snapshot contract keeps this lower auxiliary span readable during the copy.
        auxiliary_down: unsafe { bytes_from_ffi(snapshot.auxiliary_down) }?.to_vec(),
        candidates: owned_candidates,
        selected: (snapshot.has_selected != 0).then_some(snapshot.selected),
        page: snapshot.page,
        total: snapshot.total,
        visibility,
        popup_allowed: snapshot.popup_allowed != 0,
    })
}
