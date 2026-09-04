#![forbid(unsafe_code)]

//! Real-Fcitx engine E2E acceptance harness — Rust port of the deleted C++
//! `fcitx_engine_integration_test.cpp` (078 engine-E2E slice).
//!
//! This is the retained final *mixed-binary* E2E: it drives the REAL shipping
//! `fcitx5-engine.exe` (C++ Fcitx core/addons) and the REAL `fcitx5-ui.exe`
//! (C++ renderer host) through the frozen named-pipe protocol using the Safe
//! Rust [`fcitx5_ipc_client::EngineClient`]. Every scenario and assertion of
//! the deleted C++ acceptance is preserved verbatim in behaviour; the release
//! gate in `tools/test-fcitx.ps1` invokes this binary exactly as it invoked
//! the C++ one.
//!
//! Scenarios (single flag each, plus the shared baseline flow):
//! * baseline                 — pinyin key drive, candidate UI select, navigation
//! * `--safe-mode`            — engine started in safe mode
//! * `--first-run-rime`       — Rime first context isolation
//! * `--rime-lua`             — Rime Lua translator probe candidate (implies rime)
//! * `--typing-fuzz`          — reconnect + 4000-iteration stateful typing fuzz
//! * `--chttrans`             — Ctrl+Shift+F toggle, pinyin "shu", 書 commit

use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use fcitx5_ipc_client::{EngineClient, KeyOutcome};
use fcitx5_windows_common_core::{
    current_runtime_generation_for_current_process, deadline_after, deadline_has_time_remaining,
    process_cpu_time_100ns, process_private_usage_bytes, CurrentUserRuntimeIdentity, NamedEvent,
};

// Flat protocol-core key flag and budget constants (mirrors the C++ test).
const KEY_FLAG_SHIFT: u32 = 1 << 0;
const KEY_FLAG_CONTROL: u32 = 1 << 1;
const KEY_FLAG_RELEASE: u32 = 1 << 4;
const MAX_PREEDIT_UTF8: usize = 16 * 1024;
const MAX_COMMIT_UTF8: usize = 16 * 1024;
const MAX_CANDIDATES: usize = 128;

const VK_SPACE: u32 = 0x20;
const VK_BACK: u32 = 0x08;
const VK_RETURN: u32 = 0x0D;
const VK_ESCAPE: u32 = 0x1B;
const VK_LEFT: u32 = 0x25;
const VK_RIGHT: u32 = 0x27;
const VK_UP: u32 = 0x26;
const VK_DOWN: u32 = 0x28;
const VK_PRIOR: u32 = 0x21;
const VK_NEXT: u32 = 0x22;
const VK_OEM_1: u32 = 0xBA; // ';'
const VK_OEM_7: u32 = 0xDE; // '\''
const VK_SHIFT: u32 = 0x10;

#[derive(Default)]
struct Flags {
    safe_mode: bool,
    first_run_rime: bool,
    rime_lua: bool,
    typing_fuzz: bool,
    chttrans: bool,
}

fn parse_flags(arguments: &[String]) -> Option<Flags> {
    let mut flags = Flags::default();
    for argument in arguments {
        match argument.as_str() {
            "--safe-mode" => flags.safe_mode = true,
            "--first-run-rime" => flags.first_run_rime = true,
            "--rime-lua" => {
                flags.first_run_rime = true;
                flags.rime_lua = true;
            }
            "--typing-fuzz" => flags.typing_fuzz = true,
            "--chttrans" => flags.chttrans = true,
            _ => return None,
        }
    }
    if flags.safe_mode && flags.first_run_rime {
        return None;
    }
    Some(flags)
}

/// A spawned real-engine process plus its ready/stop kernel events, mirroring
/// the C++ `Process` RAII (signal stop on drop, wait, then terminate).
struct EngineProcess {
    child: Child,
    stop: Option<NamedEvent>,
}

fn identity_security() -> Result<CurrentUserRuntimeIdentity, String> {
    CurrentUserRuntimeIdentity::current()
        .ok_or_else(|| "current user runtime identity could not be resolved".to_owned())
}

