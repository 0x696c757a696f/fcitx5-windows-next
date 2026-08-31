//! Shared FCW4 wire protocol codec.
//!
//! This crate is the Rust-authoritative implementation of the FCW4 frame
//! protocol previously defined by `protocol/protocol.h` + `protocol/protocol.cpp`.
//! Wire compatibility (bytes, limits, and decode rejection) is preserved exactly:
//! the UTF-8 validation below deliberately mirrors the C++ byte-structure check
//! (which accepts overlong encodings), not the stricter Rust standard-library
//! `from_utf8` semantics.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::c_void;
use std::panic;

// ---------------------------------------------------------------------------
// Limits (mirror `protocol/protocol.h`)
// ---------------------------------------------------------------------------

pub const MAGIC: u32 = 0x3457_4346; // "FCW4"
pub const VERSION: u16 = 14;
pub const HEADER_SIZE: usize = 64;
pub const MAX_HOT_FRAME_SIZE: usize = 256 * 1024;
pub const MAX_CONTROL_FRAME_SIZE: usize = 1024 * 1024;
pub const MAX_FRAME_SIZE: usize = MAX_HOT_FRAME_SIZE;
pub const MAX_COMMIT_UTF8: usize = 16 * 1024;
pub const MAX_PREEDIT_UTF8: usize = 16 * 1024;
pub const MAX_CANDIDATES: usize = 128;
pub const MAX_CANDIDATE_FIELD_UTF8: usize = 4096;
pub const MAX_LOGICAL_KEY_UTF8: usize = 64;
pub const MAX_INPUT_METHOD_ID_UTF8: usize = 64;
pub const MAX_INPUT_METHOD_NAME_UTF8: usize = 128;
pub const MAX_LOCALE_UTF8: usize = 35;
pub const MAX_SURROUNDING_TEXT_UTF8: usize = 16 * 1024;

pub const KEY_FLAG_SHIFT: u32 = 1 << 0;
pub const KEY_FLAG_CONTROL: u32 = 1 << 1;
pub const KEY_FLAG_ALT: u32 = 1 << 2;
pub const KEY_FLAG_SUPER: u32 = 1 << 3;
pub const KEY_FLAG_RELEASE: u32 = 1 << 4;
pub const KEY_FLAG_ALTGR: u32 = 1 << 5;
pub const KEY_FLAG_DEAD_KEY: u32 = 1 << 6;
pub const KNOWN_KEY_FLAGS: u32 = KEY_FLAG_SHIFT
    | KEY_FLAG_CONTROL
    | KEY_FLAG_ALT
    | KEY_FLAG_SUPER
    | KEY_FLAG_RELEASE
    | KEY_FLAG_ALTGR
    | KEY_FLAG_DEAD_KEY;

pub const COORDINATE_LIMIT: i32 = 1_000_000;

// ---------------------------------------------------------------------------
// Enums and DTOs (mirror `protocol/protocol.h`)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum MessageType {
    HelloRequest = 1,
    HelloResponse = 2,
    KeyRequest = 3,
    KeyResponse = 4,
    LauncherRequest = 5,
    LauncherResponse = 6,
    CandidateSelectRequest = 7,
    CandidateSelectResponse = 8,
    StateRequest = 9,
    EngineStatusRequest = 10,
    EngineStatusResponse = 11,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum Status {
    Ok = 0,
    #[default]
    Malformed = 1,
    VersionMismatch = 2,
    Unsupported = 3,
    StaleIdentity = 4,
    AccessDenied = 5,
}

