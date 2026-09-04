#![forbid(unsafe_code)]

//! Safe Rust IPC wire-test client used to replace the five migrated C++ wire
//! integration tests (078 Stage 3). Every test here drives the real
//! `fcitx5-mock-engine.exe` fixture over the real named-pipe protocol and
//! asserts the exact contracts the deleted C++ `ipc_*_test.cpp` files covered.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use fcitx5_protocol_core as protocol;
use fcitx5_windows_common_core::{
    current_runtime_generation_for_current_process, deadline_after, deadline_has_time_remaining,
    CurrentUserRuntimeIdentity, NamedEvent, VerifiedPipeClient,
};

/// Cold-context start deadline (mirrors `kContextStartDeadlineMilliseconds`).
pub const COLD_START_DEADLINE_MS: u32 = 7500;
/// Warm input deadline (mirrors `kInputDeadlineMilliseconds`).
pub const INPUT_DEADLINE_MS: u32 = 250;
/// Readiness timeout when spawning a fixture engine.
pub const READY_TIMEOUT: Duration = Duration::from_millis(2000);
/// Default transact timeout used by roundtrip/status probes.
pub const IO_TIMEOUT: Duration = Duration::from_millis(2000);

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> u64 {
    NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

/// Result of a key request that the wire tests assert on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyOutcome {
    pub handled: bool,
    pub commit: Vec<u8>,
    pub preedit: Vec<u8>,
    pub preedit_caret_utf8: u32,
    pub selected_candidate: u32,
    pub candidate_page: u32,
    pub candidate_total: u32,
    pub candidate_visibility: u8,
}

/// Result of an engine-status request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusOutcome {
    pub input_method_id: Vec<u8>,
    pub input_method_name: Vec<u8>,
    pub input_method_native_name: Vec<u8>,
    pub short_label: Vec<u8>,
}

/// A safe, stateful engine client used by the wire tests. It owns one verified
/// pipe, performs the hello handshake once, and tracks per-context
/// composition/revision exactly like the deleted C++ `PipeClient`.
pub struct EngineClient {
    pipe_name: OsString,
    expected_server: PathBuf,
    session_id: u32,
    process_id: u32,
    pipe: Option<VerifiedPipeClient>,
    handshake_complete: bool,
    engine_epoch: u64,
    /// context_id -> composition_id
    compositions: HashMap<u64, u64>,
    /// context_id -> revision
    revisions: HashMap<u64, u64>,
}

impl EngineClient {
    /// Creates a client for `pipe_name`, verifying the server is exactly
    /// `expected_server` under the current user/session.
    pub fn new(pipe_name: OsString, expected_server: PathBuf) -> Option<Self> {
        let identity = CurrentUserRuntimeIdentity::current()?;
        if identity.session_id() == 0 {
            return None;
        }
        Some(Self {
            pipe_name,
            expected_server,
            session_id: identity.session_id(),
            process_id: identity.process_id(),
            pipe: None,
            handshake_complete: false,
            engine_epoch: 0,
            compositions: HashMap::new(),
            revisions: HashMap::new(),
        })
    }

    /// Resolves the current identity's generation-scoped engine endpoint.
    pub fn default_pipe(generation: &str) -> Option<OsString> {
        let identity = CurrentUserRuntimeIdentity::current()?;
        identity.local_endpoint_name(generation, "engine")
    }

    fn connect(&mut self, timeout: Duration) -> bool {
        if self.pipe.is_some() {
            return true;
        }
        self.pipe =
            VerifiedPipeClient::connect_exact(&self.pipe_name, &self.expected_server, timeout);
        self.pipe.is_some()
    }

    fn disconnect(&mut self) {
        self.pipe = None;
        self.handshake_complete = false;
        self.engine_epoch = 0;
    }

