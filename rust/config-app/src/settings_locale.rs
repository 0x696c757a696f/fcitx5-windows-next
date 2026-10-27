#![forbid(unsafe_code)]

use std::collections::BTreeMap;

const EN_US: &str = include_str!("../../../locales/en-US.json");
const ZH_CN: &str = include_str!("../../../locales/zh-CN.json");
const ZH_TW: &str = include_str!("../../../locales/zh-TW.json");
const JA_JP: &str = include_str!("../../../locales/ja-JP.json");
const KO_KR: &str = include_str!("../../../locales/ko-KR.json");
const VI_VN: &str = include_str!("../../../locales/vi-VN.json");
const TH_TH: &str = include_str!("../../../locales/th-TH.json");
const SI_LK: &str = include_str!("../../../locales/si-LK.json");

const CATALOGS: &[(&str, &str)] = &[
    ("en-US", EN_US),
    ("zh-CN", ZH_CN),
    ("zh-TW", ZH_TW),
    ("ja-JP", JA_JP),
    ("ko-KR", KO_KR),
    ("vi-VN", VI_VN),
    ("th-TH", TH_TH),
    ("si-LK", SI_LK),
];
const NAVIGATION_KEYS: [&str; 4] = ["nav.general", "nav.appearance", "nav.packages", "nav.fcitx"];
#[cfg(test)]
const WINDUI_SETTINGS_KEYS: &[&str] = &[
    "settings.input.subtitle",
    "settings.input.section",
    "settings.input.list_hint",
    "settings.input.default",
    "settings.input.make_default",
    "settings.input.move_up",
    "settings.input.remove",
    "settings.input.add",
    "settings.input.add_hint",
    "settings.input.applied",
    "settings.input.restart_failed",
    "settings.shortcuts.section",
    "settings.shortcuts.toggle_input_method",
    "settings.shortcuts.next_input_method",
    "settings.shortcuts.same",
    "settings.startup.section",
    "settings.startup.sign_in",
    "settings.startup.sign_in_hint",
    "settings.appearance.subtitle",
    "settings.theme.section",
    "settings.theme.choice",
    "settings.theme.choice_hint",
    "settings.appearance.mode_hint",
    "theme.default",
    "settings.candidate.text_section",
    "settings.candidate.font_size",
    "settings.candidate.font_size_hint",
    "settings.candidate.font",
    "settings.candidate.font_hint",
    "settings.candidate.font_theme",
    "settings.candidate.opacity",
    "settings.candidate.opacity_hint",
    "settings.saved",
    "settings.theme.toggle",
    "settings.candidate.section",
    "settings.candidate.layout_hint",
    "settings.candidate.mode.vertical_text",
    "settings.candidate.scroll_direction",
    "settings.candidate.scroll_direction_hint",
    "settings.candidate.horizontal_scroll",
    "settings.candidate.vertical_scroll",
    "settings.candidate.column_direction",
    "settings.candidate.column_direction_hint",
    "settings.candidate.right_to_left",
    "settings.candidate.left_to_right",
    "settings.candidate.orientation.horizontal",
    "settings.candidate.orientation.vertical",
    "settings.candidate.orientation.horizontal_hint",
    "settings.candidate.orientation.vertical_hint",
    "settings.candidate.overflow.paging",
    "settings.candidate.overflow.scrolling",
    "settings.candidate.overflow.wrapping",
    "settings.candidate.overflow_hint",
    "settings.candidate.writing.horizontal",
    "settings.candidate.writing.vertical_rl",
    "settings.candidate.writing.vertical_lr",
    "settings.candidate.writing_hint",
    "settings.candidate.page_size_hint",
    "settings.candidate.reset",
    "settings.candidate.draft_updated",
    "settings.candidate.draft_loaded",
    "settings.candidate.draft_reset",
    "settings.plugins.subtitle",
    "settings.plugins.repository.loading",
    "settings.plugins.repository.ready",
    "settings.plugins.repository.unavailable",
    "settings.plugins.repository.not_downloaded",
    "settings.plugins.fallback_category",
    "settings.plugins.fallback_metadata",
    "settings.plugins.operation.list",
    "settings.plugins.operation.refresh",
    "settings.plugins.operation.install",
    "settings.plugins.operation.update",
    "settings.plugins.operation.enable",
    "settings.plugins.operation.disable",
    "settings.plugins.operation.remove",
    "settings.plugins.operation.repair",
    "settings.plugins.operation.running",
    "settings.plugins.operation.repairing",
    "settings.plugins.catalog_count",
    "settings.plugins.reference_note",
    "settings.plugins.refresh_signed",
    "settings.plugins.control_reading",
    "settings.plugins.choose_operation",
    "settings.plugins.completed",
    "settings.plugins.operation_failed",
    "settings.plugins.error.invalid_id",
    "settings.plugins.error.catalog_too_large",
    "settings.plugins.error.catalog_version_invalid",
    "settings.plugins.error.catalog_fields_invalid",
    "settings.plugins.error.control_unavailable",
    "nav.fcitx",
    "settings.fcitx.subtitle",
    "settings.fcitx.global",
    "settings.fcitx.global_hint",
    "settings.fcitx.addons",
    "settings.fcitx.addons_hint",
    "settings.fcitx.open",
    "settings.fcitx.back",
    "settings.fcitx.undo",
    "settings.fcitx.reset",
    "settings.fcitx.reset_all",
    "settings.fcitx.loading",
    "settings.fcitx.saved",
    "settings.fcitx.save_failed",
    "settings.fcitx.load_failed",
    "settings.fcitx.unsupported",
    "settings.fcitx.keys_hint",
    "settings.fcitx.lines_hint",
    "settings.fcitx.empty",
    "settings.fcitx.retry",
    "settings.fcitx.error.engine_unavailable",
    "settings.fcitx.error.no_response",
    "settings.fcitx.error.invalid",
    "settings.input.options",
];