fn start_engine(
    engine: &Path,
    sequence: u32,
    safe_mode: bool,
    test_clients: u32,
    stderr_file: Option<File>,
) -> Result<EngineProcess, String> {
    let identity = identity_security()?;
    let security = identity
        .security_attributes()
        .ok_or_else(|| "security attributes unavailable".to_owned())?;
    let pid = std::process::id();
    let ready_name = OsString::from(format!(
        "Local\\Fcitx5WindowsNext.RealEngine.Ready.{pid}.{sequence}"
    ));
    let ready = NamedEvent::create(&ready_name, &security)
        .map_err(|error| format!("create ready event failed: {error}"))?;
    let stop = if test_clients == 0 {
        let stop_name = OsString::from(format!(
            "Local\\Fcitx5WindowsNext.RealEngine.Stop.{pid}.{sequence}"
        ));
        Some(
            NamedEvent::create(&stop_name, &security)
                .map_err(|error| format!("create stop event failed: {error}"))?,
        )
    } else {
        None
    };

    use std::os::windows::process::CommandExt;
    let mut command = Command::new(engine);
    command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    if test_clients == 0 {
        let stop_name = OsString::from(format!(
            "Local\\Fcitx5WindowsNext.RealEngine.Stop.{pid}.{sequence}"
        ));
        command.arg("--stop-event").arg(stop_name);
    } else {
        command.arg("--test-clients").arg(test_clients.to_string());
    }
    command.arg("--ready-event").arg(ready_name);
    if safe_mode {
        command.arg("--safe-mode");
    }
    if let Some(stderr) = stderr_file {
        command.stderr(stderr);
    }
    let child = command
        .spawn()
        .map_err(|error| format!("real engine creation failed: {error}"))?;

    let mut engine = EngineProcess { child, stop };
    let deadline = deadline_after(30_000);
    while !ready.is_signaled() && deadline_has_time_remaining(deadline) {
        if engine.exited() {
            return Err("real engine exited before signalling readiness".to_owned());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    if !ready.is_signaled() {
        return Err("real engine readiness timed out".to_owned());
    }
    Ok(engine)
}

impl EngineProcess {
    fn exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    fn request_stop(&self) {
        if let Some(stop) = &self.stop {
            let _ = stop.signal();
        }
    }

    /// Waits up to `timeout` for a clean exit (signal stop first when a stop
    /// event exists). Kills on timeout. Returns the exit success.
    fn stop_and_wait(&mut self, timeout: Duration) -> bool {
        self.request_stop();
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return status.success(),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(None) | Err(_) => {
                    let _ = self.child.kill();
                    let _ = self.child.wait();
                    return false;
                }
            }
        }
    }
}

/// Mirrors the C++ engine-start + idle-settle probe: waits for a steady idle
/// (a 100 ms window with <= 5 ms process CPU), rejects busy-loop engines, and
/// samples private memory.
fn start_and_settle(
    engine: &Path,
    sequence: u32,
    safe_mode: bool,
    test_clients: u32,
    stderr_file: Option<File>,
    first_run_rime: bool,
) -> Result<EngineProcess, String> {
    let mut process = start_engine(engine, sequence, safe_mode, test_clients, stderr_file)?;
    let startup = Instant::now();
    let pid = process.child.id();
    let required_quiet = if first_run_rime { 20 } else { 3 };
    let maximum_samples = if first_run_rime { 1_200 } else { 300 };
    let mut quiet = 0_u32;
    let initial = process_cpu_time_100ns(pid).ok_or("initial cpu query failed")?;
    let settle_begin = Instant::now();
    let mut peak_cpu = 0_u64;
    let mut total_cpu = 0_u64;
    let mut sampled = 0_u32;
    let mut busy = 0_u32;
    let mut last_kernel = initial.0;
    let mut last_user = initial.1;
    for _ in 0..maximum_samples {
        std::thread::sleep(Duration::from_millis(100));
        if process.exited() {
            return Err("engine exited during idle settle".to_owned());
        }
        let Some(current) = process_cpu_time_100ns(pid) else {
            return Err("engine cpu query failed during idle settle".to_owned());
        };
        let cpu_100ns = (current.0 - last_kernel) + (current.1 - last_user);
        peak_cpu = peak_cpu.max(cpu_100ns);
        total_cpu += cpu_100ns;
        sampled += 1;
        if cpu_100ns > 50_000 {
            busy += 1;
        }
        quiet = if cpu_100ns <= 50_000 { quiet + 1 } else { 0 };
        last_kernel = current.0;
        last_user = current.1;
        if quiet >= required_quiet {
            break;
        }
    }
    if quiet < required_quiet {
        return Err(format!(
            "engine did not reach a steady idle state peak-cpu-us={} avg-cpu-us={} busy-windows={}/{}",
            peak_cpu / 10,
            if sampled == 0 { 0 } else { total_cpu / u64::from(sampled) / 10 },
            busy,
            sampled
        ));
    }
    let settle = settle_begin.elapsed();
    let private_kib = process_private_usage_bytes(pid).unwrap_or(0) / 1024;
    // 1 s idle CPU cap: <= 50 ms of process CPU in that second.
    std::thread::sleep(Duration::from_millis(1000));
    let Some(after) = process_cpu_time_100ns(pid) else {
        return Err("engine cpu query after settle failed".to_owned());
    };
    let idle_cpu_100ns = (after.0 - last_kernel) + (after.1 - last_user);
    if idle_cpu_100ns > 500_000 {
        return Err(format!(
            "engine idle CPU exceeded 50 ms per second: {} us",
            idle_cpu_100ns / 10
        ));
    }
    println!(
        "engine-startup-ms={} idle-cpu-us={} settle-ms={} private-kib={}",
        startup.elapsed().as_millis(),
        idle_cpu_100ns / 10,
        settle.as_millis(),
        private_kib
    );
    Ok(process)
}

fn utf8(value: &[u8]) -> String {
    String::from_utf8_lossy(value).into_owned()
}

fn local_engine_pipe(identity: &CurrentUserRuntimeIdentity) -> Result<OsString, String> {
    let generation = current_runtime_generation_for_current_process();
    identity
        .local_endpoint_name(&generation, "engine")
        .ok_or_else(|| "engine endpoint name unavailable".to_owned())
}

fn connect_engine(
    identity: &CurrentUserRuntimeIdentity,
    engine_path: &Path,
) -> Result<EngineClient, String> {
    let pipe = local_engine_pipe(identity)?;
    EngineClient::new(pipe, engine_path.to_path_buf())
        .ok_or_else(|| "engine client could not be created".to_owned())
}

