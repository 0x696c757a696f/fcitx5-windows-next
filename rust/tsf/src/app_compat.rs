//! Per-app compatibility. Apps listed in the excluded-apps file keep the
//! profile but get no key handling, so their keys arrive unchanged (plain
//! English input), which suits games whose anti-cheat or input handling does
//! not get along with a third-party input method. Separately, when a process
//! is not allowed to open the engine pipe (some anti-cheat and sandbox
//! policies deny it), key handling stays off for a while instead of retrying,
//! and starting the launcher, on every key.

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use fcitx5_windows_common_core as common;

/// One executable name (`game.exe`) or path per line, under the data root.
pub(crate) const EXCLUDED_APPS_FILE_NAME: &str = "tsf-excluded-apps.txt";
const EXCLUDED_APPS_FILE_LIMIT: u64 = 64 * 1024;
pub(crate) const ERROR_ACCESS_DENIED_CODE: i32 = 5;
/// The engine pipe's ACL does not change while the engine runs, so a denied
/// open would fail the same way on the next key.
const IPC_DENIED_BACKOFF_MILLISECONDS: u64 = 10 * 60 * 1000;

static CURRENT_PROCESS_EXCLUDED: OnceLock<bool> = OnceLock::new();
pub(crate) static IPC_DENIAL: IpcDenial = IpcDenial::new();

/// Lower-cased executable names from the file's text. Blank lines and lines
/// starting with `#` are skipped; a full path counts by its file name.
pub(crate) fn parse_excluded_apps(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.trim_matches('"').rsplit(['\\', '/']).next())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_lowercase)
        .collect()
}

pub(crate) fn is_app_excluded(excluded: &[String], executable: &Path) -> bool {
    let Some(name) = executable.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let name = name.to_lowercase();
    excluded.contains(&name)
}

fn read_excluded_apps(module_path: &Path) -> Vec<String> {
    let Some(root) = common::local_data_directory_for_module(module_path) else {
        return Vec::new();
    };
    let Ok(file) = File::open(root.join(EXCLUDED_APPS_FILE_NAME)) else {
        return Vec::new();
    };
    let mut text = String::new();
    if file
        .take(EXCLUDED_APPS_FILE_LIMIT)
        .read_to_string(&mut text)
        .is_err()
    {
        return Vec::new();
    }
    parse_excluded_apps(text.trim_start_matches('\u{feff}'))
}

/// Whether the user excluded the app this DLL is loaded in. Read once per
/// process; an edit applies when the app restarts.
pub(crate) fn current_process_excluded(module_path: &Path) -> bool {
    *CURRENT_PROCESS_EXCLUDED.get_or_init(|| {
        let Ok(executable) = std::env::current_exe() else {
            return false;
        };
        is_app_excluded(&read_excluded_apps(module_path), &executable)
    })
}

/// Until when (tick milliseconds) this process skips the engine after its
/// pipe open was denied.
pub(crate) struct IpcDenial(AtomicU64);

impl IpcDenial {
    const fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    pub(crate) fn record(&self, now: u64) {
        self.0.fetch_max(
            now.saturating_add(IPC_DENIED_BACKOFF_MILLISECONDS),
            Ordering::Relaxed,
        );
    }

    pub(crate) fn active(&self, now: u64) -> bool {
        now < self.0.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excluded_apps_parse_names_paths_and_comments() {
        let parsed = parse_excluded_apps(
            "# games\r\nValorant.exe\r\n\r\n  \"C:\\Games\\Apex\\r5apex.exe\"  \n D:/x/Game.EXE \n#skip.exe\n\\\n",
        );
        assert_eq!(parsed, ["valorant.exe", "r5apex.exe", "game.exe"]);
    }

    #[test]
    fn excluded_apps_match_file_name_ignoring_case() {
        let excluded = parse_excluded_apps("valorant.exe\n");
        assert!(is_app_excluded(
            &excluded,
            Path::new("C:\\Riot Games\\VALORANT\\live\\VALORANT.exe")
        ));
        assert!(!is_app_excluded(
            &excluded,
            Path::new("C:\\Windows\\notepad.exe")
        ));
        assert!(!is_app_excluded(&[], Path::new("valorant.exe")));
    }

    #[test]
    fn ipc_denial_lasts_for_the_back_off() {
        let denial = IpcDenial::new();
        assert!(!denial.active(1_000));
        denial.record(1_000);
        assert!(denial.active(1_000));
        assert!(denial.active(1_000 + IPC_DENIED_BACKOFF_MILLISECONDS - 1));
        assert!(!denial.active(1_000 + IPC_DENIED_BACKOFF_MILLISECONDS));
    }
}
