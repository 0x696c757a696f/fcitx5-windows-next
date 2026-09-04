#![forbid(unsafe_code)]
#![allow(dead_code)]

//! Shared helpers for the IPC wire tests (078 Stage 3). Each test file spawns
//! the real `fcitx5-mock-engine.exe` fixture and asserts the exact contract
//! the deleted C++ `ipc_*_test.cpp` covered.

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use fcitx5_ipc_client::{mock_engine_path, MockEngine, StatusOutcome};
use fcitx5_windows_common_core::{
    current_runtime_generation_for_current_process, deadline_after, deadline_has_time_remaining,
    CurrentUserRuntimeIdentity, NamedEvent,
};

/// Waits up to `milliseconds` for a named event to become signaled.
pub fn wait_event(ready: &NamedEvent, milliseconds: u64) -> bool {
    let deadline = deadline_after(u32::try_from(milliseconds).unwrap_or(u32::MAX));
    while !ready.is_signaled() && deadline_has_time_remaining(deadline) {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    ready.is_signaled()
}

/// Spawns a shipping process with `CREATE_NO_WINDOW`, inheriting the test
/// environment (so `FCITX5_TEST_NAMESPACE` / `FCITX_TEST_SAFE_MODE_EVENT` and
/// generation env vars reach the launcher and its engine child).
pub fn spawn_nowindow(executable: &std::path::Path, arguments: &[OsString]) -> std::process::Child {
    use std::os::windows::process::CommandExt;
    std::process::Command::new(executable)
        .args(arguments.iter().cloned())
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .spawn()
        .expect("spawn child")
}

/// Resolves the current deployment generation for pipe names, mirroring the
/// C++ `makeLocalEndpointName` default overload.
pub fn current_generation() -> String {
    current_runtime_generation_for_current_process()
}

/// Builds the generation-scoped engine/launcher endpoint name for `channel`.
pub fn endpoint_name(channel: &str) -> OsString {
    let identity = CurrentUserRuntimeIdentity::current().expect("current identity");
    identity
        .local_endpoint_name(&current_generation(), channel)
        .expect("endpoint")
}

/// Resolves a unique pipe/ready suffix for a test process.
pub fn process_suffix() -> String {
    std::process::id().to_string()
}

/// Creates a local named event for a test-generated ready/stop signal.
pub fn create_event(name: &str) -> NamedEvent {
    let identity = CurrentUserRuntimeIdentity::current().expect("current identity");
    let security = identity.security_attributes().expect("security attributes");
    NamedEvent::create(std::ffi::OsStr::new(name), &security).expect("create event")
}

/// Pipe name used by the mock engine when invoked with `--pipe`.
pub fn pipe_name(tag: &str, suffix: &str) -> OsString {
    OsString::from(format!(r"\\.\pipe\Fcitx5WindowsNext.{tag}.{suffix}"))
}

/// Ready-event name paired with a spawned mock engine.
pub fn ready_name(tag: &str, suffix: &str) -> OsString {
    OsString::from(format!("Local\\Fcitx5WindowsNext.{tag}.Ready.{suffix}"))
}

/// Spawns the mock engine serving exactly `test_clients` connections on
/// `pipe_name`, returns the child plus its readiness event.
pub fn spawn_engine(tag: &str, test_clients: u32) -> (std::process::Child, NamedEvent) {
    let engine = mock_engine_path();
    let suffix = process_suffix();
    let pipe = pipe_name(tag, &suffix);
    let ready_os = ready_name(tag, &suffix);
    let ready = create_event(&ready_os.to_string_lossy());
    let child = MockEngine::spawn(
        &engine,
        &ready_os,
        &[
            OsString::from("--pipe"),
            pipe,
            OsString::from("--test-clients"),
            OsString::from(test_clients.to_string()),
        ],
    )
    .expect("spawn mock engine");
    (child, ready)
}

/// Builds a client for `pipe` whose peer must be `expected`.
#[allow(dead_code)]
pub fn client_for(pipe: OsString, expected: PathBuf) -> fcitx5_ipc_client::EngineClient {
    fcitx5_ipc_client::EngineClient::new(pipe, expected).expect("engine client")
}

/// Awaits the fixture ready event.
pub fn wait_ready(ready: &NamedEvent) -> bool {
    MockEngine::wait_ready(ready)
}

/// Waits for the child to exit with code 0.
pub fn engine_exit_ok(child: &mut std::process::Child) -> bool {
    MockEngine::stop(child, Duration::from_secs(3))
}

/// Asserts the mock-pinyin status contract.
pub fn assert_mock_pinyin_status(status: &StatusOutcome) {
    assert_eq!(status.input_method_id, b"mock-pinyin");
    assert_eq!(status.input_method_name, b"Mock Pinyin");
    assert_eq!(status.input_method_native_name, "小企鹅".as_bytes());
    assert_eq!(status.short_label, "小".as_bytes());
}