pub const STATUS_ACCESS_DENIED: u32 = Status::AccessDenied as u32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Metadata {
    pub request_id: u64,
    pub response_to: u64,
    pub engine_epoch: u64,
    pub session_id: u32,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CaretRect {
    pub valid: bool,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub dpi: u32,
}

impl Default for CaretRect {
    fn default() -> Self {
        // Mirrors the C++ default `CaretRect` (dpi defaults to 96).
        CaretRect {
            valid: false,
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
            dpi: 96,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelloRequest {
    pub metadata: Metadata,
    pub client_architecture_bits: u32,
    pub client_process_id: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HelloResponse {
    pub metadata: Metadata,
    pub status: Status,
    pub server_architecture_bits: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyRequest {
    pub metadata: Metadata,
    pub virtual_key: u32,
    pub key_flags: u32,
    pub scan_code: u32,
    pub extended_key: bool,
    pub popup_allowed: bool,
    pub keyboard_layout: u64,
    pub logical_text_utf8: Vec<u8>,
    pub input_method_utf8: Vec<u8>,
    pub surrounding_text_valid: bool,
    pub surrounding_text_utf8: Vec<u8>,
    pub surrounding_cursor: u32,
    pub surrounding_anchor: u32,
    pub caret: CaretRect,
}

impl Default for KeyRequest {
    fn default() -> Self {
        // Mirrors the C++ default `KeyRequest` (`popupAllowed{true}`).
        KeyRequest {
            metadata: Metadata::default(),
            virtual_key: 0,
            key_flags: 0,
            scan_code: 0,
            extended_key: false,
            popup_allowed: true,
            keyboard_layout: 0,
            logical_text_utf8: Vec::new(),
            input_method_utf8: Vec::new(),
            surrounding_text_valid: false,
            surrounding_text_utf8: Vec::new(),
            surrounding_cursor: 0,
            surrounding_anchor: 0,
            caret: CaretRect::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateRecord {
    pub id: u64,
    pub label_utf8: Vec<u8>,
    pub text_utf8: Vec<u8>,
    pub comment_utf8: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyResponse {
    pub metadata: Metadata,
    pub status: Status,
    pub handled: bool,
    pub commit_utf8: Vec<u8>,
    pub preedit_utf8: Vec<u8>,
    pub preedit_caret_utf8: u32,
    pub candidates: Vec<CandidateRecord>,
    pub selected_candidate: u32,
    pub candidate_page: u32,
    pub candidate_total: u32,
    pub candidate_visibility: u8,
    pub candidate_page_size: u32,
    pub candidate_bulk: bool,
    pub candidate_end: bool,
    pub delete_surrounding_text: bool,
    pub delete_surrounding_offset: i32,
    pub delete_surrounding_size: u32,
    pub forward_key: bool,
    pub forward_key_sym: u32,
    pub forward_key_states: u32,
    pub forward_key_code: i32,
    pub forward_key_release: bool,
    pub caret: CaretRect,
    pub popup_allowed: bool,
    pub content_locale_utf8: Vec<u8>,
}

impl Default for KeyResponse {
    fn default() -> Self {
        // Mirrors the C++ default `KeyResponse`:
        // `selectedCandidate{UINT32_MAX}` and `popupAllowed{true}`.
        KeyResponse {
            metadata: Metadata::default(),
            status: Status::default(),
            handled: false,
            commit_utf8: Vec::new(),
            preedit_utf8: Vec::new(),
            preedit_caret_utf8: 0,
            candidates: Vec::new(),
            selected_candidate: u32::MAX,
            candidate_page: 0,
            candidate_total: 0,
            candidate_visibility: 0,
            candidate_page_size: 0,
            candidate_bulk: false,
            candidate_end: false,
            delete_surrounding_text: false,
            delete_surrounding_offset: 0,
            delete_surrounding_size: 0,
            forward_key: false,
            forward_key_sym: 0,
            forward_key_states: 0,
            forward_key_code: 0,
            forward_key_release: false,
            caret: CaretRect::default(),
            popup_allowed: true,
            content_locale_utf8: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateSelectRequest {
    pub metadata: Metadata,
    pub target_process_id: u32,
    pub candidate_id: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CandidateSelectResponse {
    pub metadata: Metadata,
    pub status: Status,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StateRequest {
    pub metadata: Metadata,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EngineStatusRequest {
    pub metadata: Metadata,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EngineStatusResponse {
    pub metadata: Metadata,
    pub status: Status,
    pub current_input_method_id: Vec<u8>,
    pub current_input_method_name: Vec<u8>,
    pub current_input_method_native_name: Vec<u8>,
    pub current_input_method_short_label: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum LauncherCommand {
    StartDemand = 1,
    UserStop = 2,
    Resume = 3,
    BeginUpdate = 4,
    EndUpdate = 5,
    BeginUninstall = 6,
    ResetSafeMode = 7,
    #[default]
    Status = 8,
    Shutdown = 9,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LauncherRequest {
    pub metadata: Metadata,
    pub command: LauncherCommand,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LauncherResponse {
    pub metadata: Metadata,
    pub status: Status,
    pub launcher_state: u32,
    pub engine_state: u32,
    pub start_disposition: u32,
    pub safe_mode: bool,
    pub retry_after_milliseconds: u64,
    pub current_input_method_id: Vec<u8>,
    pub current_input_method_name: Vec<u8>,
    pub current_input_method_native_name: Vec<u8>,
    pub current_input_method_short_label: Vec<u8>,
}

// ---------------------------------------------------------------------------
// Writer / Reader (little-endian, mirror `protocol/protocol.cpp`)
// ---------------------------------------------------------------------------

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn new(message_type: MessageType, metadata: &Metadata) -> Self {
        let mut writer = Writer { bytes: Vec::new() };
        writer.bytes.reserve(HEADER_SIZE);
        writer.append_u32(MAGIC);
        writer.append_u16(VERSION);
        writer.append_u16(message_type as u16);
        writer.append_u32(0); // body size placeholder
        writer.append_u64(metadata.request_id);
        writer.append_u64(metadata.response_to);
        writer.append_u64(metadata.engine_epoch);
        writer.append_u32(metadata.session_id);
        writer.append_u64(metadata.context_id);
        writer.append_u64(metadata.composition_id);
        writer.append_u64(metadata.revision);
        writer
    }

    fn append_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn append_u32(&mut self, value: u32) {
        for shift in (0..32).step_by(8) {
            self.bytes.push(((value >> shift) & 0xff) as u8);
        }
    }

    fn append_i32(&mut self, value: i32) {
        self.append_u32(value as u32);
    }

    fn append_u16(&mut self, value: u16) {
        self.bytes.push((value & 0xff) as u8);
        self.bytes.push(((value >> 8) & 0xff) as u8);
    }

    fn append_u64(&mut self, value: u64) {
        for shift in (0..64).step_by(8) {
            self.bytes.push(((value >> shift) & 0xff) as u8);
        }
    }

    fn append_string(&mut self, value: &[u8]) {
        self.append_u32(value.len() as u32);
        self.bytes.extend_from_slice(value);
    }

    fn finish(mut self) -> Vec<u8> {
        let body_size = (self.bytes.len() - HEADER_SIZE) as u32;
        for index in 0..4 {
            self.bytes[8 + index] = ((body_size >> (index * 8)) & 0xff) as u8;
        }
        self.bytes
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes, offset: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }

    fn read_u8(&mut self) -> Option<u8> {
        if self.remaining() < 1 {
            return None;
        }
        let value = self.bytes[self.offset];
        self.offset += 1;
        Some(value)
    }

    fn read_u16(&mut self) -> Option<u16> {
        if self.remaining() < 2 {
            return None;
        }
        let value =
            u16::from(self.bytes[self.offset]) | (u16::from(self.bytes[self.offset + 1]) << 8);
        self.offset += 2;
        Some(value)
    }

    fn read_u32(&mut self) -> Option<u32> {
        if self.remaining() < 4 {
            return None;
        }
        let mut value = 0u32;
        for index in 0..4 {
            value |= u32::from(self.bytes[self.offset + index]) << (index * 8);
        }
        self.offset += 4;
        Some(value)
    }

    fn read_i32(&mut self) -> Option<i32> {
        self.read_u32().map(|raw| raw as i32)
    }

    fn read_u64(&mut self) -> Option<u64> {
        if self.remaining() < 8 {
            return None;
        }
        let mut value = 0u64;
        for index in 0..8 {
            value |= u64::from(self.bytes[self.offset + index]) << (index * 8);
        }
        self.offset += 8;
        Some(value)
    }

    fn read_string(&mut self) -> Option<Vec<u8>> {
        let size = self.read_u32()? as usize;
        if size > self.remaining() {
            return None;
        }
        let bytes = self.bytes[self.offset..self.offset + size].to_vec();
        self.offset += size;
        Some(bytes)
    }

    fn done(&self) -> bool {
        self.offset == self.bytes.len()
    }
}

// ---------------------------------------------------------------------------
// Validation (mirror `protocol/protocol.cpp` byte-for-byte semantics)
// ---------------------------------------------------------------------------

fn is_request(message_type: MessageType) -> bool {
    matches!(
        message_type,
        MessageType::HelloRequest
            | MessageType::KeyRequest
            | MessageType::LauncherRequest
            | MessageType::CandidateSelectRequest
            | MessageType::StateRequest
            | MessageType::EngineStatusRequest
    )
}

fn valid_metadata(message_type: MessageType, metadata: &Metadata) -> bool {
    if metadata.request_id == 0 {
        return false;
    }
    if is_request(message_type) {
        metadata.response_to == 0
    } else {
        metadata.response_to != 0
    }
}

fn valid_hello_request_metadata(metadata: &Metadata) -> bool {
    metadata.engine_epoch == 0
        && metadata.context_id == 0
        && metadata.composition_id == 0
        && metadata.revision == 0
}

fn valid_hello_response_metadata(metadata: &Metadata) -> bool {
    metadata.engine_epoch != 0
        && metadata.context_id == 0
        && metadata.composition_id == 0
        && metadata.revision == 0
}

fn valid_key_metadata(metadata: &Metadata) -> bool {
    metadata.engine_epoch != 0 && metadata.context_id != 0
}

fn valid_engine_status_metadata(metadata: &Metadata) -> bool {
    metadata.engine_epoch != 0
        && metadata.context_id == 0
        && metadata.composition_id == 0
        && metadata.revision == 0
}

fn valid_launcher_metadata(metadata: &Metadata) -> bool {
    metadata.context_id == 0 && metadata.composition_id == 0 && metadata.revision == 0
}

fn valid_caret(caret: &CaretRect) -> bool {
    if !caret.valid {
        return caret.left == 0
            && caret.top == 0
            && caret.right == 0
            && caret.bottom == 0
            && caret.dpi == 96;
    }
    caret.left >= -COORDINATE_LIMIT
        && caret.top >= -COORDINATE_LIMIT
        && caret.right <= COORDINATE_LIMIT
        && caret.bottom <= COORDINATE_LIMIT
        && caret.right >= caret.left
        && caret.bottom >= caret.top
        && caret.dpi >= 48
        && caret.dpi <= 960
}

/// Byte-structure UTF-8 check matching the C++ implementation exactly.
/// Returns the code-point count when the text is structurally valid.
fn utf8_code_point_count(text: &[u8]) -> Option<usize> {
    let mut count = 0usize;
    let mut index = 0usize;
    while index < text.len() {
        let byte = text[index];
        let length = if byte & 0x80 == 0 {
            1
        } else if byte & 0xe0 == 0xc0 {
            if byte < 0xc2 {
                return None;
            }
            2
        } else if byte & 0xf0 == 0xe0 {
            3
        } else if byte & 0xf8 == 0xf0 {
            if byte > 0xf4 {
                return None;
            }
            4
        } else {
            return None;
        };
        if index + length > text.len() {
            return None;
        }
        for offset in 1..length {
            if text[index + offset] & 0xc0 != 0x80 {
                return None;
            }
        }
        index += length;
        count += 1;
    }
    Some(count)
}

fn valid_surrounding_text(message: &KeyRequest) -> bool {
    if !message.surrounding_text_valid {
        return message.surrounding_text_utf8.is_empty()
            && message.surrounding_cursor == 0
            && message.surrounding_anchor == 0;
    }
    if message.surrounding_text_utf8.len() > MAX_SURROUNDING_TEXT_UTF8 {
        return false;
    }
    match utf8_code_point_count(&message.surrounding_text_utf8) {
        Some(length) => {
            message.surrounding_cursor as usize <= length
                && message.surrounding_anchor as usize <= length
        }
        None => false,
    }
}

fn valid_input_method_text(text: &[u8], maximum_bytes: usize) -> bool {
    text.len() <= maximum_bytes && utf8_code_point_count(text).is_some()
}

fn valid_input_method_status(
    id: &[u8],
    name: &[u8],
    native_name: &[u8],
    short_label: &[u8],
) -> bool {
    valid_input_method_text(id, MAX_INPUT_METHOD_ID_UTF8)
        && valid_input_method_text(name, MAX_INPUT_METHOD_NAME_UTF8)
        && valid_input_method_text(native_name, MAX_INPUT_METHOD_NAME_UTF8)
        && valid_input_method_text(short_label, MAX_INPUT_METHOD_NAME_UTF8)
}

fn valid_launcher_response_payload(message: &LauncherResponse) -> bool {
    valid_input_method_status(
        &message.current_input_method_id,
        &message.current_input_method_name,
        &message.current_input_method_native_name,
        &message.current_input_method_short_label,
    )
}

fn valid_engine_status_response_payload(message: &EngineStatusResponse) -> bool {
    valid_input_method_status(
        &message.current_input_method_id,
        &message.current_input_method_name,
        &message.current_input_method_native_name,
        &message.current_input_method_short_label,
    )
}

fn valid_content_locale(locale: &[u8]) -> bool {
    locale.len() <= MAX_LOCALE_UTF8
        && locale
            .iter()
            .copied()
            .all(|character| character.is_ascii_alphanumeric() || character == b'-')
}

fn valid_key_response_payload(message: &KeyResponse) -> bool {
    if message.commit_utf8.len() > MAX_COMMIT_UTF8
        || message.preedit_utf8.len() > MAX_PREEDIT_UTF8
        || message.preedit_caret_utf8 as usize > message.preedit_utf8.len()
        || message.candidates.len() > MAX_CANDIDATES
        || message.candidate_visibility > 2
        || message.candidate_page_size as usize > MAX_CANDIDATES
        || (message.selected_candidate != u32::MAX
            && message.selected_candidate as usize >= message.candidates.len())
        || message.candidate_total < message.candidates.len() as u32
        || !valid_caret(&message.caret)
    {
        return false;
    }
    if message.candidate_visibility == 0 && !message.candidates.is_empty() {
        return false;
    }
    if !message.delete_surrounding_text
        && (message.delete_surrounding_offset != 0 || message.delete_surrounding_size != 0)
    {
        return false;
    }
    if !message.forward_key
        && (message.forward_key_sym != 0
            || message.forward_key_states != 0
            || message.forward_key_code != 0
            || message.forward_key_release)
    {
        return false;
    }
    if !valid_content_locale(&message.content_locale_utf8) {
        return false;
    }
    message.candidates.iter().all(|candidate| {
        candidate.id != 0
            && candidate.label_utf8.len() <= MAX_CANDIDATE_FIELD_UTF8
            && candidate.text_utf8.len() <= MAX_CANDIDATE_FIELD_UTF8
            && candidate.comment_utf8.len() <= MAX_CANDIDATE_FIELD_UTF8
    })
}

fn valid_key_request_payload(message: &KeyRequest) -> bool {
    (message.key_flags & !KNOWN_KEY_FLAGS) == 0
        && message.scan_code <= 0xff
        && message.logical_text_utf8.len() <= MAX_LOGICAL_KEY_UTF8
        && message.input_method_utf8.len() <= MAX_INPUT_METHOD_ID_UTF8
        && valid_surrounding_text(message)
        && valid_caret(&message.caret)
}

fn maximum_frame_size(message_type: MessageType) -> usize {
    if matches!(
        message_type,
        MessageType::LauncherRequest | MessageType::LauncherResponse
    ) {
        MAX_CONTROL_FRAME_SIZE
    } else {
        MAX_HOT_FRAME_SIZE
    }
}

fn status_in_range(status: Status) -> bool {
    (status as u32) <= STATUS_ACCESS_DENIED
}

// ---------------------------------------------------------------------------
// Header / frame decoding (mirror `protocol/protocol.cpp`)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
pub struct FrameView<'a> {
    pub message_type: MessageType,
    pub metadata: Metadata,
    pub body: &'a [u8],
}

fn message_type_from_raw(raw: u16) -> Option<MessageType> {
    match raw {
        1 => Some(MessageType::HelloRequest),
        2 => Some(MessageType::HelloResponse),
        3 => Some(MessageType::KeyRequest),
        4 => Some(MessageType::KeyResponse),
        5 => Some(MessageType::LauncherRequest),
        6 => Some(MessageType::LauncherResponse),
        7 => Some(MessageType::CandidateSelectRequest),
        8 => Some(MessageType::CandidateSelectResponse),
        9 => Some(MessageType::StateRequest),
        10 => Some(MessageType::EngineStatusRequest),
        11 => Some(MessageType::EngineStatusResponse),
        _ => None,
    }
}

pub fn decode_header(bytes: &[u8]) -> Option<(MessageType, u32, Metadata)> {
    if bytes.len() != HEADER_SIZE {
        return None;
    }
    let mut reader = Reader::new(bytes);
    let magic = reader.read_u32()?;
    let version = reader.read_u16()?;
    let raw_type = reader.read_u16()?;
    let body_size = reader.read_u32()?;
    let metadata = Metadata {
        request_id: reader.read_u64()?,
        response_to: reader.read_u64()?,
        engine_epoch: reader.read_u64()?,
        session_id: reader.read_u32()?,
        context_id: reader.read_u64()?,
        composition_id: reader.read_u64()?,
        revision: reader.read_u64()?,
    };
    if !reader.done() {
        return None;
    }
    if magic != MAGIC || version != VERSION {
        return None;
    }
    let message_type = message_type_from_raw(raw_type)?;
    if body_size as usize > maximum_frame_size(message_type) - HEADER_SIZE {
        return None;
    }
    if !valid_metadata(message_type, &metadata) {
        return None;
    }
    Some((message_type, body_size, metadata))
}

pub fn decode_frame(bytes: &[u8]) -> Option<FrameView<'_>> {
    if bytes.len() < HEADER_SIZE || bytes.len() > MAX_CONTROL_FRAME_SIZE {
        return None;
    }
    let (message_type, body_size, metadata) = decode_header(&bytes[..HEADER_SIZE])?;
    if body_size as usize != bytes.len() - HEADER_SIZE {
        return None;
    }
    Some(FrameView {
        message_type,
        metadata,
        body: &bytes[HEADER_SIZE..],
    })
}

// ---------------------------------------------------------------------------
// Message encoding (mirror `protocol/protocol.cpp`; empty result = rejection)
// ---------------------------------------------------------------------------

pub fn encode_hello_request(message: &HelloRequest) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::HelloRequest, &message.metadata)
        || !valid_hello_request_metadata(&message.metadata)
        || (message.client_architecture_bits != 32 && message.client_architecture_bits != 64)
        || message.client_process_id == 0
    {
        return None;
    }
    let mut writer = Writer::new(MessageType::HelloRequest, &message.metadata);
    writer.append_u32(message.client_architecture_bits);
    writer.append_u32(message.client_process_id);
    Some(writer.finish())
}

pub fn encode_hello_response(message: &HelloResponse) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::HelloResponse, &message.metadata)
        || !valid_hello_response_metadata(&message.metadata)
        || (message.server_architecture_bits != 32 && message.server_architecture_bits != 64)
        || !status_in_range(message.status)
    {
        return None;
    }
    let mut writer = Writer::new(MessageType::HelloResponse, &message.metadata);
    writer.append_u32(message.status as u32);
    writer.append_u32(message.server_architecture_bits);
    Some(writer.finish())
}

pub fn encode_key_request(message: &KeyRequest) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::KeyRequest, &message.metadata)
        || !valid_key_metadata(&message.metadata)
        || !valid_key_request_payload(message)
    {
        return None;
    }
    let mut writer = Writer::new(MessageType::KeyRequest, &message.metadata);
    writer.append_u32(message.virtual_key);
    writer.append_u32(message.key_flags);
    writer.append_u32(message.scan_code);
    writer.append_u8(message.extended_key as u8);
    writer.append_u8(message.popup_allowed as u8);
    writer.append_u64(message.keyboard_layout);
    writer.append_string(&message.logical_text_utf8);
    writer.append_string(&message.input_method_utf8);
    writer.append_u8(message.surrounding_text_valid as u8);
    writer.append_string(&message.surrounding_text_utf8);
    writer.append_u32(message.surrounding_cursor);
    writer.append_u32(message.surrounding_anchor);
    writer.append_u8(message.caret.valid as u8);
    writer.append_i32(message.caret.left);
    writer.append_i32(message.caret.top);
    writer.append_i32(message.caret.right);
    writer.append_i32(message.caret.bottom);
    writer.append_u32(message.caret.dpi);
    Some(writer.finish())
}

pub fn encode_key_response(message: &KeyResponse) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::KeyResponse, &message.metadata)
        || !valid_key_metadata(&message.metadata)
        || !valid_key_response_payload(message)
        || !status_in_range(message.status)
    {
        return None;
    }
    let mut writer = Writer::new(MessageType::KeyResponse, &message.metadata);
    writer.append_u32(message.status as u32);
    writer.append_u8(message.handled as u8);
    writer.append_string(&message.commit_utf8);
    writer.append_string(&message.preedit_utf8);
    writer.append_u32(message.preedit_caret_utf8);
    writer.append_u32(message.candidates.len() as u32);
    writer.append_u32(message.selected_candidate);
    writer.append_u32(message.candidate_page);
    writer.append_u32(message.candidate_total);
    writer.append_u8(message.candidate_visibility);
    writer.append_u32(message.candidate_page_size);
    writer.append_u8(message.candidate_bulk as u8);
    writer.append_u8(message.candidate_end as u8);
    writer.append_u8(message.delete_surrounding_text as u8);
    writer.append_i32(message.delete_surrounding_offset);
    writer.append_u32(message.delete_surrounding_size);
    writer.append_u8(message.forward_key as u8);
    writer.append_u32(message.forward_key_sym);
    writer.append_u32(message.forward_key_states);
    writer.append_i32(message.forward_key_code);
    writer.append_u8(message.forward_key_release as u8);
    writer.append_u8(message.caret.valid as u8);
    writer.append_i32(message.caret.left);
    writer.append_i32(message.caret.top);
    writer.append_i32(message.caret.right);
    writer.append_i32(message.caret.bottom);
    writer.append_u32(message.caret.dpi);
    writer.append_u8(message.popup_allowed as u8);
    writer.append_string(&message.content_locale_utf8);
    for candidate in &message.candidates {
        writer.append_u64(candidate.id);
        writer.append_string(&candidate.label_utf8);
        writer.append_string(&candidate.text_utf8);
        writer.append_string(&candidate.comment_utf8);
    }
    Some(writer.finish())
}

pub fn encode_candidate_select_request(message: &CandidateSelectRequest) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::CandidateSelectRequest, &message.metadata)
        || !valid_key_metadata(&message.metadata)
        || message.target_process_id == 0
        || message.candidate_id == 0
    {
        return None;
    }
    let mut writer = Writer::new(MessageType::CandidateSelectRequest, &message.metadata);
    writer.append_u32(message.target_process_id);
    writer.append_u64(message.candidate_id);
    Some(writer.finish())
}

pub fn encode_candidate_select_response(message: &CandidateSelectResponse) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::CandidateSelectResponse, &message.metadata)
        || !valid_key_metadata(&message.metadata)
        || !status_in_range(message.status)
    {
        return None;
    }
    let mut writer = Writer::new(MessageType::CandidateSelectResponse, &message.metadata);
    writer.append_u32(message.status as u32);
    Some(writer.finish())
}

pub fn encode_state_request(message: &StateRequest) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::StateRequest, &message.metadata)
        || !valid_key_metadata(&message.metadata)
    {
        return None;
    }
    let writer = Writer::new(MessageType::StateRequest, &message.metadata);
    Some(writer.finish())
}

pub fn encode_engine_status_request(message: &EngineStatusRequest) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::EngineStatusRequest, &message.metadata)
        || !valid_engine_status_metadata(&message.metadata)
    {
        return None;
    }
    let writer = Writer::new(MessageType::EngineStatusRequest, &message.metadata);
    Some(writer.finish())
}

pub fn encode_engine_status_response(message: &EngineStatusResponse) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::EngineStatusResponse, &message.metadata)
        || !valid_engine_status_metadata(&message.metadata)
        || !valid_engine_status_response_payload(message)
        || !status_in_range(message.status)
    {
        return None;
    }
    let mut writer = Writer::new(MessageType::EngineStatusResponse, &message.metadata);
    writer.append_u32(message.status as u32);
    writer.append_string(&message.current_input_method_id);
    writer.append_string(&message.current_input_method_name);
    writer.append_string(&message.current_input_method_native_name);
    writer.append_string(&message.current_input_method_short_label);
    Some(writer.finish())
}

