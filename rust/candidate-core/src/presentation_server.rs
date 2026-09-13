//! Rust presentation pipe server (082 slices 1 & 4).
//!
//! Replaces the C++ `servePresentation`/`decodePresentationFrame` pair: one
//! inbound named pipe per cycle, same-principal peer verification with the
//! engine-executable identity gate, frozen wire constants (64-byte header,
//! 256 KiB ceiling), and protocol-core `KeyResponse` decoding. The blocking
//! serve entry delivers each decoded response to the host callback as a flat
//! self-contained snapshot valid for the duration of the call.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::time::Duration;

use fcitx5_protocol_core::{FrameView, KeyResponse, MessageType};
use fcitx5_windows_common_core::{
    paths_refer_to_same_file, wait_for_handle, CurrentUserRuntimeIdentity, NamedPipeServer,
};

use crate::frame_ffi::{Fcitx5CandidateFrameRecord, Fcitx5CandidateFrameResponse};

/// Frozen wire constants from the shipping C++ `servePresentation`.
pub const PRESENTATION_HEADER_SIZE: usize = 64;
pub const PRESENTATION_MAX_FRAME_SIZE: usize = 256 * 1024;
const PRESENTATION_PIPE_CHANNEL: &str = "presentation";
/// Blocking read horizon. A frame can only arrive from a live composition;
/// a long-idle connection is recycled like a C++ `ReadFile` disconnect.
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
    let engine_units: Vec<u16> = engine_executable.encode_wide().collect();
    paths_refer_to_same_file(&peer_units, &engine_units)
}

fn wide_until_nul(pointer: *const u16) -> Vec<u16> {
    let mut units = Vec::new();
    // SAFETY: the C ABI contract guarantees NUL termination.
    let mut cursor = pointer;
    // SAFETY: `cursor` starts at a valid NUL-terminated UTF-16 sequence and advances only to its terminator.
    unsafe {
        loop {
            let unit = *cursor;
            if unit == 0 {
                break;
            }
            units.push(unit);
            cursor = cursor.add(1);
        }
    }
    units
}

/// Self-contained flat response snapshot handed to the host callback. String
/// storage lives in `strings` and stays valid while the snapshot is alive.
struct FlatFrameResponse {
    response: Fcitx5CandidateFrameResponse,
    #[allow(dead_code)] // read by the raw frame bytes handed to the host ABI
    candidates: Vec<Fcitx5CandidateFrameRecord>,
    #[allow(dead_code)] // arena keeping the referenced byte storage alive
    strings: Vec<Vec<u8>>,
}

impl FlatFrameResponse {
    fn build(response: &KeyResponse) -> Self {
        let mut strings: Vec<Vec<u8>> = Vec::new();
        let mut owned = |bytes: &[u8]| -> *const u8 {
            strings.push(bytes.to_vec());
            let stored = strings.last().expect("pushed above");
            stored.as_ptr()
        };
        let mut candidates = Vec::with_capacity(response.candidates.len());
        for candidate in &response.candidates {
            candidates.push(Fcitx5CandidateFrameRecord {
                id: candidate.id,
                label: owned(&candidate.label_utf8),
                label_len: candidate.label_utf8.len(),
                text: owned(&candidate.text_utf8),
                text_len: candidate.text_utf8.len(),
                comment: owned(&candidate.comment_utf8),
                comment_len: candidate.comment_utf8.len(),
            });
        }
        let preedit = owned(&response.preedit_utf8);
        let preedit_len = response.preedit_utf8.len();
        let content_locale = owned(&response.content_locale_utf8);
        let content_locale_len = response.content_locale_utf8.len();
        let flat = Fcitx5CandidateFrameResponse {
            engine_epoch: response.metadata.engine_epoch,
            context_id: response.metadata.context_id,
            composition_id: response.metadata.composition_id,
            revision: response.metadata.revision,
            preedit,
            preedit_len,
            content_locale,
            content_locale_len,
            status: 0,
            selected_candidate: response.selected_candidate,
            candidate_page: response.candidate_page,
            candidate_page_size: response.candidate_page_size,
            candidate_total: response.candidate_total,
            candidate_visibility: response.candidate_visibility,
            candidate_bulk: u8::from(response.candidate_bulk),
            candidate_end: u8::from(response.candidate_end),
            popup_allowed: u8::from(response.popup_allowed),
            caret_valid: u8::from(response.caret.valid),
            caret_left: response.caret.left,
            caret_top: response.caret.top,
            caret_right: response.caret.right,
            caret_bottom: response.caret.bottom,
            caret_dpi: response.caret.dpi,
            candidates: candidates.as_ptr(),
            candidate_count: candidates.len(),
        };
        Self {
            response: flat,
            candidates,
            strings,
        }
    }

