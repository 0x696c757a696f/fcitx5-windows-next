#![deny(unsafe_op_in_unsafe_fn)]

//! Narrow C ABI for the E1 cutover.
//!
//! Consumers (the engine adapter, IPC clients, and the Candidate UI) marshal
//! their data into the flat `Fcitx*C` structures below and call these
//! functions; all validation and byte layout authority lives in Rust. The C++
//! `protocol.h`/`protocol.cpp` DTO bridge has been deleted.
//!
//! ABI conventions:
//!
//! * Encode: `fcitx5_protocol_core_encode_<type>(message, out, out_capacity,
//!   out_length)`. Returns 1 on success (writes `*out_length` = encoded
//!   length). Returns 0 on rejection without touching `*out_length`, or on
//!   insufficient space after writing `*out_length` = required length.
//! * Decode: `fcitx5_protocol_core_decode_<type>(metadata, body, body_length,
//!   out, strings, strings_capacity, strings_needed, [candidates,
//!   candidates_capacity, candidates_needed])`. Returns 1 on success; string
//!   bytes are appended into the caller-provided `strings` arena and each
//!   `out` string field points into it (`*strings_needed` = bytes used).
//!   Returns 0 on rejection (needed counters untouched), or on insufficient
//!   space after writing the required sizes.
//!
//! Every exported function contains panics; a panic surfaces as return 0.

use crate::{
    decode_candidate_select_request, decode_candidate_select_response,
    decode_engine_status_request, decode_engine_status_response, decode_header,
    decode_hello_request, decode_hello_response, decode_key_request, decode_key_response,
    decode_launcher_request, decode_launcher_response, decode_state_request,
    encode_candidate_select_request, encode_candidate_select_response,
    encode_engine_status_request, encode_engine_status_response, encode_hello_request,
    encode_hello_response, encode_key_request, encode_key_response, encode_launcher_request,
    encode_launcher_response, encode_state_request, CandidateRecord, CandidateSelectRequest,
    CandidateSelectResponse, CaretRect, EngineStatusRequest, EngineStatusResponse, FrameView,
    HelloRequest, HelloResponse, KeyRequest, KeyResponse, LauncherCommand, LauncherRequest,
    LauncherResponse, MessageType, Metadata, StateRequest, Status,
};
use std::panic;

// ---------------------------------------------------------------------------
// Flat C structures (field order mirrors `protocol/protocol.h`)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxMetadataC {
    pub request_id: u64,
    pub response_to: u64,
    pub engine_epoch: u64,
    pub session_id: u32,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxCaretRectC {
    pub valid: u8,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub dpi: u32,
}

