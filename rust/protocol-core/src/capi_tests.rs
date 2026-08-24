//! C ABI layer tests: the exported `fcitx5_protocol_core_*` functions must
//! match the Rust typed API byte-for-byte and roundtrip through the flat C
//! structures.

use super::*;
use crate::{Status, KEY_FLAG_DEAD_KEY};

fn md(request_id: u64) -> FcitxMetadataC {
    FcitxMetadataC {
        request_id,
        response_to: 0,
        engine_epoch: 99,
        session_id: 3,
        context_id: 7,
        composition_id: 11,
        revision: 13,
    }
}

fn md_response(request_id: u64, response_to: u64) -> FcitxMetadataC {
    FcitxMetadataC {
        response_to,
        ..md(request_id)
    }
}

fn bytes(value: &'static str) -> FcitxBytesC {
    FcitxBytesC {
        data: value.as_ptr(),
        len: value.len(),
    }
}

fn caret() -> FcitxCaretRectC {
    FcitxCaretRectC {
        valid: 1,
        left: -100,
        top: 200,
        right: -98,
        bottom: 222,
        dpi: 144,
    }
}

fn key_request_c() -> FcitxKeyRequestC {
    FcitxKeyRequestC {
        metadata: md(42),
        virtual_key: 'A' as u32,
        key_flags: KEY_FLAG_DEAD_KEY,
        scan_code: 0x1e,
        extended_key: 0,
        popup_allowed: 1,
        keyboard_layout: 0x0409_0409,
        logical_text: bytes("a"),
        input_method: bytes("pinyin"),
        surrounding_text_valid: 1,
        surrounding_text: bytes("\u{4f60}"),
        surrounding_cursor: 1,
        surrounding_anchor: 1,
        caret: caret(),
    }
}

fn key_response_c() -> FcitxKeyResponseC {
    let candidate = Box::leak(Box::new(FcitxCandidateRecordC {
        id: 101,
        label: bytes("1"),
        text: bytes("\u{4f60}"),
        comment: bytes("n\u{107}"),
    }));
    FcitxKeyResponseC {
        metadata: md_response(43, 42),
        status: Status::Ok as u32,
        handled: 1,
        commit: bytes("a"),
        preedit: bytes("ni"),
        preedit_caret_utf8: 2,
        selected_candidate: 0,
        candidate_page: 0,
        candidate_total: 1,
        candidate_visibility: 1,
        candidate_page_size: 0,
        candidate_bulk: 0,
        candidate_end: 0,
        delete_surrounding_text: 1,
        delete_surrounding_offset: -1,
        delete_surrounding_size: 1,
        forward_key: 1,
        forward_key_sym: 0xff0d,
        forward_key_states: 4,
        forward_key_code: 28,
        forward_key_release: 1,
        caret: caret(),
        popup_allowed: 0,
        content_locale: bytes("ja-JP"),
        candidates: candidate,
        candidate_count: 1,
    }
}

/// Calls an encode FFI once with a large buffer and returns the bytes.
unsafe fn encode_once<C>(
    f: unsafe extern "C" fn(*const C, *mut u8, usize, *mut usize) -> u8,
    c: &C,
) -> Option<Vec<u8>> {
    let mut needed = 0usize;
    // First call with no buffer: reports the required size (0 on rejection).
    if unsafe { f(c, std::ptr::null_mut(), 0, &mut needed) } == 0 && needed == 0 {
        return None;
    }
    let mut out = vec![0u8; needed];
    let mut written = 0usize;
    if unsafe { f(c, out.as_mut_ptr(), out.len(), &mut written) } == 0 {
        return None;
    }
    out.truncate(written);
    Some(out)
}

