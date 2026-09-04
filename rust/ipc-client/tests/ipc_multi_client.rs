#![forbid(unsafe_code)]

//! Port of the deleted C++ `ipc_multi_client_test.cpp`: the mock engine serves
//! `N` simultaneous established clients; each performs a setup key and a second
//! key, and all N commits must be correct before the engine exits 0.

mod support;

use fcitx5_ipc_client::EngineClient;

use support::{engine_exit_ok, pipe_name, process_suffix, spawn_engine, wait_ready};

fn run_multi_client(tag: &str, client_count: u32) {
    assert!(
        (1..=64).contains(&client_count),
        "client count must be in [1, 64]"
    );
    let (mut child, ready) = spawn_engine(tag, client_count);
    assert!(wait_ready(&ready), "mock engine readiness");

    let suffix = process_suffix();
    let pipe = pipe_name(tag, &suffix);
    let engine = fcitx5_ipc_client::mock_engine_path();

    // Establish all N clients sequentially (one key each to bring each pipe
    // up), then fire a second key from every client with all connections live.
    let mut clients: Vec<EngineClient> = Vec::with_capacity(client_count as usize);
    for index in 0..client_count {
        let key = u32::from(b'A') + (index % 26);
        let mut client = EngineClient::new(pipe.clone(), engine.clone()).expect("client");
        let outcome = client
            .process_key(u64::from(index) + 1, key, 0, 0)
            .expect("setup key succeeds");
        assert!(outcome.handled, "setup key {index} must be handled");
        assert_eq!(
            outcome.commit,
            vec![b'a' + u8::try_from(key - u32::from(b'A')).unwrap()],
            "setup key {index} commit mismatch"
        );
        clients.push(client);
    }

    // All connections live: fire the second key on each distinct context.
    let mut succeeded = 0_u32;
    for (index, client) in clients.iter_mut().enumerate() {
        let key = u32::from(b'A') + u32::try_from(index % 26).unwrap();
        let outcome = client
            .process_key(0x1000_0000 + u64::try_from(index).unwrap(), key, 0, 0)
            .expect("second key succeeds");
        if outcome.handled
            && outcome.commit == vec![b'a' + u8::try_from(key - u32::from(b'A')).unwrap()]
        {
            succeeded += 1;
        }
    }
    assert_eq!(
        succeeded, client_count,
        "all {client_count} concurrent second keys must succeed"
    );

    // Drop all clients so the engine's completed-client count reaches N.
    drop(clients);
    assert!(
        engine_exit_ok(&mut child),
        "mock engine must exit 0 after {client_count} clients disconnect"
    );
}

#[test]
fn multi_client_default_four() {
    // Distinct pipe tags keep the two tests in this file collision-free when
    // the harness runs them in parallel (they share one process id suffix).
    run_multi_client("Multi4", 4);
}

#[test]
fn multi_client_thirty_two() {
    run_multi_client("Multi32", 32);
}