/// Byte span used for every string field: encode inputs point at C++ string
/// storage, decode outputs point into the caller-provided strings arena.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxBytesC {
    pub data: *const u8,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxHelloRequestC {
    pub metadata: FcitxMetadataC,
    pub client_architecture_bits: u32,
    pub client_process_id: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxHelloResponseC {
    pub metadata: FcitxMetadataC,
    pub status: u32,
    pub server_architecture_bits: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxKeyRequestC {
    pub metadata: FcitxMetadataC,
    pub virtual_key: u32,
    pub key_flags: u32,
    pub scan_code: u32,
    pub extended_key: u8,
    pub popup_allowed: u8,
    pub keyboard_layout: u64,
    pub logical_text: FcitxBytesC,
    pub input_method: FcitxBytesC,
    pub surrounding_text_valid: u8,
    pub surrounding_text: FcitxBytesC,
    pub surrounding_cursor: u32,
    pub surrounding_anchor: u32,
    pub caret: FcitxCaretRectC,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxCandidateRecordC {
    pub id: u64,
    pub label: FcitxBytesC,
    pub text: FcitxBytesC,
    pub comment: FcitxBytesC,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxKeyResponseC {
    pub metadata: FcitxMetadataC,
    pub status: u32,
    pub handled: u8,
    pub commit: FcitxBytesC,
    pub preedit: FcitxBytesC,
    pub preedit_caret_utf8: u32,
    pub selected_candidate: u32,
    pub candidate_page: u32,
    pub candidate_total: u32,
    pub candidate_visibility: u8,
    pub candidate_page_size: u32,
    pub candidate_bulk: u8,
    pub candidate_end: u8,
    pub delete_surrounding_text: u8,
    pub delete_surrounding_offset: i32,
    pub delete_surrounding_size: u32,
    pub forward_key: u8,
    pub forward_key_sym: u32,
    pub forward_key_states: u32,
    pub forward_key_code: i32,
    pub forward_key_release: u8,
    pub caret: FcitxCaretRectC,
    pub popup_allowed: u8,
    pub content_locale: FcitxBytesC,
    pub candidates: *const FcitxCandidateRecordC,
    pub candidate_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxCandidateSelectRequestC {
    pub metadata: FcitxMetadataC,
    pub target_process_id: u32,
    pub candidate_id: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxCandidateSelectResponseC {
    pub metadata: FcitxMetadataC,
    pub status: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxStateRequestC {
    pub metadata: FcitxMetadataC,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxEngineStatusRequestC {
    pub metadata: FcitxMetadataC,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxEngineStatusResponseC {
    pub metadata: FcitxMetadataC,
    pub status: u32,
    pub current_input_method_id: FcitxBytesC,
    pub current_input_method_name: FcitxBytesC,
    pub current_input_method_native_name: FcitxBytesC,
    pub current_input_method_short_label: FcitxBytesC,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxLauncherRequestC {
    pub metadata: FcitxMetadataC,
    pub command: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FcitxLauncherResponseC {
    pub metadata: FcitxMetadataC,
    pub status: u32,
    pub launcher_state: u32,
    pub engine_state: u32,
    pub start_disposition: u32,
    pub safe_mode: u8,
    pub retry_after_milliseconds: u64,
    pub current_input_method_id: FcitxBytesC,
    pub current_input_method_name: FcitxBytesC,
    pub current_input_method_native_name: FcitxBytesC,
    pub current_input_method_short_label: FcitxBytesC,
}

// ---------------------------------------------------------------------------
// Marshalling helpers
// ---------------------------------------------------------------------------

fn bytes_from_c(bytes: &FcitxBytesC) -> Vec<u8> {
    if bytes.len == 0 || bytes.data.is_null() {
        return Vec::new();
    }
    // SAFETY: the FFI entry points require a valid buffer of `len` bytes when
    // `len` is non-zero; a null pointer with a non-zero length is treated as
    // empty defensively (never dereferenced).
    unsafe { std::slice::from_raw_parts(bytes.data, bytes.len) }.to_vec()
}

fn metadata_from_c(metadata: &FcitxMetadataC) -> Metadata {
    Metadata {
        request_id: metadata.request_id,
        response_to: metadata.response_to,
        engine_epoch: metadata.engine_epoch,
        session_id: metadata.session_id,
        context_id: metadata.context_id,
        composition_id: metadata.composition_id,
        revision: metadata.revision,
    }
}

fn metadata_to_c(metadata: &Metadata) -> FcitxMetadataC {
    FcitxMetadataC {
        request_id: metadata.request_id,
        response_to: metadata.response_to,
        engine_epoch: metadata.engine_epoch,
        session_id: metadata.session_id,
        context_id: metadata.context_id,
        composition_id: metadata.composition_id,
        revision: metadata.revision,
    }
}

fn caret_from_c(caret: &FcitxCaretRectC) -> CaretRect {
    CaretRect {
        valid: caret.valid != 0,
        left: caret.left,
        top: caret.top,
        right: caret.right,
        bottom: caret.bottom,
        dpi: caret.dpi,
    }
}

fn caret_to_c(caret: &CaretRect) -> FcitxCaretRectC {
    FcitxCaretRectC {
        valid: caret.valid as u8,
        left: caret.left,
        top: caret.top,
        right: caret.right,
        bottom: caret.bottom,
        dpi: caret.dpi,
    }
}

fn status_from_c(raw: u32) -> Option<Status> {
    match raw {
        0 => Some(Status::Ok),
        1 => Some(Status::Malformed),
        2 => Some(Status::VersionMismatch),
        3 => Some(Status::Unsupported),
        4 => Some(Status::StaleIdentity),
        5 => Some(Status::AccessDenied),
        _ => None,
    }
}

fn launcher_command_from_c(raw: u32) -> Option<LauncherCommand> {
    match raw {
        1 => Some(LauncherCommand::StartDemand),
        2 => Some(LauncherCommand::UserStop),
        3 => Some(LauncherCommand::Resume),
        4 => Some(LauncherCommand::BeginUpdate),
        5 => Some(LauncherCommand::EndUpdate),
        6 => Some(LauncherCommand::BeginUninstall),
        7 => Some(LauncherCommand::ResetSafeMode),
        8 => Some(LauncherCommand::Status),
        9 => Some(LauncherCommand::Shutdown),
        _ => None,
    }
}

/// Copies `bytes` into `out` (which is writable for `out_capacity` bytes) and
/// reports the required length through `out_length`. Returns 1 on success,
/// 0 when `out_length` was set to the required size (caller reallocates).
///
/// # Safety
/// `out`/`out_capacity` must describe a writable buffer (or `out` may be null
/// with capacity 0) and `out_length` must be writable.
unsafe fn write_out(out: *mut u8, out_capacity: usize, out_length: *mut usize, bytes: &[u8]) -> u8 {
    if bytes.len() > out_capacity {
        // SAFETY: `out_length` is writable (checked by every caller).
        unsafe { *out_length = bytes.len() };
        return 0;
    }
    if !bytes.is_empty() {
        // SAFETY: `out` is writable for `out_capacity` bytes and `bytes.len()`
        // <= out_capacity.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len()) };
    }
    // SAFETY: `out_length` is writable.
    unsafe { *out_length = bytes.len() };
    1
}

/// Appends string bytes into the caller-provided arena. The caller must have
/// pre-verified total capacity; this function asserts the invariant.
struct ArenaWriter {
    base: *mut u8,
    capacity: usize,
    offset: usize,
}

impl ArenaWriter {
    fn new(base: *mut u8, capacity: usize) -> Self {
        ArenaWriter {
            base,
            capacity,
            offset: 0,
        }
    }

    /// # Safety
    /// `base` must be writable for `capacity` bytes and the accumulated
    /// writes must stay within `capacity`.
    unsafe fn write(&mut self, bytes: &[u8]) -> FcitxBytesC {
        if bytes.is_empty() {
            return FcitxBytesC {
                data: std::ptr::null(),
                len: 0,
            };
        }
        assert!(
            self.offset + bytes.len() <= self.capacity,
            "protocol-core arena capacity overflow"
        );
        // SAFETY: total writes stay within the caller-provided buffer; base is
        // non-null because `bytes.len() > 0` implies capacity > offset >= 0.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.base.add(self.offset), bytes.len());
        }
        let result = FcitxBytesC {
            // SAFETY: offset + len stays within the buffer.
            data: unsafe { self.base.add(self.offset) },
            len: bytes.len(),
        };
        self.offset += bytes.len();
        result
    }
}

// ---------------------------------------------------------------------------
// Frame header
// ---------------------------------------------------------------------------

/// Decodes and validates a 64-byte FCW4 header. Mirrors C++ `decodeHeader`.
///
/// # Safety
/// `bytes`/`length` must describe a readable buffer (null with length 0 is
/// allowed); `out_type`, `out_body_size`, `out_metadata` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_header(
    bytes: *const u8,
    length: usize,
    out_type: *mut u16,
    out_body_size: *mut u32,
    out_metadata: *mut FcitxMetadataC,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if (bytes.is_null() && length != 0)
            || out_type.is_null()
            || out_body_size.is_null()
            || out_metadata.is_null()
        {
            return 0;
        }
        let slice = if length == 0 {
            &[][..]
        } else {
            // SAFETY: caller provides a valid buffer of `length` bytes.
            unsafe { std::slice::from_raw_parts(bytes, length) }
        };
        let Some((message_type, body_size, metadata)) = decode_header(slice) else {
            return 0;
        };
        // SAFETY: output pointers are writable (checked above).
        unsafe {
            *out_type = message_type as u16;
            *out_body_size = body_size;
            *out_metadata = metadata_to_c(&metadata);
        }
        1
    });
    result.unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Encode (mirror the C++ typed `encode` overloads)
// ---------------------------------------------------------------------------

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_hello_request(
    message: *const FcitxHelloRequestC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let rust = HelloRequest {
            metadata: metadata_from_c(&m.metadata),
            client_architecture_bits: m.client_architecture_bits,
            client_process_id: m.client_process_id,
        };
        let Some(bytes) = encode_hello_request(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_hello_response(
    message: *const FcitxHelloResponseC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let Some(status) = status_from_c(m.status) else {
            return 0;
        };
        let rust = HelloResponse {
            metadata: metadata_from_c(&m.metadata),
            status,
            server_architecture_bits: m.server_architecture_bits,
        };
        let Some(bytes) = encode_hello_response(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_key_request(
    message: *const FcitxKeyRequestC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let rust = KeyRequest {
            metadata: metadata_from_c(&m.metadata),
            virtual_key: m.virtual_key,
            key_flags: m.key_flags,
            scan_code: m.scan_code,
            extended_key: m.extended_key != 0,
            popup_allowed: m.popup_allowed != 0,
            keyboard_layout: m.keyboard_layout,
            logical_text_utf8: bytes_from_c(&m.logical_text),
            input_method_utf8: bytes_from_c(&m.input_method),
            surrounding_text_valid: m.surrounding_text_valid != 0,
            surrounding_text_utf8: bytes_from_c(&m.surrounding_text),
            surrounding_cursor: m.surrounding_cursor,
            surrounding_anchor: m.surrounding_anchor,
            caret: caret_from_c(&m.caret),
        };
        let Some(bytes) = encode_key_request(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_key_response(
    message: *const FcitxKeyResponseC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let Some(status) = status_from_c(m.status) else {
            return 0;
        };
        let mut candidates = Vec::with_capacity(m.candidate_count);
        if !m.candidates.is_null() && m.candidate_count != 0 {
            // SAFETY: `candidates` points at `candidate_count` valid records.
            let records = unsafe { std::slice::from_raw_parts(m.candidates, m.candidate_count) };
            for record in records {
                candidates.push(CandidateRecord {
                    id: record.id,
                    label_utf8: bytes_from_c(&record.label),
                    text_utf8: bytes_from_c(&record.text),
                    comment_utf8: bytes_from_c(&record.comment),
                });
            }
        }
        let rust = KeyResponse {
            metadata: metadata_from_c(&m.metadata),
            status,
            handled: m.handled != 0,
            commit_utf8: bytes_from_c(&m.commit),
            preedit_utf8: bytes_from_c(&m.preedit),
            preedit_caret_utf8: m.preedit_caret_utf8,
            candidates,
            selected_candidate: m.selected_candidate,
            candidate_page: m.candidate_page,
            candidate_total: m.candidate_total,
            candidate_visibility: m.candidate_visibility,
            candidate_page_size: m.candidate_page_size,
            candidate_bulk: m.candidate_bulk != 0,
            candidate_end: m.candidate_end != 0,
            delete_surrounding_text: m.delete_surrounding_text != 0,
            delete_surrounding_offset: m.delete_surrounding_offset,
            delete_surrounding_size: m.delete_surrounding_size,
            forward_key: m.forward_key != 0,
            forward_key_sym: m.forward_key_sym,
            forward_key_states: m.forward_key_states,
            forward_key_code: m.forward_key_code,
            forward_key_release: m.forward_key_release != 0,
            caret: caret_from_c(&m.caret),
            popup_allowed: m.popup_allowed != 0,
            content_locale_utf8: bytes_from_c(&m.content_locale),
        };
        let Some(bytes) = encode_key_response(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_candidate_select_request(
    message: *const FcitxCandidateSelectRequestC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let rust = CandidateSelectRequest {
            metadata: metadata_from_c(&m.metadata),
            target_process_id: m.target_process_id,
            candidate_id: m.candidate_id,
        };
        let Some(bytes) = encode_candidate_select_request(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_candidate_select_response(
    message: *const FcitxCandidateSelectResponseC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let Some(status) = status_from_c(m.status) else {
            return 0;
        };
        let rust = CandidateSelectResponse {
            metadata: metadata_from_c(&m.metadata),
            status,
        };
        let Some(bytes) = encode_candidate_select_response(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_state_request(
    message: *const FcitxStateRequestC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let rust = StateRequest {
            metadata: metadata_from_c(&m.metadata),
        };
        let Some(bytes) = encode_state_request(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_engine_status_request(
    message: *const FcitxEngineStatusRequestC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let rust = EngineStatusRequest {
            metadata: metadata_from_c(&m.metadata),
        };
        let Some(bytes) = encode_engine_status_request(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_engine_status_response(
    message: *const FcitxEngineStatusResponseC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let Some(status) = status_from_c(m.status) else {
            return 0;
        };
        let rust = EngineStatusResponse {
            metadata: metadata_from_c(&m.metadata),
            status,
            current_input_method_id: bytes_from_c(&m.current_input_method_id),
            current_input_method_name: bytes_from_c(&m.current_input_method_name),
            current_input_method_native_name: bytes_from_c(&m.current_input_method_native_name),
            current_input_method_short_label: bytes_from_c(&m.current_input_method_short_label),
        };
        let Some(bytes) = encode_engine_status_response(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_launcher_request(
    message: *const FcitxLauncherRequestC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let Some(command) = launcher_command_from_c(m.command) else {
            return 0;
        };
        let rust = LauncherRequest {
            metadata: metadata_from_c(&m.metadata),
            command,
        };
        let Some(bytes) = encode_launcher_request(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

/// # Safety
/// `message` must point at a valid structure; `out`/`out_capacity` must
/// describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_encode_launcher_response(
    message: *const FcitxLauncherResponseC,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if message.is_null() || out_length.is_null() || (out.is_null() && out_capacity != 0) {
            return 0;
        }
        // SAFETY: `message` points at a valid structure.
        let m = unsafe { &*message };
        let Some(status) = status_from_c(m.status) else {
            return 0;
        };
        let rust = LauncherResponse {
            metadata: metadata_from_c(&m.metadata),
            status,
            launcher_state: m.launcher_state,
            engine_state: m.engine_state,
            start_disposition: m.start_disposition,
            safe_mode: m.safe_mode != 0,
            retry_after_milliseconds: m.retry_after_milliseconds,
            current_input_method_id: bytes_from_c(&m.current_input_method_id),
            current_input_method_name: bytes_from_c(&m.current_input_method_name),
            current_input_method_native_name: bytes_from_c(&m.current_input_method_native_name),
            current_input_method_short_label: bytes_from_c(&m.current_input_method_short_label),
        };
        let Some(bytes) = encode_launcher_response(&rust) else {
            return 0;
        };
        // SAFETY: pointer contracts checked above.
        unsafe { write_out(out, out_capacity, out_length, &bytes) }
    });
    result.unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Decode (mirror the C++ typed `decode` overloads)
// ---------------------------------------------------------------------------

/// # Safety
/// `metadata`, `out`, `strings_needed` must be writable/valid pointers;
/// `body`/`body_length` must describe a readable buffer; `strings`/
/// `strings_capacity` must describe a writable buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_hello_request(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxHelloRequestC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::HelloRequest,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(msg) = decode_hello_request(&frame) else {
            return 0;
        };
        // SAFETY: `out` is writable (checked above).
        let o = unsafe { &mut *out };
        o.client_architecture_bits = msg.client_architecture_bits;
        o.client_process_id = msg.client_process_id;
        // SAFETY: `strings_needed` is writable (checked above).
        unsafe { *strings_needed = 0 };
        1
    });
    result.unwrap_or(0)
}

/// # Safety
/// `metadata`, `out`, `strings_needed` must be writable/valid pointers;
/// `body`/`body_length` must describe a readable buffer; `strings`/
/// `strings_capacity` must describe a writable buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_hello_response(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxHelloResponseC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::HelloResponse,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(msg) = decode_hello_response(&frame) else {
            return 0;
        };
        // SAFETY: `out` is writable (checked above).
        let o = unsafe { &mut *out };
        o.status = msg.status as u32;
        o.server_architecture_bits = msg.server_architecture_bits;
        // SAFETY: `strings_needed` is writable (checked above).
        unsafe { *strings_needed = 0 };
        1
    });
    result.unwrap_or(0)
}

/// # Safety
/// `metadata`, `out`, `strings_needed` must be writable/valid pointers;
/// `body`/`body_length` must describe a readable buffer; `strings`/
/// `strings_capacity` must describe a writable buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_key_request(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxKeyRequestC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::KeyRequest,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(msg) = decode_key_request(&frame) else {
            return 0;
        };
        let needed = msg.logical_text_utf8.len()
            + msg.input_method_utf8.len()
            + msg.surrounding_text_utf8.len();
        if needed > strings_capacity {
            // SAFETY: `strings_needed` is writable (checked above).
            unsafe { *strings_needed = needed };
            return 0;
        }
        // SAFETY: `strings` is writable for `strings_capacity` bytes and
        // `needed <= strings_capacity`.
        let mut arena = ArenaWriter::new(strings, strings_capacity);
        // SAFETY: `out` is writable (checked above).
        let o = unsafe { &mut *out };
        o.virtual_key = msg.virtual_key;
        o.key_flags = msg.key_flags;
        o.scan_code = msg.scan_code;
        o.extended_key = msg.extended_key as u8;
        o.popup_allowed = msg.popup_allowed as u8;
        o.keyboard_layout = msg.keyboard_layout;
        // SAFETY: arena capacity pre-verified.
        o.logical_text = unsafe { arena.write(&msg.logical_text_utf8) };
        // SAFETY: arena capacity pre-verified.
        o.input_method = unsafe { arena.write(&msg.input_method_utf8) };
        o.surrounding_text_valid = msg.surrounding_text_valid as u8;
        // SAFETY: arena capacity pre-verified.
        o.surrounding_text = unsafe { arena.write(&msg.surrounding_text_utf8) };
        o.surrounding_cursor = msg.surrounding_cursor;
        o.surrounding_anchor = msg.surrounding_anchor;
        o.caret = caret_to_c(&msg.caret);
        // SAFETY: `strings_needed` is writable (checked above).
        unsafe { *strings_needed = needed };
        1
    });
    result.unwrap_or(0)
}

/// # Safety
/// `metadata`, `out`, `strings_needed`, `candidates_needed` must be
/// writable/valid pointers; `body`/`body_length` must describe a readable
/// buffer; `strings`/`strings_capacity` and `candidates`/`candidates_capacity`
/// must describe writable buffers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_key_response(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxKeyResponseC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
    candidates: *mut FcitxCandidateRecordC,
    candidates_capacity: usize,
    candidates_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || candidates_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
            || (candidates.is_null() && candidates_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::KeyResponse,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(msg) = decode_key_response(&frame) else {
            return 0;
        };
        let mut strings_total =
            msg.commit_utf8.len() + msg.preedit_utf8.len() + msg.content_locale_utf8.len();
        for candidate in &msg.candidates {
            strings_total += candidate.label_utf8.len()
                + candidate.text_utf8.len()
                + candidate.comment_utf8.len();
        }
        if strings_total > strings_capacity || msg.candidates.len() > candidates_capacity {
            // SAFETY: `strings_needed`/`candidates_needed` are writable.
            unsafe {
                *strings_needed = strings_total;
                *candidates_needed = msg.candidates.len();
            }
            return 0;
        }
        // SAFETY: arena capacity pre-verified.
        let mut arena = ArenaWriter::new(strings, strings_capacity);
        // SAFETY: `out` is writable (checked above).
        let o = unsafe { &mut *out };
        o.status = msg.status as u32;
        o.handled = msg.handled as u8;
        // SAFETY: arena capacity pre-verified.
        o.commit = unsafe { arena.write(&msg.commit_utf8) };
        // SAFETY: arena capacity pre-verified.
        o.preedit = unsafe { arena.write(&msg.preedit_utf8) };
        o.preedit_caret_utf8 = msg.preedit_caret_utf8;
        o.selected_candidate = msg.selected_candidate;
        o.candidate_page = msg.candidate_page;
        o.candidate_total = msg.candidate_total;
        o.candidate_visibility = msg.candidate_visibility;
        o.candidate_page_size = msg.candidate_page_size;
        o.candidate_bulk = msg.candidate_bulk as u8;
        o.candidate_end = msg.candidate_end as u8;
        o.delete_surrounding_text = msg.delete_surrounding_text as u8;
        o.delete_surrounding_offset = msg.delete_surrounding_offset;
        o.delete_surrounding_size = msg.delete_surrounding_size;
        o.forward_key = msg.forward_key as u8;
        o.forward_key_sym = msg.forward_key_sym;
        o.forward_key_states = msg.forward_key_states;
        o.forward_key_code = msg.forward_key_code;
        o.forward_key_release = msg.forward_key_release as u8;
        o.caret = caret_to_c(&msg.caret);
        o.popup_allowed = msg.popup_allowed as u8;
        // SAFETY: arena capacity pre-verified.
        o.content_locale = unsafe { arena.write(&msg.content_locale_utf8) };
        // SAFETY: `candidates` is writable for `candidates_capacity` records
        // and `msg.candidates.len() <= candidates_capacity`.
        if !msg.candidates.is_empty() {
            let out_candidates =
                unsafe { std::slice::from_raw_parts_mut(candidates, msg.candidates.len()) };
            for (target, candidate) in out_candidates.iter_mut().zip(&msg.candidates) {
                target.id = candidate.id;
                // SAFETY: arena capacity pre-verified.
                target.label = unsafe { arena.write(&candidate.label_utf8) };
                // SAFETY: arena capacity pre-verified.
                target.text = unsafe { arena.write(&candidate.text_utf8) };
                // SAFETY: arena capacity pre-verified.
                target.comment = unsafe { arena.write(&candidate.comment_utf8) };
            }
        }
        o.candidates = candidates;
        o.candidate_count = msg.candidates.len();
        // SAFETY: `strings_needed`/`candidates_needed` are writable.
        unsafe {
            *strings_needed = strings_total;
            *candidates_needed = msg.candidates.len();
        }
        1
    });
    result.unwrap_or(0)
}

/// # Safety
/// `metadata`, `out`, `strings_needed` must be writable/valid pointers;
/// `body`/`body_length` must describe a readable buffer; `strings`/
/// `strings_capacity` must describe a writable buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_candidate_select_request(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxCandidateSelectRequestC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::CandidateSelectRequest,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(msg) = decode_candidate_select_request(&frame) else {
            return 0;
        };
        // SAFETY: `out` is writable (checked above).
        let o = unsafe { &mut *out };
        o.target_process_id = msg.target_process_id;
        o.candidate_id = msg.candidate_id;
        // SAFETY: `strings_needed` is writable (checked above).
        unsafe { *strings_needed = 0 };
        1
    });
    result.unwrap_or(0)
}

/// # Safety
/// `metadata`, `out`, `strings_needed` must be writable/valid pointers;
/// `body`/`body_length` must describe a readable buffer; `strings`/
/// `strings_capacity` must describe a writable buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_candidate_select_response(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxCandidateSelectResponseC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::CandidateSelectResponse,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(msg) = decode_candidate_select_response(&frame) else {
            return 0;
        };
        // SAFETY: `out` is writable (checked above).
        let o = unsafe { &mut *out };
        o.status = msg.status as u32;
        // SAFETY: `strings_needed` is writable (checked above).
        unsafe { *strings_needed = 0 };
        1
    });
    result.unwrap_or(0)
}

/// # Safety
/// `metadata`, `out`, `strings_needed` must be writable/valid pointers;
/// `body`/`body_length` must describe a readable buffer; `strings`/
/// `strings_capacity` must describe a writable buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_state_request(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxStateRequestC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::StateRequest,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(_msg) = decode_state_request(&frame) else {
            return 0;
        };
        // SAFETY: `strings_needed` is writable (checked above).
        unsafe { *strings_needed = 0 };
        1
    });
    result.unwrap_or(0)
}

