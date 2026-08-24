use super::{ContextKey, ContextLedger, LedgerError};

fn key_a() -> ContextKey {
    ContextKey::new(100, 1, 7)
}

fn key_b() -> ContextKey {
    ContextKey::new(200, 2, 9)
}

#[test]
fn new_ledger_reports_zero_state() {
    let ledger = ContextLedger::new();
    assert_eq!(ledger.revision_of(key_a()), 0);
    assert_eq!(ledger.composition_of(key_a()), 0);
}

#[test]
fn begin_key_accepts_unknown_context_at_zero() {
    let ledger = ContextLedger::new();
    assert!(ledger.begin_key(key_a(), 0, 0).is_ok());
}

#[test]
fn begin_key_rejects_stale_metadata() {
    let ledger = ContextLedger::new();
    assert_eq!(
        ledger.begin_key(key_a(), 1, 0),
        Err(LedgerError::StaleState)
    );
    assert_eq!(
        ledger.begin_key(key_a(), 0, 1),
        Err(LedgerError::StaleState)
    );
}

#[test]
fn end_result_allocates_composition_on_first_content() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    assert_eq!(composition, 1);
    assert_eq!(revision, 1);
    assert_eq!(ledger.composition_of(key_a()), 1);
    assert_eq!(ledger.revision_of(key_a()), 1);
}

#[test]
fn end_result_keeps_composition_while_content_active() {
    let mut ledger = ContextLedger::new();
    let (first, _) = ledger.end_result(key_a(), true);
    assert_eq!(first, 1);
    let (second, revision) = ledger.end_result(key_a(), true);
    assert_eq!(second, 1);
    assert_eq!(revision, 2);
}

#[test]
fn end_result_resets_composition_without_content() {
    let mut ledger = ContextLedger::new();
    let (_, _) = ledger.end_result(key_a(), true);
    let (composition, revision) = ledger.end_result(key_a(), false);
    assert_eq!(composition, 0);
    assert_eq!(revision, 2);
    // Next content allocates a fresh composition id.
    let (composition, revision) = ledger.end_result(key_a(), true);
    assert_eq!(composition, 2);
    assert_eq!(revision, 3);
}

#[test]
fn end_result_increments_revision_even_without_content() {
    let mut ledger = ContextLedger::new();
    let (_, revision) = ledger.end_result(key_a(), false);
    assert_eq!(revision, 1);
    let (_, revision) = ledger.end_result(key_a(), false);
    assert_eq!(revision, 2);
}

#[test]
fn begin_key_follows_end_result_state() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    assert!(ledger.begin_key(key_a(), revision, composition).is_ok());
    // Pre-result metadata is now stale.
    assert_eq!(
        ledger.begin_key(key_a(), 0, 0),
        Err(LedgerError::StaleState)
    );
}

#[test]
fn select_candidate_accepts_encoded_candidate() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    // candidate id = (composition << 8) | (index + 1)
    let candidate_id = (composition << 8) | 1;
    assert!(ledger
        .select_candidate(key_a(), revision, composition, candidate_id)
        .is_ok());
}

#[test]
fn select_candidate_rejects_zero_id() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    assert_eq!(
        ledger.select_candidate(key_a(), revision, composition, 0),
        Err(LedgerError::InvalidCandidate)
    );
}

#[test]
fn select_candidate_rejects_wrong_composition_bits() {
    let mut ledger = ContextLedger::new();
    let (composition, revision) = ledger.end_result(key_a(), true);
    let wrong = (composition.wrapping_add(1) << 8) | 1;
    assert_eq!(
        ledger.select_candidate(key_a(), revision, composition, wrong),
        Err(LedgerError::InvalidCandidate)
    );
}

#[test]
fn select_candidate_rejects_stale_metadata() {
    let mut ledger = ContextLedger::new();
    let (composition, _) = ledger.end_result(key_a(), true);
    assert_eq!(
        ledger.select_candidate(key_a(), 0, composition, (composition << 8) | 1),
        Err(LedgerError::StaleState)
    );
}

#[test]
fn forget_resets_context_state() {
    let mut ledger = ContextLedger::new();
    let (_, _) = ledger.end_result(key_a(), true);
    ledger.forget(key_a());
    assert_eq!(ledger.revision_of(key_a()), 0);
    assert_eq!(ledger.composition_of(key_a()), 0);
    assert!(ledger.begin_key(key_a(), 0, 0).is_ok());
}

#[test]
fn contexts_are_isolated() {
    let mut ledger = ContextLedger::new();
    let (composition_a, revision_a) = ledger.end_result(key_a(), true);
    let (composition_b, revision_b) = ledger.end_result(key_b(), true);
    assert_eq!(composition_a, 1);
    assert_eq!(composition_b, 2);
    assert_eq!(revision_a, 1);
    assert_eq!(revision_b, 1);
    // key_a state does not affect key_b.
    assert!(ledger.begin_key(key_b(), revision_b, composition_b).is_ok());
}

#[test]
fn composition_allocation_wraps_and_skips_zero() {
    let mut ledger = ContextLedger::new();
    ledger.next_composition_id = u64::MAX;
    let (composition, _) = ledger.end_result(key_a(), true);
    assert_eq!(composition, u64::MAX);
    // Second allocation wraps: id 0 is reserved and skipped.
    let (composition, _) = ledger.end_result(key_a(), false);
    assert_eq!(composition, 0);
    let (composition, _) = ledger.end_result(key_a(), true);
    assert_eq!(composition, 1);
    assert_eq!(ledger.next_composition_id, 2);
}