pub fn encode_launcher_request(message: &LauncherRequest) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::LauncherRequest, &message.metadata)
        || !valid_launcher_metadata(&message.metadata)
        || (message.command as u32) < LauncherCommand::StartDemand as u32
        || (message.command as u32) > LauncherCommand::Shutdown as u32
    {
        return None;
    }
    let mut writer = Writer::new(MessageType::LauncherRequest, &message.metadata);
    writer.append_u32(message.command as u32);
    Some(writer.finish())
}

pub fn encode_launcher_response(message: &LauncherResponse) -> Option<Vec<u8>> {
    if !valid_metadata(MessageType::LauncherResponse, &message.metadata)
        || !valid_launcher_metadata(&message.metadata)
        || !valid_launcher_response_payload(message)
        || !status_in_range(message.status)
    {
        return None;
    }
    let mut writer = Writer::new(MessageType::LauncherResponse, &message.metadata);
    writer.append_u32(message.status as u32);
    writer.append_u32(message.launcher_state);
    writer.append_u32(message.engine_state);
    writer.append_u32(message.start_disposition);
    writer.append_u8(message.safe_mode as u8);
    writer.append_u64(message.retry_after_milliseconds);
    writer.append_string(&message.current_input_method_id);
    writer.append_string(&message.current_input_method_name);
    writer.append_string(&message.current_input_method_native_name);
    writer.append_string(&message.current_input_method_short_label);
    Some(writer.finish())
}