/// The shared baseline flow every scenario runs first (C++ order preserved).
fn run_baseline(client: &mut EngineClient, engine_path: &Path) -> Result<(), String> {
    let context_id = 0x3141_5926_u64;
    let first = client
        .process_key(context_id, b'N' as u32, 0, 0)
        .ok_or("first pinyin key transport failed")?;
    if !first.handled || first.preedit != b"n" || !first.commit.is_empty() {
        return Err(format!(
            "first pinyin key failed: handled={} preedit={} commit={}",
            first.handled,
            utf8(&first.preedit),
            utf8(&first.commit)
        ));
    }
    let second = client
        .process_key(context_id, b'I' as u32, 0, 0)
        .ok_or("second pinyin key transport failed")?;
    if !second.handled
        || second.preedit != b"ni"
        || !second.commit.is_empty()
        || second.candidates.is_empty()
        || second.candidate_visibility != 1
    {
        return Err(format!(
            "second pinyin key failed: preedit={} candidates={} visibility={}",
            utf8(&second.preedit),
            second.candidates.len(),
            second.candidate_visibility
        ));
    }
    let commit = client
        .process_key(context_id, VK_SPACE, 0, 0)
        .ok_or("pinyin commit transport failed")?;
    if !commit.handled || commit.commit.is_empty() || !commit.preedit.is_empty() {
        return Err(format!(
            "pinyin commit failed: commit={} preedit={}",
            utf8(&commit.commit),
            utf8(&commit.preedit)
        ));
    }
    run_candidate_ui_and_navigation(client, engine_path)?;
    Ok(())
}

/// Candidate UI cross-process select + state poll + navigation keys.
fn run_candidate_ui_and_navigation(
    client: &mut EngineClient,
    engine_path: &Path,
) -> Result<(), String> {
    let candidate_context = 0x4341_4E44_4944_4154_u64;
    client
        .process_key(candidate_context, b'N' as u32, 0, 0)
        .ok_or("candidate setup N transport failed")?;
    let i = client
        .process_key(candidate_context, b'I' as u32, 0, 0)
        .ok_or("candidate setup I transport failed")?;
    if i.candidates.is_empty() || i.composition_id == 0 || i.revision == 0 {
        return Err("candidate selection setup failed".to_owned());
    }
    let engine_epoch = i.engine_epoch;
    let composition_id = i.composition_id;
    let revision = i.revision;
    let candidate_id = i.candidates[0].id;

    // The real fcitx5-ui.exe selects the candidate through the Rust opaque
    // candidate-select client; the engine then signals the UI notification
    // event (candidate-{pid}) that this harness waits on.
    let identity = identity_security()?;
    let generation = current_runtime_generation_for_current_process();
    let notification_name = identity
        .local_object_name(&generation, &format!("candidate-{}", std::process::id()))
        .ok_or("notification object name unavailable")?;
    let security = identity
        .security_attributes()
        .ok_or("security attributes unavailable")?;
    let notification = NamedEvent::create(&notification_name, &security)
        .map_err(|error| format!("create notification event failed: {error}"))?;

    let ui = engine_path
        .parent()
        .map(|directory| directory.join("fcitx5-ui.exe"))
        .ok_or("engine parent directory unavailable")?;
    use std::os::windows::process::CommandExt;
    let mut ui_child = Command::new(&ui)
        .arg("--candidate-select-test")
        .arg(engine_path)
        .arg(std::process::id().to_string())
        .arg(engine_epoch.to_string())
        .arg(candidate_context.to_string())
        .arg(composition_id.to_string())
        .arg(revision.to_string())
        .arg(candidate_id.to_string())
        .creation_flags(0x0800_0000)
        .spawn()
        .map_err(|error| format!("ui spawn failed: {error}"))?;
    let ui_exit_ok = ui_child
        .wait()
        .map_err(|error| format!("ui wait failed: {error}"))?
        .success();
    let notified = {
        let deadline = deadline_after(2_000);
        while !notification.is_signaled() && deadline_has_time_remaining(deadline) {
            std::thread::sleep(Duration::from_millis(5));
        }
        notification.is_signaled()
    };
    if !ui_exit_ok || !notified {
        return Err(
            "semantic candidate selection did not commit through Engine and TSF state".to_owned(),
        );
    }
    let polled = client
        .poll_state(candidate_context)
        .ok_or("candidate poll state failed")?;
    if polled.commit.is_empty() || !polled.preedit.is_empty() {
        return Err("polled candidate commit missing".to_owned());
    }

    let nav_context = 0x4E41_5649_4741_5445_u64;
    let type_ni = |client: &mut EngineClient| -> Result<KeyOutcome, String> {
        client
            .process_key(nav_context, b'N' as u32, 0, 0)
            .ok_or_else(|| "nav N transport failed".to_owned())?;
        client
            .process_key(nav_context, b'I' as u32, 0, 0)
            .ok_or_else(|| "nav I transport failed".to_owned())
    };
    let ni = type_ni(client)?;
    if !ni.handled || ni.candidates.len() < 3 || ni.selected_candidate == u32::MAX {
        return Err("navigation setup failed (need >= 3 candidates)".to_owned());
    }
    let second_candidate = ni.candidates[1].text_utf8.clone();
    let number_two = client
        .process_key(nav_context, b'2' as u32, 0, 0)
        .ok_or("number 2 transport failed")?;
    if !number_two.handled || number_two.commit != second_candidate {
        return Err("number 2 committed a candidate other than its label".to_owned());
    }
    let ni = type_ni(client)?;
    if !ni.handled || ni.candidates.len() < 2 {
        return Err(format!(
            "semicolon setup failed: preedit={} candidates={}",
            utf8(&ni.preedit),
            ni.candidates.len()
        ));
    }
    let semicolon_candidate = ni.candidates[1].text_utf8.clone();
    let semicolon = client
        .process_key(nav_context, VK_OEM_1, 0, 0)
        .ok_or("semicolon transport failed")?;
    if !semicolon.handled || semicolon.commit != semicolon_candidate {
        return Err("semicolon did not select the 2nd candidate".to_owned());
    }
    let ni = type_ni(client)?;
    if !ni.handled {
        return Err("apostrophe setup failed".to_owned());
    }
    let apostrophe = client
        .process_key(nav_context, VK_OEM_7, 0, 0)
        .ok_or("apostrophe transport failed")?;
    if !apostrophe.handled || apostrophe.commit.is_empty() {
        return Err("apostrophe did not select the 3rd candidate".to_owned());
    }
    let ni = type_ni(client)?;
    if !ni.handled
        || ni.selected_candidate == u32::MAX
        || ni.selected_candidate as usize + 1 >= ni.candidates.len()
    {
        return Err("left/right setup failed".to_owned());
    }
    let focus_before = ni.selected_candidate;
    let right = client
        .process_key(nav_context, VK_RIGHT, 0, 0)
        .ok_or("right transport failed")?;
    if !right.handled
        || !right.commit.is_empty()
        || right.selected_candidate == focus_before
        || right.selected_candidate as usize >= right.candidates.len()
    {
        return Err(format!(
            "Right did not advance the highlight without committing: {} -> {} commit={}",
            focus_before,
            right.selected_candidate,
            utf8(&right.commit)
        ));
    }
    let focus_after_right = right.selected_candidate;
    let left = client
        .process_key(nav_context, VK_LEFT, 0, 0)
        .ok_or("left transport failed")?;
    if !left.handled || !left.commit.is_empty() || left.selected_candidate != focus_before {
        return Err("Left did not restore the highlight without committing".to_owned());
    }
    let right_again = client
        .process_key(nav_context, VK_RIGHT, 0, 0)
        .ok_or("right-again transport failed")?;
    if !right_again.handled
        || right_again.selected_candidate != focus_after_right
        || right_again.selected_candidate as usize >= right_again.candidates.len()
    {
        return Err("Right did not prepare an Enter-selectable highlight".to_owned());
    }
    let highlighted = right_again.candidates[right_again.selected_candidate as usize]
        .text_utf8
        .clone();
    let enter = client
        .process_key(nav_context, VK_RETURN, 0, 0)
        .ok_or("enter transport failed")?;
    if !enter.handled || enter.commit != highlighted || !enter.preedit.is_empty() {
        return Err("Enter committed a candidate other than the Left/Right highlight".to_owned());
    }
    Ok(())
}

