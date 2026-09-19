#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::de::{self, MapAccess, Visitor};
use serde::Deserialize;
use serde_json::Value;

pub const SUPPORTED_UI_LOCALES: [&str; 8] = [
    "en-US", "zh-CN", "zh-TW", "ja-JP", "ko-KR", "vi-VN", "th-TH", "si-LK",
];

static ACTIVE_CATALOG: OnceLock<LocaleCatalog> = OnceLock::new();

pub struct LocaleCatalog {
    values: BTreeMap<String, String>,
}

struct JsonObject(BTreeMap<String, Value>);

impl<'de> Deserialize<'de> for JsonObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct JsonObjectVisitor;

        impl<'de> Visitor<'de> for JsonObjectVisitor {
            type Value = JsonObject;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut values = BTreeMap::new();
                while let Some((key, value)) = access.next_entry::<String, Value>()? {
                    if values.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate JSON object key"));
                    }
                }
                Ok(JsonObject(values))
            }
        }

        deserializer.deserialize_map(JsonObjectVisitor)
    }
}

pub fn validate_locale_name(locale: &str) -> Result<(), String> {
    if SUPPORTED_UI_LOCALES.contains(&locale) {
        Ok(())
    } else {
        Err(format!(
            "unsupported locale '{locale}'; allowed locales: {}",
            SUPPORTED_UI_LOCALES.join(", ")
        ))
    }
}

pub fn parse_catalog(bytes: &[u8], locale: &str) -> Result<LocaleCatalog, String> {
    validate_locale_name(locale)?;
    let JsonObject(mut raw) = serde_json::from_slice(bytes)
        .map_err(|error| format!("failed to parse locale catalog '{locale}': {error}"))?;

    match raw.remove("format_version") {
        Some(Value::Number(version)) if version.as_i64() == Some(1) => {}
        Some(_) => {
            return Err(format!(
                "locale catalog '{locale}' requires format_version 1"
            ))
        }
        None => {
            return Err(format!(
                "locale catalog '{locale}' is missing format_version"
            ))
        }
    }

    let mut values = BTreeMap::new();
    for (key, value) in raw {
        let text = value.as_str().ok_or_else(|| {
            format!("locale catalog '{locale}' value for '{key}' must be a string")
        })?;
        if text.trim().is_empty() {
            return Err(format!(
                "locale catalog '{locale}' value for '{key}' is empty"
            ));
        }
        values.insert(key, text.to_owned());
    }
    Ok(LocaleCatalog { values })
}

impl LocaleCatalog {
    pub fn text(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn key_count(&self) -> usize {
        self.values.len()
    }

    pub fn validate_against_schema(&self) -> Result<(), String> {
        let schema = parse_catalog(include_bytes!("../../../locales/en-US.json"), "en-US")?;
        let missing = schema
            .values
            .keys()
            .filter(|key| !self.values.contains_key(*key))
            .cloned()
            .collect::<Vec<_>>();
        let extra = self
            .values
            .keys()
            .filter(|key| !schema.values.contains_key(*key))
            .cloned()
            .collect::<Vec<_>>();
        if missing.is_empty() && extra.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "locale catalog key set differs from en-US schema; missing: {missing:?}; extra: {extra:?}"
            ))
        }
    }
}

pub fn install(locale: &str) -> Result<(), String> {
    validate_locale_name(locale)?;
    let executable_locale = std::env::current_exe()
        .map_err(|error| format!("failed to locate current executable: {error}"))?
        .parent()
        .map(|directory| directory.join("locales").join(format!("{locale}.json")));
    let manifest_locale = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("locales")
        .join(format!("{locale}.json"));
    let path = executable_locale
        .filter(|path| path.is_file())
        .unwrap_or(manifest_locale);
    let bytes = std::fs::read(&path).map_err(|error| {
        format!(
            "failed to read locale catalog '{}': {error}",
            path.display()
        )
    })?;
    let catalog = parse_catalog(&bytes, locale)?;
    catalog.validate_against_schema()?;
    ACTIVE_CATALOG
        .set(catalog)
        .map_err(|_| "locale catalog has already been installed".to_owned())
}

pub fn label(key: &str, fallback: &str) -> String {
    ACTIVE_CATALOG
        .get()
        .and_then(|catalog| catalog.text(key))
        .unwrap_or(fallback)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOCALES: [(&str, &str); 8] = [
        ("en-US", include_str!("../../../locales/en-US.json")),
        ("zh-CN", include_str!("../../../locales/zh-CN.json")),
        ("zh-TW", include_str!("../../../locales/zh-TW.json")),
        ("ja-JP", include_str!("../../../locales/ja-JP.json")),
        ("ko-KR", include_str!("../../../locales/ko-KR.json")),
        ("vi-VN", include_str!("../../../locales/vi-VN.json")),
        ("th-TH", include_str!("../../../locales/th-TH.json")),
        ("si-LK", include_str!("../../../locales/si-LK.json")),
    ];

    #[test]
    fn fallback_label_is_verbatim_without_catalog() {
        assert_eq!(label("missing.key", "Fallback text"), "Fallback text");
    }

    #[test]
    fn locale_names_are_case_sensitive_and_bounded() {
        for locale in SUPPORTED_UI_LOCALES {
            assert!(validate_locale_name(locale).is_ok());
        }
        for locale in ["fr-FR", "", "zh-cn"] {
            assert!(validate_locale_name(locale).is_err());
        }
    }

    #[test]
    fn malformed_catalogs_are_rejected() {
        for json in [
            r#"{"hello":"world"}"#,
            r#"{"format_version":1,"hello":true}"#,
            r#"{"format_version":1,"hello":"   "}"#,
            r#"{"format_version":2,"hello":"world"}"#,
            r#"{"format_version":1,"hello":"one","hello":"two"}"#,
        ] {
            assert!(parse_catalog(json.as_bytes(), "en-US").is_err(), "{json}");
        }
    }

    #[test]
    fn schema_rejects_missing_and_extra_keys() {
        let schema = parse_catalog(include_bytes!("../../../locales/en-US.json"), "en-US")
            .expect("schema should parse");
        let mut missing = schema.values.clone();
        missing.pop_first();
        assert!(LocaleCatalog { values: missing }
            .validate_against_schema()
            .is_err());
        let mut extra = schema.values;
        extra.insert("extra.key".to_owned(), "Extra".to_owned());
        assert!(LocaleCatalog { values: extra }
            .validate_against_schema()
            .is_err());
    }

    #[test]
    fn repository_catalogs_have_schema_parity() {
        let schema_key_count =
            parse_catalog(include_bytes!("../../../locales/en-US.json"), "en-US")
                .expect("schema should parse")
                .key_count();
        assert!(schema_key_count > 167);
        for (locale, json) in LOCALES {
            let catalog = parse_catalog(json.as_bytes(), locale).expect("catalog should parse");
            catalog
                .validate_against_schema()
                .expect("catalog should match schema");
            assert_eq!(catalog.key_count(), schema_key_count);
        }
    }
}
