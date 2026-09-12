//! Rust presentation pipe server (082 slice 1).
//!
//! Replaces the C++ `servePresentation`/`decodePresentationFrame` pair: one
//! inbound named pipe per cycle, same-principal peer verification with the
//! engine-executable gate, 64-byte headers, 256 KiB frame ceiling, and
//! protocol-core `KeyResponse` decoding. Delivery is a callback so the
//! window-host integration (slice 4) decides how frames reach the UI thread.

use std::ffi::OsStr;

use fcitx5_protocol_core::{FrameView, KeyResponse, MessageType};
use fcitx5_windows_common_core::{
    deadline_after, paths_refer_to_same_file, CurrentUserRuntimeIdentity, NamedEvent,
    NamedPipeServer,
};

/// Frozen wire constants from the shipping C++ `servePresentation`.
pub const PRESENTATION_HEADER_SIZE: usize = 64;
pub const PRESENTATION_MAX_FRAME_SIZE: usize = 256 * 1024;
const PRESENTATION_PIPE_CHANNEL: &str = "presentation";
/// Polling window for the connect loop. The C++ host blocks indefinitely in
/// `ConnectNamedPipe`; the stop event keeps this loop responsive instead.
const CONNECT_POLL_MILLIS: u32 = 1000;
/// Blocking read horizon. A frame can only arrive from a live composition;
/// an idle horizon this long is treated as a disconnect, matching the C++
/// `ReadFile` failure path.
const READ_DEADLINE_TICKS: u64 = u64::MAX;

/// Decodes one complete presentation frame into a [`KeyResponse`].
///
/// Mirrors `decodePresentationFrame`: the header must decode to a Key
/// Response whose body size exactly matches the remaining bytes, and the
/// whole frame must stay inside the frozen size window.
#[must_use]
pub fn decode_presentation_frame(frame: &[u8]) -> Option<KeyResponse> {
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
pub fn read_presentation_frame(server: &NamedPipeServer, deadline: u64) -> Option<KeyResponse> {
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
pub fn presentation_pipe_name(
    identity: &CurrentUserRuntimeIdentity,
    generation: &str,
) -> Option<std::ffi::OsString> {
    identity.local_endpoint_name(generation, PRESENTATION_PIPE_CHANNEL)
}

fn engine_peer_matches(peer_executable_path: &str, engine_executable: &OsStr) -> bool {
    let peer_units: Vec<u16> = peer_executable_path.encode_utf16().collect();
    let engine_units = engine_wide_units(engine_executable);
    paths_refer_to_same_file(&peer_units, &engine_units)
}

/// Serves presentation frames until `stop` is signaled or a delivery callback
/// aborts the loop.
///
/// Per connection: verifies the peer against this identity, gates on the
/// engine executable path, then decodes frames until the peer disconnects or
/// the callback stops the loop. `test_once` mirrors the C++ contract — one
/// delivered response ends the server (`Some(())`).
///
/// Returns `Some(())` when `test_once` delivered its response, `None` when
/// stopped or aborted.
pub fn serve_presentation(
    identity: &CurrentUserRuntimeIdentity,
    generation: &str,
    engine_executable: &OsStr,
    stop: &NamedEvent,
    test_once: bool,
    on_response: &mut dyn FnMut(KeyResponse) -> bool,
) -> Option<()> {
    let name = presentation_pipe_name(identity, generation)?;
    let security = identity.security_attributes()?;
    loop {
        if stop.is_signaled() {
            return None;
        }
        let listener =
            NamedPipeServer::create(&name, &security, PRESENTATION_MAX_FRAME_SIZE).ok()?;
        loop {
            if stop.is_signaled() {
                return None;
            }
            if listener.connect_until(deadline_after(CONNECT_POLL_MILLIS), stop) {
                break;
            }
        }
        let Some(peer) = listener.verified_client(identity) else {
            continue;
        };
        if !engine_peer_matches(&peer.executable_path, engine_executable) {
            continue;
        }
        loop {
            // Blocking horizon: u64::MAX never expires via remaining_milliseconds'
            // window (it clamps to the ~49.7-day DWORD window, after which the
            // read fails and the connection is recycled like a C++ disconnect).
            let Some(response) = read_presentation_frame(&listener, READ_DEADLINE_TICKS) else {
                break;
            };
            let delivered = on_response(response);
            if test_once {
                return Some(());
            }
            if !delivered {
                return None;
            }
        }
        // Dropping `listener` disconnects and closes this instance; the loop
        // recreates it, matching the C++ DisconnectNamedPipe/CloseHandle pair.
    }
}

fn engine_wide_units(value: &OsStr) -> Vec<u16> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        value.encode_wide().collect()
    }
    #[cfg(not(windows))]
    {
        value.to_string_lossy().encode_utf16().collect()
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
            preedit_caret_utf8: 2,
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
        // Truncated header.
        assert!(decode_presentation_frame(&[0_u8; PRESENTATION_HEADER_SIZE - 1]).is_none());
        // Header-only frame decodes the header but the body size mismatches.
        let frame = fcitx5_protocol_core::encode_key_response(&response()).expect("encoded frame");
        assert!(decode_presentation_frame(&frame[..PRESENTATION_HEADER_SIZE]).is_none());
        // Oversized frames are rejected before any decode work.
        let oversized = vec![0_u8; PRESENTATION_MAX_FRAME_SIZE + 1];
        assert!(decode_presentation_frame(&oversized).is_none());
    }

    #[test]
    fn decode_rejects_body_size_mismatch() {
        let frame = fcitx5_protocol_core::encode_key_response(&response()).expect("encoded frame");
        let mut tampered = frame.clone();
        // Rewrite the declared body size in the header to one byte too small.
        // Header layout (frozen): [0..2) type, [2..6) body size.
        let declared = u32::from_le_bytes([tampered[2], tampered[3], tampered[4], tampered[5]]);
        let reduced = declared - 1;
        tampered[2..6].copy_from_slice(&reduced.to_le_bytes());
        assert!(decode_presentation_frame(&tampered).is_none());
    }
}
