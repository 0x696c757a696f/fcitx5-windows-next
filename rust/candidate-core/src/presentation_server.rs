//! Rust presentation pipe server (082 slices 1 & 4).
//!
//! Replaces the C++ `servePresentation`/`decodePresentationFrame` pair: one
//! inbound named pipe per cycle, same-principal peer verification with the
//! engine-executable identity gate, frozen wire constants (64-byte header,
//! 256 KiB ceiling), and protocol-core `KeyResponse` decoding. The blocking
//! serve entry hands each decoded response to the host callback.

#![forbid(unsafe_code)]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::BorrowedHandle;
use std::path::Path;
use std::time::{Duration, Instant};

use fcitx5_protocol_core::{FrameView, KeyResponse, MessageType};
use fcitx5_windows_common_core::{
    deadline_after, paths_refer_to_same_file, wait_for_handle, CurrentUserRuntimeIdentity,
    NamedPipeServer,
};

/// Frozen wire constants from the shipping C++ `servePresentation`.
pub(crate) const PRESENTATION_HEADER_SIZE: usize = 64;
pub(crate) const PRESENTATION_MAX_FRAME_SIZE: usize = 256 * 1024;
const PRESENTATION_PIPE_CHANNEL: &str = "presentation";
/// Blocking read horizon. A frame can only arrive from a live composition;
/// a long-idle connection is recycled like a C++ `ReadFile` disconnect.
const READ_DEADLINE_TICKS: u64 = u64::MAX;
/// How long one listener waits for the engine before it is recreated.
const CONNECT_WAIT_MILLISECONDS: u32 = 60_000;
const RETRY_INITIAL: Duration = Duration::from_millis(50);
const RETRY_MAX: Duration = Duration::from_secs(5);

/// Decodes one complete presentation frame into a [`KeyResponse`].
///
/// Mirrors `decodePresentationFrame`: the header must decode to a Key
/// Response whose body size exactly matches the remaining bytes, and the
/// whole frame must stay inside the frozen size window.
#[must_use]
pub(crate) fn decode_presentation_frame(frame: &[u8]) -> Option<KeyResponse> {
    if frame.len() < PRESENTATION_HEADER_SIZE || frame.len() > PRESENTATION_MAX_FRAME_SIZE {
        return None;
    }
    let (message_type, body_size, metadata) =
        fcitx5_protocol_core::decode_header(&frame[..PRESENTATION_HEADER_SIZE])?;
    if message_type != MessageType::KeyResponse {
        return None;
    }
    if body_size as usize != frame.len() - PRESENTATION_HEADER_SIZE {
        return None;
    }
    let view = FrameView {
        message_type,
        metadata,
        body: &frame[PRESENTATION_HEADER_SIZE..],
    };
    fcitx5_protocol_core::decode_key_response(&view)
}

/// Reads and decodes one frame from a connected pipe server.
///
/// Header failures, type mismatches, and oversized bodies return `None`,
/// matching the C++ break-and-recreate behavior; the body is only read after
/// the header validates.
#[must_use]
pub(crate) fn read_presentation_frame(
    server: &NamedPipeServer,
    deadline: u64,
) -> Option<KeyResponse> {
    let mut header = [0_u8; PRESENTATION_HEADER_SIZE];
    if !server.read_exact(&mut header, deadline) {
        return None;
    }
    let (message_type, body_size, _metadata) = fcitx5_protocol_core::decode_header(&header)?;
    if message_type != MessageType::KeyResponse {
        return None;
    }
    let body_size = body_size as usize;
    if body_size > PRESENTATION_MAX_FRAME_SIZE - PRESENTATION_HEADER_SIZE {
        return None;
    }
    let mut frame = vec![0_u8; PRESENTATION_HEADER_SIZE + body_size];
    frame[..PRESENTATION_HEADER_SIZE].copy_from_slice(&header);
    if body_size > 0 && !server.read_exact(&mut frame[PRESENTATION_HEADER_SIZE..], deadline) {
        return None;
    }
    decode_presentation_frame(&frame)
}

/// Resolves the presentation endpoint name for this identity/generation.
#[must_use]
pub(crate) fn presentation_pipe_name(
    identity: &CurrentUserRuntimeIdentity,
    generation: &str,
) -> Option<std::ffi::OsString> {
    identity.local_endpoint_name(generation, PRESENTATION_PIPE_CHANNEL)
}

