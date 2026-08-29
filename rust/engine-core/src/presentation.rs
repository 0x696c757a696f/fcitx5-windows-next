#![forbid(unsafe_code)]

//! Latest-value publication policy for Engine presentation responses.

use std::time::Duration;

use crate::protocol::KeyResponse;

/// Next action selected by the presentation publication policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationPublicationAction<'a> {
    /// Wait for a response to publish or a stop request.
    Wait,
    /// Stop before handling any pending response.
    Stop,
    /// Deliver the current latest response.
    Deliver(&'a KeyResponse),
    /// Disconnect the native transport, then retry after this delay.
    DisconnectAndRetryAfter(Duration),
}

/// Owns the latest response awaiting presentation delivery.
///
/// Transport code borrows [`Self::pending`] to send a response and calls
/// [`Self::acknowledge_delivery`] only after that send succeeds. A newer
/// response replaces an older pending response, and acknowledgement clears
/// only the response identity handled by the C++ publisher.
#[derive(Default)]
pub struct PresentationPublicationQueue {
    latest: Option<KeyResponse>,
    stopping: bool,
}

impl PresentationPublicationQueue {
    /// Creates an empty presentation publication queue.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            latest: None,
            stopping: false,
        }
    }

    /// Replaces any pending response with the newest presentation response.
    pub fn publish(&mut self, response: KeyResponse) {
        self.latest = Some(response);
    }

    /// Returns the response currently awaiting delivery.
    #[must_use]
    pub fn pending(&self) -> Option<&KeyResponse> {
        self.latest.as_ref()
    }

    /// Requests shutdown; [`Self::next_action`] then returns stop before work.
    pub fn stop(&mut self) {
        self.stopping = true;
    }

    /// Returns the next native wait or delivery action.
    #[must_use]
    pub fn next_action(&self) -> PresentationPublicationAction<'_> {
        if self.stopping {
            PresentationPublicationAction::Stop
        } else if let Some(response) = self.latest.as_ref() {
            PresentationPublicationAction::Deliver(response)
        } else {
            PresentationPublicationAction::Wait
        }
    }

    /// Returns the fixed action after a failed native delivery.
    ///
    /// The queue retains its pending response so delivery can be retried.
    #[must_use]
    pub const fn delivery_failed() -> PresentationPublicationAction<'static> {
        PresentationPublicationAction::DisconnectAndRetryAfter(Duration::from_millis(25))
    }

    #[must_use]
    pub const fn delivery_failed_delay() -> Duration {
        Duration::from_millis(25)
    }

    /// Clears the pending response when a successful delivery matches it.
    ///
    /// Matching intentionally uses the C++ publisher's delivery identity,
    /// rather than the whole response payload.
    pub fn acknowledge_delivery(&mut self, delivered: &KeyResponse) -> bool {
        let matches_pending = self
            .latest
            .as_ref()
            .is_some_and(|pending| same_delivery_identity(pending, delivered));
        if matches_pending {
            self.latest = None;
        }
        matches_pending
    }
}

fn same_delivery_identity(left: &KeyResponse, right: &KeyResponse) -> bool {
    let left = left.metadata;
    let right = right.metadata;
    left.request_id == right.request_id
        && left.response_to == right.response_to
        && left.engine_epoch == right.engine_epoch
        && left.session_id == right.session_id
        && left.context_id == right.context_id
        && left.composition_id == right.composition_id
        && left.revision == right.revision
}
