#![forbid(unsafe_code)]

//! Port of the deleted C++ `launcher_integration_test.cpp`: spawns the real
//! shipping `fcitx5-launcher.exe` supervising the real mock engine over the
//! real launcher pipe, drives the launcher state machine
//! (start-demand / engine-ready / status / user-stop / resume / shutdown) and
//! asserts the exact launcherState/engineState/startDisposition + mock-pinyin
//! engine-status + `'A' -> "a"` key-commit contract the C++ test covered.

mod support;

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use fcitx5_ipc_client::{send_launcher_command, EngineClient};
use fcitx5_protocol_core as protocol;
use fcitx5_windows_common_core::CurrentUserRuntimeIdentity;

use support::{create_event, endpoint_name, spawn_nowindow, wait_event};

const K_LAUNCHER_USER_STOPPED: u32 = 1;
const K_START_SUPPRESSED: u32 = 2;

struct LauncherHarness {
    launcher: PathBuf,
    engine: PathBuf,
    engine_ready: fcitx5_windows_common_core::NamedEvent,
    launcher_ready: fcitx5_windows_common_core::NamedEvent,
    /// Kept alive so the named stop event stays open for the launcher.
    _stop: fcitx5_windows_common_core::NamedEvent,
    state_file: PathBuf,
    child: std::process::Child,
}

impl LauncherHarness {
    fn start(launcher: PathBuf, engine: PathBuf, tag: &str) -> Self {
        let identity = CurrentUserRuntimeIdentity::current().expect("current identity");
        assert!(
            identity.session_id() != 0,
            "launcher E2E requires an interactive session"
        );
        let suffix = std::process::id();
        let namespace = format!("{tag}-{suffix}");
        std::env::set_var("FCITX5_TEST_NAMESPACE", &namespace);

        let engine_ready_name =
            format!("Local\\Fcitx5WindowsNext.LauncherTest.EngineReady.{suffix}");
        let launcher_ready_name =
            format!("Local\\Fcitx5WindowsNext.LauncherTest.LauncherReady.{suffix}");
        let stop_name = format!("Local\\Fcitx5WindowsNext.LauncherTest.Stop.{suffix}");

        let engine_ready = create_event(&engine_ready_name);
        let launcher_ready = create_event(&launcher_ready_name);
        let stop = create_event(&stop_name);

        let state_file = std::env::temp_dir().join(format!("fcitx5-launcher-test-{suffix}.state"));
        let _ = std::fs::remove_file(&state_file);

        let arguments = vec![
            OsString::from("--engine"),
            OsString::from(engine.to_string_lossy().as_ref()),
            OsString::from("--no-warmup"),
            OsString::from("--engine-ready-event"),
            OsString::from(&engine_ready_name),
            OsString::from("--ready-event"),
            OsString::from(&launcher_ready_name),
            OsString::from("--stop-event"),
            OsString::from(&stop_name),
            OsString::from("--state-file"),
            OsString::from(state_file.to_string_lossy().as_ref()),
        ];
        let child = spawn_nowindow(&launcher, &arguments);
        let harness = Self {
            launcher,
            engine,
            engine_ready,
            launcher_ready,
            _stop: stop,
            state_file,
            child,
        };
        assert!(
            wait_event(&harness.launcher_ready, 2000),
            "launcher readiness"
        );
        harness
    }

    fn send_and_await_next(
        &self,
        command: protocol::LauncherCommand,
    ) -> Option<fcitx5_ipc_client::LauncherOutcome> {
        let _ = self.launcher_ready.reset();
        let outcome = send_launcher_command(&self.launcher, command)?;
        assert!(
            wait_event(&self.launcher_ready, 2000),
            "launcher ready after command"
        );
        Some(outcome)
    }
}

#[test]
fn launcher_engine_lifecycle() {
    let launcher = fcitx5_ipc_client::launcher_exe_path();
    let engine = fcitx5_ipc_client::mock_engine_path();
    let harness = LauncherHarness::start(launcher, engine, "launcher");

    // 1. StartDemand -> status Ok, then the engine becomes ready.
    let demand = harness
        .send_and_await_next(protocol::LauncherCommand::StartDemand)
        .expect("start demand response");
    assert_eq!(
        demand.status,
        protocol::Status::Ok,
        "start demand must be accepted"
    );
    assert!(
        wait_event(&harness.engine_ready, 2000),
        "engine readiness after start demand"
    );

    // 2. Direct engine key roundtrip: 'A' -> handled + "a".
    let endpoint = endpoint_name("engine");
    let mut client = EngineClient::new(endpoint, harness.engine.clone()).expect("engine client");
    let key = client
        .process_key(1, u32::from(b'A'), 0, 0)
        .expect("key roundtrip succeeds");
    assert!(key.handled, "key must be handled");
    assert_eq!(key.commit, b"a", "key must commit the lowercased key");
    client.close();

    // 3. Status while the engine is ready reports the mock-pinyin identity.
    let status = harness
        .send_and_await_next(protocol::LauncherCommand::Status)
        .expect("status response");
    assert_eq!(status.status, protocol::Status::Ok, "status must be Ok");
    assert_eq!(
        status.current_input_method_id, b"mock-pinyin",
        "engine input method id"
    );
    assert_eq!(
        status.current_input_method_name, b"Mock Pinyin",
        "engine input method name"
    );
    assert_eq!(
        status.current_input_method_native_name,
        "小企鹅".as_bytes(),
        "engine native name"
    );
    assert_eq!(
        status.current_input_method_short_label,
        "小".as_bytes(),
        "engine short label"
    );

    // 4. UserStop -> launcherState UserStopped; a fresh StartDemand is
    //    suppressed while the launcher is user-stopped.
    let user_stop = harness
        .send_and_await_next(protocol::LauncherCommand::UserStop)
        .expect("user stop response");
    assert_eq!(
        user_stop.status,
        protocol::Status::Ok,
        "user stop must be Ok"
    );
    assert_eq!(
        user_stop.launcher_state, K_LAUNCHER_USER_STOPPED,
        "launcher must enter UserStopped"
    );
    let suppressed = harness
        .send_and_await_next(protocol::LauncherCommand::StartDemand)
        .expect("suppressed start response");
    assert_eq!(suppressed.status, protocol::Status::Ok);
    assert_eq!(
        suppressed.start_disposition, K_START_SUPPRESSED,
        "start demand must be suppressed while user-stopped"
    );

    // 5. Resume -> Ok; a fresh StartDemand restarts the engine.
    let resume = harness
        .send_and_await_next(protocol::LauncherCommand::Resume)
        .expect("resume response");
    assert_eq!(resume.status, protocol::Status::Ok, "resume must be Ok");
    let demand2 = harness
        .send_and_await_next(protocol::LauncherCommand::StartDemand)
        .expect("second start demand response");
    assert_eq!(demand2.status, protocol::Status::Ok);
    assert!(
        wait_event(&harness.engine_ready, 2000),
        "engine ready after resume + start demand"
    );

    // 6. Shutdown -> launcher exits 0.
    let shutdown = send_launcher_command(&harness.launcher, protocol::LauncherCommand::Shutdown);
    assert!(shutdown.is_some(), "shutdown command must be accepted");
    let mut child = harness.child;
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let exit = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                break Some(-1);
            }
        }
    };
    assert_eq!(exit, Some(0), "launcher must exit 0");
    let _ = std::fs::remove_file(&harness.state_file);
    std::env::remove_var("FCITX5_TEST_NAMESPACE");
}