// ---------------------------------------------------------------------------
// Message decoding (mirror `protocol/protocol.cpp`)
// ---------------------------------------------------------------------------

pub fn decode_hello_request(frame: &FrameView<'_>) -> Option<HelloRequest> {
    if frame.message_type != MessageType::HelloRequest
        || !valid_hello_request_metadata(&frame.metadata)
    {
        return None;
    }
    let mut reader = Reader::new(frame.body);
    let client_architecture_bits = reader.read_u32()?;
    let client_process_id = reader.read_u32()?;
    if !reader.done()
        || (client_architecture_bits != 32 && client_architecture_bits != 64)
        || client_process_id == 0
    {
        return None;
    }
    Some(HelloRequest {
        metadata: frame.metadata,
        client_architecture_bits,
        client_process_id,
    })
}

pub fn decode_hello_response(frame: &FrameView<'_>) -> Option<HelloResponse> {
    if frame.message_type != MessageType::HelloResponse
        || !valid_hello_response_metadata(&frame.metadata)
    {
        return None;
    }
    let mut reader = Reader::new(frame.body);
    let status = reader.read_u32()?;
    let server_architecture_bits = reader.read_u32()?;
    if !reader.done()
        || status > STATUS_ACCESS_DENIED
        || (server_architecture_bits != 32 && server_architecture_bits != 64)
    {
        return None;
    }
    Some(HelloResponse {
        metadata: frame.metadata,
        status: status_from_raw(status),
        server_architecture_bits,
    })
}

