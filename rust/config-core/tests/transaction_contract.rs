#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;

use fcitx5_config_core::{
    CommitFault, ConfigCommand, ConfigCore, ConfigEdit, ConfigError, ConfigField, FileStore,
    RecoverySource,
};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "fcitx5-config-core-{name}-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("test directory should be created");
        Self(path)
    }

    fn config_path(&self) -> PathBuf {
        self.0.join("config.toml")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn apply_cancel_and_reset_keep_current_draft_and_defaults_distinct() {
    let directory = TestDirectory::new("apply-cancel-reset");
    let path = directory.config_path();
    let store = FileStore::new();
    let mut core = ConfigCore::compiled_defaults();

    core.execute(
        ConfigCommand::Set(ConfigEdit::CandidatePageSize(7)),
        &store,
        &path,
    )
    .expect("GUI set should edit only Draft");
    assert_eq!(core.current().candidate().page_size(), 5);
    assert_eq!(core.preview().candidate().page_size(), 7);
    assert_eq!(core.diff().len(), 1);

    core.cancel();
    assert_eq!(core.preview().candidate().page_size(), 5);
    assert!(core.diff().is_empty());

    core.execute(
        ConfigCommand::Set(ConfigEdit::CandidatePageSize(8)),
        &store,
        &path,
    )
    .expect("CLI set should edit Draft");
    core.apply(&store, &path, CommitFault::None)
        .expect("valid draft should commit");
    assert_eq!(core.current().candidate().page_size(), 8);
    assert_eq!(core.preview().candidate().page_size(), 8);

    core.execute(
        ConfigCommand::Reset(ConfigField::CandidatePageSize),
        &store,
        &path,
    )
    .expect("reset should remove the sparse override from Draft");
    assert_eq!(core.preview().candidate().page_size(), 5);
    core.apply(&store, &path, CommitFault::None)
        .expect("reset draft should commit");
    assert_eq!(core.current().candidate().page_size(), 5);
    assert_eq!(core.defaults().candidate().page_size(), 5);
}

#[test]
fn invalid_draft_never_replaces_current() {
    let directory = TestDirectory::new("invalid-draft");
    let path = directory.config_path();
    let store = FileStore::new();
    let mut core = ConfigCore::compiled_defaults();
    core.apply(&store, &path, CommitFault::None)
        .expect("defaults should commit");
    let before = fs::read_to_string(&path).expect("current file should exist");

    core.execute(
        ConfigCommand::Set(ConfigEdit::CandidatePageSize(0)),
        &store,
        &path,
    )
    .expect("Draft accepts a pending user edit");
    assert!(matches!(
        core.validate(),
        Err(ConfigError::Validation { .. })
    ));
    assert!(matches!(
        core.apply(&store, &path, CommitFault::None),
        Err(ConfigError::Validation { .. })
    ));
    assert_eq!(core.current().candidate().page_size(), 5);
    assert_eq!(
        fs::read_to_string(&path).expect("current file should remain"),
        before
    );
}

#[test]
fn staged_write_and_reread_faults_leave_current_unchanged() {
    let directory = TestDirectory::new("write-faults");
    let path = directory.config_path();
    let store = FileStore::new();
    let mut core = ConfigCore::compiled_defaults();
    core.apply(&store, &path, CommitFault::None)
        .expect("defaults should commit");
    let before = fs::read_to_string(&path).expect("current file should exist");
    let last_known_good = fs::read_to_string(FileStore::last_known_good_path(&path))
        .expect("last-known-good file should exist");

    for fault in [
        CommitFault::StagedWrite,
        CommitFault::StagedFlush,
        CommitFault::RereadMismatch,
        CommitFault::LastKnownGoodReplace,
        CommitFault::CurrentReplace,
    ] {
        core.execute(
            ConfigCommand::Set(ConfigEdit::CandidatePageSize(7)),
            &store,
            &path,
        )
        .expect("Draft edit should succeed");
        assert!(core.apply(&store, &path, fault).is_err());
        assert_eq!(core.current().candidate().page_size(), 5);
        assert_eq!(
            fs::read_to_string(&path).expect("current file should remain"),
            before
        );
        assert_eq!(
            fs::read_to_string(FileStore::last_known_good_path(&path))
                .expect("last-known-good file should remain"),
            last_known_good
        );
        core.cancel();
    }
}

#[test]
fn every_failed_apply_preserves_the_prior_recovery_snapshot() {
    let directory = TestDirectory::new("failed-apply-recovery");
    let path = directory.config_path();
    let lkg_path = FileStore::last_known_good_path(&path);
    let store = FileStore::new();
    let mut core = ConfigCore::compiled_defaults();

    core.set(ConfigEdit::CandidatePageSize(7));
    core.apply(&store, &path, CommitFault::None)
        .expect("first committed snapshot should create Current and LKG");
    core.set(ConfigEdit::CandidatePageSize(8));
    core.apply(&store, &path, CommitFault::None)
        .expect("second committed snapshot should advance Current and retain prior LKG");

    let expected_current = fs::read_to_string(&path).expect("Current should exist");
    let expected_lkg = fs::read_to_string(&lkg_path).expect("LKG should exist");
    let expected_recovery = ConfigCore::recover(&store, &path)
        .expect("Current should recover")
        .core
        .current();

    for fault in [
        CommitFault::StagedWrite,
        CommitFault::StagedFlush,
        CommitFault::RereadMismatch,
        CommitFault::LastKnownGoodReplace,
        CommitFault::CurrentReplace,
    ] {
        core.set(ConfigEdit::CandidatePageSize(9));
        assert!(core.apply(&store, &path, fault).is_err());

        assert_eq!(
            fs::read_to_string(&path).expect("failed apply must preserve Current"),
            expected_current
        );
        assert_eq!(
            fs::read_to_string(&lkg_path).expect("failed apply must preserve LKG"),
            expected_lkg
        );
        let recovered = ConfigCore::recover(&store, &path).expect("Current recovery should work");
        assert_eq!(recovered.source, RecoverySource::Current);
        assert_eq!(recovered.core.current(), expected_recovery);

        fs::write(&path, "candidate.page_size = 0\n")
            .expect("test should be able to corrupt Current");
        let recovered_lkg = ConfigCore::recover(&store, &path).expect("LKG recovery should work");
        assert_eq!(recovered_lkg.source, RecoverySource::LastKnownGood);
        assert_eq!(recovered_lkg.core.current().candidate().page_size(), 7);
        fs::write(&path, &expected_current).expect("Current should be restored for the next fault");
        core.cancel();
    }
}

#[test]
fn failed_first_apply_does_not_create_a_recovery_record_from_uncommitted_draft() {
    let directory = TestDirectory::new("failed-first-apply");
    let path = directory.config_path();
    let store = FileStore::new();
    let mut core = ConfigCore::compiled_defaults();

    core.set(ConfigEdit::CandidatePageSize(7));
    assert!(matches!(
        core.apply(&store, &path, CommitFault::CurrentReplace),
        Err(ConfigError::InjectedFault { .. })
    ));

    assert!(!path.exists(), "failed first apply must not create Current");
    assert!(
        !FileStore::last_known_good_path(&path).exists(),
        "failed first apply must not create LKG"
    );
    let recovered = ConfigCore::recover(&store, &path).expect("safe defaults should recover");
    assert_eq!(recovered.source, RecoverySource::SafeDefaults);
    assert_eq!(recovered.core.current().candidate().page_size(), 5);
}

#[test]
fn recovery_prefers_current_then_last_known_good_then_safe_defaults_without_rewriting_bad_files() {
    let directory = TestDirectory::new("recovery-order");
    let path = directory.config_path();
    let store = FileStore::new();
    let mut valid = ConfigCore::compiled_defaults();
    valid
        .execute(
            ConfigCommand::Set(ConfigEdit::CandidatePageSize(7)),
            &store,
            &path,
        )
        .expect("valid Draft should accept edit");
    valid
        .apply(&store, &path, CommitFault::None)
        .expect("valid config should commit with LKG");
    let good = fs::read_to_string(&path).expect("current config should exist");

    let recovered_current = ConfigCore::recover(&store, &path).expect("current should recover");
    assert_eq!(recovered_current.source, RecoverySource::Current);

    fs::write(&path, "candidate.page_size = 0\n").expect("bad current fixture should write");
    let recovered_lkg = ConfigCore::recover(&store, &path).expect("LKG should recover");
    assert_eq!(recovered_lkg.source, RecoverySource::LastKnownGood);
    assert_eq!(recovered_lkg.core.current().candidate().page_size(), 7);
    assert_eq!(
        fs::read_to_string(&path).expect("bad file should not be overwritten"),
        "candidate.page_size = 0\n"
    );

    fs::write(
        FileStore::last_known_good_path(&path),
        "candidate.page_size = 0\n",
    )
    .expect("bad LKG fixture should write");
    let recovered_defaults =
        ConfigCore::recover(&store, &path).expect("safe defaults should recover");
    assert_eq!(recovered_defaults.source, RecoverySource::SafeDefaults);
    assert_eq!(recovered_defaults.core.current().candidate().page_size(), 5);
    assert_eq!(
        fs::read_to_string(&path).expect("bad current should remain"),
        "candidate.page_size = 0\n"
    );
    assert_ne!(good, "candidate.page_size = 0\n");
}

#[test]
fn gui_and_cli_commands_have_identical_shared_core_semantics() {
    let directory = TestDirectory::new("frontend-equivalence");
    let path = directory.config_path();
    let store = FileStore::new();
    let mut gui = ConfigCore::compiled_defaults();
    let mut cli = ConfigCore::compiled_defaults();

    let commands = [
        ConfigCommand::Set(ConfigEdit::CandidatePageSize(7)),
        ConfigCommand::Set(ConfigEdit::AppearanceMode("dark".to_owned())),
        ConfigCommand::Reset(ConfigField::CandidatePageSize),
    ];
    for command in commands {
        gui.execute(command.clone(), &store, &path)
            .expect("GUI should route through Config Core");
        cli.execute(command, &store, &path)
            .expect("CLI should route through Config Core");
    }

    assert_eq!(gui.preview(), cli.preview());
    assert_eq!(gui.diff(), cli.diff());
    gui.execute(ConfigCommand::Validate, &store, &path)
        .expect("GUI validation should use Config Core");
    cli.execute(ConfigCommand::Validate, &store, &path)
        .expect("CLI validation should use Config Core");
}

#[test]
fn full_config_schema_round_trips_without_dropping_unedited_overrides() {
    let directory = TestDirectory::new("full-schema-round-trip");
    let path = directory.config_path();
    let store = FileStore::new();
    let mut core = ConfigCore::compiled_defaults();

    core.execute(
        ConfigCommand::Import(include_str!("../../../resources/config.toml").to_owned()),
        &store,
        &path,
    )
    .expect("full documented schema should import through the Core");
    core.execute(
        ConfigCommand::Set(ConfigEdit::CandidatePageSize(7)),
        &store,
        &path,
    )
    .expect("typed edit should preserve imported overrides");
    core.apply(&store, &path, CommitFault::None)
        .expect("full documented schema should commit through the Core");

    let persisted = fs::read_to_string(&path).expect("committed config should exist");
    assert!(persisted.contains("[candidate.geometry]"));
    assert!(persisted.contains("[candidate.label]"));
    assert!(persisted.contains("[fonts.candidate]"));
    assert!(persisted.contains("size_dip = 18.0"));
    assert!(persisted.contains("page_size = 7"));
    ConfigCore::load(&store, &path).expect("full committed schema should reload");
}

#[test]
fn production_corpus_round_trips_product_fields_and_preserves_fcitx_and_future_data() {
    let directory = TestDirectory::new("production-corpus-round-trip");
    let path = directory.config_path();
    let store = FileStore::new();
    let mut core = ConfigCore::compiled_defaults();

    core.execute(
        ConfigCommand::Import(include_str!("fixtures/legacy-production-config-v1.toml").to_owned()),
        &store,
        &path,
    )
    .expect("the frozen production corpus should import through Rust Config Core");
    core.execute(
        ConfigCommand::Set(ConfigEdit::CandidatePageSize(7)),
        &store,
        &path,
    )
    .expect("typed product edits should preserve unrelated fields");
    core.apply(&store, &path, CommitFault::None)
        .expect("the corpus should commit through Rust Config Core");

    let persisted = fs::read_to_string(&path).expect("committed corpus should exist");
    for expected in [
        "scroll_cell_width_dip = 96.0",
        "preedit_mode = \"inline\"",
        "sequence = [\"1\", \"2\", \"3\", \"4\", \"5\", \"6\", \"7\", \"8\", \"9\"]",
        "background = \"#FFFFFF\"",
        "[input_methods]",
        "enabled = [\"pinyin\", \"rime\", \"wbx\"]",
        "[hotkeys]",
        "toggle_input_method = \"Ctrl+Space\"",
        "[future-addon]",
        "setting = \"preserve-me\"",
        "page_size = 7",
    ] {
        assert!(
            persisted.contains(expected),
            "missing {expected:?} from {persisted}"
        );
    }
    ConfigCore::load(&store, &path).expect("the persisted corpus should reload");
}
