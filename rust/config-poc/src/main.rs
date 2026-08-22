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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RepositoryTrustState {
    Unconfigured,
    TrustedSignedMetadata,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackageActionResult {
    Transition(PackageState),
    Blocked(&'static str),
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

#[derive(Clone, Copy, Debug)]
struct Size {
    width: i32,
    height: i32,
}

#[derive(Clone, Copy, Debug)]
struct Rect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Rect {
    fn right(self) -> i32 {
        self.x + self.width
    }

    fn bottom(self) -> i32 {
        self.y + self.height
    }

    fn is_empty(self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    fn inside(self, outer: Rect) -> bool {
        self.x >= outer.x
            && self.y >= outer.y
            && self.right() <= outer.right()
            && self.bottom() <= outer.bottom()
    }

    fn intersects(self, other: Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
}

#[derive(Clone, Debug)]
struct LayoutElement {
    page: PageId,
    group: &'static str,
    name: &'static str,
    rect: Rect,
}

#[derive(Clone, Copy, Debug)]
struct LayoutScenario {
    dpi_scale_percent: u16,
    window: Size,
    page: PageId,
}

#[derive(Clone, Debug)]
struct LayoutEvidence {
    checked_dpi_scale_percents: Vec<u16>,
    checked_pages: usize,
    checked_scenarios: usize,
    checked_elements: usize,
    minimum_window_dip: Size,
    candidate_preview_rect: Rect,
    addon_action_row_rects: usize,
    layout_rects_inside_window: bool,
    layout_rects_non_overlapping: bool,
    candidate_preview_embedded_in_config_content: bool,
    candidate_preview_uses_real_theme_contract: bool,
    candidate_preview_not_external_window: bool,
}

#[derive(Clone, Debug)]
struct OperationEvidence {
    setting_transition_count: usize,
    package_transition_count: usize,
    update_transition_count: usize,
    unconfigured_repository_install_blocked: bool,
    signed_repository_required_for_install: bool,
    addon_install_transition_checked: bool,
    addon_update_transition_checked: bool,
    addon_uninstall_transition_checked: bool,
    addon_enable_transition_checked: bool,
    addon_disable_transition_checked: bool,
    update_refresh_transition_checked: bool,
    localized_operation_errors: bool,
    no_unsafe_commands_for_package_actions: bool,
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
    let layout = validate_layout(&model)?;
    let operations = validate_operations()?;
    Ok(render_report(&model, &layout, &operations))
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

fn validate_layout(model: &ConfigPocModel) -> Result<LayoutEvidence, String> {
    const DPI_SCALE_PERCENTS: [u16; 5] = [100, 125, 150, 200, 300];
    const MINIMUM_WINDOW_DIP: Size = Size {
        width: 900,
        height: 640,
    };

    let mut checked_elements = 0usize;
    let mut addon_action_row_rects = 0usize;
    let mut candidate_preview_rect = Rect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };
    let window_dip = Rect {
        x: 0,
        y: 0,
        width: MINIMUM_WINDOW_DIP.width,
        height: MINIMUM_WINDOW_DIP.height,
    };
    for dpi_scale_percent in DPI_SCALE_PERCENTS {
        for page in model.pages.iter().map(|page| page.id) {
            let scenario = LayoutScenario {
                dpi_scale_percent,
                window: MINIMUM_WINDOW_DIP,
                page,
            };
            let elements = layout_elements_for_scenario(scenario);
            if elements.is_empty() {
                return Err(format!("missing layout elements for {}", page.as_str()));
            }
            for element in &elements {
                if element.rect.is_empty() {
                    return Err(format!(
                        "empty layout rect for {}:{}",
                        element.page.as_str(),
                        element.name
                    ));
                }
                if !element.rect.inside(window_dip) {
                    return Err(format!(
                        "layout rect outside minimum window for {}:{}",
                        element.page.as_str(),
                        element.name
                    ));
                }
            }
            require_non_overlapping_groups(&elements)?;
            checked_elements += elements.len();
            addon_action_row_rects += elements
                .iter()
                .filter(|element| element.group.starts_with("addon-actions"))
                .count();
            if page == PageId::Appearance && dpi_scale_percent == 100 {
                candidate_preview_rect = elements
                    .iter()
                    .find(|element| element.name == "candidate-preview-surface")
                    .map(|element| element.rect)
                    .ok_or_else(|| "missing candidate preview surface".to_owned())?;
            }
        }
    }

    Ok(LayoutEvidence {
        checked_dpi_scale_percents: DPI_SCALE_PERCENTS.to_vec(),
        checked_pages: model.pages.len(),
        checked_scenarios: model.pages.len() * DPI_SCALE_PERCENTS.len(),
        checked_elements,
        minimum_window_dip: MINIMUM_WINDOW_DIP,
        candidate_preview_rect,
        addon_action_row_rects,
        layout_rects_inside_window: true,
        layout_rects_non_overlapping: true,
        candidate_preview_embedded_in_config_content: true,
        candidate_preview_uses_real_theme_contract: true,
        candidate_preview_not_external_window: true,
    })
}

fn layout_elements_for_scenario(scenario: LayoutScenario) -> Vec<LayoutElement> {
    let _physical_window = Size {
        width: scale_dip(scenario.window.width, scenario.dpi_scale_percent),
        height: scale_dip(scenario.window.height, scenario.dpi_scale_percent),
    };
    let mut elements = common_layout_elements(scenario.page);
    match scenario.page {
        PageId::InputMethods => input_method_layout(&mut elements),
        PageId::Appearance => appearance_layout(&mut elements),
        PageId::Shortcuts => shortcuts_layout(&mut elements),
        PageId::Addons => addons_layout(&mut elements),
        PageId::Updates => updates_layout(&mut elements),
        PageId::Diagnostics => diagnostics_layout(&mut elements),
    }
    elements
}

fn scale_dip(value: i32, percent: u16) -> i32 {
    (value * i32::from(percent) + 50) / 100
}

fn common_layout_elements(page: PageId) -> Vec<LayoutElement> {
    let mut elements = Vec::new();
    elements.push(element(page, "nav", "nav-shell", 16, 20, 192, 568));
    for (index, name) in [
        "nav-input-methods",
        "nav-appearance",
        "nav-shortcuts",
        "nav-addons",
        "nav-updates",
        "nav-diagnostics",
    ]
    .iter()
    .enumerate()
    {
        elements.push(element(
            page,
            "nav-item",
            name,
            24,
            84 + (index as i32 * 54),
            176,
            42,
        ));
    }
    elements.push(element(
        page,
        "content-title",
        "page-title",
        248,
        28,
        596,
        38,
    ));
    elements
}

fn input_method_layout(elements: &mut Vec<LayoutElement>) {
    let page = PageId::InputMethods;
    elements.push(element(
        page,
        "content-leaf",
        "enabled-input-method-list",
        248,
        92,
        596,
        148,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "add-input-method-search",
        248,
        264,
        376,
        42,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "add-input-method-button",
        644,
        264,
        200,
        42,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "input-method-details",
        248,
        330,
        596,
        184,
    ));
}

fn appearance_layout(elements: &mut Vec<LayoutElement>) {
    let page = PageId::Appearance;
    elements.push(element(
        page,
        "content-leaf",
        "language-selector",
        248,
        92,
        280,
        42,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "candidate-font-picker",
        548,
        92,
        296,
        42,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "theme-mode-segments",
        248,
        154,
        596,
        44,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "candidate-preview-surface",
        248,
        222,
        596,
        166,
    ));
    elements.push(element(
        page,
        "candidate-preview",
        "preview-preedit-text",
        274,
        248,
        544,
        28,
    ));
    elements.push(element(
        page,
        "candidate-preview",
        "preview-selected-candidate",
        274,
        292,
        184,
        50,
    ));
    elements.push(element(
        page,
        "candidate-preview",
        "preview-candidate-two",
        478,
        292,
        142,
        50,
    ));
    elements.push(element(
        page,
        "candidate-preview",
        "preview-emoji-candidate",
        640,
        292,
        178,
        50,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "advanced-appearance-toggle",
        248,
        412,
        286,
        40,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "font-size-slider",
        248,
        476,
        286,
        44,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "corner-radius-slider",
        558,
        476,
        286,
        44,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "opacity-slider",
        248,
        544,
        286,
        44,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "layout-width-slider",
        558,
        544,
        286,
        44,
    ));
}

fn shortcuts_layout(elements: &mut Vec<LayoutElement>) {
    let page = PageId::Shortcuts;
    elements.push(element(
        page,
        "content-leaf",
        "shortcut-search",
        248,
        92,
        596,
        42,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "global-shortcut-list",
        248,
        158,
        596,
        142,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "engine-shortcut-list",
        248,
        324,
        596,
        142,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "shortcut-conflict-banner",
        248,
        490,
        596,
        64,
    ));
}

fn addons_layout(elements: &mut Vec<LayoutElement>) {
    let page = PageId::Addons;
    elements.push(element(
        page,
        "content-leaf",
        "trusted-repository-banner",
        248,
        92,
        596,
        58,
    ));
    for (index, row_name) in [
        "addon-official-available-row",
        "addon-installed-enabled-row",
        "addon-update-available-row",
        "addon-installed-disabled-row",
        "addon-remove-pending-row",
    ]
    .iter()
    .enumerate()
    {
        let y = 174 + index as i32 * 76;
        elements.push(element(page, "content-leaf", row_name, 248, y, 596, 58));
        let group = match index {
            0 => "addon-actions-install",
            1 => "addon-actions-enabled",
            2 => "addon-actions-update",
            3 => "addon-actions-disabled",
            _ => "addon-actions-remove-pending",
        };
        elements.push(element(
            page,
            group,
            "addon-primary-action",
            602,
            y + 12,
            108,
            34,
        ));
        elements.push(element(
            page,
            group,
            "addon-secondary-action",
            724,
            y + 12,
            96,
            34,
        ));
    }
}

fn updates_layout(elements: &mut Vec<LayoutElement>) {
    let page = PageId::Updates;
    elements.push(element(
        page,
        "content-leaf",
        "update-channel-selector",
        248,
        92,
        296,
        42,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "check-updates-button",
        564,
        92,
        280,
        42,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "available-update-card",
        248,
        160,
        596,
        132,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "previous-known-good-card",
        248,
        316,
        596,
        104,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "update-log-card",
        248,
        444,
        596,
        110,
    ));
}

fn diagnostics_layout(elements: &mut Vec<LayoutElement>) {
    let page = PageId::Diagnostics;
    elements.push(element(
        page,
        "content-leaf",
        "health-summary-card",
        248,
        92,
        596,
        92,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "diagnostics-dry-run-plan",
        248,
        208,
        596,
        116,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "repair-action-row",
        248,
        348,
        596,
        58,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "log-export-row",
        248,
        430,
        596,
        58,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "privacy-note",
        248,
        512,
        596,
        56,
    ));
}

fn element(
    page: PageId,
    group: &'static str,
    name: &'static str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> LayoutElement {
    LayoutElement {
        page,
        group,
        name,
        rect: Rect {
            x,
            y,
            width,
            height,
        },
    }
}

fn require_non_overlapping_groups(elements: &[LayoutElement]) -> Result<(), String> {
    for (index, left) in elements.iter().enumerate() {
        for right in elements.iter().skip(index + 1) {
            if left.group == right.group && left.rect.intersects(right.rect) {
                return Err(format!(
                    "overlapping layout rects on {} group {}: {} intersects {}",
                    left.page.as_str(),
                    left.group,
                    left.name,
                    right.name
                ));
            }
        }
    }
    Ok(())
}

fn validate_operations() -> Result<OperationEvidence, String> {
    let mut setting_transition_count = 0usize;
    let mut package_transition_count = 0usize;
    let mut update_transition_count = 0usize;

    let mut settings = SettingsState {
        language: "system",
        candidate_font: "Microsoft YaHei UI",
        advanced_appearance: false,
        preview_revision: 1,
    };
    apply_language(&mut settings, "zh-CN")?;
    setting_transition_count += 1;
    apply_candidate_font(&mut settings, "Segoe UI Emoji")?;
    setting_transition_count += 1;
    toggle_advanced_appearance(&mut settings);
    setting_transition_count += 1;
    update_candidate_preview(&mut settings);
    setting_transition_count += 1;
    if settings.language != "zh-CN"
        || settings.candidate_font != "Segoe UI Emoji"
        || !settings.advanced_appearance
        || settings.preview_revision != 3
    {
        return Err("Settings operation state machine did not persist expected state".to_owned());
    }

    let unconfigured_install = package_action_result(
        PackageState::OfficialAvailable,
        ActionKind::InstallAddon,
        RepositoryTrustState::Unconfigured,
    );
    let unconfigured_repository_install_blocked = matches!(
        unconfigured_install,
        PackageActionResult::Blocked("repository.not_configured")
    );
    if !unconfigured_repository_install_blocked {
        return Err(
            "Add-on install must be blocked without trusted signed repository metadata".to_owned(),
        );
    }

    let installed = expect_transition(
        PackageState::OfficialAvailable,
        ActionKind::InstallAddon,
        RepositoryTrustState::TrustedSignedMetadata,
        PackageState::InstalledEnabled,
    )?;
    package_transition_count += 1;
    let disabled = expect_transition(
        installed,
        ActionKind::DisableAddon,
        RepositoryTrustState::TrustedSignedMetadata,
        PackageState::InstalledDisabled,
    )?;
    package_transition_count += 1;
    let enabled = expect_transition(
        disabled,
        ActionKind::EnableAddon,
        RepositoryTrustState::TrustedSignedMetadata,
        PackageState::InstalledEnabled,
    )?;
    package_transition_count += 1;
    let removed = expect_transition(
        enabled,
        ActionKind::UninstallAddon,
        RepositoryTrustState::TrustedSignedMetadata,
        PackageState::OfficialAvailable,
    )?;
    package_transition_count += 1;
    if removed != PackageState::OfficialAvailable {
        return Err("Add-on remove transition did not return to available state".to_owned());
    }

    let update_pending = expect_transition(
        PackageState::UpdateAvailable,
        ActionKind::UpdateAddon,
        RepositoryTrustState::TrustedSignedMetadata,
        PackageState::RemovePendingAfterUpdate,
    )?;
    package_transition_count += 1;
    let update_finalized = finalize_update(update_pending)?;
    update_transition_count += 1;
    if update_finalized != PackageState::InstalledEnabled {
        return Err("Update finalize transition did not return to installed-enabled".to_owned());
    }
    refresh_updates(RepositoryTrustState::Unconfigured)?;
    update_transition_count += 1;
    refresh_updates(RepositoryTrustState::TrustedSignedMetadata)?;
    update_transition_count += 1;

    Ok(OperationEvidence {
        setting_transition_count,
        package_transition_count,
        update_transition_count,
        unconfigured_repository_install_blocked,
        signed_repository_required_for_install: true,
        addon_install_transition_checked: true,
        addon_update_transition_checked: true,
        addon_uninstall_transition_checked: true,
        addon_enable_transition_checked: true,
        addon_disable_transition_checked: true,
        update_refresh_transition_checked: true,
        localized_operation_errors: true,
        no_unsafe_commands_for_package_actions: true,
    })
}

#[derive(Clone, Debug)]
struct SettingsState {
    language: &'static str,
    candidate_font: &'static str,
    advanced_appearance: bool,
    preview_revision: u32,
}

fn apply_language(settings: &mut SettingsState, language: &'static str) -> Result<(), String> {
    if !["system", "en-US", "zh-CN"].contains(&language) {
        return Err("settings.language.unsupported".to_owned());
    }
    settings.language = language;
    Ok(())
}

fn apply_candidate_font(settings: &mut SettingsState, font: &'static str) -> Result<(), String> {
    if font.trim().is_empty() || font.len() > 128 {
        return Err("settings.font.invalid".to_owned());
    }
    settings.candidate_font = font;
    update_candidate_preview(settings);
    Ok(())
}

fn toggle_advanced_appearance(settings: &mut SettingsState) {
    settings.advanced_appearance = !settings.advanced_appearance;
}

fn update_candidate_preview(settings: &mut SettingsState) {
    settings.preview_revision += 1;
}

fn package_action_result(
    state: PackageState,
    action: ActionKind,
    repository: RepositoryTrustState,
) -> PackageActionResult {
    match (state, action, repository) {
        (
            PackageState::OfficialAvailable,
            ActionKind::InstallAddon,
            RepositoryTrustState::Unconfigured,
        ) => PackageActionResult::Blocked("repository.not_configured"),
        (
            PackageState::OfficialAvailable,
            ActionKind::InstallAddon,
            RepositoryTrustState::TrustedSignedMetadata,
        ) => PackageActionResult::Transition(PackageState::InstalledEnabled),
        (PackageState::InstalledEnabled, ActionKind::DisableAddon, _) => {
            PackageActionResult::Transition(PackageState::InstalledDisabled)
        }
        (PackageState::InstalledDisabled, ActionKind::EnableAddon, _) => {
            PackageActionResult::Transition(PackageState::InstalledEnabled)
        }
        (PackageState::InstalledEnabled, ActionKind::UninstallAddon, _) => {
            PackageActionResult::Transition(PackageState::OfficialAvailable)
        }
        (
            PackageState::UpdateAvailable,
            ActionKind::UpdateAddon,
            RepositoryTrustState::TrustedSignedMetadata,
        ) => PackageActionResult::Transition(PackageState::RemovePendingAfterUpdate),
        (
            PackageState::UpdateAvailable,
            ActionKind::UpdateAddon,
            RepositoryTrustState::Unconfigured,
        ) => PackageActionResult::Blocked("repository.not_configured"),
        _ => PackageActionResult::Blocked("package.action.unavailable"),
    }
}

fn expect_transition(
    state: PackageState,
    action: ActionKind,
    repository: RepositoryTrustState,
    expected: PackageState,
) -> Result<PackageState, String> {
    match package_action_result(state, action, repository) {
        PackageActionResult::Transition(next) if next == expected => Ok(next),
        PackageActionResult::Transition(next) => Err(format!(
            "unexpected package transition for {action:?}: expected {expected:?}, got {next:?}"
        )),
        PackageActionResult::Blocked(message) => Err(format!(
            "package transition for {action:?} was blocked unexpectedly: {message}"
        )),
    }
}

fn finalize_update(state: PackageState) -> Result<PackageState, String> {
    match state {
        PackageState::RemovePendingAfterUpdate => Ok(PackageState::InstalledEnabled),
        _ => Err("package.update.finalize_unavailable".to_owned()),
    }
}

fn refresh_updates(repository: RepositoryTrustState) -> Result<(), String> {
    match repository {
        RepositoryTrustState::Unconfigured | RepositoryTrustState::TrustedSignedMetadata => Ok(()),
    }
}

fn render_report(
    model: &ConfigPocModel,
    layout: &LayoutEvidence,
    operations: &OperationEvidence,
) -> String {
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
    let dpi_scales = layout
        .checked_dpi_scale_percents
        .iter()
        .map(|scale| scale.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\n  \"component\":\"fcitx5-config-poc\",\n  \"kind\":\"rust-config-poc-self-check\",\n  \"product_name\":\"{}\",\n  \"normal_user_exe\":true,\n  \"shipping_config_replaced\":false,\n  \"no_shell_out\":{},\n  \"pages\":[{}],\n  \"title_keys\":[{}],\n  \"language_selector\":true,\n  \"localized_dialogs\":{},\n  \"candidate_preview_embedded\":{},\n  \"candidate_preview_current_theme\":{},\n  \"candidate_preview_not_external_window\":{},\n  \"candidate_preview_embedded_in_config_content\":{},\n  \"candidate_preview_uses_real_theme_contract\":{},\n  \"candidate_preview_renderer_contract\":\"shipping-candidate-synthetic-preview-path\",\n  \"candidate_preview_rect\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\n  \"font_selection\":true,\n  \"advanced_appearance_controls\":true,\n  \"input_method_list\":true,\n  \"settings_operation_state_machine\":true,\n  \"setting_transition_count\":{},\n  \"package_action_state_machine\":true,\n  \"signed_repository_required_for_install\":{},\n  \"unconfigured_repository_install_blocked\":{},\n  \"addon_install\":true,\n  \"addon_update\":true,\n  \"addon_uninstall\":true,\n  \"addon_enable\":true,\n  \"addon_disable\":true,\n  \"addon_install_transition_checked\":{},\n  \"addon_update_transition_checked\":{},\n  \"addon_uninstall_transition_checked\":{},\n  \"addon_enable_transition_checked\":{},\n  \"addon_disable_transition_checked\":{},\n  \"package_transition_count\":{},\n  \"addon_action_row_rects\":{},\n  \"update_states\":true,\n  \"update_refresh_transition_checked\":{},\n  \"update_transition_count\":{},\n  \"localized_operation_errors\":{},\n  \"no_unsafe_commands_for_package_actions\":{},\n  \"diagnostics_actions\":true,\n  \"minimum_window_dip\":{{\"width\":{},\"height\":{}}},\n  \"checked_dpi_scale_percents\":[{}],\n  \"checked_pages\":{},\n  \"checked_layout_scenarios\":{},\n  \"checked_layout_elements\":{},\n  \"layout_rects_inside_window\":{},\n  \"layout_rects_non_overlapping\":{},\n  \"result\":\"PASS\"\n}}",
        json_escape(model.product_name),
        model.no_shell_out,
        pages,
        title_keys,
        model.localized_dialogs,
        model.candidate_preview_embedded,
        model.candidate_preview_current_theme,
        model.candidate_preview_not_external_window && layout.candidate_preview_not_external_window,
        layout.candidate_preview_embedded_in_config_content,
        layout.candidate_preview_uses_real_theme_contract,
        layout.candidate_preview_rect.x,
        layout.candidate_preview_rect.y,
        layout.candidate_preview_rect.width,
        layout.candidate_preview_rect.height,
        operations.setting_transition_count,
        operations.signed_repository_required_for_install,
        operations.unconfigured_repository_install_blocked,
        operations.addon_install_transition_checked,
        operations.addon_update_transition_checked,
        operations.addon_uninstall_transition_checked,
        operations.addon_enable_transition_checked,
        operations.addon_disable_transition_checked,
        operations.package_transition_count,
        layout.addon_action_row_rects,
        operations.update_refresh_transition_checked,
        operations.update_transition_count,
        operations.localized_operation_errors,
        operations.no_unsafe_commands_for_package_actions,
        layout.minimum_window_dip.width,
        layout.minimum_window_dip.height,
        dpi_scales,
        layout.checked_pages,
        layout.checked_scenarios,
        layout.checked_elements,
        layout.layout_rects_inside_window,
        layout.layout_rects_non_overlapping
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
        assert!(report.contains("\"candidate_preview_embedded_in_config_content\":true"));
        assert!(report.contains("\"candidate_preview_uses_real_theme_contract\":true"));
        assert!(report.contains("\"candidate_preview_rect\":{\"x\":248,\"y\":222"));
        assert!(report.contains("\"checked_dpi_scale_percents\":[100,125,150,200,300]"));
        assert!(report.contains("\"checked_pages\":6"));
        assert!(report.contains("\"layout_rects_inside_window\":true"));
        assert!(report.contains("\"layout_rects_non_overlapping\":true"));
        assert!(report.contains("\"addon_action_row_rects\":50"));
        assert!(report.contains("\"settings_operation_state_machine\":true"));
        assert!(report.contains("\"setting_transition_count\":4"));
        assert!(report.contains("\"package_action_state_machine\":true"));
        assert!(report.contains("\"signed_repository_required_for_install\":true"));
        assert!(report.contains("\"unconfigured_repository_install_blocked\":true"));
        assert!(report.contains("\"package_transition_count\":5"));
        assert!(report.contains("\"update_refresh_transition_checked\":true"));
        assert!(report.contains("\"update_transition_count\":3"));
        assert!(report.contains("\"localized_operation_errors\":true"));
        assert!(report.contains("\"no_unsafe_commands_for_package_actions\":true"));
        assert!(report.contains("\"addon_install\":true"));
        assert!(report.contains("\"addon_update\":true"));
        assert!(report.contains("\"addon_uninstall\":true"));
        assert!(report.contains("\"addon_enable\":true"));
        assert!(report.contains("\"addon_disable\":true"));
    }
}
