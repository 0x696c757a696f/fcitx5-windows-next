//! The candidate model: validated semantic snapshots, freshness, privacy policy, accessibility notifications and their C ABI.

#![forbid(unsafe_code)]

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

impl CandidateModel {
    /// Applies an owned semantic snapshot and returns the stable FFI-compatible
    /// result code: 0 applied, 1 duplicate, 2 stale, 3 invalid.
    pub(crate) fn apply_semantic_snapshot(&mut self, snapshot: CandidateSemanticSnapshot) -> u32 {
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
        self.engine_epoch = 0;
        self.freshness.clear();
        self.freshness_order.clear();
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