fn status_from_raw(raw: u32) -> Status {
    match raw {
        0 => Status::Ok,
        1 => Status::Malformed,
        2 => Status::VersionMismatch,
        3 => Status::Unsupported,
        4 => Status::StaleIdentity,
        _ => Status::AccessDenied,
    }
}

pub fn decode_key_request(frame: &FrameView<'_>) -> Option<KeyRequest> {
    if frame.message_type != MessageType::KeyRequest || !valid_key_metadata(&frame.metadata) {
        return None;
    }
    let mut reader = Reader::new(frame.body);
    let virtual_key = reader.read_u32()?;
    let key_flags = reader.read_u32()?;
    let scan_code = reader.read_u32()?;
    let extended = reader.read_u8()?;
    if extended > 1 {
        return None;
    }
    let popup_allowed = reader.read_u8()?;
    if popup_allowed > 1 {
        return None;
    }
    let keyboard_layout = reader.read_u64()?;
    let logical_text_utf8 = reader.read_string()?;
    let input_method_utf8 = reader.read_string()?;
    let surrounding_text_valid = reader.read_u8()?;
    if surrounding_text_valid > 1 {
        return None;
    }
    let surrounding_text_utf8 = reader.read_string()?;
    let surrounding_cursor = reader.read_u32()?;
    let surrounding_anchor = reader.read_u32()?;
    let caret_valid = reader.read_u8()?;
    if caret_valid > 1 {
        return None;
    }
    let caret_left = reader.read_i32()?;
    let caret_top = reader.read_i32()?;
    let caret_right = reader.read_i32()?;
    let caret_bottom = reader.read_i32()?;
    let caret_dpi = reader.read_u32()?;
    if !reader.done() {
        return None;
    }
    let message = KeyRequest {
        metadata: frame.metadata,
        virtual_key,
        key_flags,
        scan_code,
        extended_key: extended != 0,
        popup_allowed: popup_allowed != 0,
        keyboard_layout,
        logical_text_utf8,
        input_method_utf8,
        surrounding_text_valid: surrounding_text_valid != 0,
        surrounding_text_utf8,
        surrounding_cursor,
        surrounding_anchor,
        caret: CaretRect {
            valid: caret_valid != 0,
            left: caret_left,
            top: caret_top,
            right: caret_right,
            bottom: caret_bottom,
            dpi: caret_dpi,
        },
    };
    if !valid_key_request_payload(&message) {
        return None;
    }
    Some(message)
}

