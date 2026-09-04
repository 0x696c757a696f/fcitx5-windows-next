#![forbid(unsafe_code)]

//! Port of the deleted C++ `ipc_late_response_test.cpp`: client behavior when a
//! server stalls the hello/key exchange, abruptly disconnects, then answers
//! normally across three sequential connections on the same pipe. Also verifies
//! a peer whose executable differs from the expected server is rejected.

mod support;

use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use fcitx5_ipc_client::{EngineClient, COLD_START_DEADLINE_MS};
use fcitx5_protocol_core as protocol;
use fcitx5_windows_common_core::{
    deadline_after, deadline_has_time_remaining, CurrentUserRuntimeIdentity, NamedEvent,
    NamedPipeServer,
};

use support::{create_event, process_suffix};

const HEADER_SIZE: usize = 64;
const COLD_START_SLACK_MS: u64 = COLD_START_DEADLINE_MS as u64 + 1000;

fn wait_ready_long(ready: &NamedEvent) -> bool {
    // Mirrors the C++ `WaitForSingleObject(ready, kContextStartSlackMilliseconds)`
    // bound: conn1/conn2 ready events fire only after the previous connection
    // completes, which can take the full cold-start deadline.
    let deadline = deadline_after(u32::try_from(COLD_START_SLACK_MS).unwrap_or(u32::MAX));
    while !ready.is_signaled() && deadline_has_time_remaining(deadline) {
        std::thread::sleep(Duration::from_millis(10));
    }
    ready.is_signaled()
}

fn read_frame(pipe: &NamedPipeServer, deadline: u64) -> Option<Vec<u8>> {
    let mut header = [0_u8; HEADER_SIZE];
    if !pipe.read_exact(&mut header, deadline) {
        return None;
    }
    let (_, body_size, _) = protocol::decode_header(&header)?;
    let body_size = usize::try_from(body_size).ok()?;
    let mut frame = header.to_vec();
    frame.resize(HEADER_SIZE + body_size, 0);
    if body_size != 0 && !pipe.read_exact(&mut frame[HEADER_SIZE..], deadline) {
        return None;
    }
    Some(frame)
}

#[test]
fn peer_mismatch_is_rejected_and_server_receives_nothing() {
    let suffix = process_suffix();
    let pipe_name_os = OsString::from(format!(r"\\.\pipe\Fcitx5WindowsNext.PeerMismatch.{suffix}"));
    let received = Arc::new(AtomicBool::new(false));
    let received_clone = Arc::clone(&received);

    let pipe_name_for_thread = pipe_name_os.clone();
    let server_handle = std::thread::spawn(move || {
        let identity = CurrentUserRuntimeIdentity::current().expect("identity");
        let security = identity.security_attributes().expect("security");
        let stop = create_event(&format!(
            "Local\\Fcitx5WindowsNext.PeerMismatch.Stop.{suffix}"
        ));
        let server_pipe =
            NamedPipeServer::create(pipe_name_for_thread.as_os_str(), &security, 4096)
                .expect("create server");
        if server_pipe.connect_until(deadline_after(3000), &stop) {
            let mut one = [0_u8; 1];
            // A rejected peer never sends data; a read here returns quickly.
            if server_pipe.read_exact(&mut one, deadline_after(500)) {
                received_clone.store(true, Ordering::SeqCst);
            }
        }
    });

    // Expected peer is the real mock engine; the actual server is this test
    // process, so verification must fail and the client rejects.
    let engine = fcitx5_ipc_client::mock_engine_path();
    let mut client = EngineClient::new(pipe_name_os, engine).expect("client");
    let outcome = client.process_key(1, u32::from(b'A'), 0, 0);
    server_handle.join().expect("server thread");
    assert!(
        outcome.is_none(),
        "mismatched server executable must be rejected"
    );
    assert!(
        !received.load(Ordering::SeqCst),
        "rejected peer must not send data to the server"
    );
}

