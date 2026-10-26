#![forbid(unsafe_code)]

//! Lists the input methods the engine can load: the `.conf` files in the
//! bundled `share/fcitx5/inputmethod` directory and in every active package.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InputMethodChoice {
    pub(crate) id: String,
    pub(crate) name: String,
}

/// Reads the display name from the `[InputMethod]` group of an input method
/// `.conf` file, preferring the translation for `locale` (`zh-CN` matches
/// `Name[zh_CN]`, then `Name[zh]`).
pub(crate) fn input_method_name(text: &str, locale: &str) -> Option<String> {
    let full = locale.replace('-', "_");
    let language = full.split('_').next().unwrap_or_default().to_owned();
    let mut in_group = false;
    let (mut plain, mut regional, mut generic) = (None, None, None);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_group = line == "[InputMethod]";
            continue;
        }
        if !in_group {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match key.trim() {
            "Name" => plain = Some(value.to_owned()),
            key if key == format!("Name[{full}]") => regional = Some(value.to_owned()),
            key if !language.is_empty() && key == format!("Name[{language}]") => {
                generic = Some(value.to_owned());
            }
            _ => {}
        }
    }
    regional.or(generic).or(plain)
}

/// Collects the input methods from `directories` in order; the first file for
/// an ID wins, as in the engine's search path. `keyboard-us` is the
/// passthrough layout the engine always keeps and is not offered.
pub(crate) fn discover_input_methods(
    directories: &[PathBuf],
    locale: &str,
) -> Vec<InputMethodChoice> {
    let mut choices: Vec<InputMethodChoice> = Vec::new();
    for directory in directories {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        let mut files = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "conf")
            })
            .collect::<Vec<_>>();
        files.sort();
        for file in files {
            let Some(id) = file.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            if id == "keyboard-us" || choices.iter().any(|choice| choice.id == id) {
                continue;
            }
            let Ok(text) = fs::read_to_string(&file) else {
                continue;
            };
            let name = input_method_name(&text, locale).unwrap_or_else(|| id.to_owned());
            choices.push(InputMethodChoice {
                id: id.to_owned(),
                name,
            });
        }
    }
    choices
}

/// The bundled input method directory next to this executable's `bin`, then
/// each active package's.
pub(crate) fn input_method_directories(install_root: &Path, data_root: &Path) -> Vec<PathBuf> {
    let mut directories = vec![install_root
        .join("share")
        .join("fcitx5")
        .join("inputmethod")];
    #[cfg(windows)]
    directories.extend(
        fcitx5_package_core::active_package_prefixes(&data_root.join("packages"))
            .into_iter()
            .map(|prefix| prefix.join("share").join("fcitx5").join("inputmethod")),
    );
    #[cfg(not(windows))]
    let _ = data_root;
    directories
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_prefer_the_regional_then_language_translation() {
        let text =
            "[InputMethod]\nName=Pinyin\nName[zh_CN]=拼音\nName[ja]=ピンイン\nAddon=pinyin\n";
        assert_eq!(input_method_name(text, "zh-CN").as_deref(), Some("拼音"));
        assert_eq!(
            input_method_name(text, "ja-JP").as_deref(),
            Some("ピンイン")
        );
        assert_eq!(input_method_name(text, "en-US").as_deref(), Some("Pinyin"));
        assert_eq!(input_method_name("[Other]\nName=X\n", "en-US"), None);
    }

    #[test]
    fn discovery_keeps_the_first_file_for_an_id_and_skips_the_keyboard() {
        let root = std::env::temp_dir().join(format!("fcitx5-im-discovery-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let (base, package) = (root.join("base"), root.join("package"));
        fs::create_dir_all(&base).unwrap();
        fs::create_dir_all(&package).unwrap();
        fs::write(base.join("pinyin.conf"), "[InputMethod]\nName=Pinyin\n").unwrap();
        fs::write(
            base.join("keyboard-us.conf"),
            "[InputMethod]\nName=Keyboard\n",
        )
        .unwrap();
        fs::write(package.join("pinyin.conf"), "[InputMethod]\nName=Other\n").unwrap();
        fs::write(package.join("rime.conf"), "[InputMethod]\nName=Rime\n").unwrap();
        fs::write(package.join("notes.txt"), "ignored").unwrap();
        let choices = discover_input_methods(&[base, package, root.join("missing")], "en-US");
        let _ = fs::remove_dir_all(&root);
        assert_eq!(
            choices,
            [
                InputMethodChoice {
                    id: "pinyin".to_owned(),
                    name: "Pinyin".to_owned()
                },
                InputMethodChoice {
                    id: "rime".to_owned(),
                    name: "Rime".to_owned()
                },
            ]
        );
    }
}