/// Engine-side switch hotkeys through the routing layer (every scenario).
fn run_hotkeys(client: &mut EngineClient) -> Result<(), String> {
    let hotkey_context = 0x484F_544B_4559_53_u64;
    let ctrl = KEY_FLAG_CONTROL;
    let ctrl_release = ctrl | KEY_FLAG_RELEASE;
    let toggle = client
        .process_key(hotkey_context, VK_SPACE, ctrl, 0)
        .ok_or("Ctrl+Space toggle hotkey transport failed")?;
    if !toggle.handled {
        return Err("Ctrl+Space toggle hotkey was not handled".to_owned());
    }
    let passthrough = client
        .process_key(hotkey_context, b'N' as u32, 0, 0)
        .ok_or("passthrough probe transport failed")?;
    if passthrough.handled || !passthrough.preedit.is_empty() {
        return Err(format!(
            "toggle did not reach keyboard passthrough: preedit={}",
            utf8(&passthrough.preedit)
        ));
    }
    let _ = client.process_key(hotkey_context, VK_SPACE, ctrl_release, 0);
    let still_passthrough = client
        .process_key(hotkey_context, b'N' as u32, 0, 0)
        .ok_or("passthrough-after-release probe transport failed")?;
    if still_passthrough.handled {
        return Err("key-up of Ctrl+Space changed the input method".to_owned());
    }
    let restore = client
        .process_key(hotkey_context, VK_SPACE, ctrl, 0)
        .ok_or("Ctrl+Space restore transport failed")?;
    if !restore.handled {
        return Err("Ctrl+Space restore hotkey was not handled".to_owned());
    }
    let ctrl_shift = KEY_FLAG_CONTROL | KEY_FLAG_SHIFT;
    let next = client
        .process_key(hotkey_context, VK_SHIFT, ctrl_shift, 0)
        .ok_or("Ctrl+Shift next transport failed")?;
    if !next.handled {
        return Err(format!(
            "Ctrl+Shift next hotkey was not handled: handled={} preedit={} commit={} candidates={} selected={}",
            next.handled,
            utf8(&next.preedit),
            utf8(&next.commit),
            next.candidates.len(),
            next.selected_candidate
        ));
    }
    Ok(())
}

