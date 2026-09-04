#![forbid(unsafe_code)]

//! Port of the deleted C++ `ipc_idle_client_test.cpp`: established-but-idle
//! connections must not starve other clients' requests. The engine runs a pool
//! of pipe workers; this holds 8 idle connections open while 8 active clients
//! each send one key, and verifies all 8 active commits succeed.

mod support;

use fcitx5_ipc_client::EngineClient;

use support::{engine_exit_ok, pipe_name, process_suffix, spawn_engine, wait_ready};

const IDLE_COUNT: u32 = 8;
const ACTIVE_COUNT: u32 = 8;
const TOTAL_COUNT: u32 = IDLE_COUNT + ACTIVE_COUNT;

#[test]
fn idle_clients_do_not_starve_active_requests() {
    let (mut child, ready) = spawn_engine("Idle", TOTAL_COUNT);
    assert!(wait_ready(&ready), "mock engine readiness");

    let suffix = process_suffix();
    let pipe = pipe_name("Idle", &suffix);
    let engine = fcitx5_ipc_client::mock_engine_path();

    // Open the 8 idle connections first and keep them alive without further
    // traffic while the active batch runs.
    let mut idle: Vec<EngineClient> = Vec::with_capacity(IDLE_COUNT as usize);
    for index in 0..IDLE_COUNT {
        let key = u32::from(b'Z') - (index % 26);
        let mut client = EngineClient::new(pipe.clone(), engine.clone()).expect("client");
        let outcome = client
            .process_key(0x4000_0000 + u64::from(index), key, 0, 0)
            .expect("idle setup key succeeds");
        assert!(outcome.handled, "idle client {index} setup must be handled");
        idle.push(client);
    }

    // The active clients each open a fresh connection and send one key.
    let mut succeeded = 0_u32;
    for index in 0..ACTIVE_COUNT {
        let key = u32::from(b'A') + index;
        let mut client = EngineClient::new(pipe.clone(), engine.clone()).expect("active client");
        let outcome = client
            .process_key(u64::from(index) + 1, key, 0, 0)
            .expect("active key succeeds");
        if outcome.handled && outcome.commit == vec![b'a' + u8::try_from(index).unwrap()] {
            succeeded += 1;
        }
    }
    assert_eq!(
        succeeded, ACTIVE_COUNT,
        "all {ACTIVE_COUNT} active keys must succeed while idle clients hold workers"
    );

    // Release the idle connections so the engine reaches TOTAL_COUNT.
    drop(idle);
    assert!(
        engine_exit_ok(&mut child),
        "mock engine must exit 0 after all connections disconnect"
    );
}