    fn transact(&mut self, request: Vec<u8>, timeout: Duration) -> Option<Vec<u8>> {
        let pipe = self.pipe.as_mut()?;
        let mut response = vec![0_u8; protocol::MAX_FRAME_SIZE];
        let length = pipe.transact(&request, &mut response, timeout)?;
        response.truncate(length);
        Some(response)
    }

    fn handshake(&mut self, timeout: Duration) -> bool {
        if self.handshake_complete {
            return true;
        }
        let request_id = next_request_id();
        let request = protocol::HelloRequest {
            metadata: protocol::Metadata {
                request_id,
                session_id: self.session_id,
                ..protocol::Metadata::default()
            },
            client_architecture_bits: (std::mem::size_of::<usize>() * 8) as u32,
            client_process_id: self.process_id,
        };
        let Some(request_bytes) = protocol::encode_hello_request(&request) else {
            self.disconnect();
            return false;
        };
        let Some(response_bytes) = self.transact(request_bytes, timeout) else {
            self.disconnect();
            return false;
        };
        let Some(frame) = protocol::decode_frame(&response_bytes) else {
            self.disconnect();
            return false;
        };
        let Some(response) = protocol::decode_hello_response(&frame) else {
            self.disconnect();
            return false;
        };
        if response.status != protocol::Status::Ok
            || response.metadata.response_to != request_id
            || response.metadata.session_id != self.session_id
        {
            self.disconnect();
            return false;
        }
        self.engine_epoch = response.metadata.engine_epoch;
        self.handshake_complete = true;
        true
    }

    /// One key request. `new_context` selects the cold-start vs hot deadline.
    pub fn process_key(
        &mut self,
        context_id: u64,
        virtual_key: u32,
        key_flags: u32,
        scan_code: u32,
    ) -> Option<KeyOutcome> {
        let new_context = !self.revisions.contains_key(&context_id);
        let deadline_ms = if new_context {
            COLD_START_DEADLINE_MS
        } else {
            INPUT_DEADLINE_MS
        };
        let timeout = Duration::from_millis(u64::from(deadline_ms));
        if !self.connect(timeout) {
            return None;
        }
        if !self.handshake(timeout) {
            return None;
        }
        let composition_id = *self.compositions.get(&context_id).unwrap_or(&0);
        let revision = *self.revisions.get(&context_id).unwrap_or(&0);
        let request_id = next_request_id();
        let request = protocol::KeyRequest {
            metadata: protocol::Metadata {
                request_id,
                engine_epoch: self.engine_epoch,
                session_id: self.session_id,
                context_id,
                composition_id,
                revision,
                ..protocol::Metadata::default()
            },
            virtual_key,
            key_flags,
            scan_code,
            ..protocol::KeyRequest::default()
        };
        let Some(request_bytes) = protocol::encode_key_request(&request) else {
            self.disconnect();
            return None;
        };
        let Some(response_bytes) = self.transact(request_bytes, timeout) else {
            self.disconnect();
            return None;
        };
        let Some(frame) = protocol::decode_frame(&response_bytes) else {
            self.disconnect();
            return None;
        };
        let Some(response) = protocol::decode_key_response(&frame) else {
            self.disconnect();
            return None;
        };
        if response.status != protocol::Status::Ok
            || response.metadata.response_to != request_id
            || response.metadata.engine_epoch != self.engine_epoch
            || response.metadata.session_id != self.session_id
            || response.metadata.context_id != context_id
            || response.metadata.revision <= revision
        {
            self.disconnect();
            return None;
        }
        self.compositions
            .insert(context_id, response.metadata.composition_id);
        self.revisions
            .insert(context_id, response.metadata.revision);
        Some(KeyOutcome {
            handled: response.handled,
            commit: response.commit_utf8,
            preedit: response.preedit_utf8,
            preedit_caret_utf8: response.preedit_caret_utf8,
            selected_candidate: response.selected_candidate,
            candidate_page: response.candidate_page,
            candidate_total: response.candidate_total,
            candidate_visibility: response.candidate_visibility,
        })
    }