pub fn decode_key_response(frame: &FrameView<'_>) -> Option<KeyResponse> {
    if frame.message_type != MessageType::KeyResponse || !valid_key_metadata(&frame.metadata) {
        return None;
    }
    let mut reader = Reader::new(frame.body);
    let status = reader.read_u32()?;
    let handled = reader.read_u8()?;
    if handled > 1 {
        return None;
    }
    let commit_utf8 = reader.read_string()?;
    let preedit_utf8 = reader.read_string()?;
    let preedit_caret_utf8 = reader.read_u32()?;
    let candidate_count = reader.read_u32()?;
    if candidate_count as usize > MAX_CANDIDATES {
        return None;
    }
    let selected_candidate = reader.read_u32()?;
    let candidate_page = reader.read_u32()?;
    let candidate_total = reader.read_u32()?;
    let candidate_visibility = reader.read_u8()?;
    let candidate_page_size = reader.read_u32()?;
    let candidate_bulk = reader.read_u8()?;
    if candidate_bulk > 1 {
        return None;
    }
    let candidate_end = reader.read_u8()?;
    if candidate_end > 1 {
        return None;
    }
    let delete_surrounding_text = reader.read_u8()?;
    if delete_surrounding_text > 1 {
        return None;
    }
    let delete_surrounding_offset = reader.read_i32()?;
    let delete_surrounding_size = reader.read_u32()?;
    let forward_key = reader.read_u8()?;
    if forward_key > 1 {
        return None;
    }
    let forward_key_sym = reader.read_u32()?;
    let forward_key_states = reader.read_u32()?;
    let forward_key_code = reader.read_i32()?;
    let forward_key_release = reader.read_u8()?;
    if forward_key_release > 1 {
        return None;
    }
    let caret_valid = reader.read_u8()?;
    if caret_valid > 1 {
        return None;
    }
    let caret_left = reader.read_i32()?;
    let caret_top = reader.read_i32()?;
    let caret_right = reader.read_i32()?;
    let caret_bottom = reader.read_i32()?;
    let caret_dpi = reader.read_u32()?;
    let popup_allowed = reader.read_u8()?;
    if popup_allowed > 1 {
        return None;
    }
    let content_locale_utf8 = reader.read_string()?;
    let mut candidates = Vec::with_capacity(candidate_count as usize);
    for _ in 0..candidate_count {
        let id = reader.read_u64()?;
        let label_utf8 = reader.read_string()?;
        let text_utf8 = reader.read_string()?;
        let comment_utf8 = reader.read_string()?;
        candidates.push(CandidateRecord {
            id,
            label_utf8,
            text_utf8,
            comment_utf8,
        });
    }
    if !reader.done() || status > STATUS_ACCESS_DENIED {
        return None;
    }
    let message = KeyResponse {
        metadata: frame.metadata,
        status: status_from_raw(status),
        handled: handled != 0,
        commit_utf8,
        preedit_utf8,
        preedit_caret_utf8,
        candidates,
        selected_candidate,
        candidate_page,
        candidate_total,
        candidate_visibility,
        candidate_page_size,
        candidate_bulk: candidate_bulk != 0,
        candidate_end: candidate_end != 0,
        delete_surrounding_text: delete_surrounding_text != 0,
        delete_surrounding_offset,
        delete_surrounding_size,
        forward_key: forward_key != 0,
        forward_key_sym,
        forward_key_states,
        forward_key_code,
        forward_key_release: forward_key_release != 0,
        caret: CaretRect {
            valid: caret_valid != 0,
            left: caret_left,
            top: caret_top,
            right: caret_right,
            bottom: caret_bottom,
            dpi: caret_dpi,
        },
        popup_allowed: popup_allowed != 0,
        content_locale_utf8,
    };
    if !valid_key_response_payload(&message) {
        return None;
    }
    Some(message)
}