/// # Safety
/// `metadata`, `out`, `strings_needed` must be writable/valid pointers;
/// `body`/`body_length` must describe a readable buffer; `strings`/
/// `strings_capacity` must describe a writable buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_engine_status_request(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxEngineStatusRequestC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::EngineStatusRequest,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(_msg) = decode_engine_status_request(&frame) else {
            return 0;
        };
        // SAFETY: `strings_needed` is writable (checked above).
        unsafe { *strings_needed = 0 };
        1
    });
    result.unwrap_or(0)
}

/// # Safety
/// `metadata`, `out`, `strings_needed` must be writable/valid pointers;
/// `body`/`body_length` must describe a readable buffer; `strings`/
/// `strings_capacity` must describe a writable buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_engine_status_response(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxEngineStatusResponseC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::EngineStatusResponse,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(msg) = decode_engine_status_response(&frame) else {
            return 0;
        };
        let needed = msg.current_input_method_id.len()
            + msg.current_input_method_name.len()
            + msg.current_input_method_native_name.len()
            + msg.current_input_method_short_label.len();
        if needed > strings_capacity {
            // SAFETY: `strings_needed` is writable (checked above).
            unsafe { *strings_needed = needed };
            return 0;
        }
        // SAFETY: `strings` is writable for `strings_capacity` bytes.
        let mut arena = ArenaWriter::new(strings, strings_capacity);
        // SAFETY: `out` is writable (checked above).
        let o = unsafe { &mut *out };
        o.status = msg.status as u32;
        // SAFETY: arena capacity pre-verified.
        o.current_input_method_id = unsafe { arena.write(&msg.current_input_method_id) };
        // SAFETY: arena capacity pre-verified.
        o.current_input_method_name = unsafe { arena.write(&msg.current_input_method_name) };
        // SAFETY: arena capacity pre-verified.
        o.current_input_method_native_name =
            unsafe { arena.write(&msg.current_input_method_native_name) };
        // SAFETY: arena capacity pre-verified.
        o.current_input_method_short_label =
            unsafe { arena.write(&msg.current_input_method_short_label) };
        // SAFETY: `strings_needed` is writable (checked above).
        unsafe { *strings_needed = needed };
        1
    });
    result.unwrap_or(0)
}

