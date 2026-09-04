#![forbid(unsafe_code)]

//! Port of the deleted C++ `ipc_generation_routing_test.cpp`: two engines on
//! different release generations each serve one client, and the client routes
//! to the correct generation-scoped pipe.

mod support;

use std::ffi::OsString;
use std::process::Child;

use fcitx5_ipc_client::{EngineClient, MockEngine};
use fcitx5_windows_common_core::CurrentUserRuntimeIdentity;

use support::{create_event, process_suffix, wait_ready};

const GENERATION_41: &str = "00000041";
const GENERATION_42: &str = "00000042";

fn start_engine(engine: &std::path::Path, generation: &str) -> (Child, String) {
    // The mock engine derives its generation-scoped default pipe from the
    // FCITX5_TEST_NAMESPACE environment variable set below plus --generation.
    let suffix = process_suffix();
    let ready_os = OsString::from(format!(
        "Local\\Fcitx5WindowsNext.GenerationRouting.{generation}.Ready.{suffix}"
    ));
    let child = MockEngine::spawn(
        engine,
        &ready_os,
        &[
            OsString::from("--test-clients"),
            OsString::from("1"),
            OsString::from("--generation"),
            OsString::from(generation),
        ],
    )
    .expect("spawn generation engine");
    (child, ready_os.to_string_lossy().into_owned())
}

fn stop_engine(child: &mut Child) -> bool {
    MockEngine::stop(child, std::time::Duration::from_secs(3))
}

fn send_key(generation: &str, context_id: u64, key: u8, expected: char) -> bool {
    let Some(pipe) = EngineClient::default_pipe(generation) else {
        return false;
    };
    let engine = fcitx5_ipc_client::mock_engine_path();
    let mut client = EngineClient::new(pipe, engine).expect("client");
    let outcome = client
        .process_key(context_id, u32::from(key), 0, 0)
        .expect("key succeeds");
    client.close();
    outcome.handled && outcome.commit == vec![expected as u8]
}

#[test]
fn generation_scoped_routing_reaches_the_correct_engine() {
    // The test namespace must match between client and engine pipe names.
    let suffix = process_suffix();
    let namespace = format!("generation-{suffix}");
    std::env::set_var("FCITX5_TEST_NAMESPACE", &namespace);

    let engine = fcitx5_ipc_client::mock_engine_path();
    let (mut gen41, ready41) = start_engine(&engine, GENERATION_41);
    let (mut gen42, ready42) = start_engine(&engine, GENERATION_42);
    let ready41_event = create_event(&ready41);
    let ready42_event = create_event(&ready42);
    assert!(wait_ready(&ready41_event), "generation 41 engine readiness");
    assert!(wait_ready(&ready42_event), "generation 42 engine readiness");

    // Sanity: identity resolves under the namespace we set.
    assert!(
        CurrentUserRuntimeIdentity::current().is_some(),
        "identity must resolve"
    );

    let mut stage = 0;
    let mut ok = send_key(GENERATION_41, 4101, b'A', 'a');
    if !ok {
        stage = 3;
    } else {
        ok = send_key(GENERATION_42, 4201, b'B', 'b');
        if !ok {
            stage = 4;
        } else {
            ok = stop_engine(&mut gen41);
            if !ok {
                stage = 5;
            } else {
                ok = stop_engine(&mut gen42);
                if !ok {
                    stage = 6;
                }
            }
        }
    }
    // Cleanup best-effort.
    let _ = gen41.kill();
    let _ = gen42.kill();

    std::env::remove_var("FCITX5_TEST_NAMESPACE");
    assert!(ok, "generation routing failed at stage {stage}");
}