fn engine_peer_matches(peer_executable_path: &str, engine_executable: &OsStr) -> bool {
    let peer_units: Vec<u16> = peer_executable_path.encode_utf16().collect();
    let engine_units: Vec<u16> = engine_executable.encode_wide().collect();
    paths_refer_to_same_file(&peer_units, &engine_units)
}

/// Blocking presentation serve loop.
///
/// Owns pipe creation, same-principal peer verification, the engine-executable
/// identity gate, frame reads, and KeyResponse decoding; hands each decoded
/// response to `on_frame`. Returns when `stop` is signaled or `test_once`
/// delivered one response.
pub fn serve_presentation(
    generation: &str,
    engine_path: &Path,
    stop: Option<BorrowedHandle<'_>>,
    test_once: bool,
    mut on_frame: impl FnMut(&KeyResponse),
) {
    let Some(identity) = CurrentUserRuntimeIdentity::current() else {
        return;
    };
    let Some(name) = presentation_pipe_name(&identity, generation) else {
        return;
    };
    let Some(security) = identity.security_attributes() else {
        return;
    };
    let stop_signaled = || stop.is_some_and(|handle| wait_for_handle(handle, Duration::ZERO));
    let mut retry = RETRY_INITIAL;
    loop {
        if stop_signaled() {
            return;
        }
        // A failed create or an instant connect error is retried with a
        // growing pause: returning would leave the candidate window without
        // frames for the rest of the session, and spinning would burn CPU.
        let attempt = Instant::now();
        let connected = NamedPipeServer::create(&name, &security, PRESENTATION_MAX_FRAME_SIZE)
            .ok()
            .and_then(|listener| {
                listener
                    .connect_waiting(deadline_after(CONNECT_WAIT_MILLISECONDS), stop)
                    .then_some(listener)
            });
        let Some(listener) = connected else {
            if attempt.elapsed() < RETRY_INITIAL {
                std::thread::sleep(retry);
                retry = (retry * 2).min(RETRY_MAX);
            }
            continue;
        };
        retry = RETRY_INITIAL;
        let Some(peer) = listener.verified_client(&identity) else {
            continue;
        };
        if !engine_peer_matches(&peer.executable_path, engine_path.as_os_str()) {
            continue;
        }
        while let Some(response) = read_presentation_frame(&listener, READ_DEADLINE_TICKS) {
            on_frame(&response);
            if test_once {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcitx5_protocol_core::{CandidateRecord, Metadata, Status};

    fn response() -> KeyResponse {
        KeyResponse {
            metadata: Metadata {
                request_id: 7,
                response_to: 3,
                engine_epoch: 1,
                session_id: 2,
                context_id: 4,
                composition_id: 5,
                revision: 6,
            },
            status: Status::Ok,
            handled: true,
            preedit_utf8: b"ni".to_vec(),
            candidates: vec![CandidateRecord {
                id: 1,
                label_utf8: b"1".to_vec(),
                text_utf8: "你".as_bytes().to_vec(),
                comment_utf8: Vec::new(),
            }],
            candidate_total: 1,
            candidate_visibility: 1,
            candidate_page_size: 5,
            candidate_end: true,
            ..KeyResponse::default()
        }
    }

    #[test]
    fn decode_round_trips_an_encoded_key_response_frame() {
        let frame = fcitx5_protocol_core::encode_key_response(&response())
            .expect("encoder produces a framed response");
        let decoded = decode_presentation_frame(&frame).expect("frame decodes");
        assert_eq!(decoded, response());
    }

    #[test]
    fn decode_rejects_wrong_type_truncated_and_oversized_frames() {
        assert!(decode_presentation_frame(&[0_u8; PRESENTATION_HEADER_SIZE - 1]).is_none());
        let frame = fcitx5_protocol_core::encode_key_response(&response()).expect("encoded frame");
        assert!(decode_presentation_frame(&frame[..PRESENTATION_HEADER_SIZE]).is_none());
        let oversized = vec![0_u8; PRESENTATION_MAX_FRAME_SIZE + 1];
        assert!(decode_presentation_frame(&oversized).is_none());
    }

    #[test]
    fn decode_rejects_body_size_mismatch() {
        let frame = fcitx5_protocol_core::encode_key_response(&response()).expect("encoded frame");
        let mut tampered = frame;
        // Header layout (frozen): [0..2) type, [2..6) body size.
        let declared = u32::from_le_bytes([tampered[2], tampered[3], tampered[4], tampered[5]]);
        let reduced = declared - 1;
        tampered[2..6].copy_from_slice(&reduced.to_le_bytes());
        assert!(decode_presentation_frame(&tampered).is_none());
    }
}
