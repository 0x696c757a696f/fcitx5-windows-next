#![forbid(unsafe_code)]

//! Rust-owned candidate-select client used by the candidate UI host
//! (`fcitx5-ui`). Connect, peer verification, handshake and candidate-select
//! state live here; the wire codec is `fcitx5-protocol-core`.

pub mod safe {
    use crate::{
        close_pipe_client, invalid_handle_value, open_pipe_client, pipe_transact,
        verify_pipe_server_peer, CurrentUserRuntimeIdentity,
    };
    use std::ffi::c_void;

    /// Exact-executable peer policy (mirrors `PeerVerificationMode::exactExecutable`).
    const POLICY_EXACT_EXECUTABLE: u32 = 0;
    /// The input hot path deadline used by the previous C++ candidate client.
    const INPUT_DEADLINE_MILLISECONDS: u32 = 250;
    const IPC_HEADER_SIZE: usize = 64;

    /// Rust-owned candidate-select client. The handle is the `*mut c_void`
    /// boxed pointer to this struct, exposed to C++ through the shims.
    pub struct CandidateSelectClient {
        pipe_name: Vec<u16>,
        expected_peer_path: Vec<u16>,
        identity: Option<CurrentUserRuntimeIdentity>,
        pipe: *mut c_void,
        handshake_complete: bool,
        engine_epoch: u64,
    }

    impl CandidateSelectClient {
        pub fn new(pipe_name: Vec<u16>, expected_peer_path: Vec<u16>) -> Option<Self> {
            if pipe_name.is_empty() || expected_peer_path.is_empty() {
                return None;
            }
            Some(Self {
                pipe_name,
                expected_peer_path,
                identity: CurrentUserRuntimeIdentity::current(),
                pipe: invalid_handle_value(),
                handshake_complete: false,
                engine_epoch: 0,
            })
        }

        fn disconnect(&mut self) {
            if self.pipe != invalid_handle_value() {
                close_pipe_client(self.pipe);
                self.pipe = invalid_handle_value();
            }
            self.handshake_complete = false;
            self.engine_epoch = 0;
        }

        fn connect(&mut self, deadline: u64) -> bool {
            if self.pipe != invalid_handle_value() {
                return true;
            }
            let Some(identity) = &self.identity else {
                return false;
            };
            if self.pipe_name.is_empty() {
                return false;
            }
            let pipe = open_pipe_client(&self.pipe_name, deadline, true);
            if pipe == invalid_handle_value() {
                return false;
            }
            let verified = verify_pipe_server_peer(
                pipe,
                identity.service_account,
                identity.session_id,
                identity.secure_desktop,
                &identity.user_sid,
                POLICY_EXACT_EXECUTABLE,
                &self.expected_peer_path,
                false,
            );
            if !verified {
                close_pipe_client(pipe);
                return false;
            }
            self.pipe = pipe;
            true
        }

        fn transact(&mut self, request: Vec<u8>, deadline: u64) -> Option<Vec<u8>> {
            if request.is_empty() || request.len() > 256 * 1024 {
                self.disconnect();
                return None;
            }
            let mut response = vec![0_u8; 256 * 1024];
            let transferred = pipe_transact(
                self.pipe,
                &request,
                response.as_mut_ptr(),
                response.len(),
                deadline,
            );
            if transferred.status == 0
                || transferred.response_len < IPC_HEADER_SIZE
                || transferred.response_len > response.len()
            {
                self.disconnect();
                return None;
            }
            response.truncate(transferred.response_len);
            Some(response)
        }