#[test]
fn late_then_abrupt_then_normal_reconnect_sequence() {
    // A hand-rolled server that speaks the Rust protocol across three
    // sequential connections on one pipe name:
    //   conn0: hello ok, reads the key, holds the response until released.
    //   conn1: hello ok, reads the key, then disconnects without answering.
    //   conn2: hello ok, answers immediately with commit "a".
    let suffix = process_suffix();
    let pipe_name_os = OsString::from(format!(r"\\.\pipe\Fcitx5WindowsNext.Late.{suffix}"));

    let first_ready_name = format!("Local\\Fcitx5WindowsNext.Late.Ready0.{suffix}");
    let key_received_name = format!("Local\\Fcitx5WindowsNext.Late.Key0.{suffix}");
    let release_name = format!("Local\\Fcitx5WindowsNext.Late.Release.{suffix}");
    let second_ready_name = format!("Local\\Fcitx5WindowsNext.Late.Ready1.{suffix}");
    let abrupt_name = format!("Local\\Fcitx5WindowsNext.Late.Abrupt.{suffix}");
    let third_ready_name = format!("Local\\Fcitx5WindowsNext.Late.Ready2.{suffix}");
    let stop_name = format!("Local\\Fcitx5WindowsNext.Late.Stop.{suffix}");

    let server_error = Arc::new(AtomicI32::new(0));
    let server_error_clone = Arc::clone(&server_error);

    let pipe_name_for_thread = pipe_name_os.clone();
    let release_name_main = release_name.clone();
    let stop_name_main = stop_name.clone();
    // Client-side copies of the event names the server thread will own after
    // the `move` closure below consumes the originals.
    let first_ready_name_client = first_ready_name.clone();
    let key_received_name_client = key_received_name.clone();
    let second_ready_name_client = second_ready_name.clone();
    let third_ready_name_client = third_ready_name.clone();
    let server_handle = std::thread::spawn(move || {
        let identity = CurrentUserRuntimeIdentity::current().expect("identity");
        let security = identity.security_attributes().expect("security");
        let session_id = identity.session_id();
        let first_ready = create_event(&first_ready_name);
        let key_received = create_event(&key_received_name);
        let release = create_event(&release_name);
        let second_ready = create_event(&second_ready_name);
        let abrupt = create_event(&abrupt_name);
        let third_ready = create_event(&third_ready_name);
        let stop = create_event(&stop_name);

        let mut next_response_id = 1_u64;
        for connection_index in 0..3_u32 {
            let server_pipe =
                NamedPipeServer::create(pipe_name_for_thread.as_os_str(), &security, 4096)
                    .expect("create server");
            let ready_event = match connection_index {
                0 => &first_ready,
                1 => &second_ready,
                _ => &third_ready,
            };
            let _ = ready_event.signal();
            if !server_pipe.connect_until(deadline_after(3000), &stop) {
                server_error_clone.store(11, Ordering::SeqCst);
                return;
            }
            let Some(hello_frame) = read_frame(&server_pipe, deadline_after(3000)) else {
                server_error_clone.store(12, Ordering::SeqCst);
                return;
            };
            let Some(fview) = protocol::decode_frame(&hello_frame) else {
                server_error_clone.store(12, Ordering::SeqCst);
                return;
            };
            let Some(hello) = protocol::decode_hello_request(&fview) else {
                server_error_clone.store(12, Ordering::SeqCst);
                return;
            };
            let epoch = u64::from(connection_index) + 100;
            let hello_response = protocol::HelloResponse {
                metadata: protocol::Metadata {
                    request_id: next_response_id,
                    response_to: hello.metadata.request_id,
                    engine_epoch: epoch,
                    session_id,
                    ..protocol::Metadata::default()
                },
                status: protocol::Status::Ok,
                server_architecture_bits: (std::mem::size_of::<usize>() * 8) as u32,
            };
            next_response_id += 1;
            let response =
                protocol::encode_hello_response(&hello_response).expect("encode hello response");
            if !server_pipe.write_all(&response, deadline_after(3000)) {
                server_error_clone.store(131, Ordering::SeqCst);
                return;
            }
            let Some(key_frame) = read_frame(&server_pipe, deadline_after(3000)) else {
                server_error_clone.store(132, Ordering::SeqCst);
                return;
            };
            let Some(kview) = protocol::decode_frame(&key_frame) else {
                server_error_clone.store(132, Ordering::SeqCst);
                return;
            };
            let Some(key) = protocol::decode_key_request(&kview) else {
                server_error_clone.store(133, Ordering::SeqCst);
                return;
            };
            let commit = if connection_index == 0 { "late" } else { "a" };
            let key_response = protocol::KeyResponse {
                metadata: protocol::Metadata {
                    request_id: next_response_id,
                    response_to: key.metadata.request_id,
                    engine_epoch: epoch,
                    session_id,
                    context_id: key.metadata.context_id,
                    composition_id: key.metadata.composition_id,
                    revision: key.metadata.revision + 1,
                },
                status: protocol::Status::Ok,
                handled: true,
                commit_utf8: commit.as_bytes().to_vec(),
                caret: protocol::CaretRect::default(),
                ..protocol::KeyResponse::default()
            };
            next_response_id += 1;
            let encoded =
                protocol::encode_key_response(&key_response).expect("encode key response");
            match connection_index {
                0 => {
                    let _ = key_received.signal();
                    // Hold the response until the test releases it; the client
                    // should time out first.
                    let release_deadline =
                        deadline_after(u32::try_from(COLD_START_SLACK_MS).unwrap());
                    while !release.is_signaled() && deadline_has_time_remaining(release_deadline) {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    let _ = server_pipe.write_all(&encoded, deadline_after(1000));
                }
                1 => {
                    let _ = abrupt.signal();
                    // Disconnect without answering: drop closes the handle.
                }
                _ => {
                    if !server_pipe.write_all(&encoded, deadline_after(3000)) {
                        server_error_clone.store(15, Ordering::SeqCst);
                        return;
                    }
                    let mut extra = [0_u8; 1];
                    let _ = server_pipe.read_exact(&mut extra, deadline_after(2000));
                }
            }
            drop(server_pipe);
        }
    });

    // Client points at this test process (the server above), so peer verify
    // succeeds; the server stalls/aborts/normal across three reconnects. Each
    // key is issued only after the corresponding server connection signals
    // ready (mirrors the C++ test's per-connection readiness waits).
    let expected = std::env::current_exe().expect("current exe");
    let mut client = EngineClient::new(pipe_name_os, expected).expect("client");

    // conn0: server signals ready, reads the key, then holds the response.
    let first_ready = create_event(&first_ready_name_client);
    let key_received = create_event(&key_received_name_client);
    let second_ready = create_event(&second_ready_name_client);
    let third_ready = create_event(&third_ready_name_client);
    assert!(wait_ready_long(&first_ready), "conn0 readiness");
    let outcome0 = client.process_key(9, u32::from(b'A'), 0, 0);
    assert!(outcome0.is_none(), "first late key must time out");
    assert!(wait_ready_long(&key_received), "conn0 must receive the key");
    let release = create_event(&release_name_main);
    let _ = release.signal();
    // conn1: server signals ready, reads the key, disconnects without a response.
    assert!(wait_ready_long(&second_ready), "conn1 readiness");
    let outcome1 = client.process_key(9, u32::from(b'A'), 0, 0);
    assert!(outcome1.is_none(), "abrupt second connection must fail");
    // conn2: server signals ready and answers normally.
    assert!(wait_ready_long(&third_ready), "conn2 readiness");
    let outcome2 = client.process_key(9, u32::from(b'A'), 0, 0);
    match &outcome2 {
        Some(key) => {
            assert!(key.handled, "third key must be handled");
            assert_eq!(key.commit, b"a", "third key must commit 'a'");
        }
        None => {
            panic!("third key must succeed");
        }
    }
    let stop = create_event(&stop_name_main);
    let _ = stop.signal();
    server_handle.join().expect("server thread");
    assert_eq!(
        server_error.load(Ordering::SeqCst),
        0,
        "server must complete without error"
    );
}