/// # Safety
/// `metadata`, `out`, `strings_needed` must be writable/valid pointers;
/// `body`/`body_length` must describe a readable buffer; `strings`/
/// `strings_capacity` must describe a writable buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_launcher_request(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxLauncherRequestC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::LauncherRequest,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(msg) = decode_launcher_request(&frame) else {
            return 0;
        };
        // SAFETY: `out` is writable (checked above).
        let o = unsafe { &mut *out };
        o.command = msg.command as u32;
        // SAFETY: `strings_needed` is writable (checked above).
        unsafe { *strings_needed = 0 };
        1
    });
    result.unwrap_or(0)
}

/// # Safety
/// `metadata`, `out`, `strings_needed` must be writable/valid pointers;
/// `body`/`body_length` must describe a readable buffer; `strings`/
/// `strings_capacity` must describe a writable buffer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_decode_launcher_response(
    metadata: *const FcitxMetadataC,
    body: *const u8,
    body_length: usize,
    out: *mut FcitxLauncherResponseC,
    strings: *mut u8,
    strings_capacity: usize,
    strings_needed: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if metadata.is_null()
            || out.is_null()
            || strings_needed.is_null()
            || (body.is_null() && body_length != 0)
            || (strings.is_null() && strings_capacity != 0)
        {
            return 0;
        }
        let frame = FrameView {
            message_type: MessageType::LauncherResponse,
            metadata: metadata_from_c(unsafe { &*metadata }),
            body: if body_length == 0 {
                &[][..]
            } else {
                // SAFETY: caller provides a valid buffer of `body_length`.
                unsafe { std::slice::from_raw_parts(body, body_length) }
            },
        };
        let Some(msg) = decode_launcher_response(&frame) else {
            return 0;
        };
        let needed = msg.current_input_method_id.len()
            + msg.current_input_method_name.len()
            + msg.current_input_method_native_name.len()
            + msg.current_input_method_short_label.len();
        if needed > strings_capacity {
            // SAFETY: `strings_needed` is writable (checked above).
            unsafe { *strings_needed = needed };
            return 0;
        }
        // SAFETY: `strings` is writable for `strings_capacity` bytes.
        let mut arena = ArenaWriter::new(strings, strings_capacity);
        // SAFETY: `out` is writable (checked above).
        let o = unsafe { &mut *out };
        o.status = msg.status as u32;
        o.launcher_state = msg.launcher_state;
        o.engine_state = msg.engine_state;
        o.start_disposition = msg.start_disposition;
        o.safe_mode = msg.safe_mode as u8;
        o.retry_after_milliseconds = msg.retry_after_milliseconds;
        // SAFETY: arena capacity pre-verified.
        o.current_input_method_id = unsafe { arena.write(&msg.current_input_method_id) };
        // SAFETY: arena capacity pre-verified.
        o.current_input_method_name = unsafe { arena.write(&msg.current_input_method_name) };
        // SAFETY: arena capacity pre-verified.
        o.current_input_method_native_name =
            unsafe { arena.write(&msg.current_input_method_native_name) };
        // SAFETY: arena capacity pre-verified.
        o.current_input_method_short_label =
            unsafe { arena.write(&msg.current_input_method_short_label) };
        // SAFETY: `strings_needed` is writable (checked above).
        unsafe { *strings_needed = needed };
        1
    });
    result.unwrap_or(0)
}

#[cfg(test)]
#[path = "capi_tests.rs"]
mod capi_tests;
