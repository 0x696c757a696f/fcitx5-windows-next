#![forbid(unsafe_code)]

//! Fcitx's own configurations (global, per addon, per input method), read
//! from and written to the running engine. The engine sends the RawConfig of
//! `Configuration::dumpDescription` and `Configuration::save`; this module
//! turns them into options Settings can render, the way fcitx5-macos and
//! fcitx5-android build their configuration pages.

use std::path::{Path, PathBuf};

use fcitx5_ipc_client::EngineClient;
use fcitx5_protocol_core::ConfigOperation;
use fcitx5_windows_common_core::current_runtime_generation_for_current_process;
use serde::Deserialize;

pub(crate) const GLOBAL_URI: &str = "fcitx://config/global";
pub(crate) const ADDONS_URI: &str = "fcitx://config/addons";
pub(crate) const INPUT_METHODS_URI: &str = "fcitx://config/inputmethods";

// Errors are Settings locale keys.
pub(crate) const ERROR_INVALID: &str = "settings.fcitx.error.invalid";
pub(crate) const ERROR_ENGINE_UNAVAILABLE: &str = "settings.fcitx.error.engine_unavailable";
pub(crate) const ERROR_NO_RESPONSE: &str = "settings.fcitx.error.no_response";

pub(crate) fn addon_uri(addon: &str) -> String {
    format!("fcitx://config/addon/{addon}")
}

pub(crate) fn input_method_uri(id: &str) -> String {
    format!("fcitx://config/inputmethod/{id}")
}

/// One RawConfig node: its value and its named children in order.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub(crate) struct Tree {
    #[serde(rename = "v", default)]
    pub value: String,
    #[serde(rename = "c", default)]
    pub children: Vec<(String, Tree)>,
}

