#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::fs;

use fcitx5_launcher_core::{
    format_launcher_status, load_launcher_snapshot, parse_launcher_arguments,
    prepare_supervisor_start, LauncherInvocation,
};

#[test]
fn launcher_entry_resolves_paths_persists_initial_state_and_prepares_bounded_start() {
    let root = std::env::temp_dir().join(format!("fcitx5-launcher-entry-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let bin = root.join("bin");
    fs::create_dir_all(&bin).expect("launcher test bin should be created");
    fs::write(bin.join("fcitx5-engine.exe"), b"engine").expect("engine stub should be created");
    fs::write(bin.join("fcitx5-ui.exe"), b"ui").expect("ui stub should be created");
    let state_file = root.join("launcher-state.v1");

    let invocation = parse_launcher_arguments([
        OsString::from("--background"),
        OsString::from("--state-file"),
        state_file.clone().into_os_string(),
        OsString::from("--generation"),
        OsString::from("g1"),
        OsString::from("--engine-ready-event"),
        OsString::from("engine-ready"),
        OsString::from("--stop-event"),
        OsString::from("engine-stop"),
    ])
    .expect("launcher arguments should parse");
    let LauncherInvocation::Supervise(options) = invocation else {
        panic!("launcher invocation should start supervision");
    };

    let startup = prepare_supervisor_start(&options, &bin, 100)
        .expect("launcher startup skeleton should prepare");

    assert_eq!(startup.engine_path, bin.join("fcitx5-engine.exe"));
    assert_eq!(startup.ui_path, Some(bin.join("fcitx5-ui.exe")));
    assert_eq!(startup.status.launcher_state, 0);
    assert_eq!(startup.status.engine_state, 1);
    assert_eq!(startup.status.start_disposition, 0);
    assert_eq!(
        format_launcher_status(&startup.status),
        "launcher_state=0 engine_state=1 start_disposition=0 retry_after_ms=0"
    );
    assert_eq!(
        String::from_utf16(&startup.engine_command_line.expect("warmup command"))
            .expect("command should be UTF-16"),
        format!(
            "\"{}\" --ready-event \"engine-ready\" --stop-event \"engine-stop\" --generation \"g1\"",
            bin.join("fcitx5-engine.exe").display()
        )
    );
    assert_eq!(
        load_launcher_snapshot(&state_file)
            .expect("initial state should be persisted")
            .state,
        0
    );

    let _ = fs::remove_dir_all(root);
}
