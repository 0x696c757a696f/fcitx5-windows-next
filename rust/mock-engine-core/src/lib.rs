#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};

use fcitx5_protocol_core as protocol;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub test_clients: u32,
    pub pipe: Option<OsString>,
    pub ready_event: Option<OsString>,
    pub stop_event: Option<OsString>,
    pub generation: Option<OsString>,
    pub composition_test: bool,
}

pub fn parse_options<I>(args: I) -> Result<Options, String>
where
    I: IntoIterator<Item = OsString>,
{
    let mut options = Options::default();
    let mut args = args.into_iter();
    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--test-once") => options.test_clients = 1,
            Some("--safe-mode") => {}
            Some("--composition-test") => options.composition_test = true,
            Some("--test-clients") => options.test_clients = parse_count(args.next())?,
            Some("--pipe") => options.pipe = Some(required_value(args.next(), "--pipe")?),
            Some("--ready-event") => {
                options.ready_event = Some(required_value(args.next(), "--ready-event")?)
            }
            Some("--stop-event") => {
                options.stop_event = Some(required_value(args.next(), "--stop-event")?)
            }
            Some("--generation") => {
                options.generation = Some(required_value(args.next(), "--generation")?)
            }
            _ => return Err(usage()),
        }
    }
    Ok(options)
}

fn required_value(value: Option<OsString>, option: &str) -> Result<OsString, String> {
    value
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing value for {option}"))
}

fn parse_count(value: Option<OsString>) -> Result<u32, String> {
    let value = required_value(value, "--test-clients")?;
    let count = value
        .to_str()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|count| (1..=64).contains(count));
    count.ok_or_else(usage)
}

pub fn usage() -> String {
    "Usage: fcitx5-mock-engine [--test-once|--test-clients N] [--pipe NAME] [--ready-event NAME] [--stop-event NAME] [--generation GENERATION]".to_owned()
}

#[derive(Debug, Clone, Copy)]
pub struct ResponseContext {
    pub epoch: u64,
    pub response_id: u64,
    pub session_id: u32,
    pub client_process_id: u32,
    pub composition_test: bool,
}

#[derive(Debug, Default)]
pub struct ClientState {
    pub handshake: bool,
    pub last_request_id: u64,
}

pub fn response_for(
    request: &[u8],
    context: ResponseContext,
    state: &mut ClientState,
) -> Option<Vec<u8>> {
    let frame = protocol::decode_frame(request)?;
    if frame.metadata.request_id <= state.last_request_id {
        return None;
    }
    match frame.message_type {
        protocol::MessageType::HelloRequest => {
            let request = protocol::decode_hello_request(&frame)?;
            if state.handshake
                || request.metadata.session_id != context.session_id
                || request.client_process_id != context.client_process_id
            {
                return None;
            }
            state.handshake = true;
            state.last_request_id = request.metadata.request_id;
            protocol::encode_hello_response(&protocol::HelloResponse {
                metadata: response_metadata(
                    &request.metadata,
                    context.response_id,
                    context.epoch,
                    context.session_id,
                ),
                status: protocol::Status::Ok,
                server_architecture_bits: (std::mem::size_of::<usize>() * 8) as u32,
            })
        }
        protocol::MessageType::KeyRequest => {
            let request = protocol::decode_key_request(&frame)?;
            if !state.handshake
                || request.metadata.engine_epoch != context.epoch
                || request.metadata.session_id != context.session_id
            {
                return None;
            }
            state.last_request_id = request.metadata.request_id;
            let mut response = protocol::KeyResponse {
                metadata: response_metadata(
                    &request.metadata,
                    context.response_id,
                    context.epoch,
                    context.session_id,
                ),
                status: protocol::Status::Ok,
                caret: request.caret,
                ..protocol::KeyResponse::default()
            };
            response.metadata.revision = request.metadata.revision + 1;
            response.content_locale_utf8 = locale(&request.input_method_utf8);
            if context.composition_test && request.virtual_key == u32::from(b'D') {
                if !request.surrounding_text_valid {
                    return None;
                }
                response.handled = true;
                response.delete_surrounding_text = true;
                response.delete_surrounding_offset = -1;
                response.delete_surrounding_size = 1;
            } else if context.composition_test && request.virtual_key == u32::from(b'F') {
                response.handled = true;
                response.forward_key = true;
                response.forward_key_sym = u32::from(b'f');
                response.forward_key_code = request.scan_code as i32;
                response.forward_key_release = request.key_flags & protocol::KEY_FLAG_RELEASE != 0;
            } else if context.composition_test && request.virtual_key == u32::from(b'N') {
                if request.scan_code == 0
                    || request.keyboard_layout == 0
                    || (!request.input_method_utf8.is_empty()
                        && request.input_method_utf8 != b"mozc")
                    || !request.surrounding_text_valid
                {
                    return None;
                }
                response.handled = true;
                response.preedit_utf8 = b"n".to_vec();
                response.preedit_caret_utf8 = 1;
                response.candidates = vec![
                    protocol::CandidateRecord {
                        id: 101,
                        label_utf8: b"1".to_vec(),
                        text_utf8: "你".as_bytes().to_vec(),
                        comment_utf8: b"ni".to_vec(),
                    },
                    protocol::CandidateRecord {
                        id: 102,
                        label_utf8: b"2".to_vec(),
                        text_utf8: "呢".as_bytes().to_vec(),
                        comment_utf8: b"ne".to_vec(),
                    },
                ];
                response.selected_candidate = 0;
                response.candidate_total = 2;
                response.candidate_visibility = 1;
                response.candidate_page_size = 2;
            } else if context.composition_test && request.virtual_key == 0x20 {
                if request.popup_allowed {
                    return None;
                }
                response.handled = true;
                response.commit_utf8 = "你".as_bytes().to_vec();
            } else if (u32::from(b'A')..=u32::from(b'Z')).contains(&request.virtual_key) {
                response.handled = true;
                response
                    .commit_utf8
                    .push(b'a' + (request.virtual_key - u32::from(b'A')) as u8);
            }
            protocol::encode_key_response(&response)
        }
        protocol::MessageType::EngineStatusRequest => {
            let request = protocol::decode_engine_status_request(&frame)?;
            if !state.handshake
                || request.metadata.engine_epoch != context.epoch
                || request.metadata.session_id != context.session_id
            {
                return None;
            }
            state.last_request_id = request.metadata.request_id;
            protocol::encode_engine_status_response(&protocol::EngineStatusResponse {
                metadata: response_metadata(
                    &request.metadata,
                    context.response_id,
                    context.epoch,
                    context.session_id,
                ),
                status: protocol::Status::Ok,
                current_input_method_id: b"mock-pinyin".to_vec(),
                current_input_method_name: "Mock Pinyin".as_bytes().to_vec(),
                current_input_method_native_name: "小企鹅".as_bytes().to_vec(),
                current_input_method_short_label: "小".as_bytes().to_vec(),
            })
        }
        _ => None,
    }
}