pub(crate) fn resolve_locale(configured: &str, system: &str) -> &'static str {
    let requested = if configured == "system" {
        system
    } else {
        configured
    };
    CATALOGS
        .iter()
        .find_map(|(locale, _)| (*locale == requested).then_some(*locale))
        .unwrap_or("en-US")
}

pub(crate) fn is_supported_locale(locale: &str) -> bool {
    CATALOGS.iter().any(|(tag, _)| *tag == locale)
}

#[derive(Clone, Debug)]
pub(crate) struct LocaleCatalog {
    tag: &'static str,
    messages: BTreeMap<String, String>,
    english: BTreeMap<String, String>,
}

impl LocaleCatalog {
    pub(crate) fn new(configured: &str, system: &str) -> Self {
        let locale = resolve_locale(configured, system);
        let messages = catalog(locale);
        let english = catalog("en-US");
        Self {
            tag: locale,
            messages,
            english,
        }
    }

    /// The resolved locale tag, such as `zh-CN`.
    pub(crate) fn tag(&self) -> &'static str {
        self.tag
    }

    pub(crate) fn navigation_labels(&self) -> [String; 4] {
        NAVIGATION_KEYS.map(|key| self.text(key))
    }

    #[cfg(test)]
    fn contains_localized(&self, key: &str) -> bool {
        self.messages.contains_key(key)
    }

    pub(crate) fn text(&self, key: &str) -> String {
        self.messages
            .get(key)
            .or_else(|| self.english.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    pub(crate) fn format(&self, key: &str, arguments: &[(&str, &str)]) -> String {
        arguments
            .iter()
            .fold(self.text(key), |message, (name, value)| {
                message.replace(&format!("{{{name}}}"), value)
            })
    }
}

fn catalog(locale: &str) -> BTreeMap<String, String> {
    let source = CATALOGS
        .iter()
        .find_map(|(tag, json)| (*tag == locale).then_some(*json))
        .unwrap_or(EN_US);
    let values: serde_json::Value =
        serde_json::from_str(source).expect("embedded Settings locale catalog must be valid JSON");
    values
        .as_object()
        .expect("embedded Settings locale catalog must be a JSON object")
        .iter()
        .filter_map(|(key, value)| {
            value
                .as_str()
                .map(|message| (key.clone(), message.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{resolve_locale, LocaleCatalog, CATALOGS, NAVIGATION_KEYS, WINDUI_SETTINGS_KEYS};

    fn function_source<'a>(source: &'a str, signature: &str) -> &'a str {
        source
            .split_once(signature)
            .and_then(|(_, rest)| rest.split_once("\nfn ").map(|(body, _)| body))
            .unwrap_or_else(|| panic!("missing function {signature}"))
    }

    #[test]
    fn settings_locale_resolves_system_and_explicit_locales() {
        assert_eq!(resolve_locale("system", "ja-JP"), "ja-JP");
        assert_eq!(resolve_locale("zh-TW", "en-US"), "zh-TW");
        assert_eq!(resolve_locale("unsupported", "unsupported"), "en-US");
    }

    #[test]
    fn every_supported_catalog_has_the_same_live_navigation_contract() {
        for (locale, _) in CATALOGS {
            let catalog = LocaleCatalog::new(locale, "en-US");
            for key in NAVIGATION_KEYS {
                assert!(catalog.contains_localized(key), "{locale} is missing {key}");
            }
            assert_ne!(catalog.navigation_labels()[0], "nav.general");
        }
    }

    #[test]
    fn settings_catalog_serves_translated_navigation_labels() {
        assert_eq!(
            LocaleCatalog::new("en-US", "en-US").text("nav.general"),
            "Input Methods"
        );
        assert_eq!(
            LocaleCatalog::new("zh-TW", "en-US").text("nav.general"),
            "輸入法"
        );
        assert_eq!(
            LocaleCatalog::new("si-LK", "en-US").text("nav.general"),
            "ආදාන ක්‍රම"
        );
    }

    #[test]
    fn missing_translation_falls_back_to_english_then_key() {
        let locale = LocaleCatalog::new("ja-JP", "en-US");
        assert_eq!(locale.text("action.apply"), "適用");
        assert_eq!(locale.text("settings.missing.key"), "settings.missing.key");
    }

    #[test]
    fn localized_message_arguments_are_substituted_without_losing_unicode() {
        let locale = LocaleCatalog::new("zh-CN", "en-US");
        assert_eq!(
            locale.format("settings.plugins.catalog_count", &[("count", "7")]),
            "插件目录 · 7 项"
        );
    }

    #[test]
    fn windui_settings_construction_has_no_embedded_han_copy() {
        let source = include_str!("main.rs");
        for signature in [
            "fn windui_settings_root(",
            "fn windui_plugins_page(",
            "fn candidate_layout_mode_button(",
            "fn windui_config_core_candidate_layout_controls(",
        ] {
            let body = function_source(source, signature);
            assert!(
                !body
                    .chars()
                    .any(|character| matches!(character as u32, 0x3400..=0x9fff)),
                "{signature} must resolve visible text through locale catalogs"
            );
        }
    }

    #[test]
    fn every_windui_settings_key_exists_in_all_eight_catalogs() {
        for (locale, _) in CATALOGS {
            let catalog = LocaleCatalog::new(locale, "en-US");
            for key in WINDUI_SETTINGS_KEYS {
                assert!(
                    catalog.contains_localized(key),
                    "{locale} is missing WindUI Settings key {key}"
                );
            }
        }
    }
}
