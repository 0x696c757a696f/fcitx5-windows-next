#![forbid(unsafe_code)]

use std::time::Duration;

use fcitx5_engine_core::{protocol, PresentationPublicationAction, PresentationPublicationQueue};

fn response(request_id: u64, engine_epoch: u64, revision: u64) -> protocol::KeyResponse {
    protocol::KeyResponse {
        metadata: protocol::Metadata {
            request_id,
            response_to: request_id - 1,
            engine_epoch,
            session_id: 41,
            context_id: 42,
            composition_id: 43,
            revision,
        },
        commit_utf8: vec![request_id as u8],
        ..protocol::KeyResponse::default()
    }
}

#[test]
fn publication_coalesces_to_the_latest_response() {
    let mut queue = PresentationPublicationQueue::new();
    let first = response(2, 7, 1);
    let latest = response(3, 7, 2);

    queue.publish(first);
    queue.publish(latest.clone());

    assert_eq!(queue.pending(), Some(&latest));
}

#[test]
fn successful_delivery_clears_the_matching_pending_response() {
    let mut queue = PresentationPublicationQueue::new();
    let pending = response(2, 7, 1);
    queue.publish(pending.clone());

    assert!(queue.acknowledge_delivery(&pending));
    assert_eq!(queue.pending(), None);
}

#[test]
fn failed_or_stale_delivery_leaves_the_latest_generation_pending() {
    let mut queue = PresentationPublicationQueue::new();
    let delivered = response(2, 7, 1);
    let latest = response(3, 8, 2);
    queue.publish(delivered.clone());
    queue.publish(latest.clone());

    assert!(!queue.acknowledge_delivery(&delivered));
    assert_eq!(queue.pending(), Some(&latest));
}

#[test]
fn acknowledgement_matches_the_cpp_delivery_identity_not_payload_contents() {
    let mut queue = PresentationPublicationQueue::new();
    let delivered = response(2, 7, 1);
    let mut newer_payload_same_identity = delivered.clone();
    newer_payload_same_identity.commit_utf8 = b"newer payload".to_vec();
    queue.publish(newer_payload_same_identity);

    assert!(queue.acknowledge_delivery(&delivered));
    assert_eq!(queue.pending(), None);
}

#[test]
fn stop_wins_over_pending_presentation_work() {
    let mut queue = PresentationPublicationQueue::new();
    queue.publish(response(2, 7, 1));
    queue.stop();

    assert_eq!(queue.next_action(), PresentationPublicationAction::Stop);
}

#[test]
fn failed_delivery_disconnects_retries_after_25_ms_and_retains_pending_response() {
    let mut queue = PresentationPublicationQueue::new();
    let pending = response(2, 7, 1);
    queue.publish(pending.clone());

    assert_eq!(
        PresentationPublicationQueue::delivery_failed(),
        PresentationPublicationAction::DisconnectAndRetryAfter(Duration::from_millis(25))
    );
    assert_eq!(
        queue.next_action(),
        PresentationPublicationAction::Deliver(&pending)
    );

    assert!(queue.acknowledge_delivery(&pending));
    assert_eq!(queue.next_action(), PresentationPublicationAction::Wait);
}
