//! Rust API for in-process engine/launcher clients such as the TSF DLL.
//!
//! These are the same helpers the C exports wrap, without the raw-pointer
//! calling convention.

#![forbid(unsafe_code)]

use std::ffi::c_void;
use std::path::{Path, PathBuf};

use super::{
    close_pipe_client, current_process_is_app_container, current_runtime_generation,
    current_runtime_generation_for_module, install_root_for_module, invalid_handle_value,
    local_name, local_test_namespace, open_pipe_client, pipe_client_is_connected, pipe_transact,
    pipe_transact_with_error, process_image_path, process_session_id, process_user_sid,
    secure_input_desktop, verify_pipe_server_peer, Fcitx5WindowsCommonPipeTransactWithError,
};

pub use super::{
    apply_hello_response_scalars, apply_key_response_scalars, apply_launcher_response_scalars,
    deadline_after_milliseconds, deadline_has_time, ipc_status_ok, next_pipe_client_request_id,
    tick_milliseconds,
};

/// Identity of the current process as seen by the engine's peer checks.
#[derive(Clone, Debug, Default)]
pub struct ClientIdentity {
    pub service_account: bool,
    pub secure_desktop: bool,
    pub session_id: u32,
    pub user_sid: Vec<u16>,
    /// The process runs in an AppContainer (a packaged or sandboxed app). It
    /// can use a running engine but must not start one, because a child
    /// process would inherit the sandbox.
    pub app_container: bool,
}

/// Queries the current process identity. Fails when the session, user SID or
/// image path cannot be read, or the SID is empty.
#[must_use]
pub fn current_client_identity() -> Option<ClientIdentity> {
    let process_id = std::process::id();
    let session = process_session_id(process_id);
    if session.status == 0 {
        return None;
    }
    let (user_sid, service_account) = process_user_sid(process_id)?;
    process_image_path(process_id)?;
    if user_sid.is_empty() {
        return None;
    }
    Some(ClientIdentity {
        service_account,
        secure_desktop: secure_input_desktop(),
        session_id: session.session_id,
        user_sid,
        app_container: current_process_is_app_container(),
    })
}

/// The runtime generation of the current process image.
#[must_use]
pub fn process_runtime_generation() -> Option<String> {
    Some(current_runtime_generation()).filter(|generation| !generation.is_empty())
}

/// The runtime generation recorded for `module_path` only.
#[must_use]
pub fn module_runtime_generation(module_path: &Path) -> Option<String> {
    current_runtime_generation_for_module(module_path).filter(|generation| !generation.is_empty())
}

/// The installation root that contains `module_path`.
#[must_use]
pub fn installation_root_for_module(module_path: &Path) -> Option<PathBuf> {
    install_root_for_module(module_path)
}

/// The test namespace from the environment, or an empty string.
#[must_use]
pub fn test_namespace() -> String {
    local_test_namespace().unwrap_or_default()
}

/// A local pipe (`pipe == true`) or object name for this identity.
#[must_use]
pub fn local_endpoint_name(
    pipe: bool,
    identity: &ClientIdentity,
    generation: &str,
    channel: &str,
    test_namespace: &str,
) -> Option<String> {
    let user_sid = String::from_utf16(&identity.user_sid).ok()?;
    local_name(
        pipe,
        &user_sid,
        identity.session_id,
        generation,
        channel,
        test_namespace,
    )
    .filter(|name| !name.is_empty())
}

/// An owned overlapped named-pipe client handle.
pub struct PipeClient(*mut c_void);

impl PipeClient {
    /// Opens `name`, waiting for a busy server until `deadline` when asked.
    #[must_use]
    pub fn open(name: &[u16], deadline: u64, wait_when_busy: bool) -> Option<Self> {
        let pipe = open_pipe_client(name, deadline, wait_when_busy);
        (!pipe.is_null() && pipe != invalid_handle_value()).then_some(Self(pipe))
    }

    /// Like [`Self::open`], but reports the Win32 error when the pipe cannot
    /// be opened, so a caller can tell an access policy that denies the pipe
    /// from a server that is not running yet. A name or deadline that never
    /// reaches the open reports 0.
    pub fn open_with_error(name: &[u16], deadline: u64, wait_when_busy: bool) -> Result<Self, i32> {
        if name.is_empty() || !deadline_has_time(deadline) {
            return Err(0);
        }
        Self::open(name, deadline, wait_when_busy)
            .ok_or_else(|| std::io::Error::last_os_error().raw_os_error().unwrap_or(0))
    }

    /// Whether the server end is still connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        pipe_client_is_connected(self.0)
    }

    /// Verifies the server process against this identity and policy.
    #[must_use]
    pub fn verify_server(
        &self,
        identity: &ClientIdentity,
        policy_mode: u32,
        expected_executable_path: &[u16],
        development_exception_enabled: bool,
    ) -> bool {
        let Ok(user_sid) = String::from_utf16(&identity.user_sid) else {
            return false;
        };
        verify_pipe_server_peer(
            self.0,
            identity.service_account,
            identity.session_id,
            identity.secure_desktop,
            &user_sid,
            policy_mode,
            expected_executable_path,
            development_exception_enabled,
        )
    }

    /// Writes `request` and reads one response frame into `response`.
    /// Returns the response length.
    #[must_use]
    pub fn transact(&self, request: &[u8], response: &mut [u8], deadline: u64) -> Option<usize> {
        let result = pipe_transact(
            self.0,
            request,
            response.as_mut_ptr(),
            response.len(),
            deadline,
        );
        (result.status != 0 && result.response_len <= response.len()).then_some(result.response_len)
    }

    /// Like [`Self::transact`], but reports the Win32 failure code.
    #[must_use]
    pub fn transact_with_error(
        &self,
        request: &[u8],
        response: &mut [u8],
        deadline: u64,
    ) -> Fcitx5WindowsCommonPipeTransactWithError {
        pipe_transact_with_error(
            self.0,
            request,
            response.as_mut_ptr(),
            response.len(),
            deadline,
        )
    }
}

impl Drop for PipeClient {
    fn drop(&mut self) {
        close_pipe_client(self.0);
    }
}
