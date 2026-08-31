#![deny(unsafe_op_in_unsafe_fn)]

//! Tests mirroring `tests/unit/protocol_test.cpp` corpus against the Rust
//! `fcitx5-protocol-core` codec. Every acceptance/rejection case from the C++
//! contract test is reproduced here so the Rust implementation is a byte- and
//! rejection-equivalent implementation of the FCW4 protocol.

use crate::*;

fn key_request_input() -> KeyRequest {
    KeyRequest {
        metadata: Metadata {
            request_id: 42,
            response_to: 0,
            engine_epoch: 99,
            session_id: 3,
            context_id: 7,
            composition_id: 11,
            revision: 13,
        },
        virtual_key: b'A' as u32,
        key_flags: KEY_FLAG_DEAD_KEY,
        scan_code: 0x1e,
        extended_key: false,
        popup_allowed: true,
        keyboard_layout: 0x0409_0409,
        logical_text_utf8: "a".as_bytes().to_vec(),
        input_method_utf8: "pinyin".as_bytes().to_vec(),
        surrounding_text_valid: true,
        surrounding_text_utf8: "\u{4f60}a".as_bytes().to_vec(),
        surrounding_cursor: 2,
        surrounding_anchor: 2,
        caret: CaretRect {
            valid: true,
            left: -100,
            top: 200,
            right: -98,
            bottom: 222,
            dpi: 144,
        },
    }
}

