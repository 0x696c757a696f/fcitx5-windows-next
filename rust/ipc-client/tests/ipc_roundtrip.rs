#![forbid(unsafe_code)]

//! Port of the deleted C++ `ipc_roundtrip_test.cpp`: missing engine fails
//! open fast, a stalled engine fails open after the cold-start deadline, and
//! the real mock engine returns the key/status roundtrip.

mod support;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fcitx5_ipc_client::EngineClient;
use fcitx5_windows_common_core::{
    deadline_after, deadline_has_time_remaining, CurrentUserRuntimeIdentity, NamedPipeServer,
};

use support::{
    assert_mock_pinyin_status, create_event, engine_exit_ok, pipe_name, process_suffix,
    spawn_engine, wait_ready,
};

#[test]
fn missing_engine_fails_open_within_the_bound() {
    // No fixture is spawned: the pipe does not exist. The client must fail
    // open quickly (the C++ bound is 100 ms of wall time).
    let suffix = process_suffix();
    let missing = pipe_name("Missing", &suffix);
    let expected = fcitx5_ipc_client::mock_engine_path();
    let mut client = EngineClient::new(missing, expected).expect("client");
    let started = Instant::now();
    let outcome = client.process_key(1, u32::from(b'A'), 0, 0);
    let elapsed = started.elapsed();
    assert!(outcome.is_none(), "missing engine must fail open");
    assert!(
        elapsed < Duration::from_millis(2000),
        "missing engine must fail fast, took {elapsed:?}"
    );
}

#[test]
fn stalled_engine_fails_open_after_the_cold_start_deadline() {
    // A fake server accepts the pipe and reads the client's request but never
    // answers; the client must time out at the cold-start deadline and report
    // not-handled, and the server must have observed the request.
    let suffix = process_suffix();
    let stalled_pipe = pipe_name("Stalled", &suffix);
    let release_name = format!("Local\\Fcitx5WindowsNext.Stalled.Release.{suffix}");
    let stop_name = format!("Local\\Fcitx5WindowsNext.Stalled.Stop.{suffix}");

    let expected = std::env::current_exe().expect("current exe");
    let server_received = Arc::new(AtomicBool::new(false));
    let server_received_clone = Arc::clone(&server_received);
    let stalled_pipe_for_thread = stalled_pipe.clone();
    let server = std::thread::spawn(move || {
        // Create the security inside the thread: it is not Send.
        let identity = CurrentUserRuntimeIdentity::current().expect("identity");
        let security = identity.security_attributes().expect("security");
        let server_pipe =
            NamedPipeServer::create(stalled_pipe_for_thread.as_os_str(), &security, 4096)
                .expect("create stalled server");
        let release = create_event(&release_name);
        let stop = create_event(&stop_name);
        if server_pipe.connect_until(deadline_after(10_000), &stop) {
            let mut header = [0_u8; 64];
            if server_pipe.read_exact(&mut header, deadline_after(10_000)) {
                server_received_clone.store(true, Ordering::SeqCst);
            }
        }
        // Hold the connection open so the client's transact keeps waiting
        // until its own deadline; release it shortly after.
        let release_deadline = deadline_after(9_000);
        while !release.is_signaled() && deadline_has_time_remaining(release_deadline) {
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    let mut client = EngineClient::new(stalled_pipe, expected).expect("client");
    let started = Instant::now();
    let outcome = client.process_key(1, u32::from(b'A'), 0, 0);
    let elapsed = started.elapsed();
    server.join().expect("server thread");

    assert!(outcome.is_none(), "stalled engine must fail open");
    assert!(
        elapsed <= Duration::from_millis(8000),
        "stalled timeout must stay within the cold-start bound, took {elapsed:?}"
    );
    assert!(
        server_received.load(Ordering::SeqCst),
        "stalled server must have received the client request"
    );
}

#[test]
fn real_mock_engine_key_and_status_roundtrip() {
    let (mut child, ready) = spawn_engine("Roundtrip", 1);
    assert!(wait_ready(&ready), "mock engine readiness");

    let suffix = process_suffix();
    let pipe = pipe_name("Roundtrip", &suffix);
    let engine = fcitx5_ipc_client::mock_engine_path();
    let mut client = EngineClient::new(pipe, engine).expect("client");

    let key = client
        .process_key(7, u32::from(b'A'), 0, 0)
        .expect("key roundtrip succeeds");
    assert!(key.handled, "key must be handled");
    assert_eq!(key.commit, b"a", "key must commit the lowercased key");

    let status = client.query_status().expect("status roundtrip succeeds");
    assert_mock_pinyin_status(&status);

    // Dropping the client closes the pipe so the single-client fixture exits 0.
    client.close();
    assert!(
        engine_exit_ok(&mut child),
        "mock engine must exit 0 after the client disconnects"
    );
}