fn run_context_isolation(client: &mut EngineClient, first_run_rime: bool) -> Result<(), String> {
    let context_id = 0x3141_5926_u64;
    let second_context = 0x2718_2818_u64;
    if first_run_rime {
        let h = client
            .process_key(context_id, b'H' as u32, 0, 0)
            .ok_or("rime first context transport failed")?;
        if h.preedit != b"h" {
            return Err(format!(
                "Rime first context start failed: {}",
                utf8(&h.preedit)
            ));
        }
        let n = client
            .process_key(second_context, b'N' as u32, 0, 0)
            .ok_or("rime second context transport failed")?;
        if n.preedit != b"n" {
            return Err(format!(
                "Rime second context start failed: {}",
                utf8(&n.preedit)
            ));
        }
        let back = client
            .process_key(context_id, VK_BACK, 0, 0)
            .ok_or("rime backspace transport failed")?;
        if !back.preedit.is_empty() {
            return Err(format!(
                "Rime first context clear failed: {}",
                utf8(&back.preedit)
            ));
        }
        let resume = client
            .process_key(second_context, b'I' as u32, 0, 0)
            .ok_or("rime resume transport failed")?;
        if resume.preedit != b"i" {
            return Err(format!(
                "Rime second context resume failed: {}",
                utf8(&resume.preedit)
            ));
        }
    } else {
        let h = client
            .process_key(second_context, b'H' as u32, 0, 0)
            .ok_or("second context transport failed")?;
        if h.preedit != b"h" {
            return Err(format!(
                "second context did not start independently: preedit={}",
                utf8(&h.preedit)
            ));
        }
        let h1 = client
            .process_key(context_id, b'H' as u32, 0, 0)
            .ok_or("first context transport failed")?;
        if h1.preedit != b"h" {
            return Err(format!(
                "first context retained state from second context: preedit={} commit={}",
                utf8(&h1.preedit),
                utf8(&h1.commit)
            ));
        }
        let a2 = client
            .process_key(second_context, b'A' as u32, 0, 0)
            .ok_or("second context A transport failed")?;
        if a2.preedit != b"a" || a2.commit != b"h" {
            return Err(format!(
                "second context state was not preserved: preedit={} commit={}",
                utf8(&a2.preedit),
                utf8(&a2.commit)
            ));
        }
        let a1 = client
            .process_key(context_id, b'A' as u32, 0, 0)
            .ok_or("first context A transport failed")?;
        if a1.preedit != b"a" || a1.commit != b"h" {
            return Err("first context received state from second context".to_owned());
        }
    }
    Ok(())
}

fn run_rime_lua(client: &mut EngineClient) -> Result<(), String> {
    let lua_context = 0x1414_2135_u64;
    let mut probe = None;
    for key in [
        b'Z' as u32,
        b'Z' as u32,
        b'L' as u32,
        b'U' as u32,
        b'A' as u32,
    ] {
        let outcome = client
            .process_key(lua_context, key, 0, 0)
            .ok_or("Rime Lua probe input transport failed")?;
        if !outcome.handled {
            return Err("Rime Lua probe input failed".to_owned());
        }
        probe = Some(outcome);
    }
    let probe = probe.expect("five probe keys");
    let lua_index = probe
        .candidates
        .iter()
        .position(|candidate| candidate.text_utf8 == b"Lua Works");
    let Some(lua_index) = lua_index else {
        return Err("Rime Lua translator did not produce its probe candidate".to_owned());
    };
    if lua_index >= 9 {
        return Err("Rime Lua candidate index out of range".to_owned());
    }
    let select = client
        .process_key(lua_context, b'1' as u32 + lua_index as u32, 0, 0)
        .ok_or("Rime Lua select transport failed")?;
    if !select.handled || select.commit != b"Lua Works" {
        return Err(format!(
            "Rime Lua candidate did not commit: {}",
            utf8(&select.commit)
        ));
    }
    Ok(())
}

fn run_typing_fuzz_reconnect(
    identity: &CurrentUserRuntimeIdentity,
    engine_path: &Path,
) -> Result<(), String> {
    let reconnect_context = 0x4242_4242_u64;
    {
        let mut abandoned = connect_engine(identity, engine_path)?;
        let h = abandoned
            .process_key(reconnect_context, b'H' as u32, 0, 0)
            .ok_or("disconnect recovery setup transport failed")?;
        if h.preedit != b"h" {
            return Err("disconnect recovery setup failed".to_owned());
        }
        // Drop: the pipe closes, the engine keeps the context alive.
    }
    let mut recovered = connect_engine(identity, engine_path)?;
    let n = recovered
        .process_key(reconnect_context, b'N' as u32, 0, 0)
        .ok_or("same-epoch reconnect transport failed")?;
    if n.preedit != b"n" {
        return Err(format!(
            "same-epoch reconnect retained stale composition: {}",
            utf8(&n.preedit)
        ));
    }
    recovered.close();
    Ok(())
}

/// Minimal reproducible MT19937-64 (matches std::mt19937_64 so the typing-fuzz
/// key stream is byte-identical to the C++ corpus).
struct Mt19937_64 {
    state: [u64; 312],
    index: usize,
}

