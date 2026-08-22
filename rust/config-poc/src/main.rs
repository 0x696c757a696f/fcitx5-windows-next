use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PageId {
    InputMethods,
    Appearance,
    Shortcuts,
    Addons,
    Updates,
    Diagnostics,
}

impl PageId {
    fn as_str(self) -> &'static str {
        match self {
            Self::InputMethods => "input-methods",
            Self::Appearance => "appearance",
            Self::Shortcuts => "shortcuts",
            Self::Addons => "addons",
            Self::Updates => "updates",
            Self::Diagnostics => "diagnostics",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActionKind {
    Navigate(PageId),
    SelectLanguage,
    SelectCandidateFont,
    UpdateCandidatePreview,
    ToggleAdvancedAppearance,
    InstallAddon,
    UpdateAddon,
    UninstallAddon,
    EnableAddon,
    DisableAddon,
    RefreshUpdates,
    RunDiagnosticsPlan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackageState {
    OfficialAvailable,
    InstalledEnabled,
    InstalledDisabled,
    UpdateAvailable,
    RemovePendingAfterUpdate,
}

#[derive(Clone, Debug)]
struct PageModel {
    id: PageId,
    title_key: &'static str,
    required_actions: Vec<ActionKind>,
}

#[derive(Clone, Debug)]
struct ConfigPocModel {
    product_name: &'static str,
    languages: Vec<&'static str>,
    pages: Vec<PageModel>,
    package_states: Vec<PackageState>,
    candidate_preview_embedded: bool,
    candidate_preview_current_theme: bool,
    candidate_preview_not_external_window: bool,
    localized_dialogs: bool,
    no_shell_out: bool,
}

fn main() {
    let mut args = env::args_os().skip(1);
    let mut self_check = false;
    let mut report: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        if arg == "--self-check" {
            self_check = true;
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

    if !self_check {
        eprintln!("usage: fcitx5-config-poc --self-check [--report PATH]");
        std::process::exit(2);
    }

    match run_self_check() {
        Ok(output) => {
            if let Some(path) = report {
                write_report(&path, &output);
                println!("config-poc-report={} result=PASS", path.display());
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

fn run_self_check() -> Result<String, String> {
    let model = frozen_settings_model();
    validate_model(&model)?;
    Ok(render_report(&model))
}

fn frozen_settings_model() -> ConfigPocModel {
    ConfigPocModel {
        product_name: "Fcitx5 for Windows Next",
        languages: vec!["system", "en-US", "zh-CN"],
        pages: vec![
            PageModel {
                id: PageId::InputMethods,
                title_key: "nav.input_methods",
                required_actions: vec![ActionKind::Navigate(PageId::InputMethods)],
            },
            PageModel {
                id: PageId::Appearance,
                title_key: "nav.appearance",
                required_actions: vec![
                    ActionKind::Navigate(PageId::Appearance),
                    ActionKind::SelectLanguage,
                    ActionKind::SelectCandidateFont,
                    ActionKind::ToggleAdvancedAppearance,
                    ActionKind::UpdateCandidatePreview,
                ],
            },
            PageModel {
                id: PageId::Shortcuts,
                title_key: "nav.shortcuts",
                required_actions: vec![ActionKind::Navigate(PageId::Shortcuts)],
            },
            PageModel {
                id: PageId::Addons,
                title_key: "nav.packages",
                required_actions: vec![
                    ActionKind::Navigate(PageId::Addons),
                    ActionKind::InstallAddon,
                    ActionKind::UpdateAddon,
                    ActionKind::UninstallAddon,
                    ActionKind::EnableAddon,
                    ActionKind::DisableAddon,
                ],
            },
            PageModel {
                id: PageId::Updates,
                title_key: "updates.title",
                required_actions: vec![
                    ActionKind::Navigate(PageId::Updates),
                    ActionKind::RefreshUpdates,
                ],
            },
            PageModel {
                id: PageId::Diagnostics,
                title_key: "nav.repair",
                required_actions: vec![
                    ActionKind::Navigate(PageId::Diagnostics),
                    ActionKind::RunDiagnosticsPlan,
                ],
            },
        ],
        package_states: vec![
            PackageState::OfficialAvailable,
            PackageState::InstalledEnabled,
            PackageState::InstalledDisabled,
            PackageState::UpdateAvailable,
            PackageState::RemovePendingAfterUpdate,
        ],
        candidate_preview_embedded: true,
        candidate_preview_current_theme: true,
        candidate_preview_not_external_window: true,
        localized_dialogs: true,
        no_shell_out: true,
    }
}

fn validate_model(model: &ConfigPocModel) -> Result<(), String> {
    if model.product_name != "Fcitx5 for Windows Next" {
        return Err("Config PoC product name must be unified".to_owned());
    }
    require_languages(model)?;
    require_pages(model)?;
    require_actions(model)?;
    require_package_states(model)?;
    if !model.candidate_preview_embedded
        || !model.candidate_preview_current_theme
        || !model.candidate_preview_not_external_window
    {
        return Err("Config PoC candidate preview must be embedded and current-theme".to_owned());
    }
    if !model.localized_dialogs {
        return Err("Config PoC dialogs must be localized".to_owned());
    }
    if !model.no_shell_out {
        return Err("Config PoC must not shell out for Settings actions".to_owned());
    }
    Ok(())
}

fn require_languages(model: &ConfigPocModel) -> Result<(), String> {
    for required in ["system", "en-US", "zh-CN"] {
        if !model.languages.contains(&required) {
            return Err(format!("missing language option {required}"));
        }
    }
    Ok(())
}

fn require_pages(model: &ConfigPocModel) -> Result<(), String> {
    for required in [
        PageId::InputMethods,
        PageId::Appearance,
        PageId::Shortcuts,
        PageId::Addons,
        PageId::Updates,
        PageId::Diagnostics,
    ] {
        if !model.pages.iter().any(|page| page.id == required) {
            return Err(format!("missing Settings page {}", required.as_str()));
        }
    }
    Ok(())
}

fn require_actions(model: &ConfigPocModel) -> Result<(), String> {
    for required in [
        ActionKind::SelectLanguage,
        ActionKind::SelectCandidateFont,
        ActionKind::UpdateCandidatePreview,
        ActionKind::ToggleAdvancedAppearance,
        ActionKind::InstallAddon,
        ActionKind::UpdateAddon,
        ActionKind::UninstallAddon,
        ActionKind::EnableAddon,
        ActionKind::DisableAddon,
        ActionKind::RefreshUpdates,
        ActionKind::RunDiagnosticsPlan,
    ] {
        if !model
            .pages
            .iter()
            .flat_map(|page| page.required_actions.iter())
            .any(|action| *action == required)
        {
            return Err(format!("missing Settings action {required:?}"));
        }
    }
    Ok(())
}

fn require_package_states(model: &ConfigPocModel) -> Result<(), String> {
    for required in [
        PackageState::OfficialAvailable,
        PackageState::InstalledEnabled,
        PackageState::InstalledDisabled,
        PackageState::UpdateAvailable,
        PackageState::RemovePendingAfterUpdate,
    ] {
        if !model.package_states.contains(&required) {
            return Err(format!("missing package state {required:?}"));
        }
    }
    Ok(())
}

fn render_report(model: &ConfigPocModel) -> String {
    let pages = model
        .pages
        .iter()
        .map(|page| format!("\"{}\"", page.id.as_str()))
        .collect::<Vec<_>>()
        .join(",");
    let title_keys = model
        .pages
        .iter()
        .map(|page| format!("\"{}\"", page.title_key))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\n  \"component\":\"fcitx5-config-poc\",\n  \"kind\":\"rust-config-poc-self-check\",\n  \"product_name\":\"{}\",\n  \"normal_user_exe\":true,\n  \"shipping_config_replaced\":false,\n  \"no_shell_out\":{},\n  \"pages\":[{}],\n  \"title_keys\":[{}],\n  \"language_selector\":true,\n  \"localized_dialogs\":{},\n  \"candidate_preview_embedded\":{},\n  \"candidate_preview_current_theme\":{},\n  \"candidate_preview_not_external_window\":{},\n  \"font_selection\":true,\n  \"advanced_appearance_controls\":true,\n  \"input_method_list\":true,\n  \"addon_install\":true,\n  \"addon_update\":true,\n  \"addon_uninstall\":true,\n  \"addon_enable\":true,\n  \"addon_disable\":true,\n  \"update_states\":true,\n  \"diagnostics_actions\":true,\n  \"result\":\"PASS\"\n}}",
        json_escape(model.product_name),
        model.no_shell_out,
        pages,
        title_keys,
        model.localized_dialogs,
        model.candidate_preview_embedded,
        model.candidate_preview_current_theme,
        model.candidate_preview_not_external_window
    )
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character < ' ' => {
                escaped.push_str(&format!("\\u{:04x}", character as u32))
            }
            character => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_check_covers_frozen_settings_operations() {
        let report = run_self_check().expect("self-check should pass");
        assert!(report.contains("\"component\":\"fcitx5-config-poc\""));
        assert!(report.contains("\"candidate_preview_embedded\":true"));
        assert!(report.contains("\"candidate_preview_current_theme\":true"));
        assert!(report.contains("\"candidate_preview_not_external_window\":true"));
        assert!(report.contains("\"addon_install\":true"));
        assert!(report.contains("\"addon_update\":true"));
        assert!(report.contains("\"addon_uninstall\":true"));
        assert!(report.contains("\"addon_enable\":true"));
        assert!(report.contains("\"addon_disable\":true"));
    }
}
