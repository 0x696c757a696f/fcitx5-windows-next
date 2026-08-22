#![deny(unsafe_op_in_unsafe_fn)]

const STARTUP_CRASH_WINDOW_MS: u64 = 10_000;
const STABLE_RUNTIME_MS: u64 = 60_000;
const INITIAL_BACKOFF_MS: u64 = 250;
const MAXIMUM_BACKOFF_MS: u64 = 30_000;
const SAFE_MODE_CRASH_THRESHOLD: u32 = 3;

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
}
