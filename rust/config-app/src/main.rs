#![forbid(unsafe_code)]

mod input_methods;
mod settings_locale;
use input_methods::{discover_input_methods, input_method_directories, InputMethodChoice};
use settings_locale::{is_supported_locale, LocaleCatalog};

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use fcitx5_config_core::{
    CandidateOrientation as CoreCandidateOrientation, ConfigCommand, ConfigCore, ConfigEdit,
    ConfigField, ConfigSnapshot, FileStore, OverflowBehavior as CoreOverflowBehavior,
    RecoverySource, ScrollDirection, VerticalTextColumnDirection, WritingMode as CoreWritingMode,
};
use fcitx5_control_core::{
    control_startup_query, control_startup_set, control_themes_list_json_for_paths,
};
use fcitx5_package_core::PackageId;
use fcitx5_process_execution_core::run_process_bounded;
use serde::Deserialize;
use windui::icon::WindowIcon as WindUiWindowIcon;
use windui::prelude::{
    signal as windui_signal, Align as WindUiAlign, App as WindUiApp, Color as WindUiColor,
    Element as WindUiElement, Fit as WindUiFit, Intent as WindUiIntent, Role as WindUiRole,
    Sender as WindUiSender, Signal as WindUiSignal, Theme as WindUiTheme,
    ThemeHandle as WindUiThemeHandle, Truncate as WindUiTruncate,
    WindowButtonKind as WindUiWindowButtonKind,
};

const PRODUCT_NAME: &str = "Fcitx5 for Windows Next";
const CONFIG_DEV_COMPONENT: &str = "fcitx5-config-dev";
const CONFIG_SHIPPING_COMPONENT: &str = "fcitx5-config";

fn windui_settings_shell_theme(dark: bool) -> WindUiTheme {
    let mut theme = if dark {
        WindUiTheme::dark()
    } else {
        WindUiTheme::default()
    };
    apply_wechat_green_palette(&mut theme, dark);
    theme.form.label_size = Some(15.0);
    theme.form.label_weight = Some(600);
    theme.form.desc_size = Some(12.5);
    theme.form.row_height = Some(44);
    theme.form.row_pad_y = Some(0);
    theme
}

thread_local! {
    /// The running Settings window's theme, so every place that changes the
    /// appearance-mode draft can repaint the shell without threading a handle.
    static SHELL_THEME: std::cell::RefCell<Option<WindUiThemeHandle>> =
        const { std::cell::RefCell::new(None) };
}

/// Whether the Settings shell should be dark for an `appearance.mode` value.
fn shell_uses_dark(mode: &str, system_dark: bool) -> bool {
    match mode {
        "dark" => true,
        "light" => false,
        _ => system_dark,
    }
}

/// Repaints the Settings shell for the draft's appearance mode.
fn sync_settings_shell_theme(adapter: WindUiSignal<WindUiConfigAdapter>) {
    let mode = adapter.with(|adapter| adapter.preview().appearance().mode().to_owned());
    let dark = shell_uses_dark(&mode, windui::event::system_prefers_dark());
    SHELL_THEME.with(|slot| {
        if let Some(handle) = slot.borrow().as_ref() {
            if handle.current().palette.bg != windui_settings_shell_theme(dark).palette.bg {
                handle.set(windui_settings_shell_theme(dark));
            }
        }
    });
}

fn apply_wechat_green_palette(theme: &mut WindUiTheme, dark: bool) {
    theme.palette.accent = WindUiColor::hex(0x07C160);
    theme.palette.accent_hover = WindUiColor::hex(0x12D070);
    theme.palette.accent_active = WindUiColor::hex(0x06AD56);
    if dark {
        theme.palette.bg = WindUiColor::hex(0x181818);
        theme.palette.surface = WindUiColor::hex(0x1F1F1F);
        theme.palette.surface_alt = WindUiColor::hex(0x262626);
        theme.palette.border = WindUiColor::hex(0x303030);
        theme.palette.track = WindUiColor::hex(0x303030);
        theme.palette.divider = WindUiColor::hex(0x2A2A2A);
    }
}

const WINDUI_SETTINGS_TITLEBAR_HEIGHT: i32 = 44;

const FCITX5_LOGO_PNG: &[u8] = include_bytes!("../../../resources/icons/fcitx5-256.png");

/// The Fcitx5 logo for the window and title bar, decoded from the embedded PNG.
fn fcitx5_logo() -> Option<WindUiWindowIcon> {
    let image = windui::render::Image::from_png_bytes(FCITX5_LOGO_PNG).ok()?;
    WindUiWindowIcon::from_image(&image)
}

fn windui_brand_logo(size: i32) -> WindUiElement {
    let Some(icon) = fcitx5_logo() else {
        return WindUiElement::col().size(size, size);
    };
    WindUiElement::image_rgba(icon.width(), icon.height(), icon.rgba())
        .fit(WindUiFit::Contain)
        .size(size, size)
}

fn windui_settings_shell_wrap(subtitle: &str, body: WindUiElement) -> WindUiElement {
    let titlebar = WindUiElement::row()
        .width_match()
        .height(WINDUI_SETTINGS_TITLEBAR_HEIGHT)
        .cross(WindUiAlign::Center)
        .padding_xy(14, 0)
        .spacing(10)
        .bg_role(WindUiRole::SurfaceAlt)
        .window_drag()
        .child(windui_brand_logo(22))
        .child(
            WindUiElement::row()
                .cross(WindUiAlign::Center)
                .spacing(5)
                .child(
                    WindUiElement::label("Fcitx5")
                        .font_size(13.0)
                        .font_weight(600)
                        .fg_role(WindUiRole::Text),
                )
                .child(
                    WindUiElement::label("·")
                        .font_size(13.0)
                        .fg_role(WindUiRole::TextDisabled),
                )
                .child(
                    WindUiElement::label(subtitle)
                        .font_size(13.0)
                        .fg_role(WindUiRole::TextMuted),
                ),
        )
        .child(WindUiElement::leaf().weight(1.0))
        .child(
            WindUiElement::window_button(WindUiWindowButtonKind::Minimize)
                .fg_role(WindUiRole::Text),
        )
        .child(
            WindUiElement::window_button(WindUiWindowButtonKind::Maximize)
                .fg_role(WindUiRole::Text),
        )
        .child(
            WindUiElement::window_button(WindUiWindowButtonKind::Close).fg_role(WindUiRole::Text),
        );

    WindUiElement::col()
        .fill()
        .bg_role(WindUiRole::Bg)
        .child(titlebar)
        .child(WindUiElement::divider())
        .child(body.weight(1.0))
}

fn windui_settings_section_title(title: &str) -> WindUiElement {
    WindUiElement::row()
        .cross(WindUiAlign::Center)
        .spacing(10)
        .child(
            WindUiElement::leaf()
                .size(4, 18)
                .corner(2.0)
                .bg_role(WindUiRole::Accent),
        )
        .child(
            WindUiElement::label(title)
                .font_size(15.0)
                .font_weight(700)
                .fg_role(WindUiRole::Text),
        )
}

fn windui_settings_card(body: WindUiElement) -> WindUiElement {
    WindUiElement::col()
        .width_match()
        .bg_role(WindUiRole::Surface)
        .corner(12.0)
        .border_role(WindUiRole::Border, 1)
        .padding(20)
        .spacing(14)
        .child(body)
}

fn windui_settings_nav_item(
    name: String,
    glyph: &'static str,
    i: usize,
    selected: WindUiSignal<usize>,
) -> WindUiElement {
    let chip = |active: bool| {
        WindUiElement::stack()
            .size(26, 26)
            .corner(7.0)
            .bg_role(if active {
                WindUiRole::Accent
            } else {
                WindUiRole::Surface
            })
            .child(
                WindUiElement::label(glyph)
                    .font_size(14.0)
                    .fg_role(if active {
                        WindUiRole::OnAccent
                    } else {
                        WindUiRole::TextMuted
                    })
                    .align(WindUiAlign::Center),
            )
    };

    let on = WindUiElement::row()
        .width_match()
        .height(38)
        .corner(9.0)
        .cross(WindUiAlign::Center)
        .spacing(10)
        .padding_xy(10, 0)
        .bg_role_alpha(WindUiRole::Accent, 0.12)
        .child(chip(true))
        .child(
            WindUiElement::label(&name)
                .font_size(13.0)
                .font_weight(600)
                .fg_role(WindUiRole::Accent)
                .weight(1.0)
                .max_lines(1),
        )
        .visible_when(move || selected.get() == i);

    let off = WindUiElement::row()
        .clickable()
        .on_click(move |_| selected.set(i))
        .width_match()
        .height(38)
        .corner(9.0)
        .cross(WindUiAlign::Center)
        .spacing(10)
        .padding_xy(10, 0)
        .child(chip(false))
        .child(
            WindUiElement::label(&name)
                .font_size(13.0)
                .font_weight(500)
                .fg_role(WindUiRole::TextMuted)
                .weight(1.0)
                .max_lines(1),
        )
        .visible_when(move || selected.get() != i);

    let indicator = WindUiElement::row()
        .width_match()
        .height(38)
        .cross(WindUiAlign::Center)
        .child(
            WindUiElement::leaf()
                .width(3)
                .height(16)
                .corner(1.5)
                .bg_role(WindUiRole::Accent),
        )
        .visible_when(move || selected.get() == i);

    WindUiElement::stack()
        .width_match()
        .height(38)
        .child(on)
        .child(off)
        .child(indicator)
}

fn windui_settings_page_title(title: &str, subtitle: &str) -> WindUiElement {
    WindUiElement::row()
        .width_match()
        .cross(WindUiAlign::Center)
        .spacing(10)
        .child(
            WindUiElement::label(title)
                .font_size(24.0)
                .font_weight(700)
                .fg_role(WindUiRole::Text),
        )
        .child(
            WindUiElement::label(subtitle)
                .font_size(13.0)
                .fg_role(WindUiRole::TextMuted)
                .weight(1.0),
        )
}

#[derive(Clone, Copy)]
struct PluginCatalogEntry {
    id: &'static str,
    #[allow(dead_code)]
    windows_package: bool,
}