pub fn decode_candidate_select_request(frame: &FrameView<'_>) -> Option<CandidateSelectRequest> {
    if frame.message_type != MessageType::CandidateSelectRequest
        || !valid_key_metadata(&frame.metadata)
    {
        return None;
    }
    let mut reader = Reader::new(frame.body);
    let target_process_id = reader.read_u32()?;
    let candidate_id = reader.read_u64()?;
    if !reader.done() || target_process_id == 0 || candidate_id == 0 {
        return None;
    }
    Some(CandidateSelectRequest {
        metadata: frame.metadata,
        target_process_id,
        candidate_id,
    })
}

pub fn decode_candidate_select_response(frame: &FrameView<'_>) -> Option<CandidateSelectResponse> {
    if frame.message_type != MessageType::CandidateSelectResponse
        || !valid_key_metadata(&frame.metadata)
    {
        return None;
    }
    let mut reader = Reader::new(frame.body);
    let status = reader.read_u32()?;
    if !reader.done() || status > STATUS_ACCESS_DENIED {
        return None;
    }
    Some(CandidateSelectResponse {
        metadata: frame.metadata,
        status: status_from_raw(status),
    })
}

pub fn decode_state_request(frame: &FrameView<'_>) -> Option<StateRequest> {
    if frame.message_type != MessageType::StateRequest
        || !valid_key_metadata(&frame.metadata)
        || !frame.body.is_empty()
    {
        return None;
    }
    Some(StateRequest {
        metadata: frame.metadata,
    })
}

pub fn decode_engine_status_request(frame: &FrameView<'_>) -> Option<EngineStatusRequest> {
    if frame.message_type != MessageType::EngineStatusRequest
        || !valid_engine_status_metadata(&frame.metadata)
        || !frame.body.is_empty()
    {
        return None;
    }
    Some(EngineStatusRequest {
        metadata: frame.metadata,
    })
}

pub fn decode_engine_status_response(frame: &FrameView<'_>) -> Option<EngineStatusResponse> {
    if frame.message_type != MessageType::EngineStatusResponse
        || !valid_engine_status_metadata(&frame.metadata)
    {
        return None;
    }
    let mut reader = Reader::new(frame.body);
    let status = reader.read_u32()?;
    let current_input_method_id = reader.read_string()?;
    let current_input_method_name = reader.read_string()?;
    let current_input_method_native_name = reader.read_string()?;
    let current_input_method_short_label = reader.read_string()?;
    if !reader.done() || status > STATUS_ACCESS_DENIED {
        return None;
    }
    let message = EngineStatusResponse {
        metadata: frame.metadata,
        status: status_from_raw(status),
        current_input_method_id,
        current_input_method_name,
        current_input_method_native_name,
        current_input_method_short_label,
    };
    if !valid_engine_status_response_payload(&message) {
        return None;
    }
    Some(message)
}

pub fn decode_launcher_request(frame: &FrameView<'_>) -> Option<LauncherRequest> {
    if frame.message_type != MessageType::LauncherRequest
        || !valid_launcher_metadata(&frame.metadata)
    {
        return None;
    }
    let mut reader = Reader::new(frame.body);
    let command = reader.read_u32()?;
    if !reader.done()
        || command < LauncherCommand::StartDemand as u32
        || command > LauncherCommand::Shutdown as u32
    {
        return None;
    }
    Some(LauncherRequest {
        metadata: frame.metadata,
        command: launcher_command_from_raw(command),
    })
}

