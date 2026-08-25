//! Per-connection engine session state (E4-3).
//!
//! The engine server's per-connection session (handshake completion and the
//! last accepted request id) is Rust-owned. `fcitx_engine_main.cpp`
//! (`handleRequest`) only applies the Rust session through the narrow C ABI;
//! the C++ `handshakeComplete`/`lastRequestId` locals are deleted.

use crate::{accept_frame_sequence, validate_engine_epoch};

/// Per-connection engine session state (mirrors the C++ locals
/// `handshakeComplete` and `lastRequestId` in `handleRequest`).
pub struct ConnectionSession {
    handshake_complete: bool,
    last_request_id: u64,
}

impl Default for ConnectionSession {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionSession {
    /// A fresh connection: handshake not complete, no request accepted yet.
    pub fn new() -> Self {
        Self {
            handshake_complete: false,
            last_request_id: 0,
        }
    }

    /// Hello handshake: accepts only when the connection has not already
    /// completed its handshake, the frame's session id matches the client
    /// identity's session id, the request's process id matches the verified
    /// client process id, and the request id is strictly newer than any
    /// accepted id. On success the session becomes handshake-complete and the
    /// request id is recorded (mirrors `handleRequest`'s hello branch
    /// exactly: rejection on `handshakeComplete`, `sessionId` mismatch or
    /// `clientProcessId` mismatch, then `handshakeComplete = true` and
    /// `lastRequestId = requestId`).
    pub fn begin_hello(
        &mut self,
        request_id: u64,
        frame_session_id: u64,
        client_session_id: u64,
        request_process_id: u32,
        client_process_id: u32,
    ) -> bool {
        if self.handshake_complete
            || !accept_frame_sequence(request_id, self.last_request_id)
            || frame_session_id != client_session_id
            || request_process_id != client_process_id
        {
            return false;
        }
        self.handshake_complete = true;
        self.last_request_id = request_id;
        true
    }

    /// Accepts a non-hello frame when the session is handshake-complete, the
    /// frame's epoch matches the process epoch, the frame's session id
    /// matches the client identity's session id, and the request id is
    /// strictly newer than the last accepted id (mirrors the
    /// `!handshakeComplete` early return plus the epoch/session frame checks
    /// and the ordering rejection in `handleRequest`).
    pub fn accept_frame(
        &self,
        request_id: u64,
        frame_session_id: u64,
        client_session_id: u64,
        frame_epoch: u64,
        engine_epoch: u64,
    ) -> bool {
        self.handshake_complete
            && validate_engine_epoch(frame_epoch, engine_epoch)
            && frame_session_id == client_session_id
            && accept_frame_sequence(request_id, self.last_request_id)
    }

    /// Records a successfully processed request id (mirrors
    /// `lastRequestId = request.metadata.requestId` after each handled
    /// request in `handleRequest`).
    pub fn complete_request(&mut self, request_id: u64) {
        self.last_request_id = request_id;
    }
}