        fn handshake(&mut self, deadline: u64) -> bool {
            if self.handshake_complete {
                return true;
            }
            let session_id = match self.identity.as_ref() {
                Some(identity) if identity.session_id != 0 => identity.session_id,
                _ => return false,
            };
            let request_id = crate::next_pipe_client_request_id();
            let request = fcitx5_protocol_core::HelloRequest {
                metadata: fcitx5_protocol_core::Metadata {
                    request_id,
                    session_id,
                    ..Default::default()
                },
                client_architecture_bits: (std::mem::size_of::<usize>() * 8) as u32,
                client_process_id: std::process::id(),
            };
            let Some(request_bytes) = fcitx5_protocol_core::encode_hello_request(&request) else {
                self.disconnect();
                return false;
            };
            let Some(response_bytes) = self.transact(request_bytes, deadline) else {
                return false;
            };
            let Some(frame) = fcitx5_protocol_core::decode_frame(&response_bytes) else {
                self.disconnect();
                return false;
            };
            let Some(response) = fcitx5_protocol_core::decode_hello_response(&frame) else {
                self.disconnect();
                return false;
            };
            let scalars = crate::apply_hello_response_scalars(
                response.metadata.response_to,
                response.metadata.engine_epoch,
                response.metadata.session_id,
                response.status as u32,
                request_id,
                session_id,
            );
            if scalars.status == 0 {
                self.disconnect();
                return false;
            }
            self.engine_epoch = scalars.engine_epoch;
            self.handshake_complete = scalars.handshake_complete != 0;
            self.handshake_complete
        }

        pub fn select(
            &mut self,
            target_process_id: u32,
            expected_engine_epoch: u64,
            context_id: u64,
            composition_id: u64,
            revision: u64,
            candidate_id: u64,
        ) -> bool {
            let session_id = match self.identity.as_ref() {
                Some(identity) if identity.session_id != 0 => identity.session_id,
                _ => return false,
            };
            let deadline = crate::deadline_after_milliseconds(INPUT_DEADLINE_MILLISECONDS);
            if !self.connect(deadline) || !self.handshake(deadline) {
                self.disconnect();
                return false;
            }
            if !crate::accept_candidate_select_request(
                self.engine_epoch,
                expected_engine_epoch,
                target_process_id,
                context_id,
                composition_id,
                revision,
                candidate_id,
            ) {
                return false;
            }
            let request_id = crate::next_pipe_client_request_id();
            let request = fcitx5_protocol_core::CandidateSelectRequest {
                metadata: fcitx5_protocol_core::Metadata {
                    request_id,
                    engine_epoch: self.engine_epoch,
                    session_id,
                    context_id,
                    composition_id,
                    revision,
                    ..Default::default()
                },
                target_process_id,
                candidate_id,
            };
            let Some(request_bytes) =
                fcitx5_protocol_core::encode_candidate_select_request(&request)
            else {
                self.disconnect();
                return false;
            };
            let Some(response_bytes) = self.transact(request_bytes, deadline) else {
                return false;
            };
            let Some(frame) = fcitx5_protocol_core::decode_frame(&response_bytes) else {
                self.disconnect();
                return false;
            };
            let Some(response) = fcitx5_protocol_core::decode_candidate_select_response(&frame)
            else {
                self.disconnect();
                return false;
            };
            let accepted = crate::accept_candidate_select_response(
                response.metadata.response_to,
                response.metadata.engine_epoch,
                response.metadata.session_id,
                response.metadata.context_id,
                response.metadata.revision,
                response.status as u32,
                request_id,
                self.engine_epoch,
                session_id,
                context_id,
                revision,
            );
            if !accepted {
                self.disconnect();
            }
            accepted
        }
    }

    impl Drop for CandidateSelectClient {
        fn drop(&mut self) {
            self.disconnect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_on_never_connected_handle_returns_zero_without_crash() {
        let pipe_name: Vec<u16> = "Fcitx5WindowsNext.does-not-exist.select"
            .encode_utf16()
            .collect();
        let peer: Vec<u16> = "C:\\does-not-exist\\engine.exe".encode_utf16().collect();
        let Some(mut client) = safe::CandidateSelectClient::new(pipe_name, peer) else {
            return;
        };
        // Fails closed whether or not this process is an interactive user.
        let _ = client.select(1, 1, 1, 1, 1, 1);
    }
}