fn response_metadata(
    request: &protocol::Metadata,
    response_id: u64,
    epoch: u64,
    session_id: u32,
) -> protocol::Metadata {
    protocol::Metadata {
        request_id: response_id,
        response_to: request.request_id,
        engine_epoch: epoch,
        session_id,
        context_id: request.context_id,
        composition_id: request.composition_id,
        revision: request.revision,
    }
}

fn locale(input_method: &[u8]) -> Vec<u8> {
    match input_method {
        b"mozc" => b"ja-JP".to_vec(),
        b"hangul" => b"ko-KR".to_vec(),
        [] => Vec::new(),
        _ => b"zh-CN".to_vec(),
    }
}

pub fn default_pipe(
    identity: &fcitx5_windows_common_core::CurrentUserRuntimeIdentity,
) -> Option<OsString> {
    let generation = fcitx5_windows_common_core::current_runtime_generation_for_current_process();
    identity.local_endpoint_name(&generation, "engine")
}

pub fn is_pipe_name(value: &OsStr) -> bool {
    !value.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_test_client_and_generation_options() {
        let options = parse_options([
            OsString::from("--test-clients"),
            OsString::from("3"),
            OsString::from("--generation"),
            OsString::from("next"),
        ])
        .expect("options should parse");
        assert_eq!(options.test_clients, 3);
        assert_eq!(options.generation, Some(OsString::from("next")));
    }

    #[test]
    fn rejects_invalid_client_count() {
        assert!(parse_options([OsString::from("--test-clients"), OsString::from("65")]).is_err());
    }

    #[test]
    fn maps_basic_key_to_lowercase_commit() {
        let request = protocol::KeyRequest {
            metadata: protocol::Metadata {
                request_id: 1,
                engine_epoch: 7,
                session_id: 9,
                context_id: 1,
                ..protocol::Metadata::default()
            },
            virtual_key: u32::from(b'Z'),
            ..protocol::KeyRequest::default()
        };
        let bytes = protocol::encode_key_request(&request).expect("request should encode");
        let mut state = ClientState {
            handshake: true,
            ..ClientState::default()
        };
        let response_bytes = response_for(
            &bytes,
            ResponseContext {
                epoch: 7,
                response_id: 2,
                session_id: 9,
                client_process_id: 1,
                composition_test: false,
            },
            &mut state,
        )
        .expect("response should encode");
        let frame = protocol::decode_frame(&response_bytes).expect("response frame should decode");
        let response = protocol::decode_key_response(&frame).expect("response should decode");
        assert_eq!(response.commit_utf8, b"z");
        assert!(response.handled);
    }

    #[test]
    fn hello_rejects_a_process_id_that_does_not_match_the_verified_pipe_peer() {
        let request = protocol::HelloRequest {
            metadata: protocol::Metadata {
                request_id: 1,
                session_id: 9,
                ..protocol::Metadata::default()
            },
            client_process_id: 41,
            client_architecture_bits: usize::BITS,
        };
        let bytes = protocol::encode_hello_request(&request).expect("hello should encode");
        let mut state = ClientState::default();
        assert!(response_for(
            &bytes,
            ResponseContext {
                epoch: 7,
                response_id: 2,
                session_id: 9,
                client_process_id: 42,
                composition_test: false,
            },
            &mut state,
        )
        .is_none());
        assert!(!state.handshake);
        assert_eq!(state.last_request_id, 0);
    }
}