    fn as_response(&self) -> &Fcitx5CandidateFrameResponse {
        &self.response
    }
}

/// Blocking-mode serve entry for the C++ host (082 slice 4).
///
/// Owns pipe creation, same-principal peer verification, the engine-executable
/// identity gate, frame reads, and KeyResponse decoding; delivers each decoded
/// response through `on_frame` as a flat self-contained snapshot whose buffers
/// are valid only during the call. Returns when `stop` is signaled, the
/// callback reports failure, or `test_once` delivered one response.
///
/// # Safety
///
/// `generation`/`engine_executable` must be NUL-terminated UTF-16; `stop` must
/// be a live event handle; `on_frame` must invoke no re-entrant server calls
/// and must finish before the next frame is delivered.
#[no_mangle]
pub unsafe extern "C" fn fcitx5_candidate_presentation_serve(
    generation: *const u16,
    engine_executable: *const u16,
    stop: *mut c_void,
    test_once: u8,
    on_frame: unsafe extern "system" fn(*mut c_void, *const Fcitx5CandidateFrameResponse),
    user: *mut c_void,
) -> u8 {
    let generation = if generation.is_null() {
        String::new()
    } else {
        // SAFETY: NUL-terminated UTF-16 per the contract above.
        let units = wide_until_nul(generation);
        String::from_utf16_lossy(&units)
    };
    if engine_executable.is_null() {
        return 0;
    }
    // SAFETY: NUL-terminated UTF-16 per the contract above.
    let engine_units = wide_until_nul(engine_executable);
    let engine_path = {
        use std::os::windows::ffi::OsStringExt;
        std::path::PathBuf::from(std::ffi::OsString::from_wide(&engine_units))
    };
    let Some(identity) = CurrentUserRuntimeIdentity::current() else {
        return 0;
    };
    let Some(name) = presentation_pipe_name(&identity, &generation) else {
        return 0;
    };
    let Some(security) = identity.security_attributes() else {
        return 0;
    };
    let stop_handle = core::ptr::NonNull::new(stop);
    let stop_signaled = || {
        stop_handle.is_some_and(|handle| {
            // SAFETY: borrowed for a zero-time probe of a live event handle.
            unsafe {
                wait_for_handle(
                    core::mem::transmute::<
                        core::ptr::NonNull<c_void>,
                        std::os::windows::io::BorrowedHandle<'_>,
                    >(handle),
                    Duration::ZERO,
                )
            }
        })
    };
    let _peer_units: Vec<u16> = engine_path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .collect();
    loop {
        if stop_signaled() {
            return 1;
        }
        let Ok(listener) = NamedPipeServer::create(&name, &security, PRESENTATION_MAX_FRAME_SIZE)
        else {
            return 0;
        };
        loop {
            if stop_signaled() {
                return 1;
            }
            if listener.connect_poll() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let Some(peer) = listener.verified_client(&identity) else {
            continue;
        };
        if !engine_peer_matches(&peer.executable_path, &engine_path.as_os_str()) {
            continue;
        }
        loop {
            let Some(response) = read_presentation_frame(&listener, READ_DEADLINE_TICKS) else {
                break;
            };
            let snapshot = FlatFrameResponse::build(&response);
            // SAFETY: `user` ownership belongs to the host callback contract.
            unsafe { on_frame(user, snapshot.as_response()) };
            if test_once != 0 {
                return 1;
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
