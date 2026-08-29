#![forbid(unsafe_code)]

use fcitx5_engine_core::{decode_request, protocol, Request};

#[test]
fn protocol_decodes_hello_through_engine_boundary() {
    let expected = protocol::HelloRequest {
        metadata: protocol::Metadata {
            request_id: 7,
            session_id: 42,
            ..protocol::Metadata::default()
        },
        client_architecture_bits: 64,
        client_process_id: 1_234,
    };
    let bytes = protocol::encode_hello_request(&expected).expect("valid hello request");

    let decoded = decode_request(&bytes).expect("engine accepts valid hello request");

    assert_eq!(decoded, Request::Hello(expected));
}

#[test]
fn protocol_decodes_remaining_requests_through_engine_boundary() {
    let key_metadata = |request_id| protocol::Metadata {
        request_id,
        engine_epoch: 1,
        session_id: 42,
        context_id: 9,
        ..protocol::Metadata::default()
    };
    let key = protocol::KeyRequest {
        metadata: key_metadata(8),
        virtual_key: 0x41,
        scan_code: 0x1e,
        logical_text_utf8: b"a".to_vec(),
        ..protocol::KeyRequest::default()
    };
    let launcher = protocol::LauncherRequest {
        metadata: protocol::Metadata {
            request_id: 9,
            session_id: 42,
            ..protocol::Metadata::default()
        },
        command: protocol::LauncherCommand::Status,
    };
    let candidate_select = protocol::CandidateSelectRequest {
        metadata: key_metadata(10),
        target_process_id: 1_234,
        candidate_id: 7,
    };
    let state = protocol::StateRequest {
        metadata: key_metadata(11),
    };
    let engine_status = protocol::EngineStatusRequest {
        metadata: protocol::Metadata {
            request_id: 12,
            engine_epoch: 1,
            session_id: 42,
            ..protocol::Metadata::default()
        },
    };
    let cases = [
        (
            "key",
            protocol::encode_key_request(&key).expect("valid key request"),
            Request::Key(key),
        ),
        (
            "launcher",
            protocol::encode_launcher_request(&launcher).expect("valid launcher request"),
            Request::Launcher(launcher),
        ),
        (
            "candidate select",
            protocol::encode_candidate_select_request(&candidate_select)
                .expect("valid candidate select request"),
            Request::CandidateSelect(candidate_select),
        ),
        (
            "state",
            protocol::encode_state_request(&state).expect("valid state request"),
            Request::State(state),
        ),
        (
            "engine status",
            protocol::encode_engine_status_request(&engine_status)
                .expect("valid engine status request"),
            Request::EngineStatus(engine_status),
        ),
    ];

    for (name, bytes, expected) in cases {
        assert_eq!(decode_request(&bytes), Some(expected), "{name}");
    }
}

#[test]
fn protocol_rejects_malformed_request_through_engine_boundary() {
    let expected = protocol::HelloRequest {
        metadata: protocol::Metadata {
            request_id: 7,
            session_id: 42,
            ..protocol::Metadata::default()
        },
        client_architecture_bits: 64,
        client_process_id: 1_234,
    };
    let mut bytes = protocol::encode_hello_request(&expected).expect("valid hello request");
    bytes.pop();

    assert_eq!(decode_request(&bytes), None);
}

#[test]
fn protocol_rejects_response_as_engine_request() {
    let expected = protocol::HelloResponse {
        metadata: protocol::Metadata {
            request_id: 8,
            response_to: 7,
            engine_epoch: 1,
            session_id: 42,
            ..protocol::Metadata::default()
        },
        status: protocol::Status::Ok,
        server_architecture_bits: 64,
    };
    let bytes = protocol::encode_hello_response(&expected).expect("valid hello response");

    assert_eq!(decode_request(&bytes), None);
}