impl Mt19937_64 {
    fn new(seed: u64) -> Self {
        let mut state = [0_u64; 312];
        state[0] = seed;
        for i in 1..312 {
            state[i] = 6_364_136_223_846_793_005_u64
                .wrapping_mul(state[i - 1] ^ (state[i - 1] >> 62))
                + i as u64;
        }
        Mt19937_64 { state, index: 312 }
    }

    fn next_u64(&mut self) -> u64 {
        const LOWER_MASK: u64 = (1 << 31) - 1;
        const UPPER_MASK: u64 = !LOWER_MASK;
        if self.index >= 312 {
            for i in 0..312 {
                let y = (self.state[i] & UPPER_MASK) | (self.state[(i + 1) % 312] & LOWER_MASK);
                let x = (self.state[(i + 397) % 312] ^ (y >> 1))
                    ^ if y & 1 != 0 { 0xB502_6F5A_A966_19E9 } else { 0 };
                self.state[i] = x;
            }
            self.index = 0;
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= (y >> 29) & 0x5555_5555_5555_5555;
        y ^= (y << 17) & 0x71D6_7FFF_ED60_0600;
        y ^= (y << 37) & 0xFFF7_EEE0_0000_0000;
        y ^= y >> 43;
        y
    }
}

fn run_typing_fuzz(client: &mut EngineClient) -> Result<(), String> {
    const COMMON_KEYS: [u32; 48] = [
        b'A' as u32,
        b'B' as u32,
        b'C' as u32,
        b'D' as u32,
        b'E' as u32,
        b'F' as u32,
        b'G' as u32,
        b'H' as u32,
        b'I' as u32,
        b'J' as u32,
        b'K' as u32,
        b'L' as u32,
        b'M' as u32,
        b'N' as u32,
        b'O' as u32,
        b'P' as u32,
        b'Q' as u32,
        b'R' as u32,
        b'S' as u32,
        b'T' as u32,
        b'U' as u32,
        b'V' as u32,
        b'W' as u32,
        b'X' as u32,
        b'Y' as u32,
        b'Z' as u32,
        b'0' as u32,
        b'1' as u32,
        b'2' as u32,
        b'3' as u32,
        b'4' as u32,
        b'5' as u32,
        b'6' as u32,
        b'7' as u32,
        b'8' as u32,
        b'9' as u32,
        VK_SPACE,
        VK_BACK,
        VK_RETURN,
        VK_ESCAPE,
        VK_LEFT,
        VK_RIGHT,
        VK_UP,
        VK_DOWN,
        VK_PRIOR,
        VK_NEXT,
        0,
        u32::MAX,
    ];
    const LONG_STRING_ROUNDS: u32 = 300;
    const LONG_STRING_CONTEXT: u64 = 0x4841_4841;
    const LONG_STRING_KEYS: [u32; 3] = [b'H' as u32, b'A' as u32, VK_SPACE];
    let mut latencies = Vec::with_capacity(LONG_STRING_ROUNDS as usize * 3);
    for round in 0..LONG_STRING_ROUNDS {
        let mut outcome = None;
        for &key in &LONG_STRING_KEYS {
            let start = Instant::now();
            let current = client
                .process_key(LONG_STRING_CONTEXT, key, 0, 0)
                .ok_or_else(|| {
                    format!("continuous ha typing smoke failed at round {round} key=0x{key:x}")
                })?;
            latencies.push(start.elapsed().as_micros() as i64);
            outcome = Some(current);
        }
        let round_end = outcome.expect("three keys per round");
        if round_end.commit.is_empty() || !round_end.preedit.is_empty() {
            return Err(format!(
                "continuous ha typing did not commit cleanly at round {round}"
            ));
        }
    }
    latencies.sort_unstable();
    let percentile = |value: usize| -> i64 { latencies[(latencies.len() - 1) * value / 100] };
    println!(
        "continuous-typing-rounds={} keys={} p50-us={} p95-us={} p99-us={} max-us={}",
        LONG_STRING_ROUNDS,
        LONG_STRING_ROUNDS * 3,
        percentile(50),
        percentile(95),
        percentile(99),
        latencies[latencies.len() - 1]
    );

    const ITERATIONS: u32 = 4_000;
    const CONTEXT_BASE: u64 = 0xF022_0000;
    let mut random = Mt19937_64::new(0x4657_5F54_5950_494E);
    let mut active_contexts: HashMap<u64, bool> = HashMap::new();
    let mut recovered_failures = 0_u32;
    let fuzz_start = Instant::now();
    for index in 0..ITERATIONS {
        let context = CONTEXT_BASE + (random.next_u64() % 16);
        let active = active_contexts.get(&context).copied().unwrap_or(false);
        let reachable = if active { COMMON_KEYS.len() - 2 } else { 36 };
        let key = COMMON_KEYS[(random.next_u64() as usize) % reachable];
        let flags = if random.next_u64() & 1 == 0 {
            0
        } else {
            KEY_FLAG_SHIFT
        };
        let outcome = client.process_key(context, key, flags, 0);
        let Some(outcome) = outcome else {
            recovered_failures += 1;
            active_contexts.clear();
            let recovery_context = CONTEXT_BASE + 100;
            let recovered = client
                .process_key(recovery_context, b'N' as u32, 0, 0)
                .ok_or_else(|| {
                    format!("typing fuzz did not recover after transport failure at {index} key=0x{key:x}")
                })?;
            if recovered.preedit != b"n" {
                return Err(format!(
                    "typing fuzz did not recover after transport failure at {index}"
                ));
            }
            active_contexts.insert(recovery_context, true);
            continue;
        };
        if outcome.preedit.len() > MAX_PREEDIT_UTF8
            || outcome.commit.len() > MAX_COMMIT_UTF8
            || outcome.candidates.len() > MAX_CANDIDATES
            || outcome.candidate_visibility > 2
            || (outcome.selected_candidate != u32::MAX
                && outcome.selected_candidate as usize >= outcome.candidates.len())
        {
            return Err(format!(
                "typing fuzz response invariant failed at iteration {index}"
            ));
        }
        active_contexts.insert(
            context,
            !outcome.preedit.is_empty() || !outcome.candidates.is_empty(),
        );
        if index % 64 == 63 {
            for cleanup in 0..16 {
                client
                    .process_key(CONTEXT_BASE + cleanup, VK_ESCAPE, 0, 0)
                    .ok_or("typing fuzz context cleanup failed")?;
                active_contexts.insert(CONTEXT_BASE + cleanup, false);
            }
        }
    }
    if recovered_failures > ITERATIONS / 100 {
        return Err(format!(
            "typing fuzz transport failure rate exceeded 1%: {recovered_failures}/{ITERATIONS}"
        ));
    }
    println!(
        "typing-fuzz-seed=0x46575f545950494e iterations={ITERATIONS} recovered-failures={recovered_failures} elapsed-ms={}",
        fuzz_start.elapsed().as_millis()
    );
    Ok(())
}

fn run_key_repeat(client: &mut EngineClient) -> Result<u64, String> {
    let repeat_context = 0x1618_0339_u64;
    const REPEAT_COUNT: u32 = 120;
    const REPEAT_PERIOD: Duration = Duration::from_micros(16_667);
    let repeat_start = Instant::now();
    let mut deadline = repeat_start;
    let mut last_epoch = 0_u64;
    for index in 0..REPEAT_COUNT {
        deadline += REPEAT_PERIOD;
        let key = if index & 1 == 0 { b'N' as u32 } else { VK_BACK };
        let outcome = client
            .process_key(repeat_context, key, 0, 0)
            .ok_or_else(|| format!("60 Hz key-repeat request failed at {index}"))?;
        if !outcome.handled {
            return Err(format!("60 Hz key-repeat not handled at {index}"));
        }
        last_epoch = outcome.engine_epoch;
        let now = Instant::now();
        if now < deadline {
            std::thread::sleep(deadline - now);
        }
    }
    let repeat_elapsed = repeat_start.elapsed();
    let budget = REPEAT_PERIOD * REPEAT_COUNT + Duration::from_millis(750);
    if repeat_elapsed > budget {
        return Err(format!(
            "60 Hz key-repeat accumulated backlog elapsed-ms={}",
            repeat_elapsed.as_millis()
        ));
    }
    println!(
        "key-repeat-count={REPEAT_COUNT} elapsed-ms={}",
        repeat_elapsed.as_millis()
    );
    Ok(last_epoch)
}

fn run_main(engine_path: &Path, flags: &Flags) -> Result<(), String> {
    let engine_stderr = std::env::temp_dir().join(format!(
        "fcitx5-real-engine-{}.stderr.log",
        std::process::id()
    ));
    let stderr_file = File::create(&engine_stderr).ok();

    let mut process = start_and_settle(
        engine_path,
        1,
        flags.safe_mode,
        0,
        stderr_file,
        flags.first_run_rime,
    )?;
    let identity = identity_security()?;
    let mut client = connect_engine(&identity, engine_path)?;
    run_baseline(&mut client, engine_path)?;
    if flags.chttrans {
        run_chttrans(&mut client)?;
    }
    run_hotkeys(&mut client)?;
    run_context_isolation(&mut client, flags.first_run_rime)?;
    if flags.rime_lua {
        run_rime_lua(&mut client)?;
    }
    if flags.typing_fuzz {
        run_typing_fuzz_reconnect(&identity, engine_path)?;
        run_typing_fuzz(&mut client)?;
    }
    let first_epoch = run_key_repeat(&mut client)?;
    client.close();
    process.request_stop();
    if !process.stop_and_wait(Duration::from_secs(5)) {
        return Err("real engine did not stop after test client disconnected".to_owned());
    }
    if first_epoch == 0 {
        return Err("engine epoch was zero".to_owned());
    }
    let _ = std::fs::remove_file(&engine_stderr);

    // Engine restart advances the epoch.
    let mut restarted = start_engine(engine_path, 2, flags.safe_mode, 1, None)?;
    let mut restarted_client = connect_engine(&identity, engine_path)?;
    let restart_key = restarted_client
        .process_key(0x3141_5926_u64, b'N' as u32, 0, 0)
        .ok_or("engine restart key transport failed")?;
    if restart_key.engine_epoch <= first_epoch {
        return Err(format!(
            "engine restart did not advance epoch: first-epoch={first_epoch} restart-epoch={}",
            restart_key.engine_epoch
        ));
    }
    restarted_client.close();
    if !restarted.stop_and_wait(Duration::from_secs(5)) {
        return Err("restarted engine did not exit cleanly".to_owned());
    }
    run_dispatch_late(engine_path, flags.safe_mode, &identity)?;
    Ok(())
}

fn run_chttrans(client: &mut EngineClient) -> Result<(), String> {
    let conversion_context = 0x4348_5454_5241_4E53_u64;
    let conversion_flags = KEY_FLAG_CONTROL | KEY_FLAG_SHIFT;
    let toggle = client
        .process_key(conversion_context, b'F' as u32, conversion_flags, 0)
        .ok_or("chttrans toggle transport failed")?;
    if !toggle.handled {
        return Err(format!(
            "chttrans toggle hotkey was not handled: handled={} preedit={} commit={}",
            toggle.handled,
            utf8(&toggle.preedit),
            utf8(&toggle.commit)
        ));
    }
    for key in [b'S' as u32, b'H' as u32, b'U' as u32] {
        let outcome = client
            .process_key(conversion_context, key, 0, 0)
            .ok_or("chttrans pinyin input transport failed")?;
        if !outcome.handled {
            return Err("chttrans pinyin input failed".to_owned());
        }
    }
    let committed = client
        .process_key(conversion_context, VK_SPACE, 0, 0)
        .ok_or("chttrans commit transport failed")?;
    if committed.commit != "書".as_bytes() {
        return Err(format!(
            "chttrans did not convert the committed word: {}",
            utf8(&committed.commit)
        ));
    }
    let restore = client
        .process_key(conversion_context, b'F' as u32, conversion_flags, 0)
        .ok_or("chttrans restore transport failed")?;
    if !restore.handled {
        return Err("chttrans restore hotkey was not handled".to_owned());
    }
    Ok(())
}

/// REG-DISPATCH-LATE: a key request that times out must never execute late.
fn run_dispatch_late(
    engine_path: &Path,
    safe_mode: bool,
    identity: &CurrentUserRuntimeIdentity,
) -> Result<(), String> {
    let stderr_path =
        std::env::temp_dir().join(format!("fcitx5-engine-late-{}.log", std::process::id()));
    let stderr_file = File::create(&stderr_path).ok();
    env::set_var("FCITX5_TEST_DISPATCH_DELAY_MS", "8200");
    let late_start = start_engine(engine_path, 3, safe_mode, 0, stderr_file);
    env::remove_var("FCITX5_TEST_DISPATCH_DELAY_MS");
    let mut late = late_start.map_err(|_| "late-engine start failed".to_owned())?;
    let mut late_client = connect_engine(identity, engine_path)?;
    let late_context = 0x4C41_5445_4B45_59_u64;
    let late_first = late_client.process_key(late_context, b'N' as u32, 0, 0);
    if late_first.is_some() {
        return Err("stalled dispatcher task did not time out the client".to_owned());
    }
    // Give the stalled task time to reach its deadline check and be dropped.
    std::thread::sleep(Duration::from_millis(8_500));
    let late_second = late_client
        .process_key(late_context, b'N' as u32, 0, 0)
        .ok_or("late-engine recovery transport failed")?;
    if !late_second.handled || late_second.preedit != b"n" {
        return Err(format!(
            "engine unhealthy after dropped late key: handled={} preedit={}",
            late_second.handled,
            utf8(&late_second.preedit)
        ));
    }
    late_client.close();
    late.request_stop();
    if !late.stop_and_wait(Duration::from_secs(5)) {
        return Err("late engine did not stop".to_owned());
    }
    let mut log = String::new();
    if File::open(&stderr_path)
        .and_then(|mut file| file.read_to_string(&mut log))
        .is_err()
    {
        log = String::new();
    }
    let _ = std::fs::remove_file(&stderr_path);
    let marker = log.find("dispatcher-dropped=").ok_or_else(|| {
        format!(
            "late engine did not report dropped count: {}",
            log.trim_end()
        )
    })?;
    let value_start = marker + "dispatcher-dropped=".len();
    let value_end = log[value_start..]
        .find(|character: char| !character.is_ascii_digit())
        .map(|offset| value_start + offset)
        .unwrap_or(log.len());
    let dropped: u64 = log[value_start..value_end]
        .parse()
        .map_err(|_| "dispatcher-dropped count unparseable".to_owned())?;
    if dropped == 0 {
        return Err("stalled key was executed instead of dropped".to_owned());
    }
    println!("dispatcher-dropped={dropped}");
    Ok(())
}

fn main() {
    let arguments: Vec<String> = env::args().collect();
    // argv[0]=exe, argv[1]=engine, argv[2..]=flags. The C++ accepted 2..=6
    // arguments (engine plus up to four flags).
    if arguments.len() < 2 || arguments.len() > 6 {
        eprintln!("engine executable argument required");
        std::process::exit(1);
    }
    let engine_path = PathBuf::from(&arguments[1]);
    let Some(flags) = parse_flags(&arguments[2..]) else {
        eprintln!("invalid scenario flags");
        std::process::exit(1);
    };
    // Respect an injected namespace (candidate-ui runner); otherwise default
    // to a process-unique namespace exactly like the C++ test.
    let injected = env::var("FCITX5_TEST_NAMESPACE").ok();
    let namespace = injected.unwrap_or_else(|| format!("engine-{}", std::process::id()));
    env::set_var("FCITX5_TEST_NAMESPACE", namespace);

    match run_main(&engine_path, &flags) {
        Ok(()) => {}
        Err(message) => {
            eprintln!("real-Fcitx acceptance failed: {message}");
            std::process::exit(1);
        }
    }
}
