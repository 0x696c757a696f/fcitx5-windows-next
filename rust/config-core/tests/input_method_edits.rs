#![forbid(unsafe_code)]

use fcitx5_config_core::{ConfigCore, ConfigEdit};

#[test]
fn input_method_and_hotkey_edits_reach_the_preview_and_validate() {
    let mut core = ConfigCore::compiled_defaults();
    core.set(ConfigEdit::InputMethods {
        enabled: vec!["rime".to_owned(), "pinyin".to_owned()],
        default: "pinyin".to_owned(),
    });
    core.set(ConfigEdit::HotkeyToggleInputMethod(
        "Ctrl+Shift+Space".to_owned(),
    ));
    core.set(ConfigEdit::HotkeyNextInputMethod("Alt+Shift".to_owned()));
    let preview = core.preview();
    assert_eq!(preview.input_methods().enabled(), ["rime", "pinyin"]);
    assert_eq!(preview.input_methods().default_id(), "pinyin");
    assert_eq!(preview.hotkeys().toggle_input_method(), "Ctrl+Shift+Space");
    assert_eq!(preview.hotkeys().next_input_method(), "Alt+Shift");
    core.validate()
        .expect("edited input methods should validate");
}

#[test]
fn a_default_outside_the_enabled_list_is_rejected() {
    let mut core = ConfigCore::compiled_defaults();
    core.set(ConfigEdit::InputMethods {
        enabled: vec!["rime".to_owned()],
        default: "pinyin".to_owned(),
    });
    assert!(core.validate().is_err());
}
