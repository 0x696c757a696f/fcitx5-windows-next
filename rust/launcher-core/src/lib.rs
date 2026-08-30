#![deny(unsafe_op_in_unsafe_fn)]
#![forbid(unsafe_code)]

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsHandle;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command as ProcessCommand, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use fcitx5_process_execution_core::JobObject;
use fcitx5_protocol_core::{
    self as protocol, decode_engine_status_response, decode_frame, decode_header,
    decode_hello_response, decode_launcher_request, encode_engine_status_request,
    encode_hello_request, encode_launcher_response, EngineStatusRequest, EngineStatusResponse,
    HelloRequest, LauncherCommand, LauncherResponse, Metadata, Status,
};
use fcitx5_windows_common_core::{
    current_runtime_generation_for_current_process, deadline_after,
    default_fcitx5_data_root_for_current_process, monotonic_milliseconds, named_pipe_is_available,
    next_launcher_request_id, wait_for_handle, CurrentUserRuntimeIdentity,
    CurrentUserSecurityAttributes, NamedEvent, NamedPipeServer, SingleInstance, VerifiedPipeClient,
};

const STARTUP_CRASH_WINDOW_MS: u64 = 10_000;
const STABLE_RUNTIME_MS: u64 = 60_000;
const INITIAL_BACKOFF_MS: u64 = 250;
const MAXIMUM_BACKOFF_MS: u64 = 30_000;
const SAFE_MODE_CRASH_THRESHOLD: u32 = 3;
const STATE_FILE_NAME: &str = "launcher-state.v1";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const READY_TIMEOUT: Duration = Duration::from_millis(2_000);
const STOP_TIMEOUT: Duration = Duration::from_millis(2_000);
const KILL_TIMEOUT: Duration = Duration::from_millis(1_000);
const PIPE_CONNECT_TIMEOUT_MS: u32 = 250;
const PIPE_TRANSFER_TIMEOUT_MS: u32 = 100;
const STATUS_TIMEOUT: Duration = Duration::from_millis(500);
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[must_use]
pub fn launcher_tick_milliseconds() -> u64 {
    monotonic_milliseconds()
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LauncherState {
    Normal = 0,
    UserStopped = 1,
    Updating = 2,
    Uninstalling = 3,
    CrashBackoff = 4,
    SafeMode = 5,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineState {
    Stopped = 0,
    Starting = 1,
    Ready = 2,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Command {
    UserStop = 0,
    Resume = 1,
    BeginUpdate = 2,
    EndUpdate = 3,
    BeginUninstall = 4,
    ResetSafeMode = 5,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartDisposition {
    Start = 0,
    AlreadyActive = 1,
    Suppressed = 2,
    Backoff = 3,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fcitx5LauncherSnapshot {
    pub state: u32,
    pub consecutive_startup_crashes: u32,
    pub next_start_allowed_milliseconds: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fcitx5LauncherMachine {
    pub snapshot: Fcitx5LauncherSnapshot,
    pub engine_state: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fcitx5LauncherStartDecision {
    pub disposition: u32,
    pub safe_mode: bool,
    pub retry_after_milliseconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LauncherOptions {
    pub engine_path: Option<PathBuf>,
    pub ui_path: Option<PathBuf>,
    pub warmup: bool,
    pub installed_defaults: bool,
    pub engine_ready_event: Option<OsString>,
    pub ready_event: Option<OsString>,
    pub stop_event: Option<OsString>,
    pub state_file: Option<PathBuf>,
    pub generation: Option<OsString>,
}

impl Default for LauncherOptions {
    fn default() -> Self {
        Self {
            engine_path: None,
            ui_path: None,
            warmup: true,
            installed_defaults: false,
            engine_ready_event: None,
            ready_event: None,
            stop_event: None,
            state_file: None,
            generation: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LauncherInvocation {
    Version,
    Supervise(Box<LauncherOptions>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LauncherError {
    InvalidArguments,
    MissingOptionValue(&'static str),
    MissingEnginePath,
    InvalidPath(&'static str),
    MissingPath(PathBuf),
    StateMissing(PathBuf),
    StateInvalid(PathBuf),
    StateStore(PathBuf),
    Runtime(String),
}

impl fmt::Display for LauncherError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArguments => formatter.write_str("invalid launcher arguments"),
            Self::MissingOptionValue(option) => write!(formatter, "missing value for {option}"),
            Self::MissingEnginePath => formatter.write_str("missing launcher engine path"),
            Self::InvalidPath(option) => write!(formatter, "invalid path for {option}"),
            Self::MissingPath(path) => write!(
                formatter,
                "launcher path does not exist: {}",
                path.display()
            ),
            Self::StateMissing(path) => write!(
                formatter,
                "launcher state file is missing: {}",
                path.display()
            ),
            Self::StateInvalid(path) => write!(
                formatter,
                "launcher state file is invalid: {}",
                path.display()
            ),
            Self::StateStore(path) => {
                write!(formatter, "launcher state store failed: {}", path.display())
            }
            Self::Runtime(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for LauncherError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LauncherStatus {
    pub launcher_state: u32,
    pub engine_state: u32,
    pub start_disposition: u32,
    pub retry_after_milliseconds: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LauncherStartup {
    pub engine_path: PathBuf,
    pub ui_path: Option<PathBuf>,
    pub state_path: PathBuf,
    pub snapshot: Fcitx5LauncherSnapshot,
    pub status: LauncherStatus,
    pub engine_command_line: Option<Vec<u16>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineLaunch {
    pub engine_path: PathBuf,
    pub ready_event: OsString,
    pub stop_event: OsString,
    pub generation: OsString,
    pub safe_mode: bool,
}

impl EngineLaunch {
    #[must_use]
    pub fn new(
        engine_path: PathBuf,
        ready_event: OsString,
        stop_event: OsString,
        generation: OsString,
        safe_mode: bool,
    ) -> Self {
        Self {
            engine_path,
            ready_event,
            stop_event,
            generation,
            safe_mode,
        }
    }
}

pub type EngineLifecycleResult<T> = Result<T, String>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineReadyState {
    Ready,
    TimedOut,
    Exited,
}

pub trait EngineLifecycleAdapter {
    type Child;

    fn start_engine(&mut self, launch: &EngineLaunch) -> EngineLifecycleResult<Self::Child>;
    fn wait_for_ready(
        &mut self,
        child: &mut Self::Child,
        ready_event: &OsStr,
        timeout: Duration,
    ) -> EngineLifecycleResult<EngineReadyState>;
    fn signal_stop(&mut self, stop_event: &OsStr) -> EngineLifecycleResult<()>;
    fn wait_for_exit(
        &mut self,
        child: &mut Self::Child,
        timeout: Duration,
    ) -> EngineLifecycleResult<bool>;
    fn terminate(&mut self, child: &mut Self::Child) -> EngineLifecycleResult<()>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineSupervisionResult {
    pub ready: EngineReadyState,
    pub forced_termination: bool,
}

pub fn supervise_engine<A: EngineLifecycleAdapter>(
    adapter: &mut A,
    launch: &EngineLaunch,
    ready_timeout: Duration,
    exit_timeout: Duration,
) -> EngineLifecycleResult<EngineSupervisionResult> {
    let mut child = adapter.start_engine(launch)?;
    let ready = match adapter.wait_for_ready(&mut child, &launch.ready_event, ready_timeout) {
        Ok(ready) => ready,
        Err(error) => {
            let _ = adapter.terminate(&mut child);
            return Err(error);
        }
    };
    if ready != EngineReadyState::Ready {
        let _ = adapter.terminate(&mut child);
        return Err(match ready {
            EngineReadyState::TimedOut => "engine readiness timed out",
            EngineReadyState::Exited => "engine exited before readiness",
            EngineReadyState::Ready => "engine did not become ready",
        }
        .to_owned());
    }
    if let Err(error) = adapter.signal_stop(&launch.stop_event) {
        let _ = adapter.terminate(&mut child);
        return Err(error);
    }
    match adapter.wait_for_exit(&mut child, exit_timeout) {
        Ok(true) => Ok(EngineSupervisionResult {
            ready,
            forced_termination: false,
        }),
        Ok(false) => {
            adapter.terminate(&mut child)?;
            Ok(EngineSupervisionResult {
                ready,
                forced_termination: true,
            })
        }
        Err(error) => {
            let _ = adapter.terminate(&mut child);
            Err(error)
        }
    }
}

fn launcher_state(value: u32) -> Option<LauncherState> {
    match value {
        0 => Some(LauncherState::Normal),
        1 => Some(LauncherState::UserStopped),
        2 => Some(LauncherState::Updating),
        3 => Some(LauncherState::Uninstalling),
        4 => Some(LauncherState::CrashBackoff),
        5 => Some(LauncherState::SafeMode),
        _ => None,
    }
}

fn state_name(state: LauncherState) -> &'static str {
    match state {
        LauncherState::Normal => "normal",
        LauncherState::UserStopped => "user-stopped",
        LauncherState::Updating => "updating",
        LauncherState::Uninstalling => "uninstalling",
        LauncherState::CrashBackoff => "crash-backoff",
        LauncherState::SafeMode => "safe-mode",
    }
}

fn parse_state_name(value: &str) -> Option<LauncherState> {
    match value {
        "normal" => Some(LauncherState::Normal),
        "user-stopped" => Some(LauncherState::UserStopped),
        "updating" => Some(LauncherState::Updating),
        "uninstalling" => Some(LauncherState::Uninstalling),
        "crash-backoff" => Some(LauncherState::CrashBackoff),
        "safe-mode" => Some(LauncherState::SafeMode),
        _ => None,
    }
}

fn line_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let marker = format!("{key}=");
    text.lines().find_map(|line| {
        line.strip_suffix('\r')
            .unwrap_or(line)
            .strip_prefix(&marker)
    })
}

fn parse_unsigned(value: &str) -> Option<u64> {
    (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())).then_some(())?;
    value.bytes().try_fold(0_u64, |result, byte| {
        result.checked_mul(10)?.checked_add(u64::from(byte - b'0'))
    })
}

fn parse_snapshot(text: &str) -> Option<Fcitx5LauncherSnapshot> {
    if let Some(state) = text.strip_suffix('\n').and_then(parse_state_name) {
        return Some(Fcitx5LauncherSnapshot {
            state: state as u32,
            consecutive_startup_crashes: 0,
            next_start_allowed_milliseconds: 0,
        });
    }
    let crashes = parse_unsigned(line_value(text, "consecutive_startup_crashes")?)?;
    (line_value(text, "format_version")? == "2" && crashes <= u64::from(u32::MAX)).then_some(
        Fcitx5LauncherSnapshot {
            state: parse_state_name(line_value(text, "state")?)? as u32,
            consecutive_startup_crashes: crashes as u32,
            next_start_allowed_milliseconds: parse_unsigned(line_value(
                text,
                "next_start_allowed_ms",
            )?)?,
        },
    )
}

fn serialize_snapshot(snapshot: Fcitx5LauncherSnapshot) -> Option<String> {
    Some(format!(
        "format_version=2\nstate={}\nconsecutive_startup_crashes={}\nnext_start_allowed_ms={}\n",
        state_name(launcher_state(snapshot.state)?),
        snapshot.consecutive_startup_crashes,
        snapshot.next_start_allowed_milliseconds
    ))
}

fn is_absolute_windows_path(path: &Path) -> bool {
    let path = path.as_os_str().encode_wide().collect::<Vec<_>>();
    path.len() >= 3
        && ((u16::from(b'A')..=u16::from(b'Z')).contains(&path[0])
            || (u16::from(b'a')..=u16::from(b'z')).contains(&path[0]))
        && path[1] == u16::from(b':')
        && matches!(path[2], value if value == u16::from(b'\\') || value == u16::from(b'/'))
}

fn resolve_default_process_paths(directory: &Path, generation: &OsStr) -> (PathBuf, PathBuf) {
    let root = if directory.file_name() == Some(OsStr::new("bin")) {
        directory.parent().unwrap_or(directory)
    } else {
        directory
    };
    let runtime = root.join("runtime").join(generation).join("bin");
    let engine = runtime.join("fcitx5-engine.exe");
    let ui = runtime.join("fcitx5-ui.exe");
    if engine.exists() && ui.exists() {
        (engine, ui)
    } else {
        (
            directory.join("fcitx5-engine.exe"),
            directory.join("fcitx5-ui.exe"),
        )
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(format!(
        ".tmp.{}.{}",
        std::process::id(),
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    PathBuf::from(value)
}

fn load_snapshot_from_path(path: &Path) -> Result<Fcitx5LauncherSnapshot, u32> {
    if path.as_os_str().is_empty() {
        return Err(3);
    }
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Err(0),
        Err(_) => return Err(3),
    };
    let mut bytes = Vec::with_capacity(256);
    if file.take(256).read_to_end(&mut bytes).is_err() {
        return Err(3);
    }
    std::str::from_utf8(&bytes)
        .ok()
        .and_then(parse_snapshot)
        .ok_or(2)
}

fn save_snapshot_to_path(path: &Path, snapshot: Fcitx5LauncherSnapshot) -> bool {
    let Some(text) = serialize_snapshot(snapshot) else {
        return false;
    };
    let temporary = temporary_path(path);
    let published = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })()
    .is_ok();
    if !published {
        let _ = fs::remove_file(temporary);
    }
    published
}

fn default_state_store_path() -> Option<PathBuf> {
    let directory = default_fcitx5_data_root_for_current_process()?;
    fs::create_dir_all(&directory).ok()?;
    Some(directory.join(STATE_FILE_NAME))
}

fn append_quoted(output: &mut Vec<u16>, value: &OsStr) {
    output.push(u16::from(b'"'));
    output.extend(value.encode_wide());
    output.push(u16::from(b'"'));
}

fn engine_command(
    engine: &OsStr,
    ready: &OsStr,
    stop: &OsStr,
    generation: &OsStr,
    safe_mode: bool,
) -> Vec<u16> {
    let mut command = Vec::new();
    append_quoted(&mut command, engine);
    command.extend(" --ready-event ".encode_utf16());
    append_quoted(&mut command, ready);
    command.extend(" --stop-event ".encode_utf16());
    append_quoted(&mut command, stop);
    command.extend(" --generation ".encode_utf16());
    append_quoted(&mut command, generation);
    if safe_mode {
        command.extend(" --safe-mode".encode_utf16());
    }
    command
}

fn reset_crash_accounting(snapshot: &mut Fcitx5LauncherSnapshot) {
    snapshot.consecutive_startup_crashes = 0;
    snapshot.next_start_allowed_milliseconds = 0;
}

fn normalize_snapshot(
    mut snapshot: Fcitx5LauncherSnapshot,
    now: u64,
) -> Option<Fcitx5LauncherSnapshot> {
    match launcher_state(snapshot.state)? {
        LauncherState::CrashBackoff => {
            snapshot.consecutive_startup_crashes = snapshot.consecutive_startup_crashes.max(1);
            if snapshot.next_start_allowed_milliseconds == 0
                || snapshot.next_start_allowed_milliseconds > now.saturating_add(MAXIMUM_BACKOFF_MS)
            {
                snapshot.next_start_allowed_milliseconds = now.saturating_add(INITIAL_BACKOFF_MS);
            }
        }
        LauncherState::SafeMode => {
            snapshot.consecutive_startup_crashes = snapshot
                .consecutive_startup_crashes
                .max(SAFE_MODE_CRASH_THRESHOLD);
            snapshot.next_start_allowed_milliseconds = 0;
        }
        _ => reset_crash_accounting(&mut snapshot),
    }
    Some(snapshot)
}

fn decision(
    disposition: StartDisposition,
    safe_mode: bool,
    retry_after_milliseconds: u64,
) -> Fcitx5LauncherStartDecision {
    Fcitx5LauncherStartDecision {
        disposition: disposition as u32,
        safe_mode,
        retry_after_milliseconds,
    }
}

fn request_start(machine: &mut Fcitx5LauncherMachine, now: u64) -> Fcitx5LauncherStartDecision {
    let state = launcher_state(machine.snapshot.state).unwrap_or(LauncherState::UserStopped);
    if matches!(
        state,
        LauncherState::UserStopped | LauncherState::Updating | LauncherState::Uninstalling
    ) {
        return decision(StartDisposition::Suppressed, false, 0);
    }
    if machine.engine_state != EngineState::Stopped as u32 {
        return decision(
            StartDisposition::AlreadyActive,
            state == LauncherState::SafeMode,
            0,
        );
    }
    if state == LauncherState::CrashBackoff
        && now < machine.snapshot.next_start_allowed_milliseconds
    {
        return decision(
            StartDisposition::Backoff,
            false,
            machine.snapshot.next_start_allowed_milliseconds - now,
        );
    }
    if state == LauncherState::CrashBackoff {
        machine.snapshot.state = LauncherState::Normal as u32;
    }
    machine.engine_state = EngineState::Starting as u32;
    decision(
        StartDisposition::Start,
        launcher_state(machine.snapshot.state) == Some(LauncherState::SafeMode),
        0,
    )
}

fn can_apply(state: LauncherState, command: Command) -> bool {
    match command {
        Command::UserStop | Command::BeginUpdate => {
            state != LauncherState::Updating && state != LauncherState::Uninstalling
        }
        Command::Resume => state == LauncherState::UserStopped,
        Command::EndUpdate => state == LauncherState::Updating,
        Command::BeginUninstall => state != LauncherState::Uninstalling,
        Command::ResetSafeMode => state == LauncherState::SafeMode,
    }
}

fn apply_command(machine: &mut Fcitx5LauncherMachine, command: Command) -> bool {
    let Some(state) = launcher_state(machine.snapshot.state) else {
        return false;
    };
    if !can_apply(state, command) {
        return false;
    }
    machine.snapshot.state = match command {
        Command::UserStop => LauncherState::UserStopped,
        Command::BeginUpdate => LauncherState::Updating,
        Command::BeginUninstall => LauncherState::Uninstalling,
        Command::Resume | Command::EndUpdate | Command::ResetSafeMode => LauncherState::Normal,
    } as u32;
    if matches!(
        command,
        Command::UserStop | Command::BeginUpdate | Command::BeginUninstall
    ) {
        machine.engine_state = EngineState::Stopped as u32;
    } else {
        reset_crash_accounting(&mut machine.snapshot);
    }
    true
}

fn engine_exited(machine: &mut Fcitx5LauncherMachine, runtime: u64, now: u64) {
    machine.engine_state = EngineState::Stopped as u32;
    let state = launcher_state(machine.snapshot.state).unwrap_or(LauncherState::UserStopped);
    if matches!(
        state,
        LauncherState::UserStopped | LauncherState::Updating | LauncherState::Uninstalling
    ) {
        return;
    }
    if runtime >= STARTUP_CRASH_WINDOW_MS || runtime >= STABLE_RUNTIME_MS {
        machine.snapshot.state = LauncherState::Normal as u32;
        reset_crash_accounting(&mut machine.snapshot);
        return;
    }
    machine.snapshot.consecutive_startup_crashes = machine
        .snapshot
        .consecutive_startup_crashes
        .saturating_add(1);
    if machine.snapshot.consecutive_startup_crashes >= SAFE_MODE_CRASH_THRESHOLD {
        machine.snapshot.state = LauncherState::SafeMode as u32;
        machine.snapshot.next_start_allowed_milliseconds = 0;
        return;
    }
    let delay = INITIAL_BACKOFF_MS
        .saturating_mul(
            1_u64
                << machine
                    .snapshot
                    .consecutive_startup_crashes
                    .saturating_sub(1)
                    .min(16),
        )
        .min(MAXIMUM_BACKOFF_MS);
    machine.snapshot.state = LauncherState::CrashBackoff as u32;
    machine.snapshot.next_start_allowed_milliseconds = now.saturating_add(delay);
}

pub fn parse_launcher_arguments<I>(arguments: I) -> Result<LauncherInvocation, LauncherError>
where
    I: IntoIterator<Item = OsString>,
{
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    if arguments.len() == 1 && arguments[0] == OsStr::new("--version") {
        return Ok(LauncherInvocation::Version);
    }
    let mut options = LauncherOptions {
        installed_defaults: arguments.is_empty(),
        ..LauncherOptions::default()
    };
    let mut index = 0;
    while index < arguments.len() {
        let argument = arguments[index].as_os_str();
        let option = |name| -> Result<&OsString, LauncherError> {
            arguments
                .get(index + 1)
                .ok_or(LauncherError::MissingOptionValue(name))
        };
        if argument == OsStr::new("--engine") {
            options.engine_path = Some(PathBuf::from(option("--engine")?));
            index += 2;
        } else if argument == OsStr::new("--ui") {
            options.ui_path = Some(PathBuf::from(option("--ui")?));
            index += 2;
        } else if argument == OsStr::new("--no-warmup") {
            options.warmup = false;
            index += 1;
        } else if argument == OsStr::new("--background") {
            options.installed_defaults = true;
            index += 1;
        } else if argument == OsStr::new("--engine-ready-event") {
            options.engine_ready_event = Some(option("--engine-ready-event")?.clone());
            index += 2;
        } else if argument == OsStr::new("--ready-event") {
            options.ready_event = Some(option("--ready-event")?.clone());
            index += 2;
        } else if argument == OsStr::new("--stop-event") {
            options.stop_event = Some(option("--stop-event")?.clone());
            index += 2;
        } else if argument == OsStr::new("--state-file") {
            options.state_file = Some(PathBuf::from(option("--state-file")?));
            index += 2;
        } else if argument == OsStr::new("--generation") {
            options.generation = Some(option("--generation")?.clone());
            index += 2;
        } else {
            return Err(LauncherError::InvalidArguments);
        }
    }
    Ok(LauncherInvocation::Supervise(Box::new(options)))
}

pub fn load_launcher_snapshot(path: &Path) -> Result<Fcitx5LauncherSnapshot, LauncherError> {
    match load_snapshot_from_path(path) {
        Ok(snapshot) => Ok(snapshot),
        Err(0) => Err(LauncherError::StateMissing(path.to_path_buf())),
        Err(2) => Err(LauncherError::StateInvalid(path.to_path_buf())),
        _ => Err(LauncherError::StateStore(path.to_path_buf())),
    }
}

pub fn save_launcher_snapshot(
    path: &Path,
    snapshot: Fcitx5LauncherSnapshot,
) -> Result<(), LauncherError> {
    save_snapshot_to_path(path, snapshot)
        .then_some(())
        .ok_or_else(|| LauncherError::StateStore(path.to_path_buf()))
}

fn generation(options: &LauncherOptions) -> OsString {
    options
        .generation
        .clone()
        .unwrap_or_else(|| OsString::from(current_runtime_generation_for_current_process()))
}

pub fn resolve_launcher_process_paths(
    options: &LauncherOptions,
    directory: &Path,
) -> Result<(PathBuf, Option<PathBuf>), LauncherError> {
    let (engine, ui) = match (&options.engine_path, options.installed_defaults) {
        (Some(engine), _) => (engine.clone(), options.ui_path.clone()),
        (None, true) => {
            let (engine, ui) = resolve_default_process_paths(directory, &generation(options));
            (engine, Some(ui))
        }
        (None, false) => return Err(LauncherError::MissingEnginePath),
    };
    for (option, path) in [("--engine", &engine)]
        .into_iter()
        .chain(ui.iter().map(|path| ("--ui", path)))
    {
        if !is_absolute_windows_path(path) {
            return Err(LauncherError::InvalidPath(option));
        }
        if !path.exists() {
            return Err(LauncherError::MissingPath(path.to_path_buf()));
        }
    }
    Ok((engine, ui))
}

pub fn prepare_supervisor_start(
    options: &LauncherOptions,
    directory: &Path,
    now: u64,
) -> Result<LauncherStartup, LauncherError> {
    let (engine_path, ui_path) = resolve_launcher_process_paths(options, directory)?;
    let state_path = options
        .state_file
        .clone()
        .or_else(default_state_store_path)
        .ok_or_else(|| LauncherError::StateStore(PathBuf::new()))?;
    let parent = state_path.parent().unwrap_or(&state_path);
    if !is_absolute_windows_path(parent) {
        return Err(LauncherError::InvalidPath("--state-file"));
    }
    if !parent.exists() {
        return Err(LauncherError::MissingPath(parent.to_path_buf()));
    }
    let snapshot = match load_snapshot_from_path(&state_path) {
        Ok(snapshot) => snapshot,
        Err(0) => {
            let snapshot = Fcitx5LauncherSnapshot {
                state: LauncherState::Normal as u32,
                consecutive_startup_crashes: 0,
                next_start_allowed_milliseconds: 0,
            };
            save_launcher_snapshot(&state_path, snapshot)?;
            snapshot
        }
        Err(2) => Fcitx5LauncherSnapshot {
            state: LauncherState::UserStopped as u32,
            consecutive_startup_crashes: 0,
            next_start_allowed_milliseconds: 0,
        },
        _ => return Err(LauncherError::StateStore(state_path)),
    };
    let snapshot = normalize_snapshot(snapshot, now)
        .ok_or_else(|| LauncherError::StateInvalid(state_path.clone()))?;
    let mut machine = Fcitx5LauncherMachine {
        snapshot,
        engine_state: EngineState::Stopped as u32,
    };
    let start = options.warmup.then(|| request_start(&mut machine, now));
    let engine_command_line = match (
        start,
        options.engine_ready_event.as_deref(),
        options.stop_event.as_deref(),
    ) {
        (Some(start), Some(ready), Some(stop))
            if start.disposition == StartDisposition::Start as u32 =>
        {
            Some(engine_command(
                engine_path.as_os_str(),
                ready,
                stop,
                &generation(options),
                start.safe_mode,
            ))
        }
        _ => None,
    };
    let start = start.unwrap_or_else(|| decision(StartDisposition::AlreadyActive, false, 0));
    Ok(LauncherStartup {
        engine_path,
        ui_path,
        state_path,
        snapshot,
        status: LauncherStatus {
            launcher_state: machine.snapshot.state,
            engine_state: machine.engine_state,
            start_disposition: start.disposition,
            retry_after_milliseconds: start.retry_after_milliseconds,
        },
        engine_command_line,
    })
}

#[must_use]
pub fn format_launcher_status(status: &LauncherStatus) -> String {
    format!(
        "launcher_state={} engine_state={} start_disposition={} retry_after_ms={}",
        status.launcher_state,
        status.engine_state,
        status.start_disposition,
        status.retry_after_milliseconds
    )
}

struct EngineProcess {
    child: Child,
    stop: NamedEvent,
    started: u64,
}
struct UiProcess {
    child: Child,
    safe_mode: bool,
}

struct EngineStartContext<'a> {
    identity: &'a CurrentUserRuntimeIdentity,
    security: &'a CurrentUserSecurityAttributes,
    generation: &'a str,
    engine_path: &'a Path,
    ready_name: &'a OsStr,
    ready: &'a NamedEvent,
    job: &'a JobObject,
    sequence: &'a mut u64,
}

fn background(path: &Path) -> ProcessCommand {
    let mut command = ProcessCommand::new(path);
    command
        .creation_flags(CREATE_NO_WINDOW)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

fn assign(job: &JobObject, child: &mut Child, label: &str) -> Result<(), LauncherError> {
    job.assign_process(child.as_handle()).map_err(|error| {
        let _ = child.kill();
        let _ = child.wait();
        LauncherError::Runtime(format!("{label} job assignment failed: {error}"))
    })
}

fn spawn_engine(
    launch: &EngineLaunch,
    job: &JobObject,
    stop: NamedEvent,
) -> Result<EngineProcess, LauncherError> {
    let mut command = background(&launch.engine_path);
    command
        .arg("--ready-event")
        .arg(&launch.ready_event)
        .arg("--stop-event")
        .arg(&launch.stop_event)
        .arg("--generation")
        .arg(&launch.generation);
    if launch.safe_mode {
        command.arg("--safe-mode");
    }
    let mut child = command
        .spawn()
        .map_err(|error| LauncherError::Runtime(format!("engine launch failed: {error}")))?;
    assign(job, &mut child, "engine")?;
    Ok(EngineProcess {
        child,
        stop,
        started: launcher_tick_milliseconds(),
    })
}

fn spawn_ui(
    path: &Path,
    parent: u32,
    generation: &str,
    safe_mode: bool,
    job: &JobObject,
) -> Result<UiProcess, LauncherError> {
    let mut command = background(path);
    command
        .arg("--parent-pid")
        .arg(parent.to_string())
        .arg("--generation")
        .arg(generation);
    if safe_mode {
        command.arg("--safe-mode");
    }
    let mut child = command
        .spawn()
        .map_err(|error| LauncherError::Runtime(format!("UI launch failed: {error}")))?;
    assign(job, &mut child, "UI")?;
    Ok(UiProcess { child, safe_mode })
}

fn stop_engine(engine: &mut Option<EngineProcess>) {
    let Some(mut engine) = engine.take() else {
        return;
    };
    let _ = engine.stop.signal();
    if !wait_for_handle(engine.child.as_handle(), STOP_TIMEOUT) {
        let _ = engine.child.kill();
        let _ = wait_for_handle(engine.child.as_handle(), KILL_TIMEOUT);
    }
    let _ = engine.child.wait();
}

fn stop_ui(ui: &mut Option<UiProcess>) {
    let Some(mut ui) = ui.take() else {
        return;
    };
    if !wait_for_handle(ui.child.as_handle(), Duration::ZERO) {
        let _ = ui.child.kill();
        let _ = wait_for_handle(ui.child.as_handle(), KILL_TIMEOUT);
    }
    let _ = ui.child.wait();
}

fn start_engine(
    machine: &mut Fcitx5LauncherMachine,
    context: EngineStartContext<'_>,
) -> Result<(Fcitx5LauncherStartDecision, Option<EngineProcess>), LauncherError> {
    let start = request_start(machine, launcher_tick_milliseconds());
    if start.disposition != StartDisposition::Start as u32 {
        return Ok((start, None));
    }
    context
        .ready
        .reset()
        .map_err(|error| LauncherError::Runtime(format!("engine-ready reset failed: {error}")))?;
    *context.sequence = context.sequence.saturating_add(1).max(1);
    let stop_name = context
        .identity
        .local_object_name(
            context.generation,
            &format!("engine-stop-{}", context.sequence),
        )
        .ok_or_else(|| LauncherError::Runtime("invalid engine-stop name".to_owned()))?;
    let stop = NamedEvent::create(&stop_name, context.security)
        .map_err(|error| LauncherError::Runtime(format!("engine-stop event failed: {error}")))?;
    let launch = EngineLaunch::new(
        context.engine_path.to_path_buf(),
        context.ready_name.to_os_string(),
        stop_name,
        OsString::from(context.generation),
        start.safe_mode,
    );
    Ok((start, Some(spawn_engine(&launch, context.job, stop)?)))
}

fn read_frame(server: &NamedPipeServer) -> Option<Vec<u8>> {
    let mut header = [0_u8; protocol::HEADER_SIZE];
    if !server.read_exact(&mut header, deadline_after(PIPE_TRANSFER_TIMEOUT_MS)) {
        return None;
    }
    let (_, size, _) = decode_header(&header)?;
    let mut frame = header.to_vec();
    frame.resize(protocol::HEADER_SIZE.checked_add(size as usize)?, 0);
    if size != 0
        && !server.read_exact(
            &mut frame[protocol::HEADER_SIZE..],
            deadline_after(PIPE_TRANSFER_TIMEOUT_MS),
        )
    {
        return None;
    }
    Some(frame)
}

fn engine_status(
    identity: &CurrentUserRuntimeIdentity,
    generation: &str,
    engine: &Path,
) -> EngineStatusResponse {
    let Some(endpoint) = identity.local_endpoint_name(generation, "engine") else {
        return engine_status_failure("endpoint");
    };
    let Some(mut client) = VerifiedPipeClient::connect_exact(&endpoint, engine, STATUS_TIMEOUT)
    else {
        return engine_status_failure("connect");
    };
    let mut response = vec![0_u8; protocol::MAX_CONTROL_FRAME_SIZE];
    let hello_id = next_launcher_request_id();
    let hello = HelloRequest {
        metadata: Metadata {
            request_id: hello_id,
            session_id: identity.session_id(),
            ..Metadata::default()
        },
        client_architecture_bits: usize::BITS,
        client_process_id: identity.process_id(),
    };
    let Some(hello_bytes) = encode_hello_request(&hello) else {
        return engine_status_failure("encode-hello");
    };
    let Some(length) = client.transact(&hello_bytes, &mut response, STATUS_TIMEOUT) else {
        return engine_status_failure("hello-transport");
    };
    let Some(hello) =
        decode_frame(&response[..length]).and_then(|frame| decode_hello_response(&frame))
    else {
        return engine_status_failure("hello-decode");
    };
    if hello.status != Status::Ok
        || hello.metadata.response_to != hello_id
        || hello.metadata.session_id != identity.session_id()
        || hello.metadata.engine_epoch == 0
    {
        return engine_status_failure("hello-contract");
    }
    let id = next_launcher_request_id();
    let request = EngineStatusRequest {
        metadata: Metadata {
            request_id: id,
            engine_epoch: hello.metadata.engine_epoch,
            session_id: identity.session_id(),
            ..Metadata::default()
        },
    };
    let Some(bytes) = encode_engine_status_request(&request) else {
        return engine_status_failure("encode-status");
    };
    let Some(length) = client.transact(&bytes, &mut response, STATUS_TIMEOUT) else {
        return engine_status_failure("status-transport");
    };
    let Some(status) =
        decode_frame(&response[..length]).and_then(|frame| decode_engine_status_response(&frame))
    else {
        return engine_status_failure("status-decode");
    };
    if status.status == Status::Ok
        && status.metadata.response_to == id
        && status.metadata.session_id == identity.session_id()
        && status.metadata.engine_epoch == hello.metadata.engine_epoch
    {
        status
    } else {
        engine_status_failure("status-contract")
    }
}

fn engine_status_failure(_code: &'static str) -> EngineStatusResponse {
    EngineStatusResponse::default()
}

fn state_command(command: LauncherCommand) -> Option<Command> {
    match command {
        LauncherCommand::UserStop => Some(Command::UserStop),
        LauncherCommand::Resume => Some(Command::Resume),
        LauncherCommand::BeginUpdate => Some(Command::BeginUpdate),
        LauncherCommand::EndUpdate => Some(Command::EndUpdate),
        LauncherCommand::BeginUninstall => Some(Command::BeginUninstall),
        LauncherCommand::ResetSafeMode => Some(Command::ResetSafeMode),
        LauncherCommand::StartDemand | LauncherCommand::Status | LauncherCommand::Shutdown => None,
    }
}

fn response(
    request: &protocol::LauncherRequest,
    identity: &CurrentUserRuntimeIdentity,
    machine: &Fcitx5LauncherMachine,
    decision: Fcitx5LauncherStartDecision,
    epoch: u64,
    status: Status,
    engine: EngineStatusResponse,
) -> Option<Vec<u8>> {
    encode_launcher_response(&LauncherResponse {
        metadata: Metadata {
            request_id: next_launcher_request_id(),
            response_to: request.metadata.request_id,
            engine_epoch: epoch,
            session_id: identity.session_id(),
            ..Metadata::default()
        },
        status,
        launcher_state: machine.snapshot.state,
        engine_state: machine.engine_state,
        start_disposition: decision.disposition,
        safe_mode: decision.safe_mode,
        retry_after_milliseconds: decision.retry_after_milliseconds,
        current_input_method_id: engine.current_input_method_id,
        current_input_method_name: engine.current_input_method_name,
        current_input_method_native_name: engine.current_input_method_native_name,
        current_input_method_short_label: engine.current_input_method_short_label,
    })
}

pub fn run_launcher(options: LauncherOptions) -> Result<(), LauncherError> {
    let identity = CurrentUserRuntimeIdentity::current().ok_or_else(|| {
        LauncherError::Runtime("launcher requires an interactive user session".to_owned())
    })?;
    let security = identity.security_attributes().ok_or_else(|| {
        LauncherError::Runtime("launcher security policy is unavailable".to_owned())
    })?;
    let generation = generation(&options)
        .into_string()
        .map_err(|_| LauncherError::Runtime("launcher generation must be Unicode".to_owned()))?;
    let endpoint = identity
        .local_endpoint_name(&generation, "launcher")
        .ok_or_else(|| LauncherError::Runtime("invalid launcher pipe name".to_owned()))?;
    let mutex = identity
        .local_object_name(&generation, "launcher")
        .ok_or_else(|| LauncherError::Runtime("invalid launcher mutex name".to_owned()))?;
    let singleton = SingleInstance::acquire(&mutex, &security)
        .map_err(|error| LauncherError::Runtime(format!("launcher singleton failed: {error}")))?;
    if !singleton.is_primary() {
        return named_pipe_is_available(&endpoint, Duration::from_millis(100))
            .then_some(())
            .ok_or_else(|| {
                LauncherError::Runtime("existing launcher pipe is unavailable".to_owned())
            });
    }
    let directory = identity.executable_path().parent().ok_or_else(|| {
        LauncherError::Runtime("launcher executable has no parent directory".to_owned())
    })?;
    let startup = prepare_supervisor_start(&options, directory, launcher_tick_milliseconds())?;
    let job = JobObject::new_kill_on_close().map_err(|error| {
        LauncherError::Runtime(format!("launcher job creation failed: {error}"))
    })?;
    let ready_name = options
        .engine_ready_event
        .clone()
        .or_else(|| identity.local_object_name(&generation, "engine-ready"))
        .ok_or_else(|| LauncherError::Runtime("invalid engine-ready name".to_owned()))?;
    let ready = NamedEvent::create(&ready_name, &security)
        .map_err(|error| LauncherError::Runtime(format!("engine-ready event failed: {error}")))?;
    let stop_name = options
        .stop_event
        .clone()
        .or_else(|| identity.local_object_name(&generation, "launcher-stop"))
        .ok_or_else(|| LauncherError::Runtime("invalid launcher-stop name".to_owned()))?;
    let stop = NamedEvent::create(&stop_name, &security)
        .map_err(|error| LauncherError::Runtime(format!("launcher-stop event failed: {error}")))?;
    let launcher_ready = options
        .ready_event
        .as_deref()
        .map(|name| NamedEvent::create(name, &security))
        .transpose()
        .map_err(|error| LauncherError::Runtime(format!("launcher-ready event failed: {error}")))?;
    let mut machine = Fcitx5LauncherMachine {
        snapshot: startup.snapshot,
        engine_state: EngineState::Stopped as u32,
    };
    let mut engine: Option<EngineProcess> = None;
    let mut ui = startup
        .ui_path
        .as_deref()
        .map(|path| spawn_ui(path, identity.process_id(), &generation, false, &job))
        .transpose()?;
    let mut restart = options.warmup;
    let mut epoch = 0_u64;
    let mut sequence = 0_u64;
    let mut shutdown = false;
    let mut listener =
        NamedPipeServer::create(&endpoint, &security, protocol::MAX_CONTROL_FRAME_SIZE).map_err(
            |error| LauncherError::Runtime(format!("launcher pipe creation failed: {error}")),
        )?;
    while !shutdown && !stop.is_signaled() {
        let now = launcher_tick_milliseconds();
        if let Some(current) = engine.as_mut() {
            let exited = current
                .child
                .try_wait()
                .map_err(|error| LauncherError::Runtime(format!("engine status failed: {error}")))?
                .is_some();
            let timed_out = machine.engine_state == EngineState::Starting as u32
                && now.saturating_sub(current.started) >= READY_TIMEOUT.as_millis() as u64;
            if exited || timed_out {
                let runtime = now.saturating_sub(current.started);
                stop_engine(&mut engine);
                engine_exited(&mut machine, runtime, now);
                save_launcher_snapshot(&startup.state_path, machine.snapshot)?;
                restart = true;
            }
        }
        if engine.is_some()
            && machine.engine_state == EngineState::Starting as u32
            && ready.is_signaled()
        {
            machine.engine_state = EngineState::Ready as u32;
        }
        let safe_mode = machine.snapshot.state == LauncherState::SafeMode as u32;
        if ui.as_ref().is_some_and(|ui| ui.safe_mode != safe_mode) {
            stop_ui(&mut ui);
            ui = startup
                .ui_path
                .as_deref()
                .map(|path| spawn_ui(path, identity.process_id(), &generation, safe_mode, &job))
                .transpose()?;
        }
        if restart && engine.is_none() {
            match start_engine(
                &mut machine,
                EngineStartContext {
                    identity: &identity,
                    security: &security,
                    generation: &generation,
                    engine_path: &startup.engine_path,
                    ready_name: &ready_name,
                    ready: &ready,
                    job: &job,
                    sequence: &mut sequence,
                },
            ) {
                Ok((decision, started)) => {
                    if decision.disposition == StartDisposition::Start as u32 {
                        epoch = epoch.saturating_add(1).max(1);
                        engine = started;
                    }
                    restart = decision.disposition != StartDisposition::Suppressed as u32;
                }
                Err(_) => {
                    engine_exited(&mut machine, 0, now);
                    save_launcher_snapshot(&startup.state_path, machine.snapshot)?;
                    restart = true;
                }
            }
        }
        if let Some(event) = &launcher_ready {
            event.signal().map_err(|error| {
                LauncherError::Runtime(format!("launcher-ready signal failed: {error}"))
            })?;
        }
        if !listener.connect_until(deadline_after(PIPE_CONNECT_TIMEOUT_MS), &stop) {
            continue;
        }
        let server = std::mem::replace(
            &mut listener,
            NamedPipeServer::create(&endpoint, &security, protocol::MAX_CONTROL_FRAME_SIZE)
                .map_err(|error| {
                    LauncherError::Runtime(format!("launcher pipe creation failed: {error}"))
                })?,
        );
        if !server.verifies_client(&identity) {
            continue;
        }
        let Some(frame_bytes) = read_frame(&server) else {
            continue;
        };
        let Some(frame) = decode_frame(&frame_bytes) else {
            continue;
        };
        let Some(request) = decode_launcher_request(&frame) else {
            continue;
        };
        if request.metadata.session_id != identity.session_id() {
            continue;
        }
        // The readiness event can become signaled while this pipe instance is
        // waiting for a request. Refresh the observed state before serving a
        // status command so callers never see stale `Starting` state.
        if engine.is_some()
            && machine.engine_state == EngineState::Starting as u32
            && ready.is_signaled()
        {
            machine.engine_state = EngineState::Ready as u32;
        }
        let mut status = Status::Ok;
        let mut decision = decision(StartDisposition::AlreadyActive, false, 0);
        if let Some(command) = state_command(request.command) {
            if !apply_command(&mut machine, command)
                || save_launcher_snapshot(&startup.state_path, machine.snapshot).is_err()
            {
                status = Status::Unsupported;
            } else if matches!(
                request.command,
                LauncherCommand::UserStop
                    | LauncherCommand::BeginUpdate
                    | LauncherCommand::BeginUninstall
            ) {
                stop_engine(&mut engine);
                machine.engine_state = EngineState::Stopped as u32;
                restart = false;
            }
        } else if request.command == LauncherCommand::StartDemand {
            restart = true;
            match start_engine(
                &mut machine,
                EngineStartContext {
                    identity: &identity,
                    security: &security,
                    generation: &generation,
                    engine_path: &startup.engine_path,
                    ready_name: &ready_name,
                    ready: &ready,
                    job: &job,
                    sequence: &mut sequence,
                },
            ) {
                Ok((start, started)) => {
                    decision = start;
                    if start.disposition == StartDisposition::Start as u32 {
                        epoch = epoch.saturating_add(1).max(1);
                        engine = started;
                    }
                    restart = start.disposition != StartDisposition::Suppressed as u32;
                }
                Err(_) => {
                    engine_exited(&mut machine, 0, launcher_tick_milliseconds());
                    let _ = save_launcher_snapshot(&startup.state_path, machine.snapshot);
                    status = Status::Unsupported;
                }
            }
        } else if request.command == LauncherCommand::Shutdown {
            shutdown = true;
        }
        let input = if request.command == LauncherCommand::Status
            && engine.is_some()
            && machine.engine_state == EngineState::Ready as u32
        {
            engine_status(&identity, &generation, &startup.engine_path)
        } else {
            EngineStatusResponse::default()
        };
        if let Some(bytes) = response(
            &request, &identity, &machine, decision, epoch, status, input,
        ) {
            if server.write_all(&bytes, deadline_after(PIPE_TRANSFER_TIMEOUT_MS)) {
                let _ = server.wait_for_client_disconnect(deadline_after(PIPE_TRANSFER_TIMEOUT_MS));
            }
        }
    }
    stop_engine(&mut engine);
    stop_ui(&mut ui);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_crashes_enter_safe_mode_without_respawn_loop() {
        let mut machine = Fcitx5LauncherMachine {
            snapshot: Fcitx5LauncherSnapshot {
                state: LauncherState::Normal as u32,
                consecutive_startup_crashes: 0,
                next_start_allowed_milliseconds: 0,
            },
            engine_state: EngineState::Stopped as u32,
        };
        let mut now = 1_u64;
        for _ in 0..SAFE_MODE_CRASH_THRESHOLD {
            assert_eq!(
                request_start(&mut machine, now).disposition,
                StartDisposition::Start as u32
            );
            engine_exited(&mut machine, 0, now);
            now = machine.snapshot.next_start_allowed_milliseconds;
        }
        assert_eq!(machine.snapshot.state, LauncherState::SafeMode as u32);
    }

    #[test]
    fn update_marker_suppresses_demand_start() {
        let mut machine = Fcitx5LauncherMachine {
            snapshot: Fcitx5LauncherSnapshot {
                state: LauncherState::Updating as u32,
                consecutive_startup_crashes: 0,
                next_start_allowed_milliseconds: 0,
            },
            engine_state: EngineState::Stopped as u32,
        };
        assert_eq!(
            request_start(&mut machine, 0).disposition,
            StartDisposition::Suppressed as u32
        );
    }
}