fn launcher_command_from_raw(raw: u32) -> LauncherCommand {
    match raw {
        1 => LauncherCommand::StartDemand,
        2 => LauncherCommand::UserStop,
        3 => LauncherCommand::Resume,
        4 => LauncherCommand::BeginUpdate,
        5 => LauncherCommand::EndUpdate,
        6 => LauncherCommand::BeginUninstall,
        7 => LauncherCommand::ResetSafeMode,
        8 => LauncherCommand::Status,
        _ => LauncherCommand::Shutdown,
    }
}

pub fn decode_launcher_response(frame: &FrameView<'_>) -> Option<LauncherResponse> {
    if frame.message_type != MessageType::LauncherResponse
        || !valid_launcher_metadata(&frame.metadata)
    {
        return None;
    }
    let mut reader = Reader::new(frame.body);
    let status = reader.read_u32()?;
    let launcher_state = reader.read_u32()?;
    let engine_state = reader.read_u32()?;
    let start_disposition = reader.read_u32()?;
    let safe_mode = reader.read_u8()?;
    if safe_mode > 1 {
        return None;
    }
    let retry_after_milliseconds = reader.read_u64()?;
    let current_input_method_id = reader.read_string()?;
    let current_input_method_name = reader.read_string()?;
    let current_input_method_native_name = reader.read_string()?;
    let current_input_method_short_label = reader.read_string()?;
    if !reader.done() || status > STATUS_ACCESS_DENIED {
        return None;
    }
    let message = LauncherResponse {
        metadata: frame.metadata,
        status: status_from_raw(status),
        launcher_state,
        engine_state,
        start_disposition,
        safe_mode: safe_mode != 0,
        retry_after_milliseconds,
        current_input_method_id,
        current_input_method_name,
        current_input_method_native_name,
        current_input_method_short_label,
    };
    if !valid_launcher_response_payload(&message) {
        return None;
    }
    Some(message)
}

/// Typed decode + re-encode. `Some(bytes)` means the frame decoded and the
/// re-encoded bytes are identical to the input (byte-level roundtrip proof).
pub fn decode_and_reencode(bytes: &[u8]) -> Option<Vec<u8>> {
    let frame = decode_frame(bytes)?;
    match frame.message_type {
        MessageType::HelloRequest => encode_hello_request(&decode_hello_request(&frame)?),
        MessageType::HelloResponse => encode_hello_response(&decode_hello_response(&frame)?),
        MessageType::KeyRequest => encode_key_request(&decode_key_request(&frame)?),
        MessageType::KeyResponse => encode_key_response(&decode_key_response(&frame)?),
        MessageType::LauncherRequest => encode_launcher_request(&decode_launcher_request(&frame)?),
        MessageType::LauncherResponse => {
            encode_launcher_response(&decode_launcher_response(&frame)?)
        }
        MessageType::CandidateSelectRequest => {
            encode_candidate_select_request(&decode_candidate_select_request(&frame)?)
        }
        MessageType::CandidateSelectResponse => {
            encode_candidate_select_response(&decode_candidate_select_response(&frame)?)
        }
        MessageType::StateRequest => encode_state_request(&decode_state_request(&frame)?),
        MessageType::EngineStatusRequest => {
            encode_engine_status_request(&decode_engine_status_request(&frame)?)
        }
        MessageType::EngineStatusResponse => {
            encode_engine_status_response(&decode_engine_status_response(&frame)?)
        }
    }
}

// ---------------------------------------------------------------------------
// C ABI (narrow surface for the C++ differential gate and future cutover)
// ---------------------------------------------------------------------------

/// Returns 1 when `decode_frame` accepts the bytes (frame-level acceptance),
/// 0 otherwise. Mirrors C++ `decodeFrame` rejection exactly.
///
/// # Safety
/// `bytes` must describe a readable buffer of `length` bytes when `length` is
/// non-zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_accepts_frame(bytes: *const u8, length: usize) -> u8 {
    let result = panic::catch_unwind(|| {
        if bytes.is_null() && length != 0 {
            return 0;
        }
        let slice = if length == 0 {
            &[][..]
        } else {
            // SAFETY: caller provides a valid buffer of `length` bytes; the
            // pointer is non-null when `length` is non-zero.
            unsafe { std::slice::from_raw_parts(bytes, length) }
        };
        if decode_frame(slice).is_some() {
            1
        } else {
            0
        }
    });
    result.unwrap_or(0)
}

/// Typed decode + re-encode of a frame. Returns 1 and writes the re-encoded
/// bytes on success (byte-identical to the input when the input came from a
/// matching C++ encode), 0 on any decode/validation rejection.
///
/// # Safety
/// `bytes`/`length` must describe a readable buffer; `out`/`out_capacity`
/// must describe a writable buffer; `out_length` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn fcitx5_protocol_core_reencode_frame(
    bytes: *const u8,
    length: usize,
    out: *mut u8,
    out_capacity: usize,
    out_length: *mut usize,
) -> u8 {
    let result = panic::catch_unwind(|| {
        if (bytes.is_null() && length != 0)
            || (out.is_null() && out_capacity != 0)
            || out_length.is_null()
        {
            return 0;
        }
        let slice = if length == 0 {
            &[][..]
        } else {
            // SAFETY: caller provides a valid buffer of `length` bytes.
            unsafe { std::slice::from_raw_parts(bytes, length) }
        };
        let Some(reencoded) = decode_and_reencode(slice) else {
            return 0;
        };
        if reencoded.len() > out_capacity {
            return 0;
        }
        // SAFETY: `out` is writable for `out_capacity` bytes.
        unsafe { std::ptr::copy_nonoverlapping(reencoded.as_ptr(), out, reencoded.len()) };
        // SAFETY: `out_length` is writable.
        unsafe { *out_length = reencoded.len() };
        1
    });
    result.unwrap_or(0)
}

// Silence unused-import warning for the c_void fallback used by older rustc
// (kept for ABI stability documentation only).
#[allow(dead_code)]
fn _c_void_marker(_: *const c_void) {}

pub mod capi;

#[cfg(test)]
mod tests;
