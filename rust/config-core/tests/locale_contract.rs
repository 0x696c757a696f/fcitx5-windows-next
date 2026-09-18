#![forbid(unsafe_code)]

use fcitx5_config_core::{ConfigCore, ConfigEdit, ConfigError, UI_LANGUAGE_VALUES};

#[test]
fn canonical_ui_languages_are_accepted_and_unknown_values_keep_validation_error_semantics() {
    for language in UI_LANGUAGE_VALUES {
        let mut core = ConfigCore::compiled_defaults();
        core.set(ConfigEdit::UiLanguage((*language).to_owned()));
        assert_eq!(core.preview().ui().language(), *language);
        core.validate()
            .unwrap_or_else(|error| panic!("{language} should be accepted: {error}"));
    }

    let mut core = ConfigCore::compiled_defaults();
    core.set(ConfigEdit::UiLanguage("fr-FR".to_owned()));
    let error = core
        .validate()
        .expect_err("unknown locale must be rejected");
    assert!(matches!(error, ConfigError::Validation { .. }));
    assert_eq!(
        error.to_string(),
        "invalid configuration: ui.language has an unsupported value"
    );
}