impl Tree {
    pub(crate) fn leaf(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            children: Vec::new(),
        }
    }

    pub(crate) fn get(&self, name: &str) -> Option<&Tree> {
        self.children
            .iter()
            .find(|(child, _)| child == name)
            .map(|(_, tree)| tree)
    }

    fn value_of(&self, name: &str) -> &str {
        self.get(name).map_or("", |tree| tree.value.as_str())
    }

    pub(crate) fn get_path(&self, path: &[String]) -> Option<&Tree> {
        path.iter().try_fold(self, |tree, name| tree.get(name))
    }

    /// The node at `path`, created when missing.
    pub(crate) fn entry_path(&mut self, path: &[String]) -> &mut Tree {
        let mut tree = self;
        for name in path {
            let index = match tree.children.iter().position(|(child, _)| child == name) {
                Some(index) => index,
                None => {
                    tree.children.push((name.clone(), Tree::default()));
                    tree.children.len() - 1
                }
            };
            tree = &mut tree.children[index].1;
        }
        tree
    }

    /// The items of a list node ("0", "1", ...) in index order.
    pub(crate) fn list_items(&self) -> Vec<String> {
        let mut items = Vec::new();
        while let Some(item) = self.get(&items.len().to_string()) {
            items.push(item.value.clone());
        }
        items
    }

    pub(crate) fn set_list_items(&mut self, items: &[String]) {
        self.value.clear();
        self.children = items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), Tree::leaf(item.clone())))
            .collect();
    }

    /// The "<path>\t<value>" lines the engine applies, one per leaf. An empty
    /// list is written as an empty leaf so it replaces the stored items.
    pub(crate) fn to_lines(&self) -> String {
        fn escape(value: &str, out: &mut String) {
            for character in value.chars() {
                match character {
                    '\\' => out.push_str("\\\\"),
                    '\t' => out.push_str("\\t"),
                    '\n' => out.push_str("\\n"),
                    _ => out.push(character),
                }
            }
        }
        fn walk(tree: &Tree, path: &mut Vec<String>, out: &mut String) {
            if tree.children.is_empty() {
                if !path.is_empty() {
                    out.push_str(&path.join("/"));
                    out.push('\t');
                    escape(&tree.value, out);
                    out.push('\n');
                }
                return;
            }
            for (name, child) in &tree.children {
                path.push(name.clone());
                walk(child, path, out);
                path.pop();
            }
        }
        let mut out = String::new();
        walk(self, &mut Vec::new(), &mut out);
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OptionKind {
    Boolean,
    Integer {
        min: Option<i64>,
        max: Option<i64>,
    },
    String,
    Enum {
        values: Vec<String>,
        labels: Vec<String>,
    },
    Key,
    KeyList,
    StringList,
    /// A nested configuration; its options are in `ConfigOption::children`.
    Group,
    /// Opens another configuration (`LaunchSubConfig`) or a native tool.
    External {
        uri: String,
        sub_config: bool,
    },
    /// A type Settings cannot edit yet, such as a list of sub configurations.
    Unsupported(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConfigOption {
    pub name: String,
    pub description: String,
    pub tooltip: String,
    pub kind: OptionKind,
    pub default: Tree,
    pub children: Vec<ConfigOption>,
}

/// A configuration ready to render: the options and their current values.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FcitxConfig {
    pub options: Vec<ConfigOption>,
    pub value: Tree,
}

#[derive(Deserialize)]
struct Payload {
    description: Tree,
    value: Tree,
}

/// Deepest nesting of sub configurations followed, against a cyclic description.
const MAX_GROUP_DEPTH: usize = 8;

fn options_of(description: &Tree, group: &Tree, depth: usize) -> Vec<ConfigOption> {
    group
        .children
        .iter()
        .map(|(name, spec)| option_of(description, name, spec, depth))
        .collect()
}

fn option_of(description: &Tree, name: &str, spec: &Tree, depth: usize) -> ConfigOption {
    let type_name = spec.value_of("Type");
    let mut children = Vec::new();
    let kind = match type_name {
        "Boolean" => OptionKind::Boolean,
        "Integer" => OptionKind::Integer {
            min: spec.value_of("IntMin").parse().ok(),
            max: spec.value_of("IntMax").parse().ok(),
        },
        "String" | "Color" => OptionKind::String,
        "Key" => OptionKind::Key,
        "List|Key" => OptionKind::KeyList,
        "List|String" => OptionKind::StringList,
        "Enum" => {
            let values = spec.get("Enum").map(Tree::list_items).unwrap_or_default();
            let mut labels = spec
                .get("EnumI18n")
                .map(Tree::list_items)
                .unwrap_or_default();
            if labels.len() != values.len() {
                labels = values.clone();
            }
            OptionKind::Enum { values, labels }
        }
        "External" => OptionKind::External {
            uri: spec.value_of("External").to_owned(),
            sub_config: spec.value_of("LaunchSubConfig") == "True",
        },
        other => match description.get(other) {
            Some(group) if depth < MAX_GROUP_DEPTH => {
                children = options_of(description, group, depth + 1);
                OptionKind::Group
            }
            _ => OptionKind::Unsupported(other.to_owned()),
        },
    };
    ConfigOption {
        name: name.to_owned(),
        description: spec.value_of("Description").to_owned(),
        tooltip: spec.value_of("Tooltip").to_owned(),
        kind,
        default: spec.get("DefaultValue").cloned().unwrap_or_default(),
        children,
    }
}

/// Parses the engine's `{"description", "value"}` payload.
pub(crate) fn parse_config(json: &str) -> Result<FcitxConfig, String> {
    let payload: Payload = serde_json::from_str(json).map_err(|_| ERROR_INVALID.to_owned())?;
    // The root group is the configuration's own type: the one entry whose
    // name carries no "<path>$" prefix of a sub configuration.
    let root = payload
        .description
        .children
        .iter()
        .find(|(name, _)| !name.contains('$'))
        .map(|(_, group)| group)
        .ok_or_else(|| ERROR_INVALID.to_owned())?;
    Ok(FcitxConfig {
        options: options_of(&payload.description, root, 0),
        value: payload.value,
    })
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub(crate) struct AddonEntry {
    pub id: String,
    pub name: String,
    pub category: String,
    pub configurable: bool,
    pub loaded: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub(crate) struct InputMethodEntry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub native_name: String,
    #[serde(default)]
    pub addon: String,
    pub configurable: bool,
}

pub(crate) fn parse_addons(json: &str) -> Result<Vec<AddonEntry>, String> {
    serde_json::from_str(json).map_err(|_| ERROR_INVALID.to_owned())
}

pub(crate) fn parse_input_methods(json: &str) -> Result<Vec<InputMethodEntry>, String> {
    serde_json::from_str(json).map_err(|_| ERROR_INVALID.to_owned())
}

/// The engine executables Settings accepts as the pipe server: the runtime
/// generation copy and the top-level `bin` copy.
fn engine_paths(install_root: &Path, generation: &str) -> Vec<PathBuf> {
    [
        install_root
            .join("runtime")
            .join(generation)
            .join("bin")
            .join("fcitx5-engine.exe"),
        install_root.join("bin").join("fcitx5-engine.exe"),
    ]
    .into_iter()
    .filter(|path| path.is_file())
    .collect()
}

fn engine_request(
    install_root: &Path,
    operation: ConfigOperation,
    uri: &str,
    value: &str,
) -> Result<String, String> {
    let generation = current_runtime_generation_for_current_process();
    let pipe = EngineClient::default_pipe(&generation).ok_or(ERROR_ENGINE_UNAVAILABLE)?;
    for engine in engine_paths(install_root, &generation) {
        let Some(mut client) = EngineClient::new(pipe.clone(), engine) else {
            continue;
        };
        if let Some(payload) = client.config(operation, uri, value) {
            return String::from_utf8(payload).map_err(|_| ERROR_INVALID.to_owned());
        }
    }
    Err(ERROR_NO_RESPONSE.to_owned())
}

pub(crate) fn engine_get(install_root: &Path, uri: &str) -> Result<String, String> {
    engine_request(install_root, ConfigOperation::Get, uri, "")
}

pub(crate) fn engine_set(install_root: &Path, uri: &str, value: &Tree) -> Result<(), String> {
    engine_request(install_root, ConfigOperation::Set, uri, &value.to_lines()).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PINYIN: &str = r#"{"description":{"v":"","c":[
        ["PinyinEngineConfig",{"v":"","c":[
            ["ShuangpinProfile",{"v":"","c":[
                ["Type",{"v":"Enum","c":[]}],
                ["Description",{"v":"双拼方案","c":[]}],
                ["DefaultValue",{"v":"Ziranma","c":[]}],
                ["Enum",{"v":"","c":[["0",{"v":"Ziranma","c":[]}],["1",{"v":"Xiaohe","c":[]}]]}],
                ["EnumI18n",{"v":"","c":[["0",{"v":"自然码","c":[]}],["1",{"v":"小鹤","c":[]}]]}]
            ]}],
            ["PageSize",{"v":"","c":[
                ["Type",{"v":"Integer","c":[]}],
                ["Description",{"v":"每页候选词数","c":[]}],
                ["DefaultValue",{"v":"7","c":[]}],
                ["IntMin",{"v":"3","c":[]}],
                ["IntMax",{"v":"10","c":[]}]
            ]}],
            ["Fuzzy",{"v":"","c":[
                ["Type",{"v":"Fuzzy$FuzzyConfig","c":[]}],
                ["Description",{"v":"模糊音","c":[]}]
            ]}],
            ["PrevPage",{"v":"","c":[
                ["Type",{"v":"List|Key","c":[]}],
                ["Description",{"v":"上一页","c":[]}]
            ]}]
        ]}],
        ["Fuzzy$FuzzyConfig",{"v":"","c":[
            ["VE_UE",{"v":"","c":[
                ["Type",{"v":"Boolean","c":[]}],
                ["Description",{"v":"ue -> ve","c":[]}],
                ["DefaultValue",{"v":"True","c":[]}]
            ]}]
        ]}]
    ]},"value":{"v":"","c":[
        ["ShuangpinProfile",{"v":"Xiaohe","c":[]}],
        ["PageSize",{"v":"5","c":[]}],
        ["Fuzzy",{"v":"","c":[["VE_UE",{"v":"False","c":[]}]]}],
        ["PrevPage",{"v":"","c":[["0",{"v":"minus","c":[]}],["1",{"v":"Up","c":[]}]]}]
    ]}}"#;

    #[test]
    fn builds_options_from_the_description() {
        let config = parse_config(PINYIN).unwrap();
        let names: Vec<_> = config.options.iter().map(|o| o.name.as_str()).collect();
        assert_eq!(names, ["ShuangpinProfile", "PageSize", "Fuzzy", "PrevPage"]);
        assert_eq!(
            config.options[0].kind,
            OptionKind::Enum {
                values: vec!["Ziranma".into(), "Xiaohe".into()],
                labels: vec!["自然码".into(), "小鹤".into()],
            }
        );
        assert_eq!(
            config.options[1].kind,
            OptionKind::Integer {
                min: Some(3),
                max: Some(10)
            }
        );
        assert_eq!(config.options[2].kind, OptionKind::Group);
        assert_eq!(config.options[2].children[0].name, "VE_UE");
        assert_eq!(config.options[2].children[0].default.value, "True");
        assert_eq!(config.options[3].kind, OptionKind::KeyList);
        assert_eq!(
            config.value.get("PrevPage").unwrap().list_items(),
            ["minus", "Up"]
        );
    }

    #[test]
    fn writes_every_leaf_and_empty_lists() {
        let mut config = parse_config(PINYIN).unwrap();
        config
            .value
            .entry_path(&["PrevPage".to_owned()])
            .set_list_items(&[]);
        config
            .value
            .entry_path(&["Fuzzy".to_owned(), "VE_UE".to_owned()])
            .value = "True".into();
        assert_eq!(
            config.value.to_lines(),
            "ShuangpinProfile\tXiaohe\nPageSize\t5\nFuzzy/VE_UE\tTrue\nPrevPage\t\n"
        );
        let escaped = Tree {
            value: String::new(),
            children: vec![("A".into(), Tree::leaf("a\\b\tc\nd"))],
        };
        assert_eq!(escaped.to_lines(), "A\ta\\\\b\\tc\\nd\n");
    }

    #[test]
    fn parses_the_lists() {
        let addons = parse_addons(
            r#"[{"id":"pinyin","name":"拼音","category":"InputMethod","configurable":true,"loaded":true}]"#,
        )
        .unwrap();
        assert_eq!(addons[0].id, "pinyin");
        let ims = parse_input_methods(
            r#"[{"id":"shuangpin","name":"双拼","native_name":"","addon":"pinyin","configurable":true}]"#,
        )
        .unwrap();
        assert!(ims[0].configurable);
    }
}