    /// One engine-status request (mirrors `queryEngineStatus`).
    pub fn query_status(&mut self) -> Option<StatusOutcome> {
        let timeout = Duration::from_millis(u64::from(INPUT_DEADLINE_MS));
        if !self.connect(timeout) {
            return None;
        }
        if !self.handshake(timeout) {
            return None;
        }
        let request_id = next_request_id();
        let request = protocol::EngineStatusRequest {
            metadata: protocol::Metadata {
                request_id,
                engine_epoch: self.engine_epoch,
                session_id: self.session_id,
                ..protocol::Metadata::default()
            },
        };
        let Some(request_bytes) = protocol::encode_engine_status_request(&request) else {
            self.disconnect();
            return None;
        };
        let Some(response_bytes) = self.transact(request_bytes, timeout) else {
            self.disconnect();
            return None;
        };
        let Some(frame) = protocol::decode_frame(&response_bytes) else {
            self.disconnect();
            return None;
        };
        let Some(response) = protocol::decode_engine_status_response(&frame) else {
            self.disconnect();
            return None;
        };
        if response.status != protocol::Status::Ok || response.metadata.response_to != request_id {
            self.disconnect();
            return None;
        }
        Some(StatusOutcome {
            input_method_id: response.current_input_method_id,
            input_method_name: response.current_input_method_name,
            input_method_native_name: response.current_input_method_native_name,
            short_label: response.current_input_method_short_label,
        })
    }

    /// Closes the pipe so the engine observes the client disconnect.
    pub fn close(&mut self) {
        self.disconnect();
    }
}

/// A spawned mock-engine fixture handle.
pub struct MockEngine;

impl MockEngine {
    /// Spawns `fcitx5-mock-engine.exe` with the given server arguments.
    pub fn spawn(
        engine: &Path,
        ready_event_name: &OsStr,
        arguments: &[OsString],
    ) -> Result<Child, String> {
        use std::os::windows::process::CommandExt;
        Command::new(engine)
            .arg(OsString::from("--ready-event"))
            .arg(ready_event_name)
            .args(arguments.iter().cloned())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|error| format!("failed to start mock engine: {error}"))
    }

    /// Waits until the fixture signals ready.
    pub fn wait_ready(ready: &NamedEvent) -> bool {
        let deadline = deadline_after(u32::try_from(READY_TIMEOUT.as_millis()).unwrap_or(u32::MAX));
        while !ready.is_signaled() && deadline_has_time_remaining(deadline) {
            std::thread::sleep(Duration::from_millis(5));
        }
        ready.is_signaled()
    }

    /// Waits for the child to exit successfully within `timeout`.
    pub fn stop(child: &mut Child, timeout: Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => return status.success(),
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(None) | Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
            }
        }
    }
}

/// Result of one launcher control command (mirrors the deleted C++
/// `LauncherResponse` mirror field-for-field over the Rust codec).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherOutcome {
    pub status: protocol::Status,
    pub launcher_state: u32,
    pub engine_state: u32,
    pub start_disposition: u32,
    pub safe_mode: bool,
    pub retry_after_milliseconds: u64,
    pub current_input_method_id: Vec<u8>,
    pub current_input_method_name: Vec<u8>,
    pub current_input_method_native_name: Vec<u8>,
    pub current_input_method_short_label: Vec<u8>,
}

