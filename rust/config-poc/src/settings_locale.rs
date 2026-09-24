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
const NAVIGATION_KEYS: [&str; 6] = [
    "nav.general",
    "nav.appearance",
    "nav.theme",
    "nav.packages",
    "updates.title",
    "nav.repair",
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
    messages: BTreeMap<String, String>,
    english: BTreeMap<String, String>,
}

impl LocaleCatalog {
    pub(crate) fn new(configured: &str, system: &str) -> Self {
        let locale = resolve_locale(configured, system);
        let messages = catalog(locale);
        let english = catalog("en-US");
        Self { messages, english }
    }

    pub(crate) fn navigation_labels(&self) -> [String; 6] {
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
    use super::{resolve_locale, LocaleCatalog, CATALOGS, NAVIGATION_KEYS};

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
}