const FCITX5_PLUGINS_REFERENCE_COMMIT: &str = "26a94720f0a01e106046f6a7607215ff96bf2f6f";
const CONTROL_TIMEOUT_MS: u32 = 120_000;
/// Install and update download the package first, which can take far longer than a local
/// operation on a slow connection; stopping them early only leaves work to roll back.
const CONTROL_TRANSFER_TIMEOUT_MS: u32 = 30 * 60 * 1000;
const CONTROL_MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;
const FCITX5_PLUGIN_CATALOG: &[PluginCatalogEntry] = &[
    PluginCatalogEntry {
        id: "fcitx5-chinese-addons",
        windows_package: true,
    },
    PluginCatalogEntry {
        id: "fcitx5-chewing",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-jyutping",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-zhuyin",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-mozc",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-skk",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-anthy",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-kkc",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-cskk",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-hangul",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-hallelujah",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-sayura",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-thai",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-bamboo",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-unikey",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-rime",
        windows_package: true,
    },
    PluginCatalogEntry {
        id: "fcitx5-m17n",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-keyman",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-lua",
        windows_package: true,
    },
];

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ControlPackage {
    id: String,
    title: String,
    summary: String,
    #[serde(rename = "type")]
    package_type: String,
    available_version: Option<String>,
    installed_version: Option<String>,
    state: Option<String>,
    update_available: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ControlPackageList {
    format_version: u32,
    repository_available: bool,
    repository_error: Option<String>,
    packages: Vec<ControlPackage>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PluginManagerSnapshot {
    loaded: bool,
    repository_available: bool,
    repository_error: Option<String>,
    packages: Vec<ControlPackage>,
}

impl PluginManagerSnapshot {
    fn package(&self, id: &str) -> Option<&ControlPackage> {
        self.packages.iter().find(|package| package.id == id)
    }
}

/// A row shown by the plugin manager. The reference catalog supplies stable
/// human-facing grouping, while Control remains authoritative for package
/// identity, metadata, availability, and lifecycle state.
#[derive(Clone, Debug, PartialEq, Eq)]
struct PluginCatalogRow {
    id: String,
    title: String,
    summary: String,
    category: String,
    package: Option<ControlPackage>,
    loaded: bool,
}

fn is_reference_plugin(id: &str) -> Option<&'static PluginCatalogEntry> {
    FCITX5_PLUGIN_CATALOG.iter().find(|plugin| plugin.id == id)
}

fn is_plugin_package(package: &ControlPackage) -> bool {
    matches!(package.package_type.as_str(), "addon" | "inputmethod-data")
        || (package.package_type == "unknown" && package.installed_version.is_some())
}

fn plugin_package_type_label(locale: &LocaleCatalog, package_type: &str) -> String {
    let key = match package_type {
        "addon" => Some("packages.type.addon"),
        "inputmethod-data" => Some("packages.type.input_method_data"),
        "theme" => Some("packages.type.theme"),
        "translation" => Some("packages.type.translation"),
        "core" => Some("packages.type.core"),
        "component" => Some("packages.type.component"),
        _ => None,
    };
    key.map(|key| locale.text(key))
        .unwrap_or_else(|| package_type.to_owned())
}

fn plugin_catalog_rows(
    snapshot: &PluginManagerSnapshot,
    locale: &LocaleCatalog,
) -> Vec<PluginCatalogRow> {
    let mut rows = Vec::with_capacity(FCITX5_PLUGIN_CATALOG.len() + snapshot.packages.len());
    for reference in FCITX5_PLUGIN_CATALOG {
        let package = snapshot
            .package(reference.id)
            .filter(|package| is_plugin_package(package))
            .cloned();
        let (title, summary, category) = package
            .as_ref()
            .map(|package| {
                (
                    package.title.clone(),
                    package.summary.clone(),
                    plugin_row_category(locale, package),
                )
            })
            .unwrap_or_else(|| {
                (
                    reference.id.to_owned(),
                    locale.text("settings.plugins.fallback_metadata"),
                    locale.text("settings.plugins.fallback_category"),
                )
            });
        rows.push(PluginCatalogRow {
            id: reference.id.to_owned(),
            title,
            summary,
            category,
            package,
            loaded: snapshot.loaded,
        });
    }

    // Addons first, then input method data such as tables, each by id.
    let mut extra = snapshot
        .packages
        .iter()
        .filter(|package| is_plugin_package(package) && is_reference_plugin(&package.id).is_none())
        .cloned()
        .collect::<Vec<_>>();
    extra.sort_by(|left, right| {
        (left.package_type == "inputmethod-data", &left.id)
            .cmp(&(right.package_type == "inputmethod-data", &right.id))
    });
    rows.extend(extra.into_iter().map(|package| PluginCatalogRow {
        id: package.id.clone(),
        title: package.title.clone(),
        summary: package.summary.clone(),
        category: plugin_row_category(locale, &package),
        package: Some(package),
        loaded: snapshot.loaded,
    }));
    rows
}

fn plugin_row_category(locale: &LocaleCatalog, package: &ControlPackage) -> String {
    if package.package_type == "inputmethod-data" {
        plugin_package_type_label(locale, &package.package_type)
    } else {
        locale.text("settings.plugins.fallback_category")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum PluginOperation {
    List,
    /// The first list on page open. When no verified catalog has been
    /// downloaded yet, the page follows it with a Refresh.
    InitialLoad,
    Refresh,
    Install(String),
    Update(String),
    SetState {
        id: String,
        enabled: bool,
    },
    Remove(String),
    Repair,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PluginAction {
    Install,
    Update,
    Toggle,
    Remove,
}

impl PluginOperation {
    fn label(&self, locale: &LocaleCatalog) -> String {
        match self {
            Self::List | Self::InitialLoad => locale.text("settings.plugins.operation.list"),
            Self::Refresh => locale.text("settings.plugins.operation.refresh"),
            Self::Install(_) => locale.text("settings.plugins.operation.install"),
            Self::Update(_) => locale.text("settings.plugins.operation.update"),
            Self::SetState { enabled: true, .. } => {
                locale.text("settings.plugins.operation.enable")
            }
            Self::SetState { enabled: false, .. } => {
                locale.text("settings.plugins.operation.disable")
            }
            Self::Remove(_) => locale.text("settings.plugins.operation.remove"),
            Self::Repair => locale.text("settings.plugins.operation.repair"),
        }
    }
}

#[derive(Debug)]
struct PluginResponse {
    operation: PluginOperation,
    result: Result<PluginManagerSnapshot, String>,
}

fn valid_package_id(id: &str) -> bool {
    PackageId::parse(id).is_ok()
}

fn plugin_control_arguments(operation: &PluginOperation) -> Result<Vec<OsString>, String> {
    let package_id = |id: &str| {
        valid_package_id(id)
            .then(|| OsString::from(id))
            .ok_or_else(|| format!("插件包 ID 无效：{id}"))
    };
    Ok(match operation {
        PluginOperation::List | PluginOperation::InitialLoad => {
            vec![OsString::from("--packages-list")]
        }
        PluginOperation::Refresh => vec![OsString::from("--packages-refresh")],
        PluginOperation::Install(id) => {
            vec![OsString::from("--packages-install"), package_id(id)?]
        }
        PluginOperation::Update(id) => {
            vec![OsString::from("--packages-update"), package_id(id)?]
        }
        PluginOperation::SetState { id, enabled } => vec![
            OsString::from("--packages-state"),
            package_id(id)?,
            OsString::from(if *enabled { "enabled" } else { "disabled" }),
        ],
        PluginOperation::Remove(id) => {
            vec![OsString::from("--packages-remove"), package_id(id)?]
        }
        PluginOperation::Repair => vec![OsString::from("--packages-repair")],
    })
}

fn plugin_operation_for(
    snapshot: &PluginManagerSnapshot,
    id: &str,
    action: PluginAction,
) -> Option<PluginOperation> {
    let package = snapshot.package(id)?;
    if !is_plugin_package(package) {
        return None;
    }
    match action {
        PluginAction::Install => (snapshot.repository_available
            && package.available_version.is_some()
            && package.installed_version.is_none()
            && !matches!(
                package.state.as_deref(),
                Some("trust-failed" | "incompatible" | "pending-restart")
            ))
        .then(|| PluginOperation::Install(id.to_owned())),
        PluginAction::Update => (package.update_available
            && package_allows_installed_action(package))
        .then(|| PluginOperation::Update(id.to_owned())),
        PluginAction::Toggle => {
            package_allows_installed_action(package).then(|| PluginOperation::SetState {
                id: id.to_owned(),
                enabled: package.state.as_deref() == Some("disabled"),
            })
        }
        PluginAction::Remove => {
            package_allows_installed_action(package).then(|| PluginOperation::Remove(id.to_owned()))
        }
    }
}

fn parse_control_package_list(output: &str) -> Result<PluginManagerSnapshot, String> {
    if output.len() > CONTROL_MAX_OUTPUT_BYTES {
        return Err("Control 返回的插件目录超过大小限制".to_owned());
    }
    let parsed: ControlPackageList = serde_json::from_str(output)
        .map_err(|error| format!("Control 插件目录 JSON 无效：{error}"))?;
    if parsed.format_version != 1 || parsed.packages.len() > 4096 {
        return Err("Control 插件目录版本或条目数无效".to_owned());
    }
    if parsed
        .repository_error
        .as_ref()
        .is_some_and(|error| error.len() > 64)
        || parsed.packages.iter().any(|package| {
            PackageId::parse(&package.id).is_err()
                || package.title.is_empty()
                || package.title.len() > 256
                || package.summary.len() > 4096
                || package.package_type.is_empty()
                || package.package_type.len() > 64
                || package
                    .available_version
                    .as_ref()
                    .is_some_and(|version| version.is_empty() || version.len() > 128)
                || package
                    .installed_version
                    .as_ref()
                    .is_some_and(|version| version.is_empty() || version.len() > 128)
                || package.state.as_deref().is_some_and(|state| {
                    !matches!(
                        state,
                        "installed"
                            | "enabled"
                            | "disabled"
                            | "pending_update"
                            | "pending_remove"
                            | "broken"
                            | "quarantined"
                            | "bundled"
                            | "trust-failed"
                            | "incompatible"
                            | "pending-restart"
                    )
                })
        })
    {
        return Err("Control 插件目录字段无效".to_owned());
    }
    Ok(PluginManagerSnapshot {
        loaded: true,
        repository_available: parsed.repository_available,
        repository_error: parsed.repository_error,
        packages: parsed.packages,
    })
}

fn localized_plugin_error(locale: &LocaleCatalog, error: &str) -> String {
    if let Some(id) = error.strip_prefix("插件包 ID 无效：") {
        return locale.format("settings.plugins.error.invalid_id", &[("id", id)]);
    }
    let key = match error {
        "Control 返回的插件目录超过大小限制" => {
            "settings.plugins.error.catalog_too_large"
        }
        "Control 插件目录版本或条目数无效" => {
            "settings.plugins.error.catalog_version_invalid"
        }
        "Control 插件目录字段无效" => "settings.plugins.error.catalog_fields_invalid",
        "配置程序路径没有父目录" => "settings.plugins.error.control_unavailable",
        _ if error.starts_with("Control 插件目录 JSON 无效：")
            || error.starts_with("无法定位配置程序：")
            || error.starts_with("运行 fcitx5-control.exe 失败：")
            || error.starts_with("fcitx5-control.exe 返回失败") =>
        {
            "settings.plugins.error.control_unavailable"
        }
        _ => return error.to_owned(),
    };
    locale.text(key)
}

fn control_executable() -> Result<PathBuf, String> {
    let executable = env::current_exe().map_err(|error| format!("无法定位配置程序：{error}"))?;
    let directory = executable
        .parent()
        .ok_or_else(|| "配置程序路径没有父目录".to_owned())?;
    Ok(directory.join("fcitx5-control.exe"))
}

fn run_control(arguments: &[OsString]) -> Result<String, String> {
    run_control_within(arguments, CONTROL_TIMEOUT_MS)
}

fn run_control_within(arguments: &[OsString], timeout_ms: u32) -> Result<String, String> {
    let executable = control_executable()?;
    let result = run_process_bounded(&executable, arguments, timeout_ms, CONTROL_MAX_OUTPUT_BYTES)
        .map_err(|error| format!("运行 fcitx5-control.exe 失败：{error}"))?;
    if result.success {
        Ok(result.output)
    } else {
        let detail = result.output.trim();
        Err(if detail.is_empty() {
            "fcitx5-control.exe 返回失败".to_owned()
        } else {
            format!("fcitx5-control.exe 返回失败：{detail}")
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// UI-side mirror of the legacy single `layout_type` string Config Core still
/// persists. Decoded once from the snapshot and re-encoded on every change.
enum CandidateLayoutMode {
    Automatic,
    Stacked,
    Flow,
    Scroll,
    VerticalText,
}

impl CandidateLayoutMode {
    #[cfg_attr(not(test), allow(dead_code))]
    fn label(self) -> &'static str {
        match self {
            Self::Automatic => "自动",
            Self::Stacked => "纵排",
            Self::Flow => "横排",
            Self::Scroll => "卷轴",
            Self::VerticalText => "竖排文字",
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn preview_description(self, page_size: u8) -> String {
        match self {
            Self::Automatic => "自动布局".to_owned(),
            Self::Stacked => "纵排布局（候选逐行）".to_owned(),
            Self::Flow => "横排布局（多行分页）".to_owned(),
            Self::Scroll => format!("卷轴布局（每页最多 {page_size} 个候选）"),
            Self::VerticalText => "竖排文字布局".to_owned(),
        }
    }
}

/// UI-side mirror of `fcitx5_config_core::CandidateOrientation`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CandidateLayoutOrientation {
    #[default]
    Horizontal,
    Vertical,
}

/// UI-side mirror of `fcitx5_config_core::OverflowBehavior`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CandidateLayoutOverflow {
    #[default]
    Paging,
    Scrolling,
    Wrapping,
}

/// UI-side mirror of `fcitx5_config_core::WritingMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CandidateWritingMode {
    #[default]
    Horizontal,
    VerticalRl,
    VerticalLr,
}

/// The two-axis UI state decoded from the legacy `layout_type` string exactly
/// as `fcitx5_config_core::decode_candidate_layout_options` decodes it, plus
/// the explicit writing-mode axis that legacy maps onto `vertical_text`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CandidateLayoutUiOptions {
    orientation: CandidateLayoutOrientation,
    overflow: CandidateLayoutOverflow,
    writing_mode: CandidateWritingMode,
    scroll_direction: ScrollDirection,
    vertical_text_column_direction: VerticalTextColumnDirection,
}

impl CandidateLayoutUiOptions {
    #[allow(dead_code)]
    fn overflow_choices(self) -> &'static [CandidateLayoutOverflow] {
        match self.orientation {
            CandidateLayoutOrientation::Horizontal => &[
                CandidateLayoutOverflow::Paging,
                CandidateLayoutOverflow::Scrolling,
                CandidateLayoutOverflow::Wrapping,
            ],
            CandidateLayoutOrientation::Vertical => &[
                CandidateLayoutOverflow::Paging,
                CandidateLayoutOverflow::Scrolling,
            ],
        }
    }

    /// Encodes back to the single legacy `layout_type` string Config Core
    /// accepts, mirroring the decode table: V+Paging→stacked, H+Wrapping→flow,
    /// (H|V)+Scrolling→scroll, VerticalRl/Lr→vertical_text, H+Paging→automatic.
    fn layout_type(self) -> &'static str {
        match self.writing_mode {
            CandidateWritingMode::Horizontal => match (self.orientation, self.overflow) {
                (CandidateLayoutOrientation::Horizontal, CandidateLayoutOverflow::Paging) => {
                    "automatic"
                }
                (CandidateLayoutOrientation::Vertical, CandidateLayoutOverflow::Paging) => {
                    "stacked"
                }
                (_, CandidateLayoutOverflow::Scrolling) => "scroll",
                (CandidateLayoutOrientation::Horizontal, CandidateLayoutOverflow::Wrapping) => {
                    "flow"
                }
                // Vertical + Wrapping is not selectable; Vertical + Scrolling
                // above covers every other reachable pair.
                (CandidateLayoutOrientation::Vertical, CandidateLayoutOverflow::Wrapping) => {
                    "stacked"
                }
            },
            CandidateWritingMode::VerticalRl | CandidateWritingMode::VerticalLr => "vertical_text",
        }
    }

    #[allow(dead_code)]
    fn with_writing(self, writing_mode: CandidateWritingMode) -> Self {
        match writing_mode {
            CandidateWritingMode::Horizontal => Self {
                writing_mode,
                ..self
            },
            _ => Self {
                orientation: CandidateLayoutOrientation::Vertical,
                overflow: CandidateLayoutOverflow::Paging,
                writing_mode,
                ..self
            },
        }
    }
}

impl CandidateLayoutMode {
    /// Decodes the legacy mode into the two-axis model exactly as Config Core
    /// does (`layout_type` → orientation/overflow, writing→`vertical_text`).
    fn options(self) -> CandidateLayoutUiOptions {
        match self {
            Self::Automatic => CandidateLayoutUiOptions {
                orientation: CandidateLayoutOrientation::Horizontal,
                overflow: CandidateLayoutOverflow::Paging,
                writing_mode: CandidateWritingMode::Horizontal,
                scroll_direction: ScrollDirection::Horizontal,
                vertical_text_column_direction: VerticalTextColumnDirection::RightToLeft,
            },
            Self::Stacked => CandidateLayoutUiOptions {
                orientation: CandidateLayoutOrientation::Vertical,
                overflow: CandidateLayoutOverflow::Paging,
                writing_mode: CandidateWritingMode::Horizontal,
                scroll_direction: ScrollDirection::Horizontal,
                vertical_text_column_direction: VerticalTextColumnDirection::RightToLeft,
            },
            Self::Flow => CandidateLayoutUiOptions {
                orientation: CandidateLayoutOrientation::Horizontal,
                overflow: CandidateLayoutOverflow::Wrapping,
                writing_mode: CandidateWritingMode::Horizontal,
                scroll_direction: ScrollDirection::Horizontal,
                vertical_text_column_direction: VerticalTextColumnDirection::RightToLeft,
            },
            Self::Scroll => CandidateLayoutUiOptions {
                orientation: CandidateLayoutOrientation::Horizontal,
                overflow: CandidateLayoutOverflow::Scrolling,
                writing_mode: CandidateWritingMode::Horizontal,
                scroll_direction: ScrollDirection::Horizontal,
                vertical_text_column_direction: VerticalTextColumnDirection::RightToLeft,
            },
            Self::VerticalText => CandidateLayoutUiOptions {
                orientation: CandidateLayoutOrientation::Vertical,
                overflow: CandidateLayoutOverflow::Paging,
                writing_mode: CandidateWritingMode::VerticalRl,
                scroll_direction: ScrollDirection::Horizontal,
                vertical_text_column_direction: VerticalTextColumnDirection::RightToLeft,
            },
        }
    }
}

impl CandidateLayoutUiOptions {
    #[allow(dead_code)]
    fn preview_description(self, page_size: u8) -> String {
        match self.writing_mode {
            CandidateWritingMode::Horizontal => match (self.orientation, self.overflow) {
                (CandidateLayoutOrientation::Horizontal, CandidateLayoutOverflow::Paging) => {
                    format!("横排布局（分页，每页最多 {page_size} 个候选）")
                }
                (CandidateLayoutOrientation::Vertical, CandidateLayoutOverflow::Paging) => {
                    format!("纵排布局（分页，每页最多 {page_size} 个候选）")
                }
                (_, CandidateLayoutOverflow::Scrolling) => {
                    "卷轴布局（视口滚动，选中项始终可见）".to_owned()
                }
                (CandidateLayoutOrientation::Horizontal, CandidateLayoutOverflow::Wrapping) => {
                    "横排自动换行（按实际宽度折行，行满后分页）".to_owned()
                }
                (CandidateLayoutOrientation::Vertical, CandidateLayoutOverflow::Wrapping) => {
                    format!("纵排布局（分页，每页最多 {page_size} 个候选）")
                }
            },
            CandidateWritingMode::VerticalRl => "竖排文字（从右到左）".to_owned(),
            CandidateWritingMode::VerticalLr => "竖排文字（从左到右）".to_owned(),
        }
    }
}

/// Control's error code when no verified repository cache exists yet.
const REPOSITORY_NOT_DOWNLOADED: &str = "repository_unavailable";

fn execute_plugin_operation(operation: &PluginOperation) -> Result<PluginManagerSnapshot, String> {
    if matches!(operation, PluginOperation::InitialLoad) {
        return execute_plugin_operation(&PluginOperation::List);
    }
    let arguments = plugin_control_arguments(operation)?;
    let timeout_ms = match operation {
        PluginOperation::Refresh | PluginOperation::Install(_) | PluginOperation::Update(_) => {
            CONTROL_TRANSFER_TIMEOUT_MS
        }
        _ => CONTROL_TIMEOUT_MS,
    };
    let output = run_control_within(&arguments, timeout_ms)?;
    if matches!(operation, PluginOperation::List) {
        return parse_control_package_list(&output);
    }
    let list = run_control(&plugin_control_arguments(&PluginOperation::List)?)?;
    parse_control_package_list(&list)
}

fn needs_initial_refresh(snapshot: &PluginManagerSnapshot) -> bool {
    !snapshot.repository_available
        && snapshot.repository_error.as_deref() == Some(REPOSITORY_NOT_DOWNLOADED)
}

fn spawn_plugin_operation(sender: WindUiSender<PluginResponse>, operation: PluginOperation) {
    thread::spawn(move || {
        // The page waits for this response; a panic must still end the operation.
        let result = std::panic::catch_unwind(|| execute_plugin_operation(&operation))
            .unwrap_or_else(|_| Err("fcitx5-control.exe 返回失败".to_owned()));
        let _ = sender.send(PluginResponse { operation, result });
    });
}

fn plugin_status(row: &PluginCatalogRow, locale: &LocaleCatalog) -> String {
    if !row.loaded {
        return locale.text("operation.status.running");
    }
    let Some(package) = row.package.as_ref() else {
        return locale.text("packages.state.unavailable");
    };
    if package.update_available {
        return locale.text("packages.state.update_available");
    }
    let key = match package.state.as_deref() {
        Some("bundled") => "packages.state.bundled",
        Some("disabled") => "packages.state.disabled",
        Some("trust-failed") => "packages.state.trust_failed",
        Some("incompatible") => "packages.state.incompatible",
        Some("pending-restart") => "packages.state.pending_restart",
        _ if package.installed_version.is_some() => "packages.state.enabled",
        _ if package.available_version.is_some() => "packages.state.available_online",
        _ => "packages.state.unavailable",
    };
    locale.text(key)
}

fn package_allows_installed_action(package: &ControlPackage) -> bool {
    package.installed_version.is_some()
        && !matches!(
            package.state.as_deref(),
            Some("bundled" | "trust-failed" | "incompatible" | "pending-restart")
        )
}

fn windui_plugin_row(
    row: PluginCatalogRow,
    selected: WindUiSignal<String>,
    snapshot: WindUiSignal<PluginManagerSnapshot>,
    locale: LocaleCatalog,
) -> WindUiElement {
    let status = plugin_status(&row, &locale);
    let row_id = row.id.clone();
    let title = row.title.clone();
    let summary = format!("{} · {}", row.id, row.summary);
    let category = row.category.clone();
    let select_id = row_id.clone();
    WindUiElement::row()
        .clickable()
        .on_click(move |_| {
            selected.set(select_id.clone());
            // Re-evaluate the detail signals immediately while keeping the
            // selected identity stable across dynamic list refreshes.
            snapshot.set(snapshot.get());
        })
        .width_match()
        .height(58)
        .cross(WindUiAlign::Center)
        .spacing(10)
        .padding_xy(12, 4)
        .bg_role(WindUiRole::SurfaceAlt)
        .child(
            WindUiElement::col()
                .weight(1.0)
                .spacing(2)
                .child(
                    WindUiElement::label(title.clone())
                        .font_size(13.5)
                        .font_weight(600)
                        .max_lines(1)
                        .truncate(WindUiTruncate::End)
                        .tooltip(title),
                )
                .child(
                    WindUiElement::label(summary)
                        .font_size(12.0)
                        .fg_role(WindUiRole::TextMuted)
                        .max_lines(1)
                        .truncate(WindUiTruncate::End),
                ),
        )
        .child(
            WindUiElement::label(category)
                .font_size(12.0)
                .fg_role(WindUiRole::TextMuted)
                .width(72),
        )
        .child(WindUiElement::badge_intent(status, WindUiIntent::Neutral))
}

fn windui_plugin_action(
    label: String,
    operation: impl Fn(&PluginManagerSnapshot, &str) -> Option<PluginOperation> + Copy + 'static,
    selected: WindUiSignal<String>,
    snapshot: WindUiSignal<PluginManagerSnapshot>,
    busy: WindUiSignal<bool>,
    status: WindUiSignal<String>,
    sender: WindUiSender<PluginResponse>,
    locale: LocaleCatalog,
) -> WindUiElement {
    let enabled_operation =
        move || !busy.get() && operation(&snapshot.get(), &selected.get()).is_some();
    WindUiElement::button(label)
        .small()
        .outline()
        .enabled_when(enabled_operation)
        .on_click(move |_| {
            let selected_id = selected.get();
            let Some(operation) = operation(&snapshot.get(), &selected_id) else {
                return;
            };
            busy.set(true);
            let operation_label = operation.label(&locale);
            status.set(locale.format(
                "settings.plugins.operation.running",
                &[("operation", &operation_label), ("id", &selected_id)],
            ));
            spawn_plugin_operation(sender.clone(), operation);
        })
}

fn windui_plugins_page(
    snapshot: WindUiSignal<PluginManagerSnapshot>,
    busy: WindUiSignal<bool>,
    operation_status: WindUiSignal<String>,
    sender: WindUiSender<PluginResponse>,
    locale: LocaleCatalog,
) -> WindUiElement {
    let selected = windui_signal("fcitx5-chinese-addons".to_owned());
    let rows_locale = locale.clone();
    let rows = snapshot.map(move |state| plugin_catalog_rows(state, &rows_locale));
    let repository_locale = locale.clone();
    let repository = snapshot.map(move |state| {
        if !state.loaded {
            repository_locale.text("settings.plugins.repository.loading")
        } else if state.repository_available {
            repository_locale.text("settings.plugins.repository.ready")
        } else if state.repository_error.as_deref() == Some(REPOSITORY_NOT_DOWNLOADED) {
            repository_locale.text("settings.plugins.repository.not_downloaded")
        } else {
            let detail = state
                .repository_error
                .as_deref()
                .map(|error| format!(": {error}"))
                .unwrap_or_default();
            repository_locale.format(
                "settings.plugins.repository.unavailable",
                &[("detail", &detail)],
            )
        }
    });
    let detail_locale = locale.clone();
    let detail = snapshot.map(move |state| {
        let id = selected.get();
        let category = detail_locale.text("settings.plugins.fallback_category");
        state
            .package(&id)
            .map(|package| {
                format!(
                    "{} · {} · {}",
                    package.title,
                    category,
                    plugin_package_type_label(&detail_locale, &package.package_type)
                )
            })
            .unwrap_or_else(|| format!("{} · {}", id, category))
    });
    let summary_locale = locale.clone();
    let summary = snapshot.map(move |state| {
        let id = selected.get();
        state
            .package(&id)
            .map(|package| package.summary.clone())
            .unwrap_or_else(|| summary_locale.text("settings.plugins.fallback_metadata"))
    });
    let row_locale = locale.clone();
    let list = WindUiElement::list_signal(
        rows,
        |row| row.id.clone(),
        move |row| windui_plugin_row(row, selected, snapshot, row_locale.clone()),
    )
    .width_match();
    let refresh_sender = sender.clone();
    let repair_locale = locale.clone();
    let refresh_locale = locale.clone();
    let install_sender = sender.clone();
    let update_sender = sender.clone();
    let toggle_sender = sender.clone();
    let remove_sender = sender.clone();
    let actions = vec![
        windui_plugin_action(
            locale.text("settings.plugins.operation.install"),
            |state, id| plugin_operation_for(state, id, PluginAction::Install),
            selected,
            snapshot,
            busy,
            operation_status,
            install_sender,
            locale.clone(),
        ),
        windui_plugin_action(
            locale.text("settings.plugins.operation.update"),
            |state, id| plugin_operation_for(state, id, PluginAction::Update),
            selected,
            snapshot,
            busy,
            operation_status,
            update_sender,
            locale.clone(),
        ),
        windui_plugin_action(
            locale.text("packages.enable_disable"),
            |state, id| plugin_operation_for(state, id, PluginAction::Toggle),
            selected,
            snapshot,
            busy,
            operation_status,
            toggle_sender,
            locale.clone(),
        ),
        windui_plugin_action(
            locale.text("packages.uninstall"),
            |state, id| plugin_operation_for(state, id, PluginAction::Remove),
            selected,
            snapshot,
            busy,
            operation_status,
            remove_sender,
            locale.clone(),
        ),
        WindUiElement::button(locale.text("settings.plugins.operation.repair"))
            .small()
            .outline()
            .enabled_when(move || !busy.get())
            .on_click(move |_| {
                busy.set(true);
                operation_status.set(repair_locale.text("settings.plugins.operation.repairing"));
                spawn_plugin_operation(sender.clone(), PluginOperation::Repair);
            }),
    ];
    let catalog_panel = windui_settings_card(
        WindUiElement::col()
            .fill()
            .spacing(8)
            .child(
                WindUiElement::label_signal(rows.map({
                    let locale = locale.clone();
                    move |rows| {
                        locale.format(
                            "settings.plugins.catalog_count",
                            &[("count", &rows.len().to_string())],
                        )
                    }
                }))
                .font_size(15.0)
                .font_weight(700),
            )
            .child(WindUiElement::scroll().weight(1.0).child(list)),
    )
    .height_match()
    .weight(1.0);
    let detail_panel = windui_settings_card(
        WindUiElement::scroll().fill().child(
            WindUiElement::col()
                .width_match()
                .spacing(12)
                .child(
                    WindUiElement::label_signal(detail)
                        .font_size(13.0)
                        .font_weight(700),
                )
                .child(
                    WindUiElement::label_signal(summary)
                        .font_size(12.5)
                        .fg_role(WindUiRole::TextMuted),
                )
                .child(WindUiElement::divider())
                .child(WindUiElement::grid(1, 6, actions).width_match())
                .child(
                    WindUiElement::label_signal(operation_status)
                        .font_size(12.5)
                        .fg_role(WindUiRole::TextMuted)
                        .width_match(),
                )
                .child(
                    WindUiElement::label(locale.format(
                        "settings.plugins.reference_note",
                        &[("commit", FCITX5_PLUGINS_REFERENCE_COMMIT)],
                    ))
                    .font_size(11.5)
                    .fg_role(WindUiRole::TextMuted)
                    .width_match(),
                ),
        ),
    )
    .width(300)
    .height_match();
    WindUiElement::col()
        .fill()
        .padding(24)
        .spacing(14)
        .child(windui_settings_page_title(
            &locale.text("nav.packages"),
            &locale.text("settings.plugins.subtitle"),
        ))
        .child(
            WindUiElement::row()
                .width_match()
                .cross(WindUiAlign::Center)
                .spacing(10)
                .child(
                    WindUiElement::label_signal(repository)
                        .font_size(12.5)
                        .fg_role(WindUiRole::TextMuted)
                        .weight(1.0),
                )
                .child(
                    WindUiElement::button(locale.text("packages.refresh"))
                        .small()
                        .outline()
                        .enabled_when(move || !busy.get())
                        .on_click(move |_| {
                            busy.set(true);
                            operation_status
                                .set(refresh_locale.text("settings.plugins.refresh_signed"));
                            spawn_plugin_operation(
                                refresh_sender.clone(),
                                PluginOperation::Refresh,
                            );
                        }),
                ),
        )
        .child(
            WindUiElement::row()
                .width_match()
                .weight(1.0)
                .spacing(12)
                .child(catalog_panel)
                .child(detail_panel),
        )
}

/// Hotkeys the engine's switch classifier recognizes.
const INPUT_METHOD_HOTKEYS: [&str; 4] =
    ["Ctrl+Space", "Ctrl+Shift", "Ctrl+Shift+Space", "Alt+Shift"];

#[derive(Clone, Debug, PartialEq)]
struct InputMethodDraft {
    enabled: Vec<String>,
    default: String,
}

impl InputMethodDraft {
    fn from_snapshot(snapshot: &ConfigSnapshot) -> Self {
        Self {
            enabled: snapshot.input_methods().enabled().to_vec(),
            default: snapshot.input_methods().default_id().to_owned(),
        }
    }

    fn add(&mut self, id: &str) {
        if !self.enabled.iter().any(|enabled| enabled == id) {
            self.enabled.push(id.to_owned());
        }
    }

    fn remove(&mut self, id: &str) {
        if self.enabled.len() <= 1 {
            return;
        }
        self.enabled.retain(|enabled| enabled != id);
        if self.default == id {
            self.default.clone_from(&self.enabled[0]);
        }
    }

    fn move_up(&mut self, id: &str) {
        if let Some(index) = self.enabled.iter().position(|enabled| enabled == id) {
            if index > 0 {
                self.enabled.swap(index - 1, index);
            }
        }
    }

    fn make_default(&mut self, id: &str) {
        if self.enabled.iter().any(|enabled| enabled == id) {
            self.default = id.to_owned();
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct InputMethodRow {
    id: String,
    name: String,
    is_default: bool,
    first: bool,
    only: bool,
}

fn input_method_display_name(id: &str, choices: &[InputMethodChoice]) -> String {
    choices
        .iter()
        .find(|choice| choice.id == id)
        .map_or_else(|| id.to_owned(), |choice| choice.name.clone())
}

fn input_method_rows(
    draft: &InputMethodDraft,
    choices: &[InputMethodChoice],
) -> Vec<InputMethodRow> {
    draft
        .enabled
        .iter()
        .enumerate()
        .map(|(index, id)| InputMethodRow {
            id: id.clone(),
            name: input_method_display_name(id, choices),
            is_default: *id == draft.default,
            first: index == 0,
            only: draft.enabled.len() == 1,
        })
        .collect()
}

fn addable_input_methods(
    draft: &InputMethodDraft,
    choices: &[InputMethodChoice],
) -> Vec<InputMethodChoice> {
    choices
        .iter()
        .filter(|choice| !draft.enabled.contains(&choice.id))
        .cloned()
        .collect()
}

fn installed_input_method_choices(locale: &LocaleCatalog) -> Vec<InputMethodChoice> {
    let install_root = settings_installation_root();
    let data_root = fcitx5_windows_common_core::default_fcitx5_data_root_for_current_process();
    match (install_root, data_root) {
        (Some(install_root), Some(data_root)) => discover_input_methods(
            &input_method_directories(&install_root, &data_root),
            locale.tag(),
        ),
        _ => Vec::new(),
    }
}

/// The installation Settings belongs to. A copy under `runtime\<generation>\bin`
/// belongs to the installation two levels above the generation.
fn settings_installation_root() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    fcitx5_windows_common_core::engine_client::installation_root_for_module(&exe)
        .or_else(|| exe.parent()?.parent().map(Path::to_path_buf))
}

fn windui_engine_restart_channel(
    app: &mut WindUiApp,
    locale: LocaleCatalog,
) -> WindUiSender<Result<(), String>> {
    app.channel::<Result<(), String>>(move |ctx, result| match result {
        Ok(()) => ctx.toast_ok(locale.text("settings.input.applied")),
        Err(error) => {
            ctx.toast_err(locale.format("settings.input.restart_failed", &[("error", &error)]))
        }
    })
}

fn windui_input_method_row(
    row: InputMethodRow,
    draft: WindUiSignal<InputMethodDraft>,
    locale: LocaleCatalog,
) -> WindUiElement {
    let (default_id, up_id, remove_id) = (row.id.clone(), row.id.clone(), row.id.clone());
    let default_control = if row.is_default {
        WindUiElement::badge_intent(locale.text("settings.input.default"), WindUiIntent::Success)
    } else {
        WindUiElement::button(locale.text("settings.input.make_default"))
            .small()
            .outline()
            .on_click(move |_| draft.update(|draft| draft.make_default(&default_id)))
    };
    WindUiElement::row()
        .width_match()
        .height(48)
        .cross(WindUiAlign::Center)
        .spacing(8)
        .padding_xy(12, 4)
        .bg_role(WindUiRole::SurfaceAlt)
        .child(
            WindUiElement::col()
                .weight(1.0)
                .spacing(2)
                .child(
                    WindUiElement::label(row.name)
                        .font_size(13.5)
                        .font_weight(600)
                        .max_lines(1)
                        .truncate(WindUiTruncate::End),
                )
                .child(
                    WindUiElement::label(row.id)
                        .font_size(12.0)
                        .fg_role(WindUiRole::TextMuted)
                        .max_lines(1),
                ),
        )
        .child(default_control)
        .child(
            WindUiElement::button(locale.text("settings.input.move_up"))
                .small()
                .outline()
                .neutral()
                .enabled(!row.first)
                .on_click(move |_| draft.update(|draft| draft.move_up(&up_id))),
        )
        .child(
            WindUiElement::button(locale.text("settings.input.remove"))
                .small()
                .outline()
                .neutral()
                .enabled(!row.only)
                .on_click(move |_| draft.update(|draft| draft.remove(&remove_id))),
        )
}

/// The input method list and switch hotkeys, written to config.toml through
/// Config Core. The engine reads both at startup, so applying restarts it.
fn windui_input_method_settings(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    choices: Vec<InputMethodChoice>,
    restart_sender: WindUiSender<Result<(), String>>,
    locale: &LocaleCatalog,
) -> WindUiElement {
    let snapshot = adapter.with(|adapter| adapter.preview());
    let draft = windui_signal(InputMethodDraft::from_snapshot(&snapshot));
    let hotkey_index = |value: &str| {
        INPUT_METHOD_HOTKEYS
            .iter()
            .position(|hotkey| *hotkey == value)
            .unwrap_or(0)
    };
    let toggle = windui_signal(hotkey_index(snapshot.hotkeys().toggle_input_method()));
    let next = windui_signal(hotkey_index(snapshot.hotkeys().next_input_method()));

    let rows_choices = choices.clone();
    let rows = draft.map(move |draft| input_method_rows(draft, &rows_choices));
    let row_locale = locale.clone();
    let list = WindUiElement::list_signal(
        rows,
        |row| row.id.clone(),
        move |row| windui_input_method_row(row, draft, row_locale.clone()),
    )
    .width_match()
    .height(208);

    let addable_choices = choices.clone();
    let addable = draft.map(move |draft| addable_input_methods(draft, &addable_choices));
    let addable_names = addable.map(|choices| {
        choices
            .iter()
            .map(|choice| format!("{} ({})", choice.name, choice.id))
            .collect::<Vec<_>>()
    });
    let add_selection = windui_signal(0usize);
    let add_row = WindUiElement::row()
        .width_match()
        .cross(WindUiAlign::Center)
        .spacing(10)
        .child(
            WindUiElement::dropdown_signal(addable_names, add_selection)
                .weight(1.0)
                .enabled_when(move || addable.with(|choices| !choices.is_empty())),
        )
        .child(
            WindUiElement::button(locale.text("settings.input.add"))
                .small()
                .enabled_when(move || addable.with(|choices| !choices.is_empty()))
                .on_click(move |_| {
                    let choice = addable.with(|choices| {
                        choices
                            .get(add_selection.get().min(choices.len().saturating_sub(1)))
                            .cloned()
                    });
                    if let Some(choice) = choice {
                        draft.update(|draft| draft.add(&choice.id));
                        add_selection.set(0);
                    }
                }),
        );

    let hotkey_labels = INPUT_METHOD_HOTKEYS.map(str::to_owned).to_vec();
    let pending_message = locale.text("status.unsaved");
    let same_message = locale.text("settings.shortcuts.same");
    let apply = WindUiElement::button(locale.text("settings.input.apply"))
        .small()
        .on_click(move |ctx| {
            if adapter.with(|adapter| !adapter.core.diff().is_empty()) {
                ctx.toast_err(pending_message.clone());
                return;
            }
            let (toggle, next) = (toggle.get(), next.get());
            if toggle == next {
                ctx.toast_err(same_message.clone());
                return;
            }
            let draft = draft.get();
            let mut result = Ok(());
            adapter.update(|adapter| {
                result = adapter
                    .set(ConfigEdit::InputMethods {
                        enabled: draft.enabled.clone(),
                        default: draft.default.clone(),
                    })
                    .and_then(|()| {
                        adapter.set(ConfigEdit::HotkeyToggleInputMethod(
                            INPUT_METHOD_HOTKEYS[toggle.min(3)].to_owned(),
                        ))
                    })
                    .and_then(|()| {
                        adapter.set(ConfigEdit::HotkeyNextInputMethod(
                            INPUT_METHOD_HOTKEYS[next.min(3)].to_owned(),
                        ))
                    })
                    .and_then(|()| adapter.apply());
                if result.is_err() {
                    adapter.cancel();
                }
            });
            if let Err(error) = result {
                ctx.toast_err(error);
                return;
            }
            let sender = restart_sender.clone();
            thread::spawn(move || {
                let _ = sender.send(run_control(&[OsString::from("--restart-engine")]).map(|_| ()));
            });
        });

    WindUiElement::col()
        .width_match()
        .spacing(20)
        .child(windui_settings_card(
            WindUiElement::col()
                .width_match()
                .spacing(12)
                .child(windui_settings_section_title(
                    &locale.text("settings.input.section"),
                ))
                .child(
                    WindUiElement::label(locale.text("settings.input.list_hint"))
                        .font_size(12.5)
                        .fg_role(WindUiRole::TextMuted)
                        .width_match(),
                )
                .child(list)
                .child(add_row)
                .child(
                    WindUiElement::label(locale.text("settings.input.add_hint"))
                        .font_size(12.0)
                        .fg_role(WindUiRole::TextMuted)
                        .width_match(),
                ),
        ))
        .child(windui_settings_card(
            WindUiElement::col()
                .width_match()
                .spacing(14)
                .child(windui_settings_section_title(
                    &locale.text("settings.shortcuts.section"),
                ))
                .child(WindUiElement::setting_row(
                    locale.text("settings.shortcuts.toggle_input_method"),
                    WindUiElement::dropdown(hotkey_labels.clone(), toggle).width(180),
                ))
                .child(WindUiElement::setting_row(
                    locale.text("settings.shortcuts.next_input_method"),
                    WindUiElement::dropdown(hotkey_labels, next).width(180),
                ))
                .child(apply),
        ))
}

/// Starting the launcher at sign-in lets packaged (UWP) apps, which cannot
/// start it themselves, type from the first keystroke.
fn windui_sign_in_startup_settings(locale: &LocaleCatalog) -> WindUiElement {
    let enabled = windui_signal(control_startup_query().unwrap_or(false));
    windui_settings_card(
        WindUiElement::col()
            .width_match()
            .spacing(14)
            .child(windui_settings_section_title(
                &locale.text("settings.startup.section"),
            ))
            .child(WindUiElement::setting_row_desc(
                locale.text("settings.startup.sign_in"),
                locale.text("settings.startup.sign_in_hint"),
                WindUiElement::switch(enabled).on_switch_change(move |_, value| {
                    if control_startup_set(value).is_err() {
                        enabled.set(!value);
                    }
                }),
            )),
    )
}

fn windui_settings_root(
    plugin_snapshot: WindUiSignal<PluginManagerSnapshot>,
    plugin_busy: WindUiSignal<bool>,
    plugin_status: WindUiSignal<String>,
    plugin_sender: WindUiSender<PluginResponse>,
    candidate_adapter: WindUiSignal<WindUiConfigAdapter>,
    candidate_status: WindUiSignal<String>,
    input_method_choices: Vec<InputMethodChoice>,
    engine_restart_sender: WindUiSender<Result<(), String>>,
    locale: LocaleCatalog,
) -> WindUiElement {
    candidate_status.set(locale.text("settings.candidate.draft_loaded"));
    let nav = windui_signal(0usize);
    let input_method_settings = windui_input_method_settings(
        candidate_adapter,
        input_method_choices,
        engine_restart_sender,
        &locale,
    );
    let themes = candidate_adapter.with(WindUiConfigAdapter::theme_choices);
    let fonts = candidate_font_choices(&locale, fcitx5_windows_common_core::system_font_families());
    let appearance = AppearanceControls::new();
    appearance.sync(candidate_adapter, &themes, &fonts);
    let theme_names: Vec<String> = themes
        .iter()
        .map(|theme| {
            if theme.id == DEFAULT_THEME_ID {
                return locale.text("theme.default");
            }
            let builtin = theme
                .id
                .strip_prefix("builtin:")
                .map(|name| format!("theme.builtin.{name}"))
                .map(|key| (locale.text(&key), key));
            match builtin {
                Some((text, key)) if text != key => text,
                _ => theme.name.clone(),
            }
        })
        .collect();
    let language_controls =
        windui_settings_language_controls(candidate_adapter, candidate_status, &locale);
    let plugin_locale = locale.clone();

    const NAV_GLYPHS: [&str; 3] = ["\u{270E}", "\u{25D0}", "\u{25A4}"];
    let mut nav_col = WindUiElement::col().width_match().spacing(3);
    for (i, (name, glyph)) in locale
        .navigation_labels()
        .into_iter()
        .zip(NAV_GLYPHS)
        .enumerate()
    {
        nav_col = nav_col.child(windui_settings_nav_item(name, glyph, i, nav));
    }

    let sidebar = WindUiElement::col()
        .width(196)
        .height_match()
        .bg_role(WindUiRole::Bg)
        .padding_xy(10, 12)
        .spacing(12)
        .child(WindUiElement::scroll().weight(1.0).child(nav_col));

    let input_page = WindUiElement::scroll().fill().child(
        WindUiElement::col()
            .width_match()
            .padding(24)
            .spacing(20)
            .child(windui_settings_page_title(
                &locale.text("nav.general"),
                &locale.text("settings.input.subtitle"),
            ))
            .child(input_method_settings)
            .child(windui_sign_in_startup_settings(&locale)),
    );

    let appearance_page = WindUiElement::scroll().fill().child(
        WindUiElement::col()
            .width_match()
            .padding(24)
            .spacing(20)
            .child(windui_settings_page_title(
                &locale.text("nav.appearance"),
                &locale.text("settings.appearance.subtitle"),
            ))
            .child(windui_settings_card(
                windui_config_core_candidate_layout_controls(
                    candidate_adapter,
                    candidate_status,
                    locale.clone(),
                ),
            ))
            .child(windui_settings_card(language_controls))
            .child(windui_settings_card(
                WindUiElement::col()
                    .width_match()
                    .spacing(16)
                    .child(windui_settings_section_title(
                        &locale.text("settings.theme.section"),
                    ))
                    .child(WindUiElement::setting_row_desc(
                        locale.text("appearance.mode"),
                        locale.text("settings.appearance.mode_hint"),
                        windui_appearance_mode_controls(
                            candidate_adapter,
                            candidate_status,
                            &locale,
                        ),
                    ))
                    .child(WindUiElement::setting_row_desc(
                        locale.text("settings.theme.choice"),
                        locale.text("settings.theme.choice_hint"),
                        WindUiElement::dropdown(theme_names, appearance.theme).width(220),
                    )),
            ))
            .child(windui_settings_card(
                WindUiElement::col()
                    .width_match()
                    .spacing(16)
                    .child(windui_settings_section_title(
                        &locale.text("settings.candidate.text_section"),
                    ))
                    .child(WindUiElement::setting_row_desc(
                        locale.text("settings.candidate.font"),
                        locale.text("settings.candidate.font_hint"),
                        WindUiElement::dropdown(fonts.clone(), appearance.font).width(220),
                    ))
                    .child(WindUiElement::setting_row_desc(
                        locale.text("settings.candidate.font_size"),
                        locale.text("settings.candidate.font_size_hint"),
                        WindUiElement::stepper(
                            appearance.font_size,
                            CANDIDATE_FONT_SIZE_RANGE.0,
                            CANDIDATE_FONT_SIZE_RANGE.1,
                            1.0,
                        ),
                    ))
                    .child(WindUiElement::setting_row_desc(
                        locale.text("settings.candidate.opacity"),
                        locale.text("settings.candidate.opacity_hint"),
                        WindUiElement::slider(appearance.opacity).width(180),
                    )),
            )),
    );

    let content = WindUiElement::stack()
        .height_match()
        .weight(1.0)
        .child(input_page.visible_when(move || nav.get() == 0))
        .child(appearance_page.visible_when(move || nav.get() == 1))
        .child(
            windui_plugins_page(
                plugin_snapshot,
                plugin_busy,
                plugin_status,
                plugin_sender,
                plugin_locale,
            )
            .visible_when(move || nav.get() == 2),
        );

    let footer = WindUiElement::row()
        .width_match()
        .height(54)
        .cross(WindUiAlign::Center)
        .padding_xy(16, 0)
        .spacing(10)
        .bg_role(WindUiRole::SurfaceAlt)
        .child(
            WindUiElement::stack()
                .size(14, 14)
                .corner(7.0)
                .bg_role_alpha(WindUiRole::Success, 0.22)
                .child(
                    WindUiElement::leaf()
                        .size(8, 8)
                        .corner(4.0)
                        .bg_role(WindUiRole::Success)
                        .align(WindUiAlign::Center),
                ),
        )
        .child(
            WindUiElement::label_signal(candidate_status)
                .font_size(12.5)
                .fg_role(WindUiRole::TextMuted),
        )
        .child(WindUiElement::flex_spacer())
        .child(
            WindUiElement::button(locale.text("settings.reload"))
                .small()
                .outline()
                .neutral()
                .on_click({
                    let locale = locale.clone();
                    let themes = themes.clone();
                    let fonts = fonts.clone();
                    move |ctx| {
                        let mut result = Ok(());
                        candidate_adapter.update(|adapter| result = adapter.reload());
                        match result {
                            Ok(()) => {
                                appearance.sync(candidate_adapter, &themes, &fonts);
                                sync_settings_shell_theme(candidate_adapter);
                                candidate_status.set(locale.text("settings.reloaded"));
                            }
                            Err(error) => ctx.toast_err(error),
                        }
                    }
                })
                .visible_when(move || nav.get() == 1),
        )
        .child(
            WindUiElement::button(locale.text("settings.save"))
                .small()
                .on_click({
                    let locale = locale.clone();
                    move |ctx| {
                        let mut edits = candidate_adapter
                            .with(|adapter| appearance.pending_edits(&adapter.preview(), &themes));
                        let font_change = appearance.font_change(&fonts);
                        if let CandidateFontChange::Use(font) = &font_change {
                            edits.push(ConfigEdit::CandidateFontFamilies(vec![
                                font.clone(),
                                "system".to_owned(),
                            ]));
                        }
                        let mut result = Ok(());
                        candidate_adapter.update(|adapter| {
                            if font_change == CandidateFontChange::FollowTheme {
                                adapter.reset(ConfigField::CandidateFontFamilies);
                            }
                            result = edits
                                .into_iter()
                                .try_for_each(|edit| adapter.set(edit))
                                .and_then(|()| adapter.apply());
                        });
                        match result {
                            Ok(()) => {
                                appearance.font_loaded.set(appearance.font.get());
                                candidate_status.set(locale.text("settings.saved"));
                                ctx.toast_ok(locale.text("settings.saved"));
                            }
                            Err(error) => ctx.toast_err(error),
                        }
                    }
                })
                .visible_when(move || nav.get() == 1),
        );

    let body = WindUiElement::col()
        .fill()
        .child(
            WindUiElement::row()
                .fill()
                .weight(1.0)
                .child(sidebar)
                .child(
                    WindUiElement::leaf()
                        .width(1)
                        .height_match()
                        .bg_role(WindUiRole::Divider),
                )
                .child(content),
        )
        .child(WindUiElement::divider())
        .child(footer);

    WindUiElement::stack()
        .fill()
        .bg_role(WindUiRole::Bg)
        .child(windui_settings_shell_wrap(&locale.text("app.title"), body))
}

fn windui_settings_language_controls(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) -> WindUiElement {
    const LANGUAGES: [(&str, &str); 9] = [
        ("system", "language.option.system"),
        ("en-US", "language.option.en-US"),
        ("zh-CN", "language.option.zh-CN"),
        ("zh-TW", "zh-TW"),
        ("ja-JP", "ja-JP"),
        ("ko-KR", "ko-KR"),
        ("vi-VN", "vi-VN"),
        ("th-TH", "th-TH"),
        ("si-LK", "si-LK"),
    ];
    const LANGUAGE_NAMES: [&str; 6] = ["繁體中文", "日本語", "한국어", "Tiếng Việt", "ไทย", "සිංහල"];
    let selected_language = adapter.with(|adapter| adapter.preview().ui().language().to_owned());
    let selected_index = LANGUAGES
        .iter()
        .position(|(language, _)| *language == selected_language)
        .unwrap_or(0);
    let selection = windui_signal(selected_index);
    let labels = LANGUAGES
        .iter()
        .enumerate()
        .map(|(index, (_, key))| match index {
            0..=2 => locale.text(key),
            other => LANGUAGE_NAMES[other - 3].to_owned(),
        })
        .collect::<Vec<_>>();
    let apply_label = locale.text("action.apply");
    let pending_message = locale.text("status.unsaved");
    let restart_message = locale.text("language.restart_required");

    WindUiElement::col()
        .width_match()
        .spacing(10)
        .child(WindUiElement::setting_row_desc(
            locale.text("language.selector"),
            &restart_message,
            WindUiElement::dropdown(labels, selection).width(230),
        ))
        .child(
            WindUiElement::button(apply_label)
                .small()
                .on_click(move |ctx| {
                    let has_pending_changes =
                        adapter.with(|adapter| !adapter.core.diff().is_empty());
                    if has_pending_changes {
                        ctx.toast_err(pending_message.clone());
                        return;
                    }
                    let index = selection.get().min(LANGUAGES.len() - 1);
                    let mut result = Ok(());
                    adapter.update(|adapter| {
                        result = adapter
                            .set(ConfigEdit::UiLanguage(LANGUAGES[index].0.to_owned()))
                            .and_then(|()| adapter.apply());
                    });
                    match result {
                        Ok(()) => {
                            status.set(restart_message.clone());
                            ctx.toast_ok(restart_message.clone());
                        }
                        Err(error) => ctx.toast_err(error),
                    }
                }),
        )
}

fn windui_plugin_manager(
    app: &mut WindUiApp,
    initial_load: bool,
    locale: LocaleCatalog,
) -> (
    WindUiSignal<PluginManagerSnapshot>,
    WindUiSignal<bool>,
    WindUiSignal<String>,
    WindUiSender<PluginResponse>,
) {
    let snapshot = windui_signal(PluginManagerSnapshot::default());
    let busy = windui_signal(initial_load);
    let status = windui_signal(if initial_load {
        locale.text("settings.plugins.control_reading")
    } else {
        locale.text("settings.plugins.choose_operation")
    });
    let response_locale = locale.clone();
    let follow_up = windui_signal(None::<WindUiSender<PluginResponse>>);
    let sender = app.channel::<PluginResponse>(move |ctx, response| {
        busy.set(false);
        match response.result {
            Ok(next)
                if response.operation == PluginOperation::InitialLoad
                    && needs_initial_refresh(&next) =>
            {
                // A fresh install has no verified catalog yet. Show the local
                // list right away and download the catalog once, so the page
                // does not stay with every plugin unavailable; a failure is
                // reported like any Refresh.
                snapshot.set(next);
                if let Some(sender) = follow_up.get() {
                    busy.set(true);
                    status.set(response_locale.text("settings.plugins.refresh_signed"));
                    spawn_plugin_operation(sender, PluginOperation::Refresh);
                }
            }
            Ok(next) => {
                let count = next.packages.len();
                let operation_label = response.operation.label(&response_locale);
                status.set(response_locale.format(
                    "settings.plugins.completed",
                    &[
                        ("operation", &operation_label),
                        ("count", &count.to_string()),
                    ],
                ));
                snapshot.set(next);
                ctx.toast_ok(operation_label);
            }
            Err(error) => {
                let operation_label = response.operation.label(&response_locale);
                let message = response_locale.format(
                    "settings.plugins.operation_failed",
                    &[("operation", &operation_label)],
                );
                let detail = localized_plugin_error(&response_locale, &error);
                status.set(format!("{message}: {detail}"));
                ctx.toast_err(format!("{message}: {detail}"));
            }
        }
    });
    follow_up.set(Some(sender.clone()));
    if initial_load {
        spawn_plugin_operation(sender.clone(), PluginOperation::InitialLoad);
    }
    (snapshot, busy, status, sender)
}

fn windui_candidate_config_manager(
    path: PathBuf,
) -> Result<(WindUiSignal<WindUiConfigAdapter>, WindUiSignal<String>), String> {
    Ok((
        windui_signal(WindUiConfigAdapter::load(path)?),
        windui_signal(String::new()),
    ))
}

fn windui_config_path() -> Result<PathBuf, String> {
    fcitx5_windows_common_core::default_fcitx5_data_root_for_current_process()
        .map(|root| root.join("config.toml"))
        .ok_or_else(|| "unable to resolve the Fcitx5 user configuration path".to_owned())
}

fn update_candidate_draft(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    label: &str,
    edit: ConfigEdit,
    locale: &LocaleCatalog,
) -> Result<(), String> {
    let mut result = Ok(());
    adapter.update(|adapter| result = adapter.set(edit));
    result?;
    sync_settings_shell_theme(adapter);
    status.set(locale.format("settings.candidate.draft_updated", &[("control", label)]));
    Ok(())
}

fn apply_candidate_draft(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) -> Result<(), String> {
    let mut result = Ok(());
    adapter.update(|adapter| result = adapter.apply());
    result?;
    status.set(locale.text("settings.candidate.applied"));
    Ok(())
}

fn cancel_candidate_draft(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) {
    adapter.update(WindUiConfigAdapter::cancel);
    sync_settings_shell_theme(adapter);
    status.set(locale.text("settings.candidate.draft_discarded"));
}

fn reset_candidate_draft(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) {
    adapter.update(WindUiConfigAdapter::reset_candidate_layout);
    status.set(locale.text("settings.candidate.draft_reset"));
}

/// Decodes the current snapshot into the two-axis UI state exactly as Config
/// Core decodes the legacy string (see `candidate_layout_mode` + `.options()`).
fn candidate_layout_ui_options(snapshot: &ConfigSnapshot) -> CandidateLayoutUiOptions {
    let candidate = snapshot.candidate();
    let layout = candidate.layout_options();
    CandidateLayoutUiOptions {
        orientation: match layout.orientation {
            CoreCandidateOrientation::Horizontal => CandidateLayoutOrientation::Horizontal,
            CoreCandidateOrientation::Vertical => CandidateLayoutOrientation::Vertical,
        },
        overflow: match layout.overflow {
            CoreOverflowBehavior::Paging => CandidateLayoutOverflow::Paging,
            CoreOverflowBehavior::Scrolling => CandidateLayoutOverflow::Scrolling,
            CoreOverflowBehavior::Wrapping => CandidateLayoutOverflow::Wrapping,
        },
        writing_mode: match layout.writing_mode {
            CoreWritingMode::Horizontal => CandidateWritingMode::Horizontal,
            CoreWritingMode::VerticalRl => CandidateWritingMode::VerticalRl,
            CoreWritingMode::VerticalLr => CandidateWritingMode::VerticalLr,
        },
        scroll_direction: candidate.scroll_direction(),
        vertical_text_column_direction: candidate.vertical_text_column_direction(),
    }
}

/// Re-encodes the edited two-axis state to the single legacy `layout_type`
/// string and writes it to the Draft as one `CandidateLayoutType` edit.
#[allow(dead_code)]
fn commit_candidate_layout_ui(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    control: &str,
    edit: impl Fn(CandidateLayoutUiOptions) -> CandidateLayoutUiOptions + 'static,
    locale: &LocaleCatalog,
) -> Result<(), String> {
    let mut result = Ok(());
    adapter.update(|adapter| {
        let current = candidate_layout_ui_options(&adapter.preview());
        let next = edit(current);
        result = adapter.set(ConfigEdit::CandidateLayoutType(
            next.layout_type().to_owned(),
        ));
    });
    result?;
    status.set(locale.format("settings.candidate.draft_updated", &[("control", control)]));
    Ok(())
}

fn candidate_layout_mode_button(
    mode: CandidateLayoutMode,
    selected: WindUiSignal<CandidateLayoutMode>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) -> WindUiElement {
    let label_key = match mode {
        CandidateLayoutMode::Automatic => "candidate.automatic",
        CandidateLayoutMode::Stacked => "candidate.vertical",
        CandidateLayoutMode::Flow => "candidate.horizontal",
        CandidateLayoutMode::Scroll => "candidate.scroll",
        CandidateLayoutMode::VerticalText => "settings.candidate.mode.vertical_text",
    };
    let label = locale.text(label_key);
    let tooltip = locale.text("settings.candidate.layout_hint");
    let callback_locale = (*locale).clone();
    let active = WindUiElement::button(label.clone())
        .small()
        .tooltip(tooltip.clone())
        .visible_when(move || selected.get() == mode);
    let inactive = WindUiElement::button(label)
        .small()
        .outline_soft()
        .neutral()
        .tooltip(tooltip)
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_draft(
                adapter,
                status,
                &callback_locale.text("candidate.layout"),
                ConfigEdit::CandidateLayoutType(mode.options().layout_type().to_owned()),
                &callback_locale,
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || selected.get() != mode);
    WindUiElement::stack().child(active).child(inactive)
}

fn windui_appearance_mode_controls(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) -> WindUiElement {
    let selected = adapter.map(|adapter| adapter.preview().appearance().mode().to_owned());
    let mut controls = WindUiElement::row().spacing(6);
    for (key, mode) in [
        ("mode.system", "system"),
        ("mode.light", "light"),
        ("mode.dark", "dark"),
    ] {
        let label = locale.text(key);
        let callback_locale = (*locale).clone();
        let active = WindUiElement::button(label.clone())
            .small()
            .visible_when(move || selected.get() == mode);
        let inactive = WindUiElement::button(label)
            .small()
            .outline_soft()
            .neutral()
            .on_click(move |ctx| {
                if let Err(error) = update_candidate_draft(
                    adapter,
                    status,
                    &callback_locale.text("appearance.mode"),
                    ConfigEdit::AppearanceMode(mode.to_owned()),
                    &callback_locale,
                ) {
                    ctx.toast_err(error);
                }
            })
            .visible_when(move || selected.get() != mode);
        controls = controls.child(WindUiElement::stack().child(active).child(inactive));
    }
    controls
}

fn candidate_scroll_direction_button(
    direction: ScrollDirection,
    mode: WindUiSignal<CandidateLayoutMode>,
    selected: WindUiSignal<CandidateLayoutUiOptions>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) -> WindUiElement {
    let label = match direction {
        ScrollDirection::Horizontal => locale.text("settings.candidate.horizontal_scroll"),
        ScrollDirection::Vertical => locale.text("settings.candidate.vertical_scroll"),
    };
    let active = WindUiElement::button(label.clone())
        .small()
        .visible_when(move || {
            mode.get() == CandidateLayoutMode::Scroll
                && selected.get().scroll_direction == direction
        });
    let callback_locale = (*locale).clone();
    let inactive = WindUiElement::button(label)
        .small()
        .outline_soft()
        .neutral()
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_draft(
                adapter,
                status,
                &callback_locale.text("settings.candidate.scroll_direction"),
                ConfigEdit::CandidateScrollDirection(direction),
                &callback_locale,
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || {
            mode.get() == CandidateLayoutMode::Scroll
                && selected.get().scroll_direction != direction
        });
    WindUiElement::stack().child(active).child(inactive)
}

fn candidate_vertical_text_column_button(
    direction: VerticalTextColumnDirection,
    mode: WindUiSignal<CandidateLayoutMode>,
    selected: WindUiSignal<CandidateLayoutUiOptions>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) -> WindUiElement {
    let label = match direction {
        VerticalTextColumnDirection::RightToLeft => locale.text("settings.candidate.right_to_left"),
        VerticalTextColumnDirection::LeftToRight => locale.text("settings.candidate.left_to_right"),
    };
    let active = WindUiElement::button(label.clone())
        .small()
        .visible_when(move || {
            mode.get() == CandidateLayoutMode::VerticalText
                && selected.get().vertical_text_column_direction == direction
        });
    let callback_locale = (*locale).clone();
    let inactive = WindUiElement::button(label)
        .small()
        .outline_soft()
        .neutral()
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_draft(
                adapter,
                status,
                &callback_locale.text("settings.candidate.column_direction"),
                ConfigEdit::CandidateVerticalTextColumnDirection(direction),
                &callback_locale,
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || {
            mode.get() == CandidateLayoutMode::VerticalText
                && selected.get().vertical_text_column_direction != direction
        });
    WindUiElement::stack().child(active).child(inactive)
}

/// Two clickable orientation cards (横向 / 纵向) with a clear selected state.
#[allow(dead_code)]
fn candidate_layout_orientation_card(
    orientation: CandidateLayoutOrientation,
    options: WindUiSignal<CandidateLayoutUiOptions>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) -> WindUiElement {
    let label = match orientation {
        CandidateLayoutOrientation::Horizontal => {
            locale.text("settings.candidate.orientation.horizontal")
        }
        CandidateLayoutOrientation::Vertical => {
            locale.text("settings.candidate.orientation.vertical")
        }
    };
    let description = match orientation {
        CandidateLayoutOrientation::Horizontal => {
            locale.text("settings.candidate.orientation.horizontal_hint")
        }
        CandidateLayoutOrientation::Vertical => {
            locale.text("settings.candidate.orientation.vertical_hint")
        }
    };
    let active = WindUiElement::col()
        .padding(10)
        .corner(9.0)
        .bg_role(WindUiRole::Accent)
        .spacing(2)
        .width(122)
        .child(
            WindUiElement::label(label.clone())
                .font_size(13.5)
                .font_weight(600)
                .fg_role(WindUiRole::OnAccent),
        )
        .child(
            WindUiElement::label(description.clone())
                .font_size(11.0)
                .fg_role(WindUiRole::OnAccent)
                .width_match(),
        )
        .visible_when(move || options.get().orientation == orientation);
    let callback_locale = (*locale).clone();
    let inactive = WindUiElement::col()
        .clickable()
        .on_click(move |ctx| {
            if let Err(error) = commit_candidate_layout_ui(
                adapter,
                status,
                &callback_locale.text("candidate.layout"),
                move |current| CandidateLayoutUiOptions {
                    orientation,
                    overflow: if orientation == CandidateLayoutOrientation::Vertical
                        && current.overflow == CandidateLayoutOverflow::Wrapping
                    {
                        CandidateLayoutOverflow::Paging
                    } else {
                        current.overflow
                    },
                    writing_mode: CandidateWritingMode::Horizontal,
                    ..current
                },
                &callback_locale,
            ) {
                ctx.toast_err(error);
            }
        })
        .padding(10)
        .corner(9.0)
        .border_role(WindUiRole::Border, 1)
        .bg_role(WindUiRole::SurfaceAlt)
        .spacing(2)
        .width(122)
        .child(
            WindUiElement::label(label)
                .font_size(13.5)
                .font_weight(600)
                .fg_role(WindUiRole::Text),
        )
        .child(
            WindUiElement::label(description)
                .font_size(11.0)
                .fg_role(WindUiRole::TextMuted)
                .width_match(),
        )
        .visible_when(move || {
            options.get().writing_mode == CandidateWritingMode::Horizontal
                && options.get().orientation != orientation
        });
    WindUiElement::stack().child(active).child(inactive)
}

/// One overflow option. Options not allowed for the current orientation, and
/// any overflow change while a vertical writing mode is active (which legacy
/// `vertical_text` fixes to Vertical+Paging), are never offered.
#[allow(dead_code)]
fn candidate_layout_overflow_button(
    overflow: CandidateLayoutOverflow,
    options: WindUiSignal<CandidateLayoutUiOptions>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) -> WindUiElement {
    let label = match overflow {
        CandidateLayoutOverflow::Paging => locale.text("settings.candidate.overflow.paging"),
        CandidateLayoutOverflow::Scrolling => locale.text("settings.candidate.overflow.scrolling"),
        CandidateLayoutOverflow::Wrapping => locale.text("settings.candidate.overflow.wrapping"),
    };
    let available = options.map(move |current| current.overflow_choices().contains(&overflow));
    let callback_locale = (*locale).clone();
    let hint = locale.text("settings.candidate.overflow_hint");
    let active = WindUiElement::button(label.clone())
        .small()
        .tooltip(hint.clone())
        .on_click(move |ctx| {
            if let Err(error) = commit_candidate_layout_ui(
                adapter,
                status,
                &callback_locale.text("settings.candidate.overflow_hint"),
                move |current| CandidateLayoutUiOptions {
                    overflow,
                    writing_mode: CandidateWritingMode::Horizontal,
                    ..current
                },
                &callback_locale,
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || available.get() && options.get().overflow == overflow);
    let callback_locale = (*locale).clone();
    let inactive = WindUiElement::button(label)
        .small()
        .outline_soft()
        .neutral()
        .tooltip(hint)
        .on_click(move |ctx| {
            if let Err(error) = commit_candidate_layout_ui(
                adapter,
                status,
                &callback_locale.text("settings.candidate.overflow_hint"),
                move |current| CandidateLayoutUiOptions {
                    overflow,
                    writing_mode: CandidateWritingMode::Horizontal,
                    ..current
                },
                &callback_locale,
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || {
            available.get()
                && options.get().writing_mode == CandidateWritingMode::Horizontal
                && options.get().overflow != overflow
        });
    WindUiElement::stack().child(active).child(inactive)
}

/// One advanced writing-mode option (高级 · 文字方向), set apart from layout.
#[allow(dead_code)]
fn candidate_layout_writing_button(
    writing: CandidateWritingMode,
    options: WindUiSignal<CandidateLayoutUiOptions>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) -> WindUiElement {
    let label = match writing {
        CandidateWritingMode::Horizontal => locale.text("settings.candidate.writing.horizontal"),
        CandidateWritingMode::VerticalRl => locale.text("settings.candidate.writing.vertical_rl"),
        CandidateWritingMode::VerticalLr => locale.text("settings.candidate.writing.vertical_lr"),
    };
    let callback_locale = (*locale).clone();
    let hint = locale.text("settings.candidate.writing_hint");
    let active = WindUiElement::button(label.clone())
        .small()
        .tooltip(hint.clone())
        .on_click(move |ctx| {
            if let Err(error) = commit_candidate_layout_ui(
                adapter,
                status,
                &callback_locale.text("settings.candidate.writing_hint"),
                move |current| current.with_writing(writing),
                &callback_locale,
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || options.get().writing_mode == writing);
    let callback_locale = (*locale).clone();
    let inactive = WindUiElement::button(label)
        .small()
        .outline_soft()
        .neutral()
        .tooltip(hint)
        .on_click(move |ctx| {
            if let Err(error) = commit_candidate_layout_ui(
                adapter,
                status,
                &callback_locale.text("settings.candidate.writing_hint"),
                move |current| current.with_writing(writing),
                &callback_locale,
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || options.get().writing_mode != writing);
    WindUiElement::stack().child(active).child(inactive)
}

fn config_core_candidate_page_size_button(
    value: u8,
    selected: WindUiSignal<u8>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: &LocaleCatalog,
) -> WindUiElement {
    let hint = locale.text("settings.candidate.page_size_hint");
    let control = locale.text("candidate.page_size");
    let active_locale = (*locale).clone();
    let inactive_locale = active_locale.clone();
    let inactive_control = control.clone();
    let active = WindUiElement::button(value.to_string())
        .small()
        .tooltip(hint.clone())
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_draft(
                adapter,
                status,
                &control,
                ConfigEdit::CandidatePageSize(value),
                &active_locale,
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || selected.get() == value);
    let inactive = WindUiElement::button(value.to_string())
        .small()
        .outline_soft()
        .neutral()
        .tooltip(hint)
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_draft(
                adapter,
                status,
                &inactive_control,
                ConfigEdit::CandidatePageSize(value),
                &inactive_locale,
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || selected.get() != value);
    WindUiElement::stack().child(active).child(inactive)
}

fn windui_settings_control_group(
    label: String,
    description: String,
    controls: WindUiElement,
) -> WindUiElement {
    WindUiElement::col()
        .width_match()
        .spacing(4)
        .child(WindUiElement::label(label).font_size(14.0).font_weight(600))
        .child(
            WindUiElement::label(description)
                .font_size(12.5)
                .fg_role(WindUiRole::TextMuted),
        )
        .child(controls)
}

fn windui_config_core_candidate_layout_controls(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    locale: LocaleCatalog,
) -> WindUiElement {
    let options = adapter.map(|adapter| candidate_layout_ui_options(&adapter.preview()));
    let mode = adapter.map(|adapter| {
        candidate_layout_mode(&adapter.preview()).unwrap_or(CandidateLayoutMode::Automatic)
    });
    let page_size = adapter.map(|adapter| adapter.preview().candidate().page_size());
    let mut modes = WindUiElement::row().spacing(4);
    for layout_mode in [
        CandidateLayoutMode::Automatic,
        CandidateLayoutMode::Stacked,
        CandidateLayoutMode::Flow,
        CandidateLayoutMode::Scroll,
        CandidateLayoutMode::VerticalText,
    ] {
        modes = modes.child(candidate_layout_mode_button(
            layout_mode,
            mode,
            adapter,
            status,
            &locale,
        ));
    }

    let scroll_directions = WindUiElement::row()
        .spacing(4)
        .child(candidate_scroll_direction_button(
            ScrollDirection::Horizontal,
            mode,
            options,
            adapter,
            status,
            &locale,
        ))
        .child(candidate_scroll_direction_button(
            ScrollDirection::Vertical,
            mode,
            options,
            adapter,
            status,
            &locale,
        ));

    let vertical_text_columns = WindUiElement::row()
        .spacing(4)
        .child(candidate_vertical_text_column_button(
            VerticalTextColumnDirection::RightToLeft,
            mode,
            options,
            adapter,
            status,
            &locale,
        ))
        .child(candidate_vertical_text_column_button(
            VerticalTextColumnDirection::LeftToRight,
            mode,
            options,
            adapter,
            status,
            &locale,
        ));

    let mut page_sizes = WindUiElement::row().spacing(4);
    // Pinyin, shuangpin and the tables page 3 to 10 candidates.
    for value in 3..=9 {
        page_sizes = page_sizes.child(config_core_candidate_page_size_button(
            value, page_size, adapter, status, &locale,
        ));
    }

    WindUiElement::col()
        .width_match()
        .spacing(14)
        .child(windui_settings_section_title(
            &locale.text("settings.candidate.section"),
        ))
        .child(windui_settings_control_group(
            locale.text("candidate.layout"),
            locale.text("settings.candidate.layout_hint"),
            modes,
        ))
        .child(
            WindUiElement::col()
                .visible_when(move || mode.get() == CandidateLayoutMode::Scroll)
                .child(windui_settings_control_group(
                    locale.text("settings.candidate.scroll_direction"),
                    locale.text("settings.candidate.scroll_direction_hint"),
                    scroll_directions,
                )),
        )
        .child(WindUiElement::col().child(WindUiElement::setting_row_desc(
            locale.text("candidate.page_size"),
            locale.text("settings.candidate.page_size_hint"),
            page_sizes,
        )))
        .child(
            WindUiElement::col()
                .visible_when(move || mode.get() == CandidateLayoutMode::VerticalText)
                .child(windui_settings_control_group(
                    locale.text("settings.candidate.column_direction"),
                    locale.text("settings.candidate.column_direction_hint"),
                    vertical_text_columns,
                )),
        )
        .child(
            WindUiElement::row()
                .spacing(8)
                .child(
                    WindUiElement::button(locale.text("settings.candidate.apply")).on_click({
                        let locale = locale.clone();
                        move |ctx| {
                            if let Err(error) = apply_candidate_draft(adapter, status, &locale) {
                                ctx.toast_err(error);
                            } else {
                                ctx.toast_ok(locale.text("settings.candidate.applied"));
                            }
                        }
                    }),
                )
                .child(
                    WindUiElement::button(locale.text("settings.candidate.cancel"))
                        .outline_soft()
                        .on_click({
                            let locale = locale.clone();
                            move |_| {
                                cancel_candidate_draft(adapter, status, &locale);
                            }
                        }),
                )
                .child(
                    WindUiElement::button(locale.text("settings.candidate.reset"))
                        .outline_soft()
                        .on_click({
                            let locale = locale.clone();
                            move |_| {
                                reset_candidate_draft(adapter, status, &locale);
                            }
                        }),
                ),
        )
        .child(
            WindUiElement::label_signal(status)
                .font_size(12.5)
                .fg_role(WindUiRole::TextMuted),
        )
}

/// Thin WindUI adapter over the sole Config Core state and transaction authority.
#[derive(Debug)]
struct WindUiConfigAdapter {
    path: PathBuf,
    installation_root: PathBuf,
    data_root: PathBuf,
    store: FileStore,
    core: ConfigCore,
}

impl WindUiConfigAdapter {
    fn load(path: PathBuf) -> Result<Self, String> {
        let store = FileStore::new();
        let core = ConfigCore::recover(&store, &path)
            .map_err(|error| error.to_string())?
            .core;
        let data_root = path.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        let installation_root = settings_installation_root().unwrap_or_default();
        Ok(Self {
            path,
            installation_root,
            data_root,
            store,
            core,
        })
    }

    #[cfg(test)]
    fn path(&self) -> &Path {
        &self.path
    }

    fn preview(&self) -> ConfigSnapshot {
        self.core
            .preview_visual_snapshot(&self.installation_root, &self.data_root, false)
    }

    fn set(&mut self, edit: ConfigEdit) -> Result<(), String> {
        self.core
            .execute(ConfigCommand::Set(edit), &self.store, &self.path)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn cancel(&mut self) {
        self.core.cancel();
    }

    fn reset(&mut self, field: ConfigField) {
        self.core.reset(field);
    }

    fn reset_candidate_layout(&mut self) {
        for field in [
            ConfigField::CandidateLayoutType,
            ConfigField::CandidateScrollDirection,
            ConfigField::CandidateVerticalTextColumnDirection,
            ConfigField::CandidatePageSize,
        ] {
            self.reset(field);
        }
    }

    fn apply(&mut self) -> Result<(), String> {
        self.core
            .apply(
                &self.store,
                &self.path,
                fcitx5_config_core::CommitFault::None,
            )
            .map_err(|error| error.to_string())?;
        fcitx5_windows_common_core::broadcast_visual_config_changed();
        Ok(())
    }

    /// Discards the draft and reads the configuration from disk again.
    fn reload(&mut self) -> Result<(), String> {
        *self = Self::load(self.path.clone())?;
        Ok(())
    }

    fn theme_choices(&self) -> Vec<ThemeChoice> {
        control_themes_list_json_for_paths(&self.installation_root, &self.data_root)
            .ok()
            .map(|json| theme_choices_from_json(&json))
            .unwrap_or_default()
    }
}

const DEFAULT_THEME_ID: &str = "builtin:default";
const CANDIDATE_FONT_SIZE_RANGE: (f64, f64) = (8.0, 48.0);
const CANDIDATE_OPACITY_MIN: f32 = 0.2;

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct ThemeChoice {
    id: String,
    name: String,
}

fn theme_choices_from_json(json: &str) -> Vec<ThemeChoice> {
    #[derive(Deserialize)]
    struct ThemeList {
        themes: Vec<ThemeChoice>,
    }
    serde_json::from_str::<ThemeList>(json)
        .map(|list| list.themes)
        .unwrap_or_default()
}

/// Maps the 0..1 slider position to the candidate opacity range Config Core accepts.
fn opacity_from_slider(position: f32) -> f32 {
    let opacity = CANDIDATE_OPACITY_MIN + (1.0 - CANDIDATE_OPACITY_MIN) * position.clamp(0.0, 1.0);
    (opacity * 100.0).round() / 100.0
}

fn slider_from_opacity(opacity: f32) -> f32 {
    ((opacity - CANDIDATE_OPACITY_MIN) / (1.0 - CANDIDATE_OPACITY_MIN)).clamp(0.0, 1.0)
}

/// Appearance values edited in place on the page and written to Config Core on Save.
#[derive(Clone, Copy)]
struct AppearanceControls {
    theme: WindUiSignal<usize>,
    /// Index into the font choices; 0 follows the theme's font.
    font: WindUiSignal<usize>,
    font_loaded: WindUiSignal<usize>,
    font_size: WindUiSignal<f64>,
    opacity: WindUiSignal<f32>,
}

/// What Save does with the candidate font choice.
#[derive(Clone, Debug, PartialEq)]
enum CandidateFontChange {
    Keep,
    FollowTheme,
    Use(String),
}

/// Font dropdown entries: the follow-theme entry, then installed families.
fn candidate_font_choices(locale: &LocaleCatalog, installed: Vec<String>) -> Vec<String> {
    std::iter::once(locale.text("settings.candidate.font_theme"))
        .chain(installed)
        .collect()
}

/// The dropdown index of the font the snapshot draws candidates with.
fn candidate_font_index(snapshot: &ConfigSnapshot, choices: &[String]) -> usize {
    snapshot
        .fonts()
        .candidate()
        .families()
        .iter()
        .filter(|family| {
            !family.eq_ignore_ascii_case("system") && !family.eq_ignore_ascii_case("inherit")
        })
        .find_map(|family| {
            choices
                .iter()
                .skip(1)
                .position(|choice| choice.eq_ignore_ascii_case(family))
                .map(|index| index + 1)
        })
        .unwrap_or(0)
}

fn candidate_font_change(
    choices: &[String],
    loaded: usize,
    selected: usize,
) -> CandidateFontChange {
    if selected == loaded {
        CandidateFontChange::Keep
    } else if selected == 0 {
        CandidateFontChange::FollowTheme
    } else {
        choices
            .get(selected)
            .map_or(CandidateFontChange::Keep, |font| {
                CandidateFontChange::Use(font.clone())
            })
    }
}

impl AppearanceControls {
    fn new() -> Self {
        Self {
            theme: windui_signal(0usize),
            font: windui_signal(0usize),
            font_loaded: windui_signal(0usize),
            font_size: windui_signal(16.0f64),
            opacity: windui_signal(1.0f32),
        }
    }

    fn sync(
        self,
        adapter: WindUiSignal<WindUiConfigAdapter>,
        themes: &[ThemeChoice],
        fonts: &[String],
    ) {
        let snapshot = adapter.with(WindUiConfigAdapter::preview);
        let font = candidate_font_index(&snapshot, fonts);
        self.font.set(font);
        self.font_loaded.set(font);
        self.theme.set(
            themes
                .iter()
                .position(|theme| theme.id == snapshot.appearance().theme())
                .unwrap_or(0),
        );
        self.font_size
            .set(f64::from(snapshot.fonts().candidate().size_dip()));
        self.opacity
            .set(slider_from_opacity(snapshot.candidate().opacity()));
    }

    fn pending_edits(self, snapshot: &ConfigSnapshot, themes: &[ThemeChoice]) -> Vec<ConfigEdit> {
        appearance_edits(
            snapshot,
            themes,
            self.theme.get(),
            self.font_size.get(),
            self.opacity.get(),
        )
    }

    fn font_change(self, fonts: &[String]) -> CandidateFontChange {
        candidate_font_change(fonts, self.font_loaded.get(), self.font.get())
    }
}

fn appearance_edits(
    snapshot: &ConfigSnapshot,
    themes: &[ThemeChoice],
    theme: usize,
    font_size: f64,
    opacity_position: f32,
) -> Vec<ConfigEdit> {
    let mut edits = Vec::new();
    if let Some(theme) = themes.get(theme) {
        if theme.id != snapshot.appearance().theme() {
            edits.push(ConfigEdit::Theme(theme.id.clone()));
        }
    }
    let font_size =
        font_size.clamp(CANDIDATE_FONT_SIZE_RANGE.0, CANDIDATE_FONT_SIZE_RANGE.1) as f32;
    if (font_size - snapshot.fonts().candidate().size_dip()).abs() >= 0.5 {
        edits.push(ConfigEdit::CandidateFontSizeDip(font_size));
    }
    let opacity = opacity_from_slider(opacity_position);
    if (opacity - snapshot.candidate().opacity()).abs() >= 0.005 {
        edits.push(ConfigEdit::CandidateOpacity(opacity));
    }
    edits
}

fn candidate_layout_mode(snapshot: &ConfigSnapshot) -> Result<CandidateLayoutMode, String> {
    match snapshot.candidate().layout_type() {
        "automatic" => Ok(CandidateLayoutMode::Automatic),
        "stacked" => Ok(CandidateLayoutMode::Stacked),
        "flow" => Ok(CandidateLayoutMode::Flow),
        "scroll" => Ok(CandidateLayoutMode::Scroll),
        "vertical_text" => Ok(CandidateLayoutMode::VerticalText),
        other => Err(format!(
            "Config Core returned an invalid candidate layout_type {other}"
        )),
    }
}

#[derive(Debug)]
enum RunMode {
    Interactive,
    WindUiScreenshot,
    ConfigCoreCli(ConfigCoreCli),
}

#[derive(Debug)]
struct ConfigCoreCli {
    path: PathBuf,
    action: ConfigCoreCliAction,
}

#[derive(Debug)]
enum ConfigCoreCliAction {
    Get,
    Set(ConfigEdit),
    Validate,
    Diff(ConfigEdit),
    Reset(ConfigField),
    Import(PathBuf),
    Export(PathBuf),
    Doctor,
}

pub(crate) fn main() {
    let mut args = env::args_os().skip(1);
    let mut mode: Option<RunMode> = None;
    let mut report: Option<PathBuf> = None;
    let mut locale_preview: Option<String> = None;

    while let Some(arg) = args.next() {
        if arg == "--config" {
            let cli = parse_config_core_cli(&mut args).unwrap_or_else(|error| {
                eprintln!("{error}");
                std::process::exit(2);
            });
            set_run_mode(&mut mode, RunMode::ConfigCoreCli(cli));
        } else if arg == "--screenshot" {
            let Some(_path) = args.next() else {
                eprintln!("--screenshot requires a path");
                std::process::exit(2);
            };
            set_run_mode(&mut mode, RunMode::WindUiScreenshot);
        } else if arg == "--locale-preview" {
            let Some(locale) = args.next().and_then(|value| value.into_string().ok()) else {
                eprintln!("--locale-preview requires a supported locale tag");
                std::process::exit(2);
            };
            if !is_supported_locale(&locale) {
                eprintln!("unsupported Settings locale preview: {locale}");
                std::process::exit(2);
            }
            locale_preview = Some(locale);
        } else if arg == "--scale" {
            let Some(_scale) = args.next() else {
                eprintln!("--scale requires a value");
                std::process::exit(2);
            };
        } else if arg == "--size" {
            let (Some(_width), Some(_height)) = (args.next(), args.next()) else {
                eprintln!("--size requires width and height");
                std::process::exit(2);
            };
        } else if arg == "--renderer" {
            let Some(_renderer) = args.next() else {
                eprintln!("--renderer requires auto, software, or gpu");
                std::process::exit(2);
            };
        } else if arg == "--click" || arg == "--rclick" || arg == "--hover" {
            let (Some(_x), Some(_y)) = (args.next(), args.next()) else {
                eprintln!("{} requires x and y", arg.to_string_lossy());
                std::process::exit(2);
            };
        } else if arg == "--drag" {
            let (Some(_x0), Some(_y0), Some(_x1), Some(_y1)) =
                (args.next(), args.next(), args.next(), args.next())
            else {
                eprintln!("--drag requires x0 y0 x1 y1");
                std::process::exit(2);
            };
        } else if arg == "--type" || arg == "--key" {
            let Some(_value) = args.next() else {
                eprintln!("{} requires a value", arg.to_string_lossy());
                std::process::exit(2);
            };
        } else if arg == "--report" {
            let Some(path) = args.next() else {
                eprintln!("--report requires a path");
                std::process::exit(2);
            };
            report = Some(PathBuf::from(path));
        } else {
            eprintln!("unknown argument: {}", arg.to_string_lossy());
            std::process::exit(2);
        }
    }

    let mode = if let Some(mode) = mode {
        mode
    } else if report.is_none() {
        RunMode::Interactive
    } else {
        eprintln!(
            "usage: fcitx5-config [--screenshot PATH | --config CONFIG_PATH COMMAND] [--report PATH]"
        );
        std::process::exit(2);
    };

    if locale_preview.is_some() && !matches!(&mode, RunMode::WindUiScreenshot) {
        eprintln!("--locale-preview is only valid with --screenshot");
        std::process::exit(2);
    }

    let result = match mode {
        RunMode::Interactive => run_default_interactive_window(),
        RunMode::WindUiScreenshot => run_windui_settings_window(true, locale_preview.as_deref()),
        RunMode::ConfigCoreCli(cli) => run_config_core_cli(cli),
    };

    match result {
        Ok(output) => {
            if let Some(path) = report {
                write_report(&path, &output);
                println!("config-report={} result=PASS", path.display());
            } else {
                println!("{output}");
            }
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

fn parse_config_core_cli(
    args: &mut impl Iterator<Item = OsString>,
) -> Result<ConfigCoreCli, String> {
    let path = PathBuf::from(
        args.next()
            .ok_or("--config requires CONFIG_PATH and COMMAND")?,
    );
    let command = config_core_cli_next(args, "command")?;
    let action = match command.as_str() {
        "get" => ConfigCoreCliAction::Get,
        "validate" => ConfigCoreCliAction::Validate,
        "doctor" => ConfigCoreCliAction::Doctor,
        "set" | "diff" => {
            let field = config_core_cli_next(args, &command)?;
            let value = config_core_cli_next(args, &command)?;
            let edit = ConfigEdit::from_cli(&field, &value).map_err(|error| error.to_string())?;
            if command == "set" {
                ConfigCoreCliAction::Set(edit)
            } else {
                ConfigCoreCliAction::Diff(edit)
            }
        }
        "reset" => {
            let field = config_core_cli_next(args, &command)?;
            ConfigCoreCliAction::Reset(
                ConfigField::from_cli(&field).map_err(|error| error.to_string())?,
            )
        }
        "import" => {
            ConfigCoreCliAction::Import(PathBuf::from(config_core_cli_next(args, &command)?))
        }
        "export" => {
            ConfigCoreCliAction::Export(PathBuf::from(config_core_cli_next(args, &command)?))
        }
        _ => return Err(format!("unsupported --config command {command}")),
    };
    if args.next().is_some() {
        return Err("--config received too many arguments".to_owned());
    }
    Ok(ConfigCoreCli { path, action })
}

fn config_core_cli_next(
    args: &mut impl Iterator<Item = OsString>,
    command: &str,
) -> Result<String, String> {
    args.next()
        .and_then(|value| value.into_string().ok())
        .ok_or_else(|| format!("--config {command} requires an argument"))
}

fn run_config_core_cli(cli: ConfigCoreCli) -> Result<String, String> {
    let store = FileStore::new();
    match cli.action {
        ConfigCoreCliAction::Get => {
            let mut core = recovered_config_core(&store, &cli.path)?;
            match core
                .execute(ConfigCommand::Get, &store, &cli.path)
                .map_err(|error| error.to_string())?
            {
                fcitx5_config_core::CommandOutput::Snapshot(snapshot) => {
                    serde_json::to_string_pretty(&snapshot).map_err(|error| error.to_string())
                }
                _ => Err("Config Core get returned an unexpected result".to_owned()),
            }
        }
        ConfigCoreCliAction::Set(edit) => {
            let mut core = recovered_config_core(&store, &cli.path)?;
            core.execute(ConfigCommand::Set(edit), &store, &cli.path)
                .map_err(|error| error.to_string())?;
            core.apply(&store, &cli.path, fcitx5_config_core::CommitFault::None)
                .map_err(|error| error.to_string())?;
            Ok("config-core set result=PASS".to_owned())
        }
        ConfigCoreCliAction::Validate => {
            let recovery =
                ConfigCore::recover(&store, &cli.path).map_err(|error| error.to_string())?;
            if recovery.source != RecoverySource::Current {
                return Err(format!(
                    "config-core validate recovery={}",
                    recovery_source_name(recovery.source)
                ));
            }
            let mut core = recovery.core;
            core.execute(ConfigCommand::Validate, &store, &cli.path)
                .map_err(|error| error.to_string())?;
            Ok("config-core validate result=PASS".to_owned())
        }
        ConfigCoreCliAction::Diff(edit) => {
            let mut core = recovered_config_core(&store, &cli.path)?;
            core.execute(ConfigCommand::Set(edit), &store, &cli.path)
                .map_err(|error| error.to_string())?;
            match core
                .execute(ConfigCommand::Diff, &store, &cli.path)
                .map_err(|error| error.to_string())?
            {
                fcitx5_config_core::CommandOutput::Diff(differences) => {
                    Ok(format!("config-core diff fields={}", differences.len()))
                }
                _ => Err("Config Core diff returned an unexpected result".to_owned()),
            }
        }
        ConfigCoreCliAction::Reset(field) => {
            let mut core = recovered_config_core(&store, &cli.path)?;
            core.execute(ConfigCommand::Reset(field), &store, &cli.path)
                .map_err(|error| error.to_string())?;
            core.apply(&store, &cli.path, fcitx5_config_core::CommitFault::None)
                .map_err(|error| error.to_string())?;
            Ok("config-core reset result=PASS".to_owned())
        }
        ConfigCoreCliAction::Import(import_path) => {
            let mut core = recovered_config_core(&store, &cli.path)?;
            core.import_from_path(&store, &import_path)
                .map_err(|error| error.to_string())?;
            core.apply(&store, &cli.path, fcitx5_config_core::CommitFault::None)
                .map_err(|error| error.to_string())?;
            Ok("config-core import result=PASS".to_owned())
        }
        ConfigCoreCliAction::Export(export_path) => {
            let core = recovered_config_core(&store, &cli.path)?;
            core.export_to(&store, &export_path)
                .map_err(|error| error.to_string())?;
            Ok("config-core export result=PASS".to_owned())
        }
        ConfigCoreCliAction::Doctor => {
            let recovery =
                ConfigCore::recover(&store, &cli.path).map_err(|error| error.to_string())?;
            Ok(format!(
                "config-core doctor recovery={}",
                recovery_source_name(recovery.source)
            ))
        }
    }
}

fn recovered_config_core(store: &FileStore, path: &Path) -> Result<ConfigCore, String> {
    ConfigCore::recover(store, path)
        .map(|recovery| recovery.core)
        .map_err(|error| error.to_string())
}

fn recovery_source_name(source: RecoverySource) -> &'static str {
    match source {
        RecoverySource::Current => "current",
        RecoverySource::LastKnownGood => "last-known-good",
        RecoverySource::SafeDefaults => "compiled-safe-defaults",
    }
}

fn set_run_mode(mode: &mut Option<RunMode>, next: RunMode) {
    if mode.replace(next).is_some() {
        eprintln!("expected exactly one Config test mode");
        std::process::exit(2);
    }
}

fn current_component_name() -> &'static str {
    let Some(stem) = env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|stem| stem.to_owned()))
    else {
        return CONFIG_DEV_COMPONENT;
    };
    let stem = stem.to_string_lossy();
    if stem.eq_ignore_ascii_case(CONFIG_SHIPPING_COMPONENT) {
        CONFIG_SHIPPING_COMPONENT
    } else {
        CONFIG_DEV_COMPONENT
    }
}

fn write_report(path: &Path, output: &str) {
    if let Some(parent) = path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("failed to create report directory: {error}");
            std::process::exit(1);
        }
    }
    if let Err(error) = fs::write(path, output.as_bytes()) {
        eprintln!("failed to write report: {error}");
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn run_default_interactive_window() -> Result<String, String> {
    run_windui_settings_window(false, None)
}

#[cfg(not(windows))]
fn run_default_interactive_window() -> Result<String, String> {
    Err("Rust Config interactive window requires Windows".to_owned())
}

#[cfg(windows)]
fn run_windui_settings_window(
    screenshot_from_args: bool,
    locale_preview: Option<&str>,
) -> Result<String, String> {
    let (candidate_adapter, candidate_status) =
        windui_candidate_config_manager(windui_config_path()?)?;
    let mode = candidate_adapter.with(|adapter| adapter.preview().appearance().mode().to_owned());
    let dark = shell_uses_dark(&mode, windui::event::system_prefers_dark());
    let mut app = WindUiApp::new(PRODUCT_NAME, 1040, 700);
    if let Some(icon) = fcitx5_logo() {
        app = app.icon(icon);
    }
    let mut app = app
        .frameless()
        .min_size(900, 620)
        .theme(windui_settings_shell_theme(dark));
    if screenshot_from_args {
        app = app.screenshot_from_args();
    }
    let shell_theme = app.theme_handle();
    SHELL_THEME.with(|slot| *slot.borrow_mut() = Some(shell_theme));
    app = app.on_system_theme_changed(move |_ctx, _dark| {
        sync_settings_shell_theme(candidate_adapter);
    });
    let configured_locale =
        candidate_adapter.with(|adapter| adapter.preview().ui().language().to_owned());
    let locale = LocaleCatalog::new(
        locale_preview.unwrap_or(&configured_locale),
        fcitx5_control_core::current_settings_ui_locale(),
    );
    let (snapshot, busy, status, sender) =
        windui_plugin_manager(&mut app, !screenshot_from_args, locale.clone());
    let restart_sender = windui_engine_restart_channel(&mut app, locale.clone());
    let input_method_choices = installed_input_method_choices(&locale);
    app.content(windui_settings_root(
        snapshot,
        busy,
        status,
        sender,
        candidate_adapter,
        candidate_status,
        input_method_choices,
        restart_sender,
        locale,
    ))
    .run();
    Ok(format!(
        "{{\n  \"component\":\"{}\",\n  \"kind\":\"rust-config-windui-settings-shell\",\n  \"real_window\":true,\n  \"no_arg_launch\":{},\n  \"windui_app_default_interactive\":true,\n  \"settings_input_visual_baseline\":true,\n  \"legacy_win32_preview_host_deleted\":true,\n  \"stage\":\"Rust wind-ui Settings Shell\",\n  \"rust_config_cutover_complete\":false,\n  \"result\":\"PASS\"\n}}",
        current_component_name(),
        !screenshot_from_args,
    ))
}

#[cfg(not(windows))]
fn run_windui_settings_window(
    _screenshot_from_args: bool,
    _locale_preview: Option<&str>,
) -> Result<String, String> {
    Err("Rust wind-ui Settings Shell requires Windows".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(std::path::PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let thread_name = std::thread::current()
                .name()
                .unwrap_or("test")
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                        character
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            let path = std::env::temp_dir().join(format!(
                "fcitx5-config-{name}-{}-{}",
                std::process::id(),
                thread_name
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn candidate_layout_uses_frozen_names_and_migrated_semantics() {
        assert_eq!(CandidateLayoutMode::Automatic.label(), "自动");
        assert_eq!(CandidateLayoutMode::Stacked.label(), "纵排");
        assert_eq!(CandidateLayoutMode::Flow.label(), "横排");
        assert_eq!(CandidateLayoutMode::Scroll.label(), "卷轴");
        assert_eq!(CandidateLayoutMode::VerticalText.label(), "竖排文字");
        assert!(CandidateLayoutMode::Scroll
            .preview_description(5)
            .contains("5"));
    }

    #[test]
    fn candidate_page_size_is_authoritative_and_strictly_bounded() {
        for page_size in 1..=9 {
            let visible_slots = (1..=9).filter(|slot| *slot <= page_size).count();
            assert_eq!(visible_slots, usize::from(page_size));
            assert!(CandidateLayoutMode::Scroll
                .preview_description(page_size)
                .contains(&page_size.to_string()));
        }
    }

    #[test]
    fn layout_labels_follow_authoritative_page_size() {
        for page_size in [3, 5, 9] {
            assert_eq!(
                CandidateLayoutMode::Scroll.preview_description(page_size),
                format!("卷轴布局（每页最多 {page_size} 个候选）")
            );
        }
    }

    #[test]
    fn candidate_page_size_control_is_not_scroll_visibility_guarded() {
        let source = include_str!("../src/main.rs");
        let controls = source
            .split_once("fn windui_config_core_candidate_layout_controls(")
            .and_then(|(_, rest)| rest.split_once("\nfn "))
            .map(|(body, _)| body)
            .expect("candidate layout controls should remain a single source section");
        let page_size_label = controls
            .find("\"candidate.page_size\"")
            .expect("candidate page-size row should remain named");
        let row_start = controls[..page_size_label]
            .rfind("WindUiElement::col()")
            .expect("candidate page-size row should have a layout container");
        let page_size_row = &controls[row_start..page_size_label];

        assert!(controls.contains("settings.candidate.page_size_hint"));
        assert!(!page_size_row.contains("CandidateLayoutMode::Scroll"));
        assert!(!page_size_row.contains(".visible_when"));
    }

    fn ui_options(
        orientation: CandidateLayoutOrientation,
        overflow: CandidateLayoutOverflow,
        writing_mode: CandidateWritingMode,
    ) -> CandidateLayoutUiOptions {
        CandidateLayoutUiOptions {
            orientation,
            overflow,
            writing_mode,
            ..CandidateLayoutUiOptions::default()
        }
    }

    #[test]
    fn two_axis_decode_matches_config_core_legacy_table() {
        assert_eq!(
            CandidateLayoutMode::Automatic.options(),
            ui_options(
                CandidateLayoutOrientation::Horizontal,
                CandidateLayoutOverflow::Paging,
                CandidateWritingMode::Horizontal
            )
        );
        assert_eq!(
            CandidateLayoutMode::Stacked.options(),
            ui_options(
                CandidateLayoutOrientation::Vertical,
                CandidateLayoutOverflow::Paging,
                CandidateWritingMode::Horizontal
            )
        );
        assert_eq!(
            CandidateLayoutMode::Flow.options(),
            ui_options(
                CandidateLayoutOrientation::Horizontal,
                CandidateLayoutOverflow::Wrapping,
                CandidateWritingMode::Horizontal
            )
        );
        assert_eq!(
            CandidateLayoutMode::Scroll.options(),
            ui_options(
                CandidateLayoutOrientation::Horizontal,
                CandidateLayoutOverflow::Scrolling,
                CandidateWritingMode::Horizontal
            )
        );
        assert_eq!(
            CandidateLayoutMode::VerticalText.options(),
            ui_options(
                CandidateLayoutOrientation::Vertical,
                CandidateLayoutOverflow::Paging,
                CandidateWritingMode::VerticalRl
            )
        );
    }

    #[test]
    fn two_axis_encode_round_trips_to_legacy_layout_type() {
        assert_eq!(
            ui_options(
                CandidateLayoutOrientation::Horizontal,
                CandidateLayoutOverflow::Paging,
                CandidateWritingMode::Horizontal
            )
            .layout_type(),
            "automatic"
        );
        assert_eq!(
            ui_options(
                CandidateLayoutOrientation::Vertical,
                CandidateLayoutOverflow::Paging,
                CandidateWritingMode::Horizontal
            )
            .layout_type(),
            "stacked"
        );
        assert_eq!(
            ui_options(
                CandidateLayoutOrientation::Horizontal,
                CandidateLayoutOverflow::Wrapping,
                CandidateWritingMode::Horizontal
            )
            .layout_type(),
            "flow"
        );
        assert_eq!(
            ui_options(
                CandidateLayoutOrientation::Vertical,
                CandidateLayoutOverflow::Scrolling,
                CandidateWritingMode::Horizontal
            )
            .layout_type(),
            "scroll"
        );
        for writing in [
            CandidateWritingMode::VerticalRl,
            CandidateWritingMode::VerticalLr,
        ] {
            assert_eq!(
                ui_options(
                    CandidateLayoutOrientation::Horizontal,
                    CandidateLayoutOverflow::Scrolling,
                    writing
                )
                .layout_type(),
                "vertical_text"
            );
        }
    }

    #[test]
    fn only_relevant_direction_survives_the_typed_config_round_trip() {
        let store = FileStore::new();
        let path = Path::new("candidate-layout-directions.toml");
        let mut core = ConfigCore::compiled_defaults();
        core.execute(
            ConfigCommand::Set(ConfigEdit::CandidateLayoutType("scroll".to_owned())),
            &store,
            path,
        )
        .expect("scroll layout should set");
        core.execute(
            ConfigCommand::Set(ConfigEdit::CandidateScrollDirection(
                ScrollDirection::Vertical,
            )),
            &store,
            path,
        )
        .expect("scroll direction should set");
        assert_eq!(
            candidate_layout_ui_options(&core.preview()).scroll_direction,
            ScrollDirection::Vertical
        );

        core.execute(
            ConfigCommand::Set(ConfigEdit::CandidateLayoutType("vertical_text".to_owned())),
            &store,
            path,
        )
        .expect("vertical text layout should set");
        core.execute(
            ConfigCommand::Set(ConfigEdit::CandidateVerticalTextColumnDirection(
                VerticalTextColumnDirection::LeftToRight,
            )),
            &store,
            path,
        )
        .expect("vertical text column direction should set");
        let options = candidate_layout_ui_options(&core.preview());
        assert_eq!(options.writing_mode, CandidateWritingMode::VerticalLr);
        assert_eq!(
            options.vertical_text_column_direction,
            VerticalTextColumnDirection::LeftToRight
        );
    }

    #[test]
    fn snapshot_layout_type_decodes_and_ui_commits_through_config_core() {
        let store = FileStore::new();
        let path = Path::new("two-axis-snapshot.toml");
        for (legacy, expected) in [
            ("automatic", CandidateLayoutMode::Automatic),
            ("stacked", CandidateLayoutMode::Stacked),
            ("flow", CandidateLayoutMode::Flow),
            ("scroll", CandidateLayoutMode::Scroll),
            ("vertical_text", CandidateLayoutMode::VerticalText),
        ] {
            let mut core = ConfigCore::compiled_defaults();
            core.execute(
                ConfigCommand::Set(ConfigEdit::CandidateLayoutType(legacy.to_owned())),
                &store,
                path,
            )
            .expect("legacy layout_type should set");
            assert_eq!(core.preview().candidate().layout_type(), legacy);
            assert_eq!(
                candidate_layout_ui_options(&core.preview()).layout_type(),
                legacy,
                "{legacy} must round-trip through the two-axis decode"
            );
            assert_eq!(
                candidate_layout_ui_options(&core.preview()),
                expected.options(),
                "{legacy} must decode to the frozen two-axis pair"
            );
        }
    }

    #[test]
    fn vertical_preview_description_is_never_wrapping() {
        let vertical_paging = ui_options(
            CandidateLayoutOrientation::Vertical,
            CandidateLayoutOverflow::Paging,
            CandidateWritingMode::Horizontal,
        );
        assert!(vertical_paging.preview_description(5).contains("纵排"));
        assert!(vertical_paging.preview_description(5).contains("分页"));
        assert!(vertical_paging
            .preview_description(5)
            .contains(&5.to_string()));
        let writing = ui_options(
            CandidateLayoutOrientation::Vertical,
            CandidateLayoutOverflow::Paging,
            CandidateWritingMode::VerticalLr,
        );
        assert!(writing.preview_description(5).contains("从左到右"));
        let horizontal_wrap = ui_options(
            CandidateLayoutOrientation::Horizontal,
            CandidateLayoutOverflow::Wrapping,
            CandidateWritingMode::Horizontal,
        );
        assert!(horizontal_wrap.preview_description(5).contains("自动换行"));
    }

    #[test]
    fn windui_candidate_adapter_uses_one_draft_for_preview_cancel_reset_and_apply() {
        for layout in ["automatic", "stacked", "flow", "scroll", "vertical_text"] {
            for page_size in 1..=9 {
                let directory =
                    TestDirectory::new(&format!("windui-candidate-adapter-{layout}-{page_size}"));
                let path = directory.path().join("config.toml");
                let mut adapter =
                    WindUiConfigAdapter::load(path).expect("adapter should load defaults");

                adapter
                    .set(ConfigEdit::CandidateLayoutType(layout.to_owned()))
                    .expect("layout edit should update Draft");
                adapter
                    .set(ConfigEdit::CandidatePageSize(page_size))
                    .expect("page-size edit should update Draft");
                let preview = adapter.preview();
                let candidate = preview.candidate();
                assert_eq!(candidate.layout_type(), layout);
                assert_eq!(candidate.page_size(), page_size);
                assert!(
                    !adapter.path().exists(),
                    "editing Draft must not write Current"
                );

                adapter.cancel();
                let preview = adapter.preview();
                let candidate = preview.candidate();
                assert_eq!(candidate.layout_type(), "scroll");
                assert_eq!(candidate.page_size(), 5);
                assert!(!adapter.path().exists(), "cancel must not write Current");

                adapter
                    .set(ConfigEdit::CandidateLayoutType(layout.to_owned()))
                    .expect("layout edit should update Draft before reset");
                adapter
                    .set(ConfigEdit::CandidatePageSize(page_size))
                    .expect("page-size edit should update Draft before reset");
                adapter.reset(ConfigField::CandidatePageSize);
                let preview = adapter.preview();
                let candidate = preview.candidate();
                assert_eq!(candidate.layout_type(), layout);
                assert_eq!(candidate.page_size(), 5);
                assert!(!adapter.path().exists(), "reset must not write Current");

                adapter
                    .set(ConfigEdit::CandidatePageSize(page_size))
                    .expect("page-size edit should update Draft before apply");
                adapter.apply().expect("apply should commit Draft");
                assert!(adapter.path().is_file(), "apply must create Current");
                assert!(
                    FileStore::last_known_good_path(adapter.path()).is_file(),
                    "apply must create a usable LKG"
                );
                let persisted = WindUiConfigAdapter::load(adapter.path().to_path_buf())
                    .expect("applied Current should be readable");
                let preview = persisted.preview();
                let candidate = preview.candidate();
                assert_eq!(candidate.layout_type(), layout);
                assert_eq!(candidate.page_size(), page_size);
            }
        }
    }

    #[test]
    fn candidate_layout_controls_cover_every_reachable_button_state() {
        let directory = TestDirectory::new("candidate-layout-controls");
        let path = directory.path().join("config.toml");
        let mut adapter = WindUiConfigAdapter::load(path).expect("adapter should load defaults");

        for layout in ["automatic", "stacked", "flow"] {
            adapter
                .set(ConfigEdit::CandidateLayoutType(layout.to_owned()))
                .expect("layout button should update Draft");
            assert_eq!(adapter.preview().candidate().layout_type(), layout);
            adapter
                .apply()
                .expect("apply should persist every layout button");
        }

        for direction in [ScrollDirection::Horizontal, ScrollDirection::Vertical] {
            for page_size in 1..=9 {
                adapter
                    .set(ConfigEdit::CandidateLayoutType("scroll".to_owned()))
                    .expect("scroll mode should update Draft");
                adapter
                    .set(ConfigEdit::CandidateScrollDirection(direction))
                    .expect("scroll direction button should update Draft");
                adapter
                    .set(ConfigEdit::CandidatePageSize(page_size))
                    .expect("candidate-count button should update Draft");
                let preview = adapter.preview();
                let candidate = preview.candidate();
                assert_eq!(candidate.layout_type(), "scroll");
                assert_eq!(candidate.scroll_direction(), direction);
                assert_eq!(candidate.page_size(), page_size);
                adapter
                    .apply()
                    .expect("apply should persist every scroll button combination");
            }
        }

        for direction in [
            VerticalTextColumnDirection::RightToLeft,
            VerticalTextColumnDirection::LeftToRight,
        ] {
            adapter
                .set(ConfigEdit::CandidateLayoutType("vertical_text".to_owned()))
                .expect("vertical-text mode should update Draft");
            adapter
                .set(ConfigEdit::CandidateVerticalTextColumnDirection(direction))
                .expect("column direction button should update Draft");
            let preview = adapter.preview();
            let candidate = preview.candidate();
            assert_eq!(candidate.layout_type(), "vertical_text");
            assert_eq!(candidate.vertical_text_column_direction(), direction);
            adapter
                .apply()
                .expect("apply should persist both column-direction buttons");
        }

        adapter
            .set(ConfigEdit::CandidatePageSize(1))
            .expect("draft change should succeed before cancel");
        adapter.cancel();
        assert_eq!(adapter.preview().candidate().layout_type(), "vertical_text");
        assert_eq!(
            adapter
                .preview()
                .candidate()
                .vertical_text_column_direction(),
            VerticalTextColumnDirection::LeftToRight
        );

        adapter.reset_candidate_layout();
        let preview = adapter.preview();
        let candidate = preview.candidate();
        assert_eq!(candidate.layout_type(), "scroll");
        assert_eq!(candidate.scroll_direction(), ScrollDirection::Horizontal);
        assert_eq!(
            candidate.vertical_text_column_direction(),
            VerticalTextColumnDirection::RightToLeft
        );
        assert_eq!(candidate.page_size(), 5);
    }

    fn package_json(id: &str, installed: Option<&str>, state: Option<&str>) -> String {
        format!(
            r#"{{"format_version":1,"repository_available":true,"repository_error":null,"packages":[{{"id":"{id}","title":"Rime","summary":"Rime input method","type":"addon","available_version":"1.2.3","installed_version":{},"state":{},"update_available":false}}]}}"#,
            installed
                .map(|value| format!(r#""{value}""#))
                .unwrap_or_else(|| "null".to_owned()),
            state
                .map(|value| format!(r#""{value}""#))
                .unwrap_or_else(|| "null".to_owned()),
        )
    }

    fn package_json_with(
        id: &str,
        title: &str,
        summary: &str,
        package_type: &str,
        available: Option<&str>,
        installed: Option<&str>,
        state: Option<&str>,
        update_available: bool,
        repository_available: bool,
    ) -> String {
        let optional = |value: Option<&str>| {
            value
                .map(|value| format!(r#""{value}""#))
                .unwrap_or_else(|| "null".to_owned())
        };
        format!(
            r#"{{"format_version":1,"repository_available":{repository_available},"repository_error":null,"packages":[{{"id":"{id}","title":"{title}","summary":"{summary}","type":"{package_type}","available_version":{},"installed_version":{},"state":{},"update_available":{update_available}}}]}}"#,
            optional(available),
            optional(installed),
            optional(state),
        )
    }

    #[test]
    fn pinned_plugin_catalog_is_complete_and_unique() {
        assert_eq!(FCITX5_PLUGIN_CATALOG.len(), 19);
        let mut ids = FCITX5_PLUGIN_CATALOG
            .iter()
            .map(|plugin| plugin.id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), FCITX5_PLUGIN_CATALOG.len());
        assert_eq!(
            FCITX5_PLUGIN_CATALOG
                .iter()
                .filter(|plugin| plugin.windows_package)
                .map(|plugin| plugin.id)
                .collect::<Vec<_>>(),
            ["fcitx5-chinese-addons", "fcitx5-rime", "fcitx5-lua"]
        );
    }

    #[test]
    fn plugin_control_arguments_validate_ids_without_a_catalog_whitelist() {
        assert_eq!(
            plugin_control_arguments(&PluginOperation::SetState {
                id: "fcitx5-rime".to_owned(),
                enabled: false,
            })
            .expect("supported package"),
            ["--packages-state", "fcitx5-rime", "disabled"]
                .map(OsString::from)
                .to_vec()
        );
        assert!(plugin_control_arguments(&PluginOperation::Install(
            "fcitx5-rime --packages-repair".to_owned()
        ))
        .is_err());
        assert_eq!(
            plugin_control_arguments(&PluginOperation::Install("fcitx5-mozc".to_owned()))
                .expect("valid package IDs are not restricted to the reference catalog"),
            ["--packages-install", "fcitx5-mozc"]
                .map(OsString::from)
                .to_vec()
        );
        assert!(
            plugin_control_arguments(&PluginOperation::Install("vendor.addon_1".to_owned()))
                .is_ok()
        );
    }

    #[test]
    fn package_list_parser_rejects_malformed_and_oversized_control_data() {
        assert!(parse_control_package_list("not json").is_err());
        let unknown = package_json("fcitx5-rime", None, None).replace(
            r#""update_available":false"#,
            r#""update_available":false,"unexpected":true"#,
        );
        assert!(parse_control_package_list(&unknown).is_err());
        assert!(parse_control_package_list(&package_json(
            "fcitx5-rime",
            Some("1.0.0"),
            Some("mystery")
        ))
        .is_err());
        assert!(parse_control_package_list(&"x".repeat(CONTROL_MAX_OUTPUT_BYTES + 1)).is_err());
    }

    #[test]
    fn plugin_manager_internal_errors_are_localized_for_every_supported_locale() {
        for locale in [
            "en-US", "zh-CN", "zh-TW", "ja-JP", "ko-KR", "vi-VN", "th-TH", "si-LK",
        ] {
            let catalog = LocaleCatalog::new(locale, "en-US");
            for (error, key) in [
                (
                    "插件包 ID 无效：bad/id",
                    "settings.plugins.error.invalid_id",
                ),
                (
                    "Control 返回的插件目录超过大小限制",
                    "settings.plugins.error.catalog_too_large",
                ),
                (
                    "Control 插件目录版本或条目数无效",
                    "settings.plugins.error.catalog_version_invalid",
                ),
                (
                    "Control 插件目录字段无效",
                    "settings.plugins.error.catalog_fields_invalid",
                ),
                (
                    "Control 插件目录 JSON 无效：details",
                    "settings.plugins.error.control_unavailable",
                ),
                (
                    "运行 fcitx5-control.exe 失败：details",
                    "settings.plugins.error.control_unavailable",
                ),
            ] {
                let translated = localized_plugin_error(&catalog, error);
                let expected = if key.ends_with("invalid_id") {
                    catalog.format(key, &[("id", "bad/id")])
                } else {
                    catalog.text(key)
                };
                assert_eq!(translated, expected, "{locale} did not use {key}");
            }
        }
    }

    #[test]
    fn plugin_operation_matrix_keeps_state_actions_available_offline() {
        let available = parse_control_package_list(&package_json_with(
            "community-addon",
            "Community",
            "Community addon",
            "addon",
            Some("1.2.3"),
            None,
            None,
            false,
            true,
        ))
        .expect("available package");
        assert_eq!(
            plugin_operation_for(&available, "community-addon", PluginAction::Install),
            Some(PluginOperation::Install("community-addon".to_owned()))
        );
        assert_eq!(
            plugin_operation_for(&available, "community-addon", PluginAction::Update),
            None
        );

        let installed = parse_control_package_list(&package_json_with(
            "community-addon",
            "Community",
            "Community addon",
            "addon",
            Some("2.0.0"),
            Some("1.0.0"),
            Some("enabled"),
            true,
            false,
        ))
        .expect("installed package");
        assert_eq!(
            plugin_operation_for(&installed, "community-addon", PluginAction::Update),
            Some(PluginOperation::Update("community-addon".to_owned()))
        );
        assert_eq!(
            plugin_operation_for(&installed, "community-addon", PluginAction::Toggle),
            Some(PluginOperation::SetState {
                id: "community-addon".to_owned(),
                enabled: false,
            })
        );
        assert_eq!(
            plugin_operation_for(&installed, "community-addon", PluginAction::Remove),
            Some(PluginOperation::Remove("community-addon".to_owned()))
        );

        let offline = PluginManagerSnapshot {
            repository_available: false,
            ..installed
        };
        assert_eq!(
            plugin_operation_for(&offline, "community-addon", PluginAction::Toggle),
            Some(PluginOperation::SetState {
                id: "community-addon".to_owned(),
                enabled: false,
            })
        );
        assert_eq!(
            plugin_operation_for(&offline, "community-addon", PluginAction::Remove),
            Some(PluginOperation::Remove("community-addon".to_owned()))
        );
    }

    #[test]
    fn settings_shell_follows_the_appearance_mode() {
        assert!(shell_uses_dark("dark", false));
        assert!(!shell_uses_dark("light", true));
        assert!(shell_uses_dark("system", true));
        assert!(!shell_uses_dark("system", false));
    }

    #[test]
    fn candidate_font_choice_maps_to_config_changes() {
        let snapshot = ConfigCore::compiled_defaults().preview();
        let locale = LocaleCatalog::new("en-US", "en-US");
        let configured = snapshot.fonts().candidate().families()[0].clone();
        let fonts = candidate_font_choices(
            &locale,
            vec!["Segoe UI".to_owned(), configured.to_uppercase()],
        );
        assert_eq!(fonts[0], "Follow theme");
        let loaded = candidate_font_index(&snapshot, &fonts);
        assert_eq!(
            loaded, 2,
            "matches the configured family case-insensitively"
        );
        assert_eq!(
            candidate_font_change(&fonts, loaded, loaded),
            CandidateFontChange::Keep
        );
        assert_eq!(
            candidate_font_change(&fonts, loaded, 1),
            CandidateFontChange::Use("Segoe UI".to_owned())
        );
        assert_eq!(
            candidate_font_change(&fonts, loaded, 0),
            CandidateFontChange::FollowTheme
        );
        let unknown = candidate_font_choices(&locale, vec!["Segoe UI".to_owned()]);
        assert_eq!(candidate_font_index(&snapshot, &unknown), 0);
    }

    #[test]
    fn appearance_save_writes_only_changed_values() {
        let snapshot = ConfigCore::compiled_defaults().preview();
        let themes = theme_choices_from_json(
            r#"{"format_version":1,"themes":[{"id":"builtin:default","source":"builtin","name":"Default","version":"1","license":"MIT","description":""},{"id":"user:ink","source":"user","name":"Ink","version":"1","license":"MIT","description":""}]}"#,
        );
        assert_eq!(themes.len(), 2);
        let current = themes
            .iter()
            .position(|theme| theme.id == snapshot.appearance().theme())
            .unwrap_or(0);
        let font_size = f64::from(snapshot.fonts().candidate().size_dip());
        let opacity = slider_from_opacity(snapshot.candidate().opacity());
        assert!(appearance_edits(&snapshot, &themes, current, font_size, opacity).is_empty());

        let edits = appearance_edits(&snapshot, &themes, 1 - current, font_size + 2.0, 0.0);
        assert_eq!(
            edits,
            vec![
                ConfigEdit::Theme(themes[1 - current].id.clone()),
                ConfigEdit::CandidateFontSizeDip(font_size as f32 + 2.0),
                ConfigEdit::CandidateOpacity(CANDIDATE_OPACITY_MIN),
            ]
        );
        assert_eq!(opacity_from_slider(1.0), 1.0);
        assert_eq!(opacity_from_slider(slider_from_opacity(0.6)), 0.6);
        assert!(theme_choices_from_json("not json").is_empty());
    }

    #[test]
    fn input_method_draft_edits_keep_a_valid_default() {
        let mut draft = InputMethodDraft {
            enabled: vec!["pinyin".to_owned(), "rime".to_owned()],
            default: "pinyin".to_owned(),
        };
        draft.add("rime");
        draft.add("mozc");
        assert_eq!(draft.enabled, ["pinyin", "rime", "mozc"]);
        draft.move_up("mozc");
        draft.move_up("pinyin");
        assert_eq!(draft.enabled, ["pinyin", "mozc", "rime"]);
        draft.make_default("unknown");
        assert_eq!(draft.default, "pinyin");
        draft.remove("pinyin");
        assert_eq!(draft.enabled, ["mozc", "rime"]);
        assert_eq!(draft.default, "mozc");
        draft.remove("rime");
        draft.remove("mozc");
        assert_eq!(draft.enabled, ["mozc"]);
    }

    #[test]
    fn input_method_rows_name_entries_and_hide_enabled_ones_from_add() {
        let choices = vec![
            InputMethodChoice {
                id: "pinyin".to_owned(),
                name: "Pinyin".to_owned(),
            },
            InputMethodChoice {
                id: "rime".to_owned(),
                name: "Rime".to_owned(),
            },
        ];
        let draft = InputMethodDraft {
            enabled: vec!["pinyin".to_owned(), "gone".to_owned()],
            default: "gone".to_owned(),
        };
        let rows = input_method_rows(&draft, &choices);
        assert_eq!(rows[0].name, "Pinyin");
        assert!(rows[0].first && !rows[0].is_default && !rows[0].only);
        assert_eq!(rows[1].name, "gone");
        assert!(rows[1].is_default);
        let addable = addable_input_methods(&draft, &choices);
        assert_eq!(addable.len(), 1);
        assert_eq!(addable[0].id, "rime");
    }

    #[test]
    fn plugin_catalog_rows_merge_control_metadata_and_dynamic_addons() {
        let snapshot = parse_control_package_list(
            r#"{"format_version":1,"repository_available":true,"repository_error":null,"packages":[
                {"id":"fcitx5-rime","title":"Control Rime","summary":"Control-owned summary","type":"addon","available_version":"2.0.0","installed_version":"1.0.0","state":"enabled","update_available":true},
                {"id":"community-addon","title":"Community Addon","summary":"Dynamic addon","type":"addon","available_version":"1.0.0","installed_version":null,"state":null,"update_available":false},
                {"id":"legacy-installed","title":"Legacy Installed","summary":"Installed unknown package","type":"unknown","available_version":null,"installed_version":"0.9.0","state":"enabled","update_available":false},
                {"id":"core-package","title":"Core","summary":"Not a plugin","type":"core","available_version":null,"installed_version":"1.0.0","state":"bundled","update_available":false},
                {"id":"theme-package","title":"Theme","summary":"Not an addon","type":"theme","available_version":null,"installed_version":"1.0.0","state":"installed","update_available":false},
                {"id":"community-addon-data","title":"Data","summary":"Dependency","type":"inputmethod-data","available_version":"1.0.0","installed_version":null,"state":null,"update_available":false}
            ]}"#,
        )
        .expect("valid Control package list");
        let locale = LocaleCatalog::new("en-US", "en-US");
        let rows = plugin_catalog_rows(&snapshot, &locale);
        let ids = rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>();
        assert_eq!(
            ids.len(),
            22,
            "19 references plus addon, installed unknown and input method data"
        );
        assert_eq!(ids.iter().filter(|id| **id == "fcitx5-rime").count(), 1);
        assert!(ids.contains(&"community-addon"));
        assert!(ids.contains(&"legacy-installed"));
        assert!(!ids.contains(&"core-package"));
        assert!(!ids.contains(&"theme-package"));
        assert_eq!(ids.last(), Some(&"community-addon-data"));
        let data = rows.last().expect("input method data row");
        assert_eq!(
            data.category,
            locale.text("packages.type.input_method_data")
        );
        assert_eq!(
            plugin_operation_for(&snapshot, "community-addon-data", PluginAction::Install),
            Some(PluginOperation::Install("community-addon-data".to_owned()))
        );

        let rime = rows
            .iter()
            .find(|row| row.id == "fcitx5-rime")
            .expect("rime row");
        assert_eq!(rime.title, "Control Rime");
        assert_eq!(rime.summary, "Control-owned summary");
        assert!(rime.package.is_some());

        let mozc = rows
            .iter()
            .find(|row| row.id == "fcitx5-mozc")
            .expect("reference row remains visible");
        assert!(mozc.package.is_none());
        assert_eq!(
            plugin_operation_for(&snapshot, "fcitx5-mozc", PluginAction::Install),
            None,
            "source-only reference rows never manufacture actions"
        );
    }

    #[test]
    fn reference_id_does_not_make_non_addon_package_actionable() {
        let snapshot = parse_control_package_list(
            r#"{"format_version":1,"repository_available":true,"repository_error":null,"packages":[
                {"id":"fcitx5-mozc","title":"Mozc translation","summary":"Not a plugin","type":"translation","available_version":"2.0.0","installed_version":"1.0.0","state":"installed","update_available":true}
            ]}"#,
        )
        .expect("valid non-addon reference collision");

        let locale = LocaleCatalog::new("en-US", "en-US");
        let row = plugin_catalog_rows(&snapshot, &locale)
            .into_iter()
            .find(|row| row.id == "fcitx5-mozc")
            .expect("reference row remains visible");
        assert!(
            row.package.is_none(),
            "non-addon metadata is not a plugin row"
        );
        for action in [
            PluginAction::Install,
            PluginAction::Update,
            PluginAction::Toggle,
            PluginAction::Remove,
        ] {
            assert_eq!(
                plugin_operation_for(&snapshot, "fcitx5-mozc", action),
                None,
                "reference collision must not expose {action:?}"
            );
        }
    }

    #[test]
    fn installed_plugin_actions_follow_control_state() {
        let available = parse_control_package_list(&package_json("fcitx5-rime", None, None))
            .expect("available package");
        assert!(!package_allows_installed_action(
            available.package("fcitx5-rime").expect("rime")
        ));

        let enabled = parse_control_package_list(&package_json(
            "fcitx5-rime",
            Some("1.0.0"),
            Some("enabled"),
        ))
        .expect("enabled package");
        assert!(package_allows_installed_action(
            enabled.package("fcitx5-rime").expect("rime")
        ));

        let bundled = parse_control_package_list(&package_json(
            "fcitx5-rime",
            Some("1.0.0"),
            Some("bundled"),
        ))
        .expect("bundled package");
        assert!(!package_allows_installed_action(
            bundled.package("fcitx5-rime").expect("rime")
        ));
    }
}