fn key_response_input() -> KeyResponse {
    KeyResponse {
        metadata: Metadata {
            request_id: 43,
            response_to: 42,
            engine_epoch: 99,
            session_id: 3,
            context_id: 7,
            composition_id: 11,
            revision: 14,
        },
        status: Status::Ok,
        handled: true,
        commit_utf8: "a".as_bytes().to_vec(),
        preedit_utf8: "ni".as_bytes().to_vec(),
        preedit_caret_utf8: 2,
        candidates: vec![CandidateRecord {
            id: 101,
            label_utf8: "1".as_bytes().to_vec(),
            text_utf8: "\u{4f60}".as_bytes().to_vec(),
            comment_utf8: "n\u{1d0}".as_bytes().to_vec(),
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
        caret: CaretRect {
            valid: true,
            left: -100,
            top: 200,
            right: -98,
            bottom: 222,
            dpi: 144,
        },
        popup_allowed: false,
        content_locale_utf8: "ja-JP".as_bytes().to_vec(),
    }
}

fn hello_request_input() -> HelloRequest {
    HelloRequest {
        metadata: Metadata {
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
    }
}

fn launcher_request_input(command: LauncherCommand) -> LauncherRequest {
    LauncherRequest {
        metadata: Metadata {
            request_id: command as u64,
            response_to: 0,
            engine_epoch: 0,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        command,
    }
}

#[test]
fn key_request_roundtrip_matches_cpp_contract() {
    let input = key_request_input();
    let bytes = encode_key_request(&input).expect("valid key request encoded");
    let frame = decode_frame(&bytes).expect("valid frame decoded");
    let output = decode_key_request(&frame).expect("valid key request decoded");
    assert_eq!(output.metadata, input.metadata);
    assert_eq!(output.virtual_key, input.virtual_key);
    assert_eq!(output.key_flags, input.key_flags);
    assert_eq!(output.scan_code, input.scan_code);
    assert_eq!(output.extended_key, input.extended_key);
    assert_eq!(output.popup_allowed, input.popup_allowed);
    assert_eq!(output.keyboard_layout, input.keyboard_layout);
    assert_eq!(output.logical_text_utf8, input.logical_text_utf8);
    assert_eq!(output.input_method_utf8, input.input_method_utf8);
    assert_eq!(output.surrounding_text_valid, input.surrounding_text_valid);
    assert_eq!(output.surrounding_text_utf8, input.surrounding_text_utf8);
    assert_eq!(output.surrounding_cursor, input.surrounding_cursor);
    assert_eq!(output.surrounding_anchor, input.surrounding_anchor);
    assert_eq!(output.caret, input.caret);
}

#[test]
fn truncated_frames_are_rejected() {
    let bytes = encode_key_request(&key_request_input()).expect("encoded");
    for size in 0..bytes.len() {
        assert!(
            decode_frame(&bytes[..size]).is_none(),
            "truncated frame at {size} accepted"
        );
    }
}

#[test]
fn wrong_version_is_rejected() {
    let mut bytes = encode_key_request(&key_request_input()).expect("encoded");
    bytes[4] = (VERSION + 1) as u8;
    assert!(decode_frame(&bytes).is_none());
}

#[test]
fn key_response_roundtrip_matches_cpp_contract() {
    let input = key_response_input();
    let bytes = encode_key_response(&input).expect("valid response encoded");
    let frame = decode_frame(&bytes).expect("valid frame decoded");
    let output = decode_key_response(&frame).expect("valid response decoded");
    assert_eq!(output.metadata, input.metadata);
    assert!(output.handled);
    assert_eq!(output.commit_utf8, "a".as_bytes());
    assert_eq!(output.preedit_utf8, "ni".as_bytes());
    assert_eq!(output.preedit_caret_utf8, 2);
    assert_eq!(output.candidates, input.candidates);
    assert_eq!(output.selected_candidate, 0);
    assert_eq!(output.candidate_visibility, 1);
    assert!(output.delete_surrounding_text);
    assert_eq!(output.delete_surrounding_offset, -1);
    assert_eq!(output.delete_surrounding_size, 1);
    assert!(output.forward_key);
    assert_eq!(output.forward_key_sym, 0xff0d);
    assert_eq!(output.forward_key_states, 4);
    assert_eq!(output.forward_key_code, 28);
    assert!(output.forward_key_release);
    assert!(!output.popup_allowed);
    assert_eq!(output.content_locale_utf8, "ja-JP".as_bytes());
    assert_eq!(output.caret, input.caret);
}

#[test]
fn hello_roundtrips() {
    let hello_input = hello_request_input();
    let bytes = encode_hello_request(&hello_input).expect("encoded");
    let frame = decode_frame(&bytes).expect("frame");
    let hello_output = decode_hello_request(&frame).expect("decoded");
    assert_eq!(hello_output.metadata, hello_input.metadata);
    assert_eq!(
        hello_output.client_architecture_bits,
        hello_input.client_architecture_bits
    );
    assert_eq!(
        hello_output.client_process_id,
        hello_input.client_process_id
    );

    let hello_response_input = HelloResponse {
        metadata: Metadata {
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
    };
    let bytes = encode_hello_response(&hello_response_input).expect("encoded");
    let frame = decode_frame(&bytes).expect("frame");
    let hello_response_output = decode_hello_response(&frame).expect("decoded");
    assert_eq!(
        hello_response_output.metadata,
        hello_response_input.metadata
    );
    assert_eq!(hello_response_output.status, Status::Ok);
    assert_eq!(
        hello_response_output.server_architecture_bits,
        hello_response_input.server_architecture_bits
    );
}

#[test]
fn candidate_select_roundtrips() {
    let select_input = CandidateSelectRequest {
        metadata: Metadata {
            request_id: 3,
            response_to: 0,
            engine_epoch: 99,
            session_id: 3,
            context_id: 7,
            composition_id: 11,
            revision: 14,
        },
        target_process_id: 1234,
        candidate_id: 0x0b02,
    };
    let bytes = encode_candidate_select_request(&select_input).expect("encoded");
    let frame = decode_frame(&bytes).expect("frame");
    let select_output = decode_candidate_select_request(&frame).expect("decoded");
    assert_eq!(select_output.metadata, select_input.metadata);
    assert_eq!(select_output.target_process_id, 1234);
    assert_eq!(select_output.candidate_id, 0x0b02);

    let select_response_input = CandidateSelectResponse {
        metadata: Metadata {
            request_id: 4,
            response_to: 3,
            engine_epoch: 99,
            session_id: 3,
            context_id: 7,
            composition_id: 0,
            revision: 15,
        },
        status: Status::Ok,
    };
    let bytes = encode_candidate_select_response(&select_response_input).expect("encoded");
    let frame = decode_frame(&bytes).expect("frame");
    let select_response_output = decode_candidate_select_response(&frame).expect("decoded");
    assert_eq!(
        select_response_output.metadata,
        select_response_input.metadata
    );
    assert_eq!(select_response_output.status, Status::Ok);
}

#[test]
fn state_and_engine_status_roundtrip() {
    let state_input = StateRequest {
        metadata: Metadata {
            request_id: 5,
            response_to: 0,
            engine_epoch: 99,
            session_id: 3,
            context_id: 7,
            composition_id: 11,
            revision: 14,
        },
    };
    let bytes = encode_state_request(&state_input).expect("encoded");
    let frame = decode_frame(&bytes).expect("frame");
    let state_output = decode_state_request(&frame).expect("decoded");
    assert_eq!(state_output.metadata, state_input.metadata);

    let status_input = EngineStatusRequest {
        metadata: Metadata {
            request_id: 6,
            response_to: 0,
            engine_epoch: 99,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
    };
    let bytes = encode_engine_status_request(&status_input).expect("encoded");
    let frame = decode_frame(&bytes).expect("frame");
    let status_output = decode_engine_status_request(&frame).expect("decoded");
    assert_eq!(status_output.metadata, status_input.metadata);

    let status_response_input = EngineStatusResponse {
        metadata: Metadata {
            request_id: 7,
            response_to: 6,
            engine_epoch: 99,
            session_id: 3,
            context_id: 0,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok,
        current_input_method_id: "pinyin".as_bytes().to_vec(),
        current_input_method_name: "Pinyin".as_bytes().to_vec(),
        current_input_method_native_name: "\u{62fc}\u{97f3}".as_bytes().to_vec(),
        current_input_method_short_label: "\u{62fc}".as_bytes().to_vec(),
    };
    let bytes = encode_engine_status_response(&status_response_input).expect("encoded");
    let frame = decode_frame(&bytes).expect("frame");
    let status_response_output = decode_engine_status_response(&frame).expect("decoded");
    assert_eq!(
        status_response_output.metadata,
        status_response_input.metadata
    );
    assert_eq!(status_response_output.status, Status::Ok);
    assert_eq!(
        status_response_output.current_input_method_id,
        "pinyin".as_bytes()
    );
    assert_eq!(
        status_response_output.current_input_method_name,
        "Pinyin".as_bytes()
    );
    assert_eq!(
        status_response_output.current_input_method_native_name,
        "\u{62fc}\u{97f3}".as_bytes()
    );
    assert_eq!(
        status_response_output.current_input_method_short_label,
        "\u{62fc}".as_bytes()
    );
}

#[test]
fn launcher_roundtrips_all_commands() {
    for raw_command in LauncherCommand::StartDemand as u32..=LauncherCommand::Shutdown as u32 {
        let launcher_input = launcher_request_input(launcher_command_from_raw(raw_command));
        let bytes = encode_launcher_request(&launcher_input).expect("encoded");
        let frame = decode_frame(&bytes).expect("frame");
        let launcher_output = decode_launcher_request(&frame).expect("decoded");
        assert_eq!(launcher_output.metadata, launcher_input.metadata);
        assert_eq!(launcher_output.command, launcher_input.command);
    }

    let launcher_response_input = LauncherResponse {
        metadata: Metadata {
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
        current_input_method_id: "rime".as_bytes().to_vec(),
        current_input_method_name: "Rime".as_bytes().to_vec(),
        current_input_method_native_name: "\u{4e2d}\u{6d32}\u{97f5}".as_bytes().to_vec(),
        current_input_method_short_label: "\u{4e2d}".as_bytes().to_vec(),
    };
    let bytes = encode_launcher_response(&launcher_response_input).expect("encoded");
    let frame = decode_frame(&bytes).expect("frame");
    let launcher_response_output = decode_launcher_response(&frame).expect("decoded");
    assert_eq!(
        launcher_response_output.metadata,
        launcher_response_input.metadata
    );
    assert_eq!(launcher_response_output.status, Status::Ok);
    assert_eq!(launcher_response_output.launcher_state, 1);
    assert_eq!(launcher_response_output.engine_state, 2);
    assert_eq!(launcher_response_output.start_disposition, 3);
    assert!(launcher_response_output.safe_mode);
    assert_eq!(launcher_response_output.retry_after_milliseconds, 250);
    assert_eq!(
        launcher_response_output.current_input_method_id,
        "rime".as_bytes()
    );
    assert_eq!(
        launcher_response_output.current_input_method_name,
        "Rime".as_bytes()
    );
    assert_eq!(
        launcher_response_output.current_input_method_native_name,
        "\u{4e2d}\u{6d32}\u{97f5}".as_bytes()
    );
    assert_eq!(
        launcher_response_output.current_input_method_short_label,
        "\u{4e2d}".as_bytes()
    );
}

/// Deterministic xorshift64* so property tests are reproducible.
struct DeterministicRng(u64);

impl DeterministicRng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
}

#[test]
fn randomized_property_roundtrip() {
    let mut random = DeterministicRng(0x3257_4346);
    for iteration in 1u64..=10_000 {
        let request_metadata = Metadata {
            request_id: iteration,
            response_to: 0,
            engine_epoch: random.next_u64() | 1,
            session_id: 1,
            context_id: random.next_u64() | 1,
            composition_id: random.next_u64(),
            revision: random.next_u64(),
        };
        let property_input = KeyRequest {
            metadata: request_metadata,
            virtual_key: random.next_u32(),
            key_flags: random.next_u32() & KNOWN_KEY_FLAGS,
            scan_code: random.next_u32() & 0xff,
            extended_key: (random.next_u32() & 1) != 0,
            popup_allowed: (random.next_u32() & 1) != 0,
            keyboard_layout: random.next_u64(),
            logical_text_utf8: "x".as_bytes().to_vec(),
            input_method_utf8: "pinyin".as_bytes().to_vec(),
            surrounding_text_valid: true,
            surrounding_text_utf8: "\u{4f60}".as_bytes().to_vec(),
            surrounding_cursor: 1,
            surrounding_anchor: 1,
            ..Default::default()
        };
        let bytes = encode_key_request(&property_input).expect("encoded");
        let frame = decode_frame(&bytes).expect("frame");
        let property_output = decode_key_request(&frame).expect("decoded");
        assert_eq!(property_output.metadata, property_input.metadata);
        assert_eq!(property_output.virtual_key, property_input.virtual_key);
        assert_eq!(property_output.key_flags, property_input.key_flags);
        assert_eq!(property_output.scan_code, property_input.scan_code);
        assert_eq!(property_output.extended_key, property_input.extended_key);
        assert_eq!(property_output.popup_allowed, property_input.popup_allowed);
        assert_eq!(
            property_output.keyboard_layout,
            property_input.keyboard_layout
        );
        assert_eq!(
            property_output.logical_text_utf8,
            property_input.logical_text_utf8
        );
        assert_eq!(
            property_output.input_method_utf8,
            property_input.input_method_utf8
        );
        assert_eq!(
            property_output.surrounding_text_valid,
            property_input.surrounding_text_valid
        );
        assert_eq!(
            property_output.surrounding_text_utf8,
            property_input.surrounding_text_utf8
        );
        assert_eq!(
            property_output.surrounding_cursor,
            property_input.surrounding_cursor
        );
        assert_eq!(
            property_output.surrounding_anchor,
            property_input.surrounding_anchor
        );

        let commit_len = (random.next_u32() % (MAX_COMMIT_UTF8 as u32 + 1)) as usize;
        let mut commit = Vec::with_capacity(commit_len);
        for _ in 0..commit_len {
            commit.push((random.next_u32() & 0xff) as u8);
        }
        let property_response = KeyResponse {
            metadata: Metadata {
                request_id: iteration + 20_000,
                response_to: iteration,
                engine_epoch: request_metadata.engine_epoch,
                session_id: request_metadata.session_id,
                context_id: request_metadata.context_id,
                composition_id: request_metadata.composition_id,
                revision: request_metadata.revision,
            },
            status: status_from_raw(random.next_u32() % (STATUS_ACCESS_DENIED + 1)),
            handled: (random.next_u32() & 1) != 0,
            commit_utf8: commit,
            preedit_utf8: "preedit".as_bytes().to_vec(),
            preedit_caret_utf8: 3,
            popup_allowed: (random.next_u32() & 1) != 0,
            content_locale_utf8: if iteration & 1 == 0 {
                "ja-JP".as_bytes().to_vec()
            } else {
                "en-US".as_bytes().to_vec()
            },
            ..Default::default()
        };
        let bytes = encode_key_response(&property_response).expect("encoded");
        let frame = decode_frame(&bytes).expect("frame");
        let property_response_output = decode_key_response(&frame).expect("decoded");
        assert_eq!(
            property_response_output.metadata,
            property_response.metadata
        );
        assert_eq!(property_response_output.status, property_response.status);
        assert_eq!(property_response_output.handled, property_response.handled);
        assert_eq!(
            property_response_output.commit_utf8,
            property_response.commit_utf8
        );
        assert_eq!(
            property_response_output.preedit_utf8,
            property_response.preedit_utf8
        );
        assert_eq!(
            property_response_output.preedit_caret_utf8,
            property_response.preedit_caret_utf8
        );
        assert_eq!(
            property_response_output.popup_allowed,
            property_response.popup_allowed
        );
        assert_eq!(
            property_response_output.content_locale_utf8,
            property_response.content_locale_utf8
        );
    }
}

#[test]
fn encode_rejections_match_cpp_contract() {
    assert!(encode_key_request(&KeyRequest::default()).is_none());
    let mut oversize_commit = KeyResponse {
        metadata: Metadata {
            request_id: 1,
            response_to: 1,
            engine_epoch: 1,
            session_id: 1,
            context_id: 1,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok,
        handled: true,
        ..Default::default()
    };
    oversize_commit.commit_utf8 = vec![b'x'; MAX_COMMIT_UTF8 + 1];
    assert!(encode_key_response(&oversize_commit).is_none());

    let invalid_caret = KeyRequest {
        metadata: Metadata {
            request_id: 88,
            response_to: 0,
            engine_epoch: 1,
            session_id: 1,
            context_id: 1,
            composition_id: 0,
            revision: 0,
        },
        virtual_key: b'A' as u32,
        caret: CaretRect {
            valid: true,
            left: 100,
            top: 100,
            right: 90,
            bottom: 110,
            dpi: 96,
        },
        ..Default::default()
    };
    assert!(encode_key_request(&invalid_caret).is_none());

    let mut oversized_logical = invalid_caret.clone();
    oversized_logical.metadata.request_id = 89;
    oversized_logical.logical_text_utf8 = vec![b'x'; MAX_LOGICAL_KEY_UTF8 + 1];
    assert!(encode_key_request(&oversized_logical).is_none());

    let mut unknown_key_flag = invalid_caret.clone();
    unknown_key_flag.metadata.request_id = 93;
    unknown_key_flag.key_flags = KNOWN_KEY_FLAGS << 1;
    assert!(encode_key_request(&unknown_key_flag).is_none());

    let mut oversized_input_method = invalid_caret.clone();
    oversized_input_method.metadata.request_id = 90;
    oversized_input_method.input_method_utf8 = vec![b'x'; MAX_INPUT_METHOD_ID_UTF8 + 1];
    assert!(encode_key_request(&oversized_input_method).is_none());

    let mut invalid_surrounding_cursor = invalid_caret.clone();
    invalid_surrounding_cursor.metadata.request_id = 91;
    invalid_surrounding_cursor.surrounding_text_valid = true;
    invalid_surrounding_cursor.surrounding_text_utf8 = "\u{4f60}".as_bytes().to_vec();
    invalid_surrounding_cursor.surrounding_cursor = 2;
    invalid_surrounding_cursor.surrounding_anchor = 1;
    assert!(encode_key_request(&invalid_surrounding_cursor).is_none());

    let mut invalid_surrounding_state = invalid_caret.clone();
    invalid_surrounding_state.metadata.request_id = 92;
    invalid_surrounding_state.surrounding_text_valid = false;
    invalid_surrounding_state.surrounding_text_utf8 = "stale".as_bytes().to_vec();
    assert!(encode_key_request(&invalid_surrounding_state).is_none());

    let disabled_delete_payload = KeyResponse {
        metadata: Metadata {
            request_id: 1,
            response_to: 1,
            engine_epoch: 1,
            session_id: 1,
            context_id: 1,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok,
        handled: true,
        selected_candidate: u32::MAX,
        delete_surrounding_offset: -1,
        delete_surrounding_size: 1,
        ..Default::default()
    };
    assert!(encode_key_response(&disabled_delete_payload).is_none());

    let mut oversize_preedit = KeyResponse {
        metadata: Metadata {
            request_id: 1,
            response_to: 1,
            engine_epoch: 1,
            session_id: 1,
            context_id: 1,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok,
        handled: true,
        ..Default::default()
    };
    oversize_preedit.preedit_utf8 = vec![b'x'; MAX_PREEDIT_UTF8 + 1];
    assert!(encode_key_response(&oversize_preedit).is_none());

    let mut out_of_range_caret = KeyResponse {
        metadata: Metadata {
            request_id: 1,
            response_to: 1,
            engine_epoch: 1,
            session_id: 1,
            context_id: 1,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok,
        handled: true,
        ..Default::default()
    };
    out_of_range_caret.preedit_utf8 = "x".as_bytes().to_vec();
    out_of_range_caret.preedit_caret_utf8 = 2;
    assert!(encode_key_response(&out_of_range_caret).is_none());

    let mut invalid_locale = KeyResponse {
        metadata: Metadata {
            request_id: 1,
            response_to: 1,
            engine_epoch: 1,
            session_id: 1,
            context_id: 1,
            composition_id: 0,
            revision: 0,
        },
        status: Status::Ok,
        handled: true,
        ..Default::default()
    };
    invalid_locale.content_locale_utf8 = "../bad".as_bytes().to_vec();
    assert!(encode_key_response(&invalid_locale).is_none());

    let mut oversized_locale = invalid_locale.clone();
    oversized_locale.content_locale_utf8 = vec![b'x'; MAX_LOCALE_UTF8 + 1];
    assert!(encode_key_response(&oversized_locale).is_none());
}

#[test]
fn unknown_type_and_oversize_body_rejected() {
    let mut bytes = encode_key_request(&key_request_input()).expect("encoded");
    bytes[6] = 0xff;
    bytes[7] = 0xff;
    assert!(decode_frame(&bytes).is_none());

    let mut invalid_length = encode_key_request(&key_request_input()).expect("encoded");
    invalid_length[8] = 0xff;
    invalid_length[9] = 0xff;
    invalid_length[10] = 0xff;
    invalid_length[11] = 0x7f;
    assert!(decode_frame(&invalid_length).is_none());
}

#[test]
fn large_control_frame_accepts_frame_but_rejects_trailing_payload() {
    let mut large_control_frame =
        encode_launcher_request(&launcher_request_input(LauncherCommand::Status)).expect("encoded");
    large_control_frame.resize(MAX_HOT_FRAME_SIZE + 1, 0);
    let control_body_size = (large_control_frame.len() - HEADER_SIZE) as u32;
    for index in 0..4 {
        large_control_frame[8 + index] = ((control_body_size >> (index * 8)) & 0xff) as u8;
    }
    let frame = decode_frame(&large_control_frame).expect("valid control-frame size was rejected");
    assert!(decode_launcher_request(&frame).is_none());
    // Frame accepted but typed decode rejected -> reencode must fail too.
    assert!(decode_and_reencode(&large_control_frame).is_none());
}

#[test]
fn decode_and_reencode_is_byte_identical() {
    let cases: Vec<Vec<u8>> = vec![
        encode_hello_request(&hello_request_input()).expect("encoded"),
        encode_key_request(&key_request_input()).expect("encoded"),
        encode_key_response(&key_response_input()).expect("encoded"),
        encode_launcher_request(&launcher_request_input(LauncherCommand::Resume)).expect("encoded"),
        encode_engine_status_response(&EngineStatusResponse {
            metadata: Metadata {
                request_id: 7,
                response_to: 6,
                engine_epoch: 99,
                session_id: 3,
                context_id: 0,
                composition_id: 0,
                revision: 0,
            },
            status: Status::Ok,
            current_input_method_id: "pinyin".as_bytes().to_vec(),
            current_input_method_name: "Pinyin".as_bytes().to_vec(),
            current_input_method_native_name: "\u{62fc}\u{97f3}".as_bytes().to_vec(),
            current_input_method_short_label: "\u{62fc}".as_bytes().to_vec(),
        })
        .expect("encoded"),
    ];
    for bytes in cases {
        let reencoded = decode_and_reencode(&bytes).expect("reencode failed");
        assert_eq!(reencoded, bytes, "reencode changed the wire bytes");
    }
}

#[test]
fn frozen_wire_corpus_is_consumed_by_rust_authority() {
    const GOLDEN_SOURCE: &str = include_str!("../../../tests/unit/protocol_wire_golden.inc");
    const SAMPLE_SIZES: &[usize] = &[
        139, 188, 72, 72, 76, 68, 64, 64, 105, 68, 68, 68, 68, 68, 68, 68, 68, 68, 125,
    ];
    let bytes: Vec<u8> = GOLDEN_SOURCE
        .split_whitespace()
        .filter_map(|token| token.strip_prefix("0x"))
        .map(|token| token.trim_end_matches(','))
        .filter(|token| token.len() == 2)
        .map(|token| u8::from_str_radix(token, 16).expect("golden byte should be hexadecimal"))
        .collect();
    assert_eq!(bytes.len(), 1585);

    let mut offset = 0;
    for (index, size) in SAMPLE_SIZES.iter().copied().enumerate() {
        let sample = &bytes[offset..offset + size];
        let reencoded = decode_and_reencode(sample)
            .unwrap_or_else(|| panic!("frozen protocol sample {index} did not decode"));
        assert_eq!(reencoded, sample, "frozen protocol sample {index} changed");
        offset += size;
    }
    assert_eq!(offset, bytes.len());
}

#[test]
fn fuzz_smoke_consumes_deterministic_random_bytes() {
    let mut state = 0x3457_4346_u64;
    for _ in 0..20_000 {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let size = (state.wrapping_mul(0x2545_f491_4f6c_dd1d) % 1024) as usize;
        let mut bytes = vec![0_u8; size];
        for byte in &mut bytes {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            *byte = state.wrapping_mul(0x2545_f491_4f6c_dd1d) as u8;
        }
        let _ = decode_and_reencode(&bytes);
    }
}

#[test]
fn utf8_check_matches_cpp_byte_structure_semantics() {
    // Overlong encodings are accepted by the C++ byte-structure check.
    assert_eq!(utf8_code_point_count(b"\x00"), Some(1));
    assert_eq!(utf8_code_point_count(b"\xe0\x80\x80"), Some(1)); // overlong NUL
    assert_eq!(utf8_code_point_count(b"\xf4\x8f\xbf\xbf"), Some(1)); // valid max
    assert_eq!(utf8_code_point_count(b"\xf5\x80\x80\x80"), None); // > U+10FFFF
    assert_eq!(utf8_code_point_count(b"\xc0\x80"), None); // 2-byte overlong rejected
    assert_eq!(utf8_code_point_count(b"\xc2"), None); // truncated
    assert_eq!(utf8_code_point_count(b"abc"), Some(3));
    assert_eq!(utf8_code_point_count("\u{4f60}".as_bytes()), Some(1)); // 你
}

#[test]
fn c_abi_accepts_and_reencodes_frames() {
    let bytes = encode_key_request(&key_request_input()).expect("encoded");
    assert_eq!(
        // SAFETY: `bytes` is a valid buffer of `bytes.len()` elements.
        unsafe { fcitx5_protocol_core_accepts_frame(bytes.as_ptr(), bytes.len()) },
        1
    );
    let mut output = vec![0u8; bytes.len()];
    let mut output_length = 0usize;
    assert_eq!(
        // SAFETY: all pointers point to valid buffers of the declared sizes.
        unsafe {
            fcitx5_protocol_core_reencode_frame(
                bytes.as_ptr(),
                bytes.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut output_length,
            )
        },
        1
    );
    assert_eq!(output_length, bytes.len());
    assert_eq!(output, bytes);

    let mut bad = bytes.clone();
    bad[4] = (VERSION + 1) as u8;
    assert_eq!(
        // SAFETY: `bad` is a valid buffer of `bad.len()` elements.
        unsafe { fcitx5_protocol_core_accepts_frame(bad.as_ptr(), bad.len()) },
        0
    );
}