/// One-shot verified launcher control client (port of `sendLauncherCommand`).
/// Connects to the launcher pipe, sends one `LauncherCommand`, decodes the
/// `LauncherResponse` through the Rust codec, and validates request/session
/// identity. The launcher control plane needs no hello handshake (the launcher
/// server decodes a `LauncherRequest` directly).
pub fn send_launcher_command(
    launcher: &Path,
    command: protocol::LauncherCommand,
) -> Option<LauncherOutcome> {
    let identity = CurrentUserRuntimeIdentity::current()?;
    if identity.session_id() == 0 {
        return None;
    }
    let generation = current_runtime_generation_for_current_process();
    let pipe_name = identity.local_endpoint_name(&generation, "launcher")?;
    let mut client = VerifiedPipeClient::connect_exact(&pipe_name, launcher, IO_TIMEOUT)?;
    let request_id = next_request_id();
    let request = protocol::LauncherRequest {
        metadata: protocol::Metadata {
            request_id,
            session_id: identity.session_id(),
            ..protocol::Metadata::default()
        },
        command,
    };
    let request_bytes = protocol::encode_launcher_request(&request)?;
    let mut response = vec![0_u8; protocol::MAX_FRAME_SIZE];
    let transferred = client.transact(&request_bytes, &mut response, IO_TIMEOUT)?;
    response.truncate(transferred);
    let frame = protocol::decode_frame(&response)?;
    let decoded = protocol::decode_launcher_response(&frame)?;
    if decoded.metadata.response_to != request_id
        || decoded.metadata.session_id != identity.session_id()
    {
        return None;
    }
    Some(LauncherOutcome {
        status: decoded.status,
        launcher_state: decoded.launcher_state,
        engine_state: decoded.engine_state,
        start_disposition: decoded.start_disposition,
        safe_mode: decoded.safe_mode,
        retry_after_milliseconds: decoded.retry_after_milliseconds,
        current_input_method_id: decoded.current_input_method_id,
        current_input_method_name: decoded.current_input_method_name,
        current_input_method_native_name: decoded.current_input_method_native_name,
        current_input_method_short_label: decoded.current_input_method_short_label,
    })
}

/// Resolves the path to `fcitx5-launcher.exe` passed by the test harness.
pub fn launcher_exe_path() -> PathBuf {
    let path = std::env::var_os("FCITX_LAUNCHER_EXE")
        .map(PathBuf::from)
        .expect(
            "FCITX_LAUNCHER_EXE must point at the single-link CMake copy of fcitx5-launcher.exe",
        );
    assert!(
        path.is_file(),
        "launcher fixture not found at {}",
        path.display()
    );
    path
}

/// Resolves the path to `fcitx5-crash-engine-fixture.exe` passed by CTest.
pub fn crash_engine_exe_path() -> PathBuf {
    let path = std::env::var_os("FCITX_CRASH_ENGINE_EXE")
        .map(PathBuf::from)
        .expect("FCITX_CRASH_ENGINE_EXE must point at fcitx5-crash-engine-fixture.exe");
    assert!(
        path.is_file(),
        "crash engine fixture not found at {}",
        path.display()
    );
    path
}

/// Resolves the path to `fcitx5-mock-engine.exe` passed by the test harness.
///
/// CTest always supplies `FCITX_MOCK_ENGINE_EXE` pointing at the single-link
/// fixture copy that CMake produced with `copy_if_different`; peer verification
/// (`number_of_links == 1` in `executable_files_match`) rejects multi-link
/// files, and Cargo hard-links every `target/.../debug/*.exe` bin to its
/// `deps/` twin. The fallback below therefore only works when a previously
/// copied single-link fixture happens to sit next to the test binary; a bare
/// `cargo build -p fcitx5-mock-engine-core` output is rejected by verification.
pub fn mock_engine_path() -> PathBuf {
    let path = std::env::var_os("FCITX_MOCK_ENGINE_EXE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            // Fall back to a sibling of the test binary so an interactive
            // `cargo test` can reuse a previously copied fixture.
            std::env::current_exe()
                .expect("current exe")
                .parent()
                .expect("deps dir")
                .parent()
                .expect("target dir")
                .join("fcitx5-mock-engine.exe")
        });
    assert!(
        path.is_file(),
        "mock engine fixture not found at {} (set FCITX_MOCK_ENGINE_EXE to a single-link copy of fcitx5-mock-engine.exe, e.g. the CMake-built one under out/build/.../Debug)",
        path.display()
    );
    path
}
