#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use fcitx5_launcher_core::{
    supervise_engine, EngineLaunch, EngineLifecycleAdapter, EngineLifecycleResult, EngineReadyState,
};

struct FakeAdapter {
    calls: Vec<&'static str>,
    start_result: EngineLifecycleResult<()>,
    ready_result: EngineLifecycleResult<EngineReadyState>,
    wait_result: EngineLifecycleResult<bool>,
    terminate_result: EngineLifecycleResult<()>,
}

impl Default for FakeAdapter {
    fn default() -> Self {
        Self {
            calls: Vec::new(),
            start_result: Ok(()),
            ready_result: Ok(EngineReadyState::Ready),
            wait_result: Ok(true),
            terminate_result: Ok(()),
        }
    }
}

impl EngineLifecycleAdapter for FakeAdapter {
    type Child = ();

    fn start_engine(&mut self, _launch: &EngineLaunch) -> EngineLifecycleResult<Self::Child> {
        self.calls.push("start");
        self.start_result.clone()
    }

    fn wait_for_ready(
        &mut self,
        _child: &mut Self::Child,
        _ready_event: &std::ffi::OsStr,
        _timeout: Duration,
    ) -> EngineLifecycleResult<EngineReadyState> {
        self.calls.push("ready");
        self.ready_result.clone()
    }

    fn signal_stop(&mut self, _stop_event: &std::ffi::OsStr) -> EngineLifecycleResult<()> {
        self.calls.push("stop");
        Ok(())
    }

    fn wait_for_exit(
        &mut self,
        _child: &mut Self::Child,
        _timeout: Duration,
    ) -> EngineLifecycleResult<bool> {
        self.calls.push("wait");
        self.wait_result.clone()
    }

    fn terminate(&mut self, _child: &mut Self::Child) -> EngineLifecycleResult<()> {
        self.calls.push("terminate");
        self.terminate_result.clone()
    }
}

fn launch() -> EngineLaunch {
    EngineLaunch::new(
        PathBuf::from(r"C:\Fcitx5\bin\fcitx5-engine.exe"),
        OsString::from("ready-event"),
        OsString::from("stop-event"),
        OsString::from("generation-1"),
        false,
    )
}

#[test]
fn supervisor_starts_waits_for_ready_then_stops_and_reaps_the_engine() {
    let launch = launch();
    let mut adapter = FakeAdapter::default();

    let result = supervise_engine(
        &mut adapter,
        &launch,
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .expect("engine lifecycle should complete");

    assert_eq!(result.ready, EngineReadyState::Ready);
    assert!(!result.forced_termination);
    assert_eq!(adapter.calls, ["start", "ready", "stop", "wait"]);
}

#[test]
fn supervisor_returns_launch_failure_without_waiting_for_readiness() {
    let mut adapter = FakeAdapter {
        start_result: Err("launch failed".to_owned()),
        ..FakeAdapter::default()
    };

    let error = supervise_engine(
        &mut adapter,
        &launch(),
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .expect_err("launch failure should be returned");

    assert_eq!(error, "launch failed");
    assert_eq!(adapter.calls, ["start"]);
}

#[test]
fn supervisor_terminates_after_readiness_timeout_or_failure() {
    let mut adapter = FakeAdapter {
        ready_result: Err("readiness timed out".to_owned()),
        ..FakeAdapter::default()
    };

    let error = supervise_engine(
        &mut adapter,
        &launch(),
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .expect_err("readiness failure should be returned");

    assert_eq!(error, "readiness timed out");
    assert_eq!(adapter.calls, ["start", "ready", "terminate"]);
}

#[test]
fn supervisor_forces_termination_when_engine_does_not_exit() {
    let mut adapter = FakeAdapter {
        wait_result: Ok(false),
        ..FakeAdapter::default()
    };

    let result = supervise_engine(
        &mut adapter,
        &launch(),
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .expect("engine should be terminated after the graceful wait expires");

    assert_eq!(result.ready, EngineReadyState::Ready);
    assert!(result.forced_termination);
    assert_eq!(
        adapter.calls,
        ["start", "ready", "stop", "wait", "terminate"]
    );
}

#[test]
fn supervisor_terminates_after_wait_for_exit_error_and_preserves_that_error() {
    let mut adapter = FakeAdapter {
        wait_result: Err("exit wait failed".to_owned()),
        ..FakeAdapter::default()
    };

    let error = supervise_engine(
        &mut adapter,
        &launch(),
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .expect_err("exit wait failure should be returned");

    assert_eq!(error, "exit wait failed");
    assert_eq!(
        adapter.calls,
        ["start", "ready", "stop", "wait", "terminate"]
    );
}

#[test]
fn supervisor_distinguishes_readiness_timeout_from_early_exit() {
    for (ready, expected_error) in [
        (EngineReadyState::TimedOut, "engine readiness timed out"),
        (EngineReadyState::Exited, "engine exited before readiness"),
    ] {
        let mut adapter = FakeAdapter {
            ready_result: Ok(ready),
            ..FakeAdapter::default()
        };

        let error = supervise_engine(
            &mut adapter,
            &launch(),
            Duration::from_millis(100),
            Duration::from_millis(100),
        )
        .expect_err("non-ready state should fail supervision");

        assert_eq!(error, expected_error);
        assert_eq!(adapter.calls, ["start", "ready", "terminate"]);
    }
}

#[test]
fn supervisor_preserves_readiness_error_when_cleanup_fails() {
    let mut adapter = FakeAdapter {
        ready_result: Err("readiness failed".to_owned()),
        terminate_result: Err("termination failed".to_owned()),
        ..FakeAdapter::default()
    };

    let error = supervise_engine(
        &mut adapter,
        &launch(),
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .expect_err("readiness failure must remain an error");

    assert_eq!(error, "readiness failed");
    assert_eq!(adapter.calls, ["start", "ready", "terminate"]);
}

#[test]
fn supervisor_returns_termination_failure_instead_of_reporting_success() {
    let mut adapter = FakeAdapter {
        wait_result: Ok(false),
        terminate_result: Err("termination failed".to_owned()),
        ..FakeAdapter::default()
    };

    let error = supervise_engine(
        &mut adapter,
        &launch(),
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .expect_err("failed forced termination must not be reported as success");

    assert_eq!(error, "termination failed");
    assert_eq!(
        adapter.calls,
        ["start", "ready", "stop", "wait", "terminate"]
    );
}