#[test]
fn encode_matches_rust_typed_api() {
    // hello request
    let hello_c = FcitxHelloRequestC {
        metadata: FcitxMetadataC {
            request_id: 1,
            response_to: 0,
            engine_epoch: 0,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        client_architecture_bits: 64,
        client_process_id: 100,
    };
    let via_ffi = unsafe { encode_once(fcitx5_protocol_core_encode_hello_request, &hello_c) }
        .expect("hello request accepted");
    let via_rust = crate::encode_hello_request(&HelloRequest {
        metadata: crate::Metadata {
            request_id: 1,
            response_to: 0,
            engine_epoch: 0,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        client_architecture_bits: 64,
        client_process_id: 100,
    })
    .expect("rust encode accepted");
    assert_eq!(via_ffi, via_rust);

    // hello response
    let hello_r_c = FcitxHelloResponseC {
        metadata: FcitxMetadataC {
            request_id: 2,
            response_to: 1,
            engine_epoch: 7,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok as u32,
        server_architecture_bits: 64,
    };
    let via_ffi = unsafe { encode_once(fcitx5_protocol_core_encode_hello_response, &hello_r_c) }
        .expect("hello response accepted");
    let via_rust = crate::encode_hello_response(&HelloResponse {
        metadata: crate::Metadata {
            request_id: 2,
            response_to: 1,
            engine_epoch: 7,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok,
        server_architecture_bits: 64,
    })
    .expect("rust encode accepted");
    assert_eq!(via_ffi, via_rust);

    // key request
    let via_ffi = unsafe { encode_once(fcitx5_protocol_core_encode_key_request, &key_request_c()) }
        .expect("key request accepted");
    let via_rust = crate::encode_key_request(&KeyRequest {
        metadata: crate::Metadata {
            request_id: 42,
            response_to: 0,
            engine_epoch: 99,
            session_id: 3,
            context_id: 7,
            composition_id: 11,
            revision: 13,
        },
        virtual_key: 'A' as u32,
        key_flags: KEY_FLAG_DEAD_KEY,
        scan_code: 0x1e,
        extended_key: false,
        popup_allowed: true,
        keyboard_layout: 0x0409_0409,
        logical_text_utf8: b"a".to_vec(),
        input_method_utf8: b"pinyin".to_vec(),
        surrounding_text_valid: true,
        surrounding_text_utf8: "\u{4f60}".as_bytes().to_vec(),
        surrounding_cursor: 1,
        surrounding_anchor: 1,
        caret: crate::CaretRect {
            valid: true,
            left: -100,
            top: 200,
            right: -98,
            bottom: 222,
            dpi: 144,
        },
    })
    .expect("rust encode accepted");
    assert_eq!(via_ffi, via_rust);

    // key response (with candidates)
    let via_ffi =
        unsafe { encode_once(fcitx5_protocol_core_encode_key_response, &key_response_c()) }
            .expect("key response accepted");
    let via_rust = crate::encode_key_response(&KeyResponse {
        metadata: crate::Metadata {
            request_id: 43,
            response_to: 42,
            engine_epoch: 99,
            session_id: 3,
            context_id: 7,
            composition_id: 11,
            revision: 13,
        },
        status: Status::Ok,
        handled: true,
        commit_utf8: b"a".to_vec(),
        preedit_utf8: b"ni".to_vec(),
        preedit_caret_utf8: 2,
        candidates: vec![CandidateRecord {
            id: 101,
            label_utf8: b"1".to_vec(),
            text_utf8: "\u{4f60}".as_bytes().to_vec(),
            comment_utf8: "n\u{107}".as_bytes().to_vec(),
        }],
        selected_candidate: 0,
        candidate_page: 0,
        candidate_total: 1,
        candidate_visibility: 1,
        candidate_page_size: 0,
        candidate_bulk: false,
        candidate_end: false,
        delete_surrounding_text: true,
        delete_surrounding_offset: -1,
        delete_surrounding_size: 1,
        forward_key: true,
        forward_key_sym: 0xff0d,
        forward_key_states: 4,
        forward_key_code: 28,
        forward_key_release: true,
        caret: crate::CaretRect {
            valid: true,
            left: -100,
            top: 200,
            right: -98,
            bottom: 222,
            dpi: 144,
        },
        popup_allowed: false,
        content_locale_utf8: b"ja-JP".to_vec(),
    })
    .expect("rust encode accepted");
    assert_eq!(via_ffi, via_rust);

    // candidate select request / response
    let select_c = FcitxCandidateSelectRequestC {
        metadata: md(3),
        target_process_id: 1234,
        candidate_id: 0x0b02,
    };
    let via_ffi = unsafe {
        encode_once(
            fcitx5_protocol_core_encode_candidate_select_request,
            &select_c,
        )
    }
    .expect("candidate select request accepted");
    let via_rust = crate::encode_candidate_select_request(&CandidateSelectRequest {
        metadata: crate::Metadata {
            request_id: 3,
            response_to: 0,
            engine_epoch: 99,
            session_id: 3,
            context_id: 7,
            composition_id: 11,
            revision: 13,
        },
        target_process_id: 1234,
        candidate_id: 0x0b02,
    })
    .expect("rust encode accepted");
    assert_eq!(via_ffi, via_rust);

    let select_r_c = FcitxCandidateSelectResponseC {
        metadata: md_response(4, 3),
        status: Status::Ok as u32,
    };
    let via_ffi = unsafe {
        encode_once(
            fcitx5_protocol_core_encode_candidate_select_response,
            &select_r_c,
        )
    }
    .expect("candidate select response accepted");
    let via_rust = crate::encode_candidate_select_response(&CandidateSelectResponse {
        metadata: crate::Metadata {
            request_id: 4,
            response_to: 3,
            engine_epoch: 99,
            session_id: 3,
            context_id: 7,
            composition_id: 11,
            revision: 13,
        },
        status: Status::Ok,
    })
    .expect("rust encode accepted");
    assert_eq!(via_ffi, via_rust);

    // state / engine status request
    let state_c = FcitxStateRequestC { metadata: md(5) };
    let via_ffi = unsafe { encode_once(fcitx5_protocol_core_encode_state_request, &state_c) }
        .expect("state request accepted");
    let via_rust = crate::encode_state_request(&StateRequest {
        metadata: crate::Metadata {
            request_id: 5,
            response_to: 0,
            engine_epoch: 99,
            session_id: 3,
            context_id: 7,
            composition_id: 11,
            revision: 13,
        },
    })
    .expect("rust encode accepted");
    assert_eq!(via_ffi, via_rust);

    let engine_c = FcitxEngineStatusRequestC {
        metadata: FcitxMetadataC {
            request_id: 6,
            response_to: 0,
            engine_epoch: 99,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
    };
    let via_ffi =
        unsafe { encode_once(fcitx5_protocol_core_encode_engine_status_request, &engine_c) }
            .expect("engine status request accepted");
    let via_rust = crate::encode_engine_status_request(&EngineStatusRequest {
        metadata: crate::Metadata {
            request_id: 6,
            response_to: 0,
            engine_epoch: 99,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
    })
    .expect("rust encode accepted");
    assert_eq!(via_ffi, via_rust);

    // engine status response
    let engine_r_c = FcitxEngineStatusResponseC {
        metadata: FcitxMetadataC {
            request_id: 7,
            response_to: 6,
            engine_epoch: 99,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok as u32,
        current_input_method_id: bytes("pinyin"),
        current_input_method_name: bytes("Pinyin"),
        current_input_method_native_name: bytes("\u{62fc}\u{97f3}"),
        current_input_method_short_label: bytes("\u{62fc}"),
    };
    let via_ffi = unsafe {
        encode_once(
            fcitx5_protocol_core_encode_engine_status_response,
            &engine_r_c,
        )
    }
    .expect("engine status response accepted");
    let via_rust = crate::encode_engine_status_response(&EngineStatusResponse {
        metadata: crate::Metadata {
            request_id: 7,
            response_to: 6,
            engine_epoch: 99,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok,
        current_input_method_id: b"pinyin".to_vec(),
        current_input_method_name: b"Pinyin".to_vec(),
        current_input_method_native_name: "\u{62fc}\u{97f3}".as_bytes().to_vec(),
        current_input_method_short_label: "\u{62fc}".as_bytes().to_vec(),
    })
    .expect("rust encode accepted");
    assert_eq!(via_ffi, via_rust);

    // launcher request for every command
    for raw in 1..=9u32 {
        let launcher_c = FcitxLauncherRequestC {
            metadata: FcitxMetadataC {
                request_id: raw as u64,
                response_to: 0,
                engine_epoch: 0,
                session_id: 3,
                context_id: 0,
                composition_id: 0,
                revision: 0,
            },
            command: raw,
        };
        let via_ffi =
            unsafe { encode_once(fcitx5_protocol_core_encode_launcher_request, &launcher_c) }
                .expect("launcher request accepted");
        let via_rust = crate::encode_launcher_request(&LauncherRequest {
            metadata: crate::Metadata {
                request_id: raw as u64,
                response_to: 0,
                engine_epoch: 0,
                session_id: 3,
                context_id: 0,
                composition_id: 0,
                revision: 0,
            },
            command: launcher_command_from_c(raw).unwrap(),
        })
        .expect("rust encode accepted");
        assert_eq!(via_ffi, via_rust);
    }

    // launcher response
    let launcher_r_c = FcitxLauncherResponseC {
        metadata: FcitxMetadataC {
            request_id: 9,
            response_to: 8,
            engine_epoch: 0,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok as u32,
        launcher_state: 1,
        engine_state: 2,
        start_disposition: 3,
        safe_mode: 1,
        retry_after_milliseconds: 250,
        current_input_method_id: bytes("rime"),
        current_input_method_name: bytes("Rime"),
        current_input_method_native_name: bytes("\u{4e2d}\u{6d32}\u{97f5}"),
        current_input_method_short_label: bytes("\u{4e2d}"),
    };
    let via_ffi =
        unsafe { encode_once(fcitx5_protocol_core_encode_launcher_response, &launcher_r_c) }
            .expect("launcher response accepted");
    let via_rust = crate::encode_launcher_response(&LauncherResponse {
        metadata: crate::Metadata {
            request_id: 9,
            response_to: 8,
            engine_epoch: 0,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok,
        launcher_state: 1,
        engine_state: 2,
        start_disposition: 3,
        safe_mode: true,
        retry_after_milliseconds: 250,
        current_input_method_id: b"rime".to_vec(),
        current_input_method_name: b"Rime".to_vec(),
        current_input_method_native_name: "\u{4e2d}\u{6d32}\u{97f5}".as_bytes().to_vec(),
        current_input_method_short_label: "\u{4e2d}".as_bytes().to_vec(),
    })
    .expect("rust encode accepted");
    assert_eq!(via_ffi, via_rust);
}

#[test]
fn decode_key_request_roundtrips_through_c_structures() {
    let bytes = unsafe { encode_once(fcitx5_protocol_core_encode_key_request, &key_request_c()) }
        .expect("key request accepted");

    let metadata = key_request_c().metadata;
    let body = &bytes[crate::HEADER_SIZE..];
    let mut out = unsafe { std::mem::zeroed::<FcitxKeyRequestC>() };
    let mut strings = [0u8; 4096];
    let mut strings_needed = 0usize;
    let ok = unsafe {
        fcitx5_protocol_core_decode_key_request(
            &metadata,
            body.as_ptr(),
            body.len(),
            &mut out,
            strings.as_mut_ptr(),
            strings.len(),
            &mut strings_needed,
        )
    };
    assert_eq!(ok, 1, "key request decode should succeed");
    assert_eq!(strings_needed, 10); // "a"(1) + "pinyin"(6) + "你"(3)
    assert_eq!(out.virtual_key, 'A' as u32);
    assert_eq!(out.key_flags, KEY_FLAG_DEAD_KEY);
    assert_eq!(out.scan_code, 0x1e);
    assert_eq!(out.extended_key, 0);
    assert_eq!(out.popup_allowed, 1);
    assert_eq!(out.keyboard_layout, 0x0409_0409);
    assert_eq!(out.surrounding_text_valid, 1);
    assert_eq!(out.surrounding_cursor, 1);
    assert_eq!(out.surrounding_anchor, 1);
    assert_eq!(out.caret.left, -100);
    assert_eq!(out.caret.dpi, 144);
    assert_c_bytes(&out.logical_text, b"a");
    assert_c_bytes(&out.input_method, b"pinyin");
    assert_c_bytes(&out.surrounding_text, "\u{4f60}".as_bytes());
}

#[test]
fn decode_key_response_roundtrips_candidates() {
    let bytes = unsafe { encode_once(fcitx5_protocol_core_encode_key_response, &key_response_c()) }
        .expect("key response accepted");

    let metadata = key_response_c().metadata;
    let body = &bytes[crate::HEADER_SIZE..];
    let mut out = unsafe { std::mem::zeroed::<FcitxKeyResponseC>() };
    let mut strings = [0u8; 4096];
    let mut candidates = [unsafe { std::mem::zeroed::<FcitxCandidateRecordC>() }; 4];
    let mut strings_needed = 0usize;
    let mut candidates_needed = 0usize;
    let ok = unsafe {
        fcitx5_protocol_core_decode_key_response(
            &metadata,
            body.as_ptr(),
            body.len(),
            &mut out,
            strings.as_mut_ptr(),
            strings.len(),
            &mut strings_needed,
            candidates.as_mut_ptr(),
            candidates.len(),
            &mut candidates_needed,
        )
    };
    assert_eq!(ok, 1, "key response decode should succeed");
    assert_eq!(strings_needed, 15); // a(1)+ni(2)+ja-JP(5) + label(1)+你(3)+nć(3)
    assert_eq!(candidates_needed, 1);
    assert_eq!(out.status, Status::Ok as u32);
    assert_eq!(out.handled, 1);
    assert_eq!(out.preedit_caret_utf8, 2);
    assert_eq!(out.selected_candidate, 0);
    assert_eq!(out.delete_surrounding_offset, -1);
    assert_eq!(out.forward_key, 1);
    assert_eq!(out.forward_key_sym, 0xff0d);
    assert_eq!(out.popup_allowed, 0);
    assert_c_bytes(&out.commit, b"a");
    assert_c_bytes(&out.preedit, b"ni");
    assert_c_bytes(&out.content_locale, b"ja-JP");
    assert_eq!(candidates[0].id, 101);
    assert_c_bytes(&candidates[0].label, b"1");
    assert_c_bytes(&candidates[0].text, "\u{4f60}".as_bytes());
    assert_c_bytes(&candidates[0].comment, "n\u{107}".as_bytes());
}

#[test]
fn decode_reports_space_needed_when_buffers_are_small() {
    let bytes = unsafe { encode_once(fcitx5_protocol_core_encode_key_request, &key_request_c()) }
        .expect("key request accepted");
    let metadata = key_request_c().metadata;
    let body = &bytes[crate::HEADER_SIZE..];

    let mut out = unsafe { std::mem::zeroed::<FcitxKeyRequestC>() };
    let mut strings_needed = 0usize;
    let ok = unsafe {
        fcitx5_protocol_core_decode_key_request(
            &metadata,
            body.as_ptr(),
            body.len(),
            &mut out,
            std::ptr::null_mut(),
            0,
            &mut strings_needed,
        )
    };
    assert_eq!(ok, 0, "small arena must report space needed");
    assert_eq!(strings_needed, 10);

    // Rejection keeps the needed counter untouched.
    let bad_metadata = FcitxMetadataC {
        request_id: 42,
        engine_epoch: 0, // invalid for a key request (validKeyMetadata)
        ..md(42)
    };
    let mut strings_needed = 99usize;
    let ok = unsafe {
        fcitx5_protocol_core_decode_key_request(
            &bad_metadata,
            body.as_ptr(),
            body.len(),
            &mut out,
            std::ptr::null_mut(),
            0,
            &mut strings_needed,
        )
    };
    assert_eq!(ok, 0);
    assert_eq!(strings_needed, 99, "rejection must not report space needed");
}

#[test]
fn encode_reports_space_needed_when_buffer_is_small() {
    let c = key_request_c();
    let mut needed = 0usize;
    let ok = unsafe {
        fcitx5_protocol_core_encode_key_request(&c, std::ptr::null_mut(), 0, &mut needed)
    };
    assert_eq!(ok, 0);
    assert!(needed > 0);
}

#[test]
fn rejected_inputs_return_zero() {
    // Invalid metadata: request_id == 0 is rejected by the typed API.
    let bad = FcitxKeyRequestC {
        metadata: md(0),
        ..key_request_c()
    };
    let mut needed = 0usize;
    let ok = unsafe {
        fcitx5_protocol_core_encode_key_request(&bad, std::ptr::null_mut(), 0, &mut needed)
    };
    assert_eq!(ok, 0);
    assert_eq!(needed, 0);

    // Out-of-range launcher command.
    let bad = FcitxLauncherRequestC {
        metadata: FcitxMetadataC {
            request_id: 1,
            response_to: 0,
            engine_epoch: 0,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        command: 99,
    };
    let mut needed = 0usize;
    let ok = unsafe {
        fcitx5_protocol_core_encode_launcher_request(&bad, std::ptr::null_mut(), 0, &mut needed)
    };
    assert_eq!(ok, 0);
    assert_eq!(needed, 0);
}

#[test]
fn decode_header_abi_reports_type_and_metadata() {
    let bytes = unsafe { encode_once(fcitx5_protocol_core_encode_key_request, &key_request_c()) }
        .expect("key request accepted");
    let mut out_type = 0u16;
    let mut out_body_size = 0u32;
    let mut out_metadata = unsafe { std::mem::zeroed::<FcitxMetadataC>() };
    let ok = unsafe {
        fcitx5_protocol_core_decode_header(
            bytes.as_ptr(),
            crate::HEADER_SIZE,
            &mut out_type,
            &mut out_body_size,
            &mut out_metadata,
        )
    };
    assert_eq!(ok, 1);
    assert_eq!(out_type, crate::MessageType::KeyRequest as u16);
    assert_eq!(out_body_size as usize, bytes.len() - crate::HEADER_SIZE);
    assert_eq!(out_metadata.request_id, 42);

    // Wrong-length header rejected.
    let ok = unsafe {
        fcitx5_protocol_core_decode_header(
            bytes.as_ptr(),
            crate::HEADER_SIZE - 1,
            &mut out_type,
            &mut out_body_size,
            &mut out_metadata,
        )
    };
    assert_eq!(ok, 0);
}

fn assert_c_bytes(actual: &FcitxBytesC, expected: &[u8]) {
    assert_eq!(actual.len, expected.len());
    let slice = if actual.len == 0 {
        &[][..]
    } else {
        // SAFETY: the arena pointer is valid for `len` bytes.
        unsafe { std::slice::from_raw_parts(actual.data, actual.len) }
    };
    assert_eq!(slice, expected);
}
