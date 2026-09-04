#![forbid(unsafe_code)]

//! Port of the deleted C++ `launcher_crash_loop_test.cpp`: spawns the real
//! shipping `fcitx5-launcher.exe` supervising the real crash-engine fixture,
//! drives a start demand, and asserts the launcher converges to Safe Mode with
//! a ready engine instead of respawning forever (the crash-loop regression the
//! C++ test covered).

mod support;

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use fcitx5_ipc_client::{send_launcher_command, LauncherOutcome};
use fcitx5_protocol_core as protocol;
use fcitx5_windows_common_core::{CurrentUserRuntimeIdentity, NamedEvent};

use support::{create_event, spawn_nowindow, wait_event};

const K_LAUNCHER_SAFE_MODE: u32 = 5;
const K_ENGINE_READY: u32 = 2;

struct CrashHarness {
    launcher: PathBuf,
    launcher_ready: NamedEvent,
    /// Kept alive so the named stop event stays open for the launcher.
    _stop: NamedEvent,
    safe: NamedEvent,
    state_file: PathBuf,
    child: std::process::Child,
}

impl CrashHarness {
    fn start(launcher: PathBuf, crash_engine: PathBuf) -> Self {
        let identity = CurrentUserRuntimeIdentity::current().expect("current identity");
        assert!(
            identity.session_id() != 0,
            "launcher crash-loop E2E requires an interactive session"
        );
        let suffix = std::process::id();
        let namespace = format!("crash-{suffix}");
        std::env::set_var("FCITX5_TEST_NAMESPACE", &namespace);

        let launcher_ready_name = format!("Local\\Fcitx5WindowsNext.CrashTest.Launcher.{suffix}");
        let stop_name = format!("Local\\Fcitx5WindowsNext.CrashTest.Stop.{suffix}");
        let safe_name = format!("Local\\Fcitx5WindowsNext.CrashTest.Safe.{suffix}");

        let launcher_ready = create_event(&launcher_ready_name);
        let stop = create_event(&stop_name);
        let safe = create_event(&safe_name);

        // The crash fixture signals this event only when the launcher starts
        // it in Safe Mode; env is inherited by the launcher and engine child.
        std::env::set_var("FCITX_TEST_SAFE_MODE_EVENT", &safe_name);

        let state_file =
            std::env::temp_dir().join(format!("fcitx5-crash-loop-test-{suffix}.state"));
        let _ = std::fs::remove_file(&state_file);

        let arguments = vec![
            OsString::from("--engine"),
            OsString::from(crash_engine.to_string_lossy().as_ref()),
            OsString::from("--no-warmup"),
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
            launcher_ready,
            _stop: stop,
            safe,
            state_file,
            child,
        };
        assert!(
            wait_event(&harness.launcher_ready, 2000),
            "launcher readiness"
        );
        harness
    }

    fn send(&self, command: protocol::LauncherCommand) -> Option<LauncherOutcome> {
        send_launcher_command(&self.launcher, command)
    }
}

#[test]
fn launcher_crash_loop_converges_to_safe_mode() {
    let launcher = fcitx5_ipc_client::launcher_exe_path();
    let crash_engine = fcitx5_ipc_client::crash_engine_exe_path();
    let harness = CrashHarness::start(launcher, crash_engine);

    // One start demand drives the crash loop: the launcher restarts the
    // crashing engine internally and must converge to Safe Mode on its own.
    let demand = harness.send(protocol::LauncherCommand::StartDemand);
    assert!(demand.is_some(), "start demand must be accepted");

    // The crash fixture signals the safe event once it is started in Safe
    // Mode; allow the launcher's crash accounting to reach the threshold.
    assert!(
        wait_event(&harness.safe, 5000),
        "crash loop must converge to a Safe Mode engine start"
    );

    let status = harness
        .send(protocol::LauncherCommand::Status)
        .expect("status response");
    assert_eq!(status.status, protocol::Status::Ok, "status must be Ok");
    assert_eq!(
        status.launcher_state, K_LAUNCHER_SAFE_MODE,
        "launcher must be in Safe Mode after the crash loop"
    );
    assert_eq!(
        status.engine_state, K_ENGINE_READY,
        "Safe Mode engine must be ready"
    );

    let shutdown = harness.send(protocol::LauncherCommand::Shutdown);
    assert!(shutdown.is_some(), "shutdown command must be accepted");

    let mut child = harness.child;
    let deadline = std::time::Instant::now() + Duration::from_secs(4);
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
    assert_eq!(exit, Some(0), "launcher must exit 0 after shutdown");
    let _ = std::fs::remove_file(&harness.state_file);
    std::env::remove_var("FCITX_TEST_SAFE_MODE_EVENT");
    std::env::remove_var("FCITX5_TEST_NAMESPACE");
}
