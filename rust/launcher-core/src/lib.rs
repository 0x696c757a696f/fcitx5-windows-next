#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{
    MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
};

const STARTUP_CRASH_WINDOW_MS: u64 = 10_000;
const STABLE_RUNTIME_MS: u64 = 60_000;
const INITIAL_BACKOFF_MS: u64 = 250;
const MAXIMUM_BACKOFF_MS: u64 = 30_000;
const SAFE_MODE_CRASH_THRESHOLD: u32 = 3;
const RELEASE_DATA_DIRECTORY_FALLBACK: &str = "Fcitx5";
const STATE_FILE_NAME: &str = "launcher-state.v1";
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LauncherState {
    Normal = 0,
    UserStopped = 1,
    Updating = 2,
    Uninstalling = 3,
    CrashBackoff = 4,
    SafeMode = 5,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EngineState {
    Stopped = 0,
    Starting = 1,
    Ready = 2,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Command {
    UserStop = 0,
    Resume = 1,
    BeginUpdate = 2,
    EndUpdate = 3,
    BeginUninstall = 4,
    ResetSafeMode = 5,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartDisposition {
    Start = 0,
    AlreadyActive = 1,
    Suppressed = 2,
    Backoff = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fcitx5LauncherSnapshot {
    pub state: u32,
    pub consecutive_startup_crashes: u32,
    pub next_start_allowed_milliseconds: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fcitx5LauncherMachine {
    pub snapshot: Fcitx5LauncherSnapshot,
    pub engine_state: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fcitx5LauncherStartDecision {
    pub disposition: u32,
    pub safe_mode: u8,
    pub reserved: [u8; 7],
    pub retry_after_milliseconds: u64,
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

fn command(value: u32) -> Option<Command> {
    match value {
        0 => Some(Command::UserStop),
        1 => Some(Command::Resume),
        2 => Some(Command::BeginUpdate),
        3 => Some(Command::EndUpdate),
        4 => Some(Command::BeginUninstall),
        5 => Some(Command::ResetSafeMode),
        _ => None,
    }
}

fn release_data_directory() -> &'static str {
    option_env!("FCITX_RELEASE_DATA_DIRECTORY").unwrap_or(RELEASE_DATA_DIRECTORY_FALLBACK)
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
    for mut line in text.split('\n') {
        if let Some(stripped) = line.strip_suffix('\r') {
            line = stripped;
        }
        if let Some(value) = line.strip_prefix(&marker) {
            return Some(value);
        }
    }
    None
}

fn parse_unsigned(value: &str) -> Option<u64> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut result = 0_u64;
    for digit in value.bytes().map(|b| u64::from(b - b'0')) {
        result = result.checked_mul(10)?.checked_add(digit)?;
    }
    Some(result)
}

fn parse_snapshot(text: &str) -> Option<Fcitx5LauncherSnapshot> {
    if let Some(legacy) = text.strip_suffix('\n') {
        if let Some(state) = parse_state_name(legacy) {
            return Some(Fcitx5LauncherSnapshot {
                state: state as u32,
                consecutive_startup_crashes: 0,
                next_start_allowed_milliseconds: 0,
            });
        }
    }
    let format = line_value(text, "format_version")?;
    let state = parse_state_name(line_value(text, "state")?)?;
    let crashes = parse_unsigned(line_value(text, "consecutive_startup_crashes")?)?;
    let next_start = parse_unsigned(line_value(text, "next_start_allowed_ms")?)?;
    if format != "2" || crashes > u64::from(u32::MAX) {
        return None;
    }
    Some(Fcitx5LauncherSnapshot {
        state: state as u32,
        consecutive_startup_crashes: crashes as u32,
        next_start_allowed_milliseconds: next_start,
    })
}

fn serialize_snapshot(snapshot: Fcitx5LauncherSnapshot) -> Option<String> {
    let state = launcher_state(snapshot.state)?;
    Some(format!(
        "format_version=2\nstate={}\nconsecutive_startup_crashes={}\nnext_start_allowed_ms={}\n",
        state_name(state),
        snapshot.consecutive_startup_crashes,
        snapshot.next_start_allowed_milliseconds
    ))
}

fn path_from_wide(path: *const u16, len: usize) -> Option<PathBuf> {
    if path.is_null() {
        return (len == 0).then(PathBuf::new);
    }
    // SAFETY: The C++ adapter passes a valid UTF-16 buffer with exactly `len`
    // elements for the duration of this call.
    let slice = unsafe { std::slice::from_raw_parts(path, len) };
    Some(PathBuf::from(OsString::from_wide(slice)))
}

fn os_string_from_wide(text: *const u16, len: usize) -> Option<OsString> {
    if text.is_null() {
        return (len == 0).then(OsString::new);
    }
    // SAFETY: The C++ adapter passes a valid UTF-16 buffer with exactly `len`
    // elements for the duration of this call.
    let slice = unsafe { std::slice::from_raw_parts(text, len) };
    Some(OsString::from_wide(slice))
}

fn wide_slice<'a>(text: *const u16, len: usize) -> Option<&'a [u16]> {
    if text.is_null() {
        return (len == 0).then_some(&[]);
    }
    // SAFETY: The C++ adapter passes a valid UTF-16 buffer with exactly `len`
    // elements for the duration of this call.
    Some(unsafe { std::slice::from_raw_parts(text, len) })
}

fn absolute_windows_path_wide(path: &[u16]) -> bool {
    path.len() >= 3
        && ((path[0] >= b'A' as u16 && path[0] <= b'Z' as u16)
            || (path[0] >= b'a' as u16 && path[0] <= b'z' as u16))
        && path[1] == b':' as u16
        && (path[2] == b'\\' as u16 || path[2] == b'/' as u16)
}

fn resolve_default_process_paths(
    executable_directory: &Path,
    generation: &OsStr,
) -> (PathBuf, PathBuf) {
    let installed_root = if executable_directory.file_name() == Some(OsStr::new("bin")) {
        executable_directory
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| executable_directory.to_path_buf())
    } else {
        executable_directory.to_path_buf()
    };
    let generation_bin = installed_root.join("runtime").join(generation).join("bin");
    let generation_engine = generation_bin.join("fcitx5-engine.exe");
    let generation_ui = generation_bin.join("fcitx5-ui.exe");
    if generation_engine.exists() && generation_ui.exists() {
        (generation_engine, generation_ui)
    } else {
        (
            executable_directory.join("fcitx5-engine.exe"),
            executable_directory.join("fcitx5-ui.exe"),
        )
    }
}

fn wide_nul(path: &Path) -> Vec<u16> {
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    wide
}

fn replace_file(source: &Path, destination: &Path) -> bool {
    let source = wide_nul(source);
    let destination = wide_nul(destination);
    // SAFETY: Both path buffers are NUL-terminated and live for the duration of
    // the call. `MOVEFILE_REPLACE_EXISTING` preserves the old C++ ledger
    // publication behavior.
    unsafe {
        MoveFileExW(
            PCWSTR(source.as_ptr()),
            PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .is_ok()
}

fn temporary_path(destination: &Path) -> PathBuf {
    let mut value = destination.as_os_str().to_os_string();
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
    let mut limited = file.take(256);
    if limited.read_to_end(&mut bytes).is_err() {
        return Err(3);
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Err(2);
    };
    parse_snapshot(text).ok_or(2)
}

fn save_snapshot_to_path(path: &Path, snapshot: Fcitx5LauncherSnapshot) -> bool {
    if path.as_os_str().is_empty() {
        return false;
    }
    let Some(text) = serialize_snapshot(snapshot) else {
        return false;
    };
    let temporary = temporary_path(path);
    let result = (|| -> std::io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        Ok(())
    })();
    if result.is_err() || !replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return false;
    }
    true
}

fn default_state_store_path() -> Option<PathBuf> {
    let local_app_data = PathBuf::from(std::env::var_os("LOCALAPPDATA")?);
    let directory = local_app_data.join(release_data_directory());
    fs::create_dir_all(&directory).ok()?;
    Some(directory.join(STATE_FILE_NAME))
}

fn write_utf16_path(value: &Path, out: *mut u16, capacity: usize) -> usize {
    let wide: Vec<u16> = value.as_os_str().encode_wide().collect();
    if !out.is_null() && capacity != 0 {
        let count = wide.len().min(capacity);
        // SAFETY: The caller supplied writable storage for `capacity` u16
        // values. We copy at most that many initialized elements.
        unsafe { std::ptr::copy_nonoverlapping(wide.as_ptr(), out, count) };
    }
    wide.len()
}

fn write_utf16_path_checked(value: &Path, out: *mut u16, capacity: usize) -> Option<usize> {
    let wide: Vec<u16> = value.as_os_str().encode_wide().collect();
    if !out.is_null() {
        if capacity < wide.len() {
            return None;
        }
        if !wide.is_empty() {
            // SAFETY: The caller supplied writable storage for at least
            // `wide.len()` u16 values.
            unsafe { std::ptr::copy_nonoverlapping(wide.as_ptr(), out, wide.len()) };
        }
    }
    Some(wide.len())
}

fn write_utf16_string(value: &str, out: *mut u16, capacity: usize) -> usize {
    let wide: Vec<u16> = value.encode_utf16().collect();
    write_utf16_units(&wide, out, capacity)
}

fn write_utf16_units(value: &[u16], out: *mut u16, capacity: usize) -> usize {
    if !out.is_null() && capacity != 0 {
        let count = value.len().min(capacity);
        if count != 0 {
            // SAFETY: The caller supplied writable storage for `capacity` u16
            // values. We copy at most that many initialized elements.
            unsafe { std::ptr::copy_nonoverlapping(value.as_ptr(), out, count) };
        }
    }
    value.len()
}

fn utf16_string_from_raw(text: *const u16, len: usize) -> Option<String> {
    let text = wide_slice(text, len)?;
    String::from_utf16(text).ok()
}

fn utf8_candidate_from_raw(text: *const u8, len: usize) -> Option<String> {
    if text.is_null() {
        return None;
    }
    // SAFETY: The caller passes a valid byte buffer with exactly `len` elements
    // for the duration of this call. We copy into an owned `String`.
    let slice = unsafe { std::slice::from_raw_parts(text, len) };
    std::str::from_utf8(slice)
        .ok()
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn tray_status_text(
    launcher_state_value: u32,
    engine_state_value: u32,
    chinese: bool,
) -> &'static str {
    let launcher_state = launcher_state(launcher_state_value).unwrap_or(LauncherState::UserStopped);
    let engine_state = match engine_state_value {
        0 => EngineState::Stopped,
        1 => EngineState::Starting,
        2 => EngineState::Ready,
        _ => EngineState::Stopped,
    };
    if launcher_state == LauncherState::SafeMode {
        return if chinese { "安全模式" } else { "Safe mode" };
    }
    if launcher_state == LauncherState::UserStopped {
        return if chinese { "已暂停" } else { "Paused" };
    }
    if launcher_state == LauncherState::CrashBackoff {
        return if chinese {
            "故障恢复中"
        } else {
            "Recovering"
        };
    }
    if launcher_state == LauncherState::Updating {
        return if chinese { "正在更新" } else { "Updating" };
    }
    if launcher_state == LauncherState::Uninstalling {
        return if chinese {
            "正在卸载"
        } else {
            "Uninstalling"
        };
    }
    if engine_state == EngineState::Ready {
        return if chinese { "运行中" } else { "Running" };
    }
    if engine_state == EngineState::Starting {
        return if chinese { "正在启动" } else { "Starting" };
    }
    if chinese {
        "服务未运行"
    } else {
        "Service stopped"
    }
}

fn input_method_display_from_raw(
    native_name: *const u8,
    native_name_len: usize,
    name: *const u8,
    name_len: usize,
    id: *const u8,
    id_len: usize,
) -> String {
    [
        utf8_candidate_from_raw(native_name, native_name_len),
        utf8_candidate_from_raw(name, name_len),
        utf8_candidate_from_raw(id, id_len),
    ]
    .into_iter()
    .flatten()
    .next()
    .unwrap_or_default()
}

fn append_literal(output: &mut Vec<u16>, text: &str) {
    output.extend(text.encode_utf16());
}

fn append_os(output: &mut Vec<u16>, value: &OsStr) {
    output.extend(value.encode_wide());
}

fn append_quoted_os(output: &mut Vec<u16>, value: &OsStr) {
    output.push(b'"' as u16);
    append_os(output, value);
    output.push(b'"' as u16);
}

fn launcher_engine_command(
    engine_path: &OsStr,
    ready_event: &OsStr,
    stop_event: &OsStr,
    generation: &OsStr,
    safe_mode: bool,
) -> Vec<u16> {
    let mut command = Vec::new();
    append_quoted_os(&mut command, engine_path);
    append_literal(&mut command, " --ready-event ");
    append_quoted_os(&mut command, ready_event);
    append_literal(&mut command, " --stop-event ");
    append_quoted_os(&mut command, stop_event);
    append_literal(&mut command, " --generation ");
    append_quoted_os(&mut command, generation);
    if safe_mode {
        append_literal(&mut command, " --safe-mode");
    }
    command
}

fn launcher_ui_command(
    ui_path: &OsStr,
    parent_pid: u32,
    generation: &OsStr,
    safe_mode: bool,
) -> Vec<u16> {
    let mut command = Vec::new();
    append_quoted_os(&mut command, ui_path);
    append_literal(&mut command, " --parent-pid ");
    append_literal(&mut command, &parent_pid.to_string());
    append_literal(&mut command, " --generation ");
    append_quoted_os(&mut command, generation);
    if safe_mode {
        append_literal(&mut command, " --safe-mode");
    }
    command
}

fn launcher_config_command(config_path: &OsStr, arguments: &OsStr) -> Vec<u16> {
    let mut command = Vec::new();
    append_quoted_os(&mut command, config_path);
    if !arguments.is_empty() {
        append_literal(&mut command, " ");
        append_os(&mut command, arguments);
    }
    command
}

fn start_suppressed(state: LauncherState) -> bool {
    matches!(
        state,
        LauncherState::UserStopped | LauncherState::Updating | LauncherState::Uninstalling
    )
}

fn reset_crash_accounting(snapshot: &mut Fcitx5LauncherSnapshot) {
    snapshot.consecutive_startup_crashes = 0;
    snapshot.next_start_allowed_milliseconds = 0;
}

fn normalize_snapshot(
    mut snapshot: Fcitx5LauncherSnapshot,
    now: u64,
) -> Option<Fcitx5LauncherSnapshot> {
    let state = launcher_state(snapshot.state)?;
    match state {
        LauncherState::CrashBackoff => {
            if snapshot.consecutive_startup_crashes == 0 {
                snapshot.consecutive_startup_crashes = 1;
            }
            if snapshot.next_start_allowed_milliseconds == 0
                || snapshot.next_start_allowed_milliseconds > now.saturating_add(MAXIMUM_BACKOFF_MS)
            {
                snapshot.next_start_allowed_milliseconds = now.saturating_add(INITIAL_BACKOFF_MS);
            }
        }
        LauncherState::SafeMode => {
            if snapshot.consecutive_startup_crashes < SAFE_MODE_CRASH_THRESHOLD {
                snapshot.consecutive_startup_crashes = SAFE_MODE_CRASH_THRESHOLD;
            }
            snapshot.next_start_allowed_milliseconds = 0;
        }
        _ => reset_crash_accounting(&mut snapshot),
    }
    Some(snapshot)
}

fn can_apply_state(state: LauncherState, command: Command) -> bool {
    match command {
        Command::UserStop | Command::BeginUpdate => {
            state != LauncherState::Uninstalling && state != LauncherState::Updating
        }
        Command::Resume => state == LauncherState::UserStopped,
        Command::EndUpdate => state == LauncherState::Updating,
        Command::BeginUninstall => state != LauncherState::Uninstalling,
        Command::ResetSafeMode => state == LauncherState::SafeMode,
    }
}

fn state_after_command(command: Command) -> LauncherState {
    match command {
        Command::UserStop => LauncherState::UserStopped,
        Command::BeginUpdate => LauncherState::Updating,
        Command::BeginUninstall => LauncherState::Uninstalling,
        Command::Resume | Command::EndUpdate | Command::ResetSafeMode => LauncherState::Normal,
    }
}

fn apply_command(machine: &mut Fcitx5LauncherMachine, command: Command) -> bool {
    let Some(state) = launcher_state(machine.snapshot.state) else {
        return false;
    };
    if !can_apply_state(state, command) {
        return false;
    }
    machine.snapshot.state = state_after_command(command) as u32;
    match command {
        Command::UserStop | Command::BeginUpdate | Command::BeginUninstall => {
            machine.engine_state = EngineState::Stopped as u32;
        }
        Command::Resume | Command::EndUpdate | Command::ResetSafeMode => {
            reset_crash_accounting(&mut machine.snapshot);
        }
    }
    true
}

fn request_start(machine: &mut Fcitx5LauncherMachine, now: u64) -> Fcitx5LauncherStartDecision {
    let state = launcher_state(machine.snapshot.state).unwrap_or(LauncherState::UserStopped);
    if start_suppressed(state) {
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

fn engine_exited(machine: &mut Fcitx5LauncherMachine, runtime_ms: u64, now: u64) {
    machine.engine_state = EngineState::Stopped as u32;
    let state = launcher_state(machine.snapshot.state).unwrap_or(LauncherState::UserStopped);
    if start_suppressed(state) {
        return;
    }
    if runtime_ms >= STABLE_RUNTIME_MS || runtime_ms >= STARTUP_CRASH_WINDOW_MS {
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
    let shift = machine
        .snapshot
        .consecutive_startup_crashes
        .saturating_sub(1)
        .min(16);
    let delay = INITIAL_BACKOFF_MS
        .saturating_mul(1_u64 << shift)
        .min(MAXIMUM_BACKOFF_MS);
    machine.snapshot.state = LauncherState::CrashBackoff as u32;
    machine.snapshot.next_start_allowed_milliseconds = now.saturating_add(delay);
}

fn decision(
    disposition: StartDisposition,
    safe_mode: bool,
    retry_after_milliseconds: u64,
) -> Fcitx5LauncherStartDecision {
    Fcitx5LauncherStartDecision {
        disposition: disposition as u32,
        safe_mode: u8::from(safe_mode),
        reserved: [0; 7],
        retry_after_milliseconds,
    }
}

#[no_mangle]
/// # Safety
///
/// `out_machine` must be either null or point to writable storage for one
/// `Fcitx5LauncherMachine`. The pointer is not retained after this call.
pub unsafe extern "C" fn fcitx5_launcher_state_init(
    now: u64,
    snapshot: Fcitx5LauncherSnapshot,
    out_machine: *mut Fcitx5LauncherMachine,
) -> i32 {
    let Some(snapshot) = normalize_snapshot(snapshot, now) else {
        return 1;
    };
    if out_machine.is_null() {
        return 1;
    }
    unsafe {
        *out_machine = Fcitx5LauncherMachine {
            snapshot,
            engine_state: EngineState::Stopped as u32,
        };
    }
    0
}

#[no_mangle]
pub extern "C" fn fcitx5_launcher_state_can_apply(state: u32, command: u32) -> u8 {
    match (launcher_state(state), self::command(command)) {
        (Some(state), Some(command)) => u8::from(can_apply_state(state, command)),
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn fcitx5_launcher_state_after(state: u32, command: u32) -> u32 {
    match (launcher_state(state), self::command(command)) {
        (Some(_state), Some(command)) => state_after_command(command) as u32,
        (Some(state), None) => state as u32,
        _ => LauncherState::UserStopped as u32,
    }
}

#[no_mangle]
/// # Safety
///
/// `machine` must be either null or point to a valid writable
/// `Fcitx5LauncherMachine` produced by this module. The pointer is not retained.
pub unsafe extern "C" fn fcitx5_launcher_state_apply(
    machine: *mut Fcitx5LauncherMachine,
    command: u32,
) -> u8 {
    if machine.is_null() {
        return 0;
    }
    let Some(command) = self::command(command) else {
        return 0;
    };
    let machine = unsafe { &mut *machine };
    u8::from(apply_command(machine, command))
}

#[no_mangle]
/// # Safety
///
/// `machine` must point to a valid writable `Fcitx5LauncherMachine`, and
/// `out_decision` must point to writable storage for one
/// `Fcitx5LauncherStartDecision`. Neither pointer is retained.
pub unsafe extern "C" fn fcitx5_launcher_state_request_start(
    machine: *mut Fcitx5LauncherMachine,
    now: u64,
    out_decision: *mut Fcitx5LauncherStartDecision,
) -> i32 {
    if machine.is_null() || out_decision.is_null() {
        return 1;
    }
    let machine = unsafe { &mut *machine };
    let decision = request_start(machine, now);
    unsafe {
        *out_decision = decision;
    }
    0
}

#[no_mangle]
/// # Safety
///
/// `machine` must be either null or point to a valid writable
/// `Fcitx5LauncherMachine`. The pointer is not retained.
pub unsafe extern "C" fn fcitx5_launcher_state_engine_ready(machine: *mut Fcitx5LauncherMachine) {
    if machine.is_null() {
        return;
    }
    let machine = unsafe { &mut *machine };
    if machine.engine_state == EngineState::Starting as u32 {
        machine.engine_state = EngineState::Ready as u32;
    }
}

#[no_mangle]
/// # Safety
///
/// `machine` must be either null or point to a valid writable
/// `Fcitx5LauncherMachine`. The pointer is not retained.
pub unsafe extern "C" fn fcitx5_launcher_state_engine_exited(
    machine: *mut Fcitx5LauncherMachine,
    runtime_ms: u64,
    now: u64,
) {
    if machine.is_null() {
        return;
    }
    let machine = unsafe { &mut *machine };
    engine_exited(machine, runtime_ms, now);
}

#[no_mangle]
/// # Safety
///
/// `machine` must be either null or point to a valid writable
/// `Fcitx5LauncherMachine`. The pointer is not retained.
pub unsafe extern "C" fn fcitx5_launcher_state_engine_stopped_intentionally(
    machine: *mut Fcitx5LauncherMachine,
) {
    if machine.is_null() {
        return;
    }
    let machine = unsafe { &mut *machine };
    machine.engine_state = EngineState::Stopped as u32;
}

#[no_mangle]
pub extern "C" fn fcitx5_launcher_state_is_persistent(state: u32) -> u8 {
    u8::from(launcher_state(state).is_some())
}

#[no_mangle]
/// # Safety
///
/// `path` must be either null with `len == 0`, or point to a valid UTF-16
/// buffer with exactly `len` elements for the duration of this call.
/// `out_snapshot` must point to writable storage when a loaded snapshot is
/// requested. The pointer is not retained.
pub unsafe extern "C" fn fcitx5_launcher_state_store_load_utf16(
    path: *const u16,
    len: usize,
    out_snapshot: *mut Fcitx5LauncherSnapshot,
) -> u32 {
    if out_snapshot.is_null() {
        return 3;
    }
    let Some(path) = path_from_wide(path, len) else {
        return 3;
    };
    match load_snapshot_from_path(&path) {
        Ok(snapshot) => {
            unsafe {
                *out_snapshot = snapshot;
            }
            1
        }
        Err(result) => result,
    }
}

#[no_mangle]
/// # Safety
///
/// `path` must be either null with `len == 0`, or point to a valid UTF-16
/// buffer with exactly `len` elements for the duration of this call. The pointer
/// is not retained.
pub unsafe extern "C" fn fcitx5_launcher_state_store_save_utf16(
    path: *const u16,
    len: usize,
    snapshot: Fcitx5LauncherSnapshot,
) -> u8 {
    let Some(path) = path_from_wide(path, len) else {
        return 0;
    };
    u8::from(save_snapshot_to_path(&path, snapshot))
}

#[no_mangle]
/// # Safety
///
/// `out` must be null or point to writable storage for `capacity` UTF-16 code
/// units. The function returns the required code-unit count, excluding a NUL
/// terminator, and does not retain the pointer.
pub unsafe extern "C" fn fcitx5_launcher_default_state_store_path_utf16(
    out: *mut u16,
    capacity: usize,
) -> usize {
    let Some(path) = default_state_store_path() else {
        return 0;
    };
    write_utf16_path(&path, out, capacity)
}

#[no_mangle]
/// # Safety
///
/// `path` must be null with `len == 0`, or point to a valid UTF-16 buffer with
/// exactly `len` elements for the duration of this call.
pub unsafe extern "C" fn fcitx5_launcher_absolute_windows_path_utf16(
    path: *const u16,
    len: usize,
) -> u8 {
    let Some(path) = wide_slice(path, len) else {
        return 0;
    };
    u8::from(absolute_windows_path_wide(path))
}

#[no_mangle]
/// # Safety
///
/// Input pointers must be null only when their corresponding length is zero, or
/// point to valid UTF-16 buffers with exactly the provided lengths. Output
/// pointers may be null for size queries; otherwise they must point to writable
/// storage with the advertised capacities. Required lengths exclude NUL
/// terminators. No pointer is retained.
pub unsafe extern "C" fn fcitx5_launcher_resolve_default_process_paths_utf16(
    executable_directory: *const u16,
    executable_directory_len: usize,
    generation: *const u16,
    generation_len: usize,
    engine_output: *mut u16,
    engine_capacity: usize,
    ui_output: *mut u16,
    ui_capacity: usize,
    required_engine_len: *mut usize,
    required_ui_len: *mut usize,
) -> u8 {
    let Some(executable_directory) = path_from_wide(executable_directory, executable_directory_len)
    else {
        return 0;
    };
    let Some(generation) = os_string_from_wide(generation, generation_len) else {
        return 0;
    };
    let (engine, ui) = resolve_default_process_paths(&executable_directory, &generation);
    let engine_len = write_utf16_path_checked(&engine, engine_output, engine_capacity);
    let ui_len = write_utf16_path_checked(&ui, ui_output, ui_capacity);
    if let Some(engine_len) = engine_len {
        if !required_engine_len.is_null() {
            unsafe {
                *required_engine_len = engine_len;
            }
        }
    }
    if let Some(ui_len) = ui_len {
        if !required_ui_len.is_null() {
            unsafe {
                *required_ui_len = ui_len;
            }
        }
    }
    u8::from(engine_len.is_some() && ui_len.is_some())
}

#[no_mangle]
pub extern "C" fn fcitx5_launcher_tray_status_text_utf16(
    launcher_state: u32,
    engine_state: u32,
    chinese: u8,
    output: *mut u16,
    capacity: usize,
) -> usize {
    write_utf16_string(
        tray_status_text(launcher_state, engine_state, chinese != 0),
        output,
        capacity,
    )
}

#[no_mangle]
/// # Safety
///
/// Non-null input pointers must point to valid byte buffers with exactly the
/// provided lengths. `output` may be null for size queries or writable UTF-16
/// storage for `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_launcher_tray_input_method_display_utf16(
    native_name: *const u8,
    native_name_len: usize,
    name: *const u8,
    name_len: usize,
    id: *const u8,
    id_len: usize,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let display =
        input_method_display_from_raw(native_name, native_name_len, name, name_len, id, id_len);
    write_utf16_string(&display, output, capacity)
}

#[no_mangle]
/// # Safety
///
/// `product_name` must be null only when `product_name_len == 0`, or point to a
/// valid UTF-16 buffer with exactly that length. Byte-string pointers follow the
/// same lifetime rule as `fcitx5_launcher_tray_input_method_display_utf16`.
/// `output` may be null for size queries or writable UTF-16 storage for
/// `capacity` code units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_launcher_tray_tooltip_utf16(
    product_name: *const u16,
    product_name_len: usize,
    launcher_state: u32,
    engine_state: u32,
    chinese: u8,
    native_name: *const u8,
    native_name_len: usize,
    name: *const u8,
    name_len: usize,
    id: *const u8,
    id_len: usize,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(product_name) = utf16_string_from_raw(product_name, product_name_len) else {
        return 0;
    };
    let status = tray_status_text(launcher_state, engine_state, chinese != 0);
    let display =
        input_method_display_from_raw(native_name, native_name_len, name, name_len, id, id_len);
    let tooltip = if display.is_empty() {
        format!("{product_name} — {status}")
    } else {
        format!("{product_name} — {status} — {display}")
    };
    write_utf16_string(&tooltip, output, capacity)
}

#[no_mangle]
/// # Safety
///
/// Input pointers must be null only when their corresponding length is zero, or
/// point to valid UTF-16 buffers with exactly the provided lengths. `output` may
/// be null for size queries or writable UTF-16 storage for `capacity` code
/// units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_launcher_engine_command_utf16(
    engine_path: *const u16,
    engine_path_len: usize,
    ready_event: *const u16,
    ready_event_len: usize,
    stop_event: *const u16,
    stop_event_len: usize,
    generation: *const u16,
    generation_len: usize,
    safe_mode: u8,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(engine_path) = os_string_from_wide(engine_path, engine_path_len) else {
        return 0;
    };
    let Some(ready_event) = os_string_from_wide(ready_event, ready_event_len) else {
        return 0;
    };
    let Some(stop_event) = os_string_from_wide(stop_event, stop_event_len) else {
        return 0;
    };
    let Some(generation) = os_string_from_wide(generation, generation_len) else {
        return 0;
    };
    let command = launcher_engine_command(
        &engine_path,
        &ready_event,
        &stop_event,
        &generation,
        safe_mode != 0,
    );
    write_utf16_units(&command, output, capacity)
}

#[no_mangle]
/// # Safety
///
/// Input pointers must be null only when their corresponding length is zero, or
/// point to valid UTF-16 buffers with exactly the provided lengths. `output` may
/// be null for size queries or writable UTF-16 storage for `capacity` code
/// units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_launcher_ui_command_utf16(
    ui_path: *const u16,
    ui_path_len: usize,
    parent_pid: u32,
    generation: *const u16,
    generation_len: usize,
    safe_mode: u8,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(ui_path) = os_string_from_wide(ui_path, ui_path_len) else {
        return 0;
    };
    let Some(generation) = os_string_from_wide(generation, generation_len) else {
        return 0;
    };
    let command = launcher_ui_command(&ui_path, parent_pid, &generation, safe_mode != 0);
    write_utf16_units(&command, output, capacity)
}

#[no_mangle]
/// # Safety
///
/// Input pointers must be null only when their corresponding length is zero, or
/// point to valid UTF-16 buffers with exactly the provided lengths. `output` may
/// be null for size queries or writable UTF-16 storage for `capacity` code
/// units. No pointer is retained.
pub unsafe extern "C" fn fcitx5_launcher_config_command_utf16(
    config_path: *const u16,
    config_path_len: usize,
    arguments: *const u16,
    arguments_len: usize,
    output: *mut u16,
    capacity: usize,
) -> usize {
    let Some(config_path) = os_string_from_wide(config_path, config_path_len) else {
        return 0;
    };
    let Some(arguments) = os_string_from_wide(arguments, arguments_len) else {
        return 0;
    };
    let command = launcher_config_command(&config_path, &arguments);
    write_utf16_units(&command, output, capacity)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(
        state: LauncherState,
        crashes: u32,
        next_start: u64,
        engine: EngineState,
    ) -> Fcitx5LauncherMachine {
        Fcitx5LauncherMachine {
            snapshot: Fcitx5LauncherSnapshot {
                state: state as u32,
                consecutive_startup_crashes: crashes,
                next_start_allowed_milliseconds: next_start,
            },
            engine_state: engine as u32,
        }
    }

    #[test]
    fn crash_backoff_safe_mode_and_stable_reset_match_frozen_launcher_contract() {
        let mut machine = machine(LauncherState::Normal, 0, 0, EngineState::Stopped);
        assert_eq!(
            request_start(&mut machine, 0).disposition,
            StartDisposition::Start as u32
        );
        assert_eq!(
            request_start(&mut machine, 0).disposition,
            StartDisposition::AlreadyActive as u32
        );
        machine.engine_state = EngineState::Ready as u32;
        engine_exited(&mut machine, 1, 0);
        assert_eq!(machine.snapshot.state, LauncherState::CrashBackoff as u32);
        assert_eq!(machine.snapshot.consecutive_startup_crashes, 1);
        assert_eq!(
            request_start(&mut machine, 0).retry_after_milliseconds,
            INITIAL_BACKOFF_MS
        );
        assert_eq!(
            request_start(&mut machine, 250).disposition,
            StartDisposition::Start as u32
        );
        engine_exited(&mut machine, 2, 250);
        assert_eq!(
            request_start(&mut machine, 250).retry_after_milliseconds,
            500
        );
        assert_eq!(
            request_start(&mut machine, 750).disposition,
            StartDisposition::Start as u32
        );
        engine_exited(&mut machine, 3, 750);
        assert_eq!(machine.snapshot.state, LauncherState::SafeMode as u32);
        assert_eq!(request_start(&mut machine, 750).safe_mode, 1);
        engine_exited(&mut machine, STABLE_RUNTIME_MS, 750);
        assert_eq!(machine.snapshot.state, LauncherState::Normal as u32);
        assert_eq!(machine.snapshot.consecutive_startup_crashes, 0);
    }

    #[test]
    fn persisted_snapshots_are_normalized_on_init() {
        let crash = normalize_snapshot(
            Fcitx5LauncherSnapshot {
                state: LauncherState::CrashBackoff as u32,
                consecutive_startup_crashes: 0,
                next_start_allowed_milliseconds: 0,
            },
            1000,
        )
        .expect("crash snapshot should normalize");
        assert_eq!(crash.consecutive_startup_crashes, 1);
        assert_eq!(crash.next_start_allowed_milliseconds, 1250);

        let safe = normalize_snapshot(
            Fcitx5LauncherSnapshot {
                state: LauncherState::SafeMode as u32,
                consecutive_startup_crashes: 1,
                next_start_allowed_milliseconds: 9999,
            },
            1000,
        )
        .expect("safe-mode snapshot should normalize");
        assert_eq!(safe.consecutive_startup_crashes, SAFE_MODE_CRASH_THRESHOLD);
        assert_eq!(safe.next_start_allowed_milliseconds, 0);
    }

    #[test]
    fn state_store_parser_matches_frozen_cpp_ledger_contract() {
        let legacy = parse_snapshot("user-stopped\n").expect("legacy v1 state should parse");
        assert_eq!(legacy.state, LauncherState::UserStopped as u32);
        assert_eq!(legacy.consecutive_startup_crashes, 0);
        assert_eq!(legacy.next_start_allowed_milliseconds, 0);

        let v2 = parse_snapshot(
            "format_version=2\r\nstate=safe-mode\r\nconsecutive_startup_crashes=3\r\nnext_start_allowed_ms=0\r\n",
        )
        .expect("v2 CRLF ledger should parse");
        assert_eq!(v2.state, LauncherState::SafeMode as u32);
        assert_eq!(v2.consecutive_startup_crashes, 3);

        assert!(
            parse_snapshot("format_version=2\nstate=safe-mode\nconsecutive_startup_crashes=")
                .is_none()
        );
        assert!(parse_snapshot(
            "format_version=2\nstate=safe-mode\nconsecutive_startup_crashes=4294967296\nnext_start_allowed_ms=0\n",
        )
        .is_none());
        assert!(parse_snapshot(
            "format_version=2\nstate=unknown\nconsecutive_startup_crashes=0\nnext_start_allowed_ms=0\n",
        )
        .is_none());
    }

    #[test]
    fn state_store_save_load_and_publish_match_frozen_cpp_contract() {
        let directory = std::env::temp_dir().join(format!(
            "fcitx5-launcher-core-state-store-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).expect("test directory should be created");
        let state_path = directory.join("launcher-state.v1");

        assert_eq!(load_snapshot_from_path(&state_path), Err(0));

        let snapshot = Fcitx5LauncherSnapshot {
            state: LauncherState::CrashBackoff as u32,
            consecutive_startup_crashes: 2,
            next_start_allowed_milliseconds: 500,
        };
        assert!(save_snapshot_to_path(&state_path, snapshot));
        assert_eq!(load_snapshot_from_path(&state_path), Ok(snapshot));

        fs::write(
            &state_path,
            b"format_version=2\nstate=safe-mode\nconsecutive_startup_crashes=",
        )
        .expect("corrupt ledger should be written");
        assert_eq!(load_snapshot_from_path(&state_path), Err(2));

        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn launcher_default_process_paths_match_cpp_generation_contract() {
        let root = std::env::temp_dir().join(format!(
            "fcitx5-launcher-core-default-paths-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let bin = root.join("bin");
        let generation_bin = root.join("runtime").join("g1").join("bin");
        fs::create_dir_all(&bin).expect("bin directory should be created");
        fs::create_dir_all(&generation_bin).expect("generation bin should be created");

        let (engine, ui) = resolve_default_process_paths(&bin, OsStr::new("g1"));
        assert_eq!(engine, bin.join("fcitx5-engine.exe"));
        assert_eq!(ui, bin.join("fcitx5-ui.exe"));

        fs::write(generation_bin.join("fcitx5-engine.exe"), b"engine")
            .expect("generation engine should be created");
        fs::write(generation_bin.join("fcitx5-ui.exe"), b"ui")
            .expect("generation ui should be created");
        let (engine, ui) = resolve_default_process_paths(&bin, OsStr::new("g1"));
        assert_eq!(engine, generation_bin.join("fcitx5-engine.exe"));
        assert_eq!(ui, generation_bin.join("fcitx5-ui.exe"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn absolute_windows_path_matches_cpp_contract() {
        let wide = |value: &str| value.encode_utf16().collect::<Vec<_>>();
        assert!(absolute_windows_path_wide(&wide(r"C:\Fcitx5\bin")));
        assert!(absolute_windows_path_wide(&wide(r"d:/Fcitx5/bin")));
        assert!(!absolute_windows_path_wide(&wide(r"\Fcitx5\bin")));
        assert!(!absolute_windows_path_wide(&wide(r"Fcitx5\bin")));
        assert!(!absolute_windows_path_wide(&wide(r"1:\Fcitx5\bin")));
    }

    #[test]
    fn tray_text_and_tooltip_match_cpp_display_contract() {
        assert_eq!(
            tray_status_text(
                LauncherState::SafeMode as u32,
                EngineState::Ready as u32,
                false
            ),
            "Safe mode"
        );
        assert_eq!(
            tray_status_text(
                LauncherState::Normal as u32,
                EngineState::Ready as u32,
                true
            ),
            "运行中"
        );
        assert_eq!(
            tray_status_text(
                LauncherState::Normal as u32,
                EngineState::Stopped as u32,
                false
            ),
            "Service stopped"
        );

        let native = "五笔".as_bytes();
        let name = "Wubi".as_bytes();
        let id = "rime-wubi".as_bytes();
        assert_eq!(
            input_method_display_from_raw(
                native.as_ptr(),
                native.len(),
                name.as_ptr(),
                name.len(),
                id.as_ptr(),
                id.len(),
            ),
            "五笔"
        );
        let invalid = [0xff_u8];
        assert_eq!(
            input_method_display_from_raw(
                invalid.as_ptr(),
                invalid.len(),
                name.as_ptr(),
                name.len(),
                id.as_ptr(),
                id.len(),
            ),
            "Wubi"
        );

        let product = "Fcitx5 for Windows Next".encode_utf16().collect::<Vec<_>>();
        let required = unsafe {
            fcitx5_launcher_tray_tooltip_utf16(
                product.as_ptr(),
                product.len(),
                LauncherState::Normal as u32,
                EngineState::Ready as u32,
                0,
                native.as_ptr(),
                native.len(),
                name.as_ptr(),
                name.len(),
                id.as_ptr(),
                id.len(),
                std::ptr::null_mut(),
                0,
            )
        };
        let mut output = vec![0_u16; required];
        let written = unsafe {
            fcitx5_launcher_tray_tooltip_utf16(
                product.as_ptr(),
                product.len(),
                LauncherState::Normal as u32,
                EngineState::Ready as u32,
                0,
                native.as_ptr(),
                native.len(),
                name.as_ptr(),
                name.len(),
                id.as_ptr(),
                id.len(),
                output.as_mut_ptr(),
                output.len(),
            )
        };
        assert_eq!(written, required);
        assert_eq!(
            String::from_utf16(&output).expect("tooltip should be valid UTF-16"),
            "Fcitx5 for Windows Next — Running — 五笔"
        );
    }

    #[test]
    fn launcher_child_command_lines_match_cpp_contract() {
        let to_string = |value: Vec<u16>| {
            String::from_utf16(&value).expect("command line should be UTF-16 text")
        };
        assert_eq!(
            to_string(launcher_engine_command(
                OsStr::new(r"C:\Fcitx5\bin\fcitx5-engine.exe"),
                OsStr::new("ready"),
                OsStr::new("stop"),
                OsStr::new("g1"),
                true,
            )),
            r#""C:\Fcitx5\bin\fcitx5-engine.exe" --ready-event "ready" --stop-event "stop" --generation "g1" --safe-mode"#
        );
        assert_eq!(
            to_string(launcher_ui_command(
                OsStr::new(r"C:\Fcitx5\bin\fcitx5-ui.exe"),
                42,
                OsStr::new("g1"),
                false,
            )),
            r#""C:\Fcitx5\bin\fcitx5-ui.exe" --parent-pid 42 --generation "g1""#
        );
        assert_eq!(
            to_string(launcher_config_command(
                OsStr::new(r"C:\Fcitx5\bin\fcitx5-config.exe"),
                OsStr::new("--diagnostics"),
            )),
            r#""C:\Fcitx5\bin\fcitx5-config.exe" --diagnostics"#
        );
    }
}
