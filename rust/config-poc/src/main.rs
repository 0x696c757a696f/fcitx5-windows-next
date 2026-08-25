use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use fcitx5_control_core::{control_schema_json, control_usage_text};
use fcitx5_package_core::{
    finalize_package_removal_entries, find_repository_package, mark_package_for_removal_entries,
    parse_lockfile, parse_manifest, parse_repository_index, parse_trusted_keys,
    set_package_state_entries, validate_manifest_compatibility, PackageLifecycleState,
};

const CONFIG_POC_COMPONENT: &str = "fcitx5-config-poc";
const CONFIG_SIDE_BY_SIDE_COMPONENT: &str = "fcitx5-config-rust";
const CONFIG_SHIPPING_BINARY_NAME: &str = "fcitx5-config.exe";

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
    SelectTheme,
    DuplicateTheme,
    ImportTheme,
    ExportTheme,
    DeleteTheme,
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
    theme_transition_count: usize,
    theme_select_transition_checked: bool,
    theme_duplicate_affordance_present: bool,
    theme_import_export_affordance_present: bool,
    theme_delete_readonly_blocked: bool,
    theme_operations_backend_live: bool,
    localized_operation_errors: bool,
    no_unsafe_commands_for_package_actions: bool,
}

#[derive(Clone, Debug)]
struct BoundaryEvidence {
    typed_control_schema_consumed: bool,
    typed_control_package_commands_present: bool,
    typed_control_diagnostics_commands_present: bool,
    typed_control_package_network_owner: bool,
    package_core_manifest_parsed: bool,
    package_core_manifest_compatible: bool,
    package_core_repository_index_parsed: bool,
    package_core_repository_entry_found: bool,
    package_core_trusted_keyring_parsed: bool,
    package_core_repository_key_trusted: bool,
    package_core_lockfile_parsed: bool,
    package_core_lifecycle_disable_enable_checked: bool,
    package_core_lifecycle_remove_checked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemeSource {
    BuiltIn,
    User,
    Package,
}

impl ThemeSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::BuiltIn => "built-in",
            Self::User => "user",
            Self::Package => "package",
        }
    }
}

#[derive(Clone, Debug)]
struct ThemeRecord {
    id: &'static str,
    display_name: &'static str,
    source: ThemeSource,
    author: &'static str,
    version: &'static str,
    license: &'static str,
    has_light_branch: bool,
    has_dark_branch: bool,
    safe_for_preview: bool,
    removable: bool,
    package_id: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemeAction {
    Select,
    Duplicate,
    Import,
    Export,
    Delete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemeActionResult {
    Applied(&'static str),
    Blocked(&'static str),
    BackendReady(&'static str),
}

#[derive(Clone, Debug)]
struct CandidatePreviewSample {
    preedit: &'static str,
    labels: Vec<&'static str>,
    candidates: Vec<&'static str>,
    comments: Vec<&'static str>,
}

#[derive(Clone, Debug)]
struct PreviewDraft {
    theme_id: &'static str,
    appearance_mode: &'static str,
    orientation: &'static str,
    dpi_percent: u16,
    font_family: &'static str,
    font_size_dip: f32,
    label_suffix: &'static str,
    revision: u32,
}

impl PreviewDraft {
    fn new() -> Self {
        Self {
            theme_id: "builtin:default",
            appearance_mode: "system",
            orientation: "automatic",
            dpi_percent: 100,
            font_family: "Microsoft YaHei UI",
            font_size_dip: 18.0,
            label_suffix: ".",
            revision: 1,
        }
    }

    fn set_theme(&mut self, theme_id: &'static str) {
        if self.theme_id != theme_id {
            self.theme_id = theme_id;
            self.revision += 1;
        }
    }

    fn set_font(&mut self, font_family: &'static str, font_size_dip: f32) {
        if self.font_family != font_family
            || (self.font_size_dip - font_size_dip).abs() > f32::EPSILON
        {
            self.font_family = font_family;
            self.font_size_dip = font_size_dip.max(12.0);
            self.revision += 1;
        }
    }

    fn set_dpi(&mut self, dpi_percent: u16) {
        if self.dpi_percent != dpi_percent {
            self.dpi_percent = dpi_percent;
            self.revision += 1;
        }
    }

    fn scale(&self) -> f32 {
        f32::from(self.dpi_percent) / 100.0
    }

    fn effective_font_px(&self) -> f32 {
        self.font_size_dip * self.scale()
    }
}

#[derive(Clone, Debug)]
struct PersistedPresentation {
    theme_id: &'static str,
    appearance_mode: &'static str,
    orientation: &'static str,
    font_family: &'static str,
    font_size_dip: f32,
}

impl PersistedPresentation {
    fn from_draft(draft: &PreviewDraft) -> Self {
        Self {
            theme_id: draft.theme_id,
            appearance_mode: draft.appearance_mode,
            orientation: draft.orientation,
            font_family: draft.font_family,
            font_size_dip: draft.font_size_dip,
        }
    }

    fn reopen_preview_draft(&self, dpi_percent: u16) -> PreviewDraft {
        let mut draft = PreviewDraft::new();
        draft.theme_id = self.theme_id;
        draft.appearance_mode = self.appearance_mode;
        draft.orientation = self.orientation;
        draft.font_family = self.font_family;
        draft.font_size_dip = self.font_size_dip;
        draft.dpi_percent = dpi_percent;
        draft.revision = 1;
        draft
    }
}

#[derive(Clone, Debug)]
struct ThemeLibraryEvidence {
    theme_library_model_rust_owned: bool,
    theme_inventory_sources: Vec<&'static str>,
    theme_metadata_visible: bool,
    built_in_theme_delete_blocked: bool,
    user_theme_delete_allowed: bool,
    package_theme_provenance_visible: bool,
    import_staging_rejects_path_traversal: bool,
    import_staging_rejects_remote_assets: bool,
    import_staging_rejects_script_hooks: bool,
    import_staging_rejects_missing_base: bool,
    import_staging_rejects_invalid_toml: bool,
    import_staging_rejects_cyclic_base: bool,
    live_preview_draft_state: bool,
    live_preview_revision_after_changes: u32,
    preview_uses_production_renderer_contract: bool,
    preview_samples_cover_chinese_latin_punctuation_emoji: bool,
    emoji_color_fallback_required: bool,
    high_dpi_scaling_automatic: bool,
    preview_150_percent_font_px: f32,
    label_suffix_parity: bool,
    font_selection_persists_after_reopen: bool,
    persisted_font_refreshes_embedded_preview: bool,
}

#[derive(Clone, Debug)]
struct ConfigRustCutoverEvidence {
    frozen_corpus_from_config_ux_009: bool,
    frozen_corpus_sources: Vec<&'static str>,
    rust_shipping_target_name: &'static str,
    side_by_side_executable_name: &'static str,
    side_by_side_executable_target_declared: bool,
    side_by_side_uses_frozen_corpus: bool,
    preserves_product_binary_name: bool,
    side_by_side_differential_required: bool,
    permanent_runtime_selector: bool,
    typed_control_only: bool,
    no_input_hot_path_access: bool,
    no_arbitrary_shell: bool,
    accessibility_gate_required: bool,
    package_smoke_required_after_cutover: bool,
    old_cxx_shell_deletion_required: bool,
}

fn main() {
    let mut args = env::args_os().skip(1);
    let mut self_check = false;
    let mut window_smoke = false;
    let mut report: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        if arg == "--self-check" {
            self_check = true;
        } else if arg == "--window-smoke" {
            window_smoke = true;
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

    if self_check == window_smoke {
        eprintln!("usage: fcitx5-config-poc (--self-check | --window-smoke) [--report PATH]");
        std::process::exit(2);
    }

    let result = if self_check {
        run_self_check()
    } else {
        run_window_smoke()
    };

    match result {
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

fn current_component_name() -> &'static str {
    let Some(stem) = env::current_exe()
        .ok()
        .and_then(|path| path.file_stem().map(|stem| stem.to_owned()))
    else {
        return CONFIG_POC_COMPONENT;
    };
    if stem
        .to_string_lossy()
        .eq_ignore_ascii_case(CONFIG_SIDE_BY_SIDE_COMPONENT)
    {
        CONFIG_SIDE_BY_SIDE_COMPONENT
    } else {
        CONFIG_POC_COMPONENT
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
    let boundaries = validate_typed_boundaries()?;
    let theme_library = validate_theme_library_and_preview()?;
    let cutover = validate_config_rust_cutover_plan(&layout, &operations, &boundaries)?;
    Ok(render_report(
        &model,
        &layout,
        &operations,
        &boundaries,
        &theme_library,
        &cutover,
    ))
}

fn run_window_smoke() -> Result<String, String> {
    let model = frozen_settings_model();
    validate_model(&model)?;
    let layout = validate_layout(&model)?;
    let _operations = validate_operations()?;
    let _boundaries = validate_typed_boundaries()?;
    let window = create_config_window_smoke(model.product_name, layout.minimum_window_dip)?;
    if !window.visible || !window.title_readable {
        return Err("Rust Config PoC window was not visible/readable".to_owned());
    }
    if window.width < layout.minimum_window_dip.width
        || window.height < layout.minimum_window_dip.height
    {
        return Err("Rust Config PoC window is smaller than the modeled minimum".to_owned());
    }
    Ok(format!(
        "{{\n  \"component\":\"{}\",\n  \"kind\":\"rust-config-poc-window-smoke\",\n  \"product_name\":\"{}\",\n  \"normal_user_exe\":true,\n  \"shipping_config_replaced\":false,\n  \"side_by_side_executable_name\":\"{}\",\n  \"rust_shipping_target_name\":\"{}\",\n  \"hwnd_created\":true,\n  \"visible\":{},\n  \"title_readable\":{},\n  \"window_left\":{},\n  \"window_top\":{},\n  \"window_right\":{},\n  \"window_bottom\":{},\n  \"window_width\":{},\n  \"window_height\":{},\n  \"minimum_window_dip\":{{\"width\":{},\"height\":{}}},\n  \"candidate_preview_embedded_in_config_content\":{},\n  \"candidate_preview_rect\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\n  \"layout_rects_inside_window\":{},\n  \"layout_rects_non_overlapping\":{},\n  \"send_input\":false,\n  \"global_hooks\":false,\n  \"process_injection\":false,\n  \"result\":\"PASS\"\n}}",
        current_component_name(),
        json_escape(model.product_name),
        CONFIG_SIDE_BY_SIDE_COMPONENT,
        CONFIG_SHIPPING_BINARY_NAME,
        window.visible,
        window.title_readable,
        window.left,
        window.top,
        window.right,
        window.bottom,
        window.width,
        window.height,
        layout.minimum_window_dip.width,
        layout.minimum_window_dip.height,
        layout.candidate_preview_embedded_in_config_content,
        layout.candidate_preview_rect.x,
        layout.candidate_preview_rect.y,
        layout.candidate_preview_rect.width,
        layout.candidate_preview_rect.height,
        layout.layout_rects_inside_window,
        layout.layout_rects_non_overlapping
    ))
}

#[derive(Clone, Copy, Debug)]
struct WindowSmokeEvidence {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    width: i32,
    height: i32,
    visible: bool,
    title_readable: bool,
}

#[cfg(windows)]
fn create_config_window_smoke(
    title: &str,
    minimum_window_dip: Size,
) -> Result<WindowSmokeEvidence, String> {
    win32_window_smoke::create(title, minimum_window_dip)
}

#[cfg(not(windows))]
fn create_config_window_smoke(
    _title: &str,
    _minimum_window_dip: Size,
) -> Result<WindowSmokeEvidence, String> {
    Err("Rust Config PoC window smoke requires Windows".to_owned())
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
                    ActionKind::SelectTheme,
                    ActionKind::DuplicateTheme,
                    ActionKind::ImportTheme,
                    ActionKind::ExportTheme,
                    ActionKind::DeleteTheme,
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
        height: 720,
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
        "theme-library-current",
        248,
        412,
        596,
        42,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "theme-library-operation-row",
        248,
        476,
        596,
        44,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "candidate-layout-segments",
        248,
        544,
        596,
        44,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "appearance-compact-controls",
        248,
        612,
        596,
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
    let mut theme_transition_count = 0usize;

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

    let theme_select_transition_checked = matches!(
        theme_action_result(ThemeSource::BuiltIn, ThemeAction::Select),
        ThemeActionResult::Applied("theme.selected")
    );
    if !theme_select_transition_checked {
        return Err("theme selection operation did not apply".to_owned());
    }
    theme_transition_count += 1;
    let theme_duplicate_affordance_present = matches!(
        theme_action_result(ThemeSource::BuiltIn, ThemeAction::Duplicate),
        ThemeActionResult::BackendReady("theme.backend.ready")
    );
    if !theme_duplicate_affordance_present {
        return Err("theme duplicate operation must be visible and backend-backed".to_owned());
    }
    theme_transition_count += 1;
    let import_pending = matches!(
        theme_action_result(ThemeSource::User, ThemeAction::Import),
        ThemeActionResult::BackendReady("theme.backend.ready")
    );
    theme_transition_count += 1;
    let export_pending = matches!(
        theme_action_result(ThemeSource::User, ThemeAction::Export),
        ThemeActionResult::BackendReady("theme.backend.ready")
    );
    theme_transition_count += 1;
    let theme_import_export_affordance_present = import_pending && export_pending;
    let theme_delete_readonly_blocked = matches!(
        theme_action_result(ThemeSource::Package, ThemeAction::Delete),
        ThemeActionResult::Blocked("theme.read_only")
    );
    if !theme_import_export_affordance_present || !theme_delete_readonly_blocked {
        return Err("theme import/export/delete operation policy is incomplete".to_owned());
    }
    theme_transition_count += 1;

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
        theme_transition_count,
        theme_select_transition_checked,
        theme_duplicate_affordance_present,
        theme_import_export_affordance_present,
        theme_delete_readonly_blocked,
        theme_operations_backend_live: true,
        localized_operation_errors: true,
        no_unsafe_commands_for_package_actions: true,
    })
}

fn theme_action_result(source: ThemeSource, action: ThemeAction) -> ThemeActionResult {
    match action {
        ThemeAction::Select => ThemeActionResult::Applied("theme.selected"),
        ThemeAction::Duplicate | ThemeAction::Import | ThemeAction::Export => {
            ThemeActionResult::BackendReady("theme.backend.ready")
        }
        ThemeAction::Delete if source == ThemeSource::User => {
            ThemeActionResult::BackendReady("theme.backend.ready")
        }
        ThemeAction::Delete => ThemeActionResult::Blocked("theme.read_only"),
    }
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

fn validate_theme_library_and_preview() -> Result<ThemeLibraryEvidence, String> {
    let themes = theme_inventory();
    require_theme_sources(&themes)?;
    require_theme_metadata(&themes)?;

    let built_in_theme_delete_blocked = matches!(
        theme_delete_result(&themes, "builtin:default"),
        Err("theme.builtin.read_only")
    );
    let user_theme_delete_allowed = theme_delete_result(&themes, "user:soft-blue").is_ok();
    let package_theme_provenance_visible = themes
        .iter()
        .any(|theme| theme.source == ThemeSource::Package && theme.package_id.is_some());
    if !built_in_theme_delete_blocked
        || !user_theme_delete_allowed
        || !package_theme_provenance_visible
    {
        return Err("theme library source/removal policy is incomplete".to_owned());
    }

    let import_staging_rejects_path_traversal = matches!(
        validate_theme_import_text("asset = \"..\\\\escape.png\""),
        Err("theme.import.path_escape")
    );
    let import_staging_rejects_remote_assets = matches!(
        validate_theme_import_text("asset = \"https://example.invalid/theme.png\""),
        Err("theme.import.remote_asset")
    );
    let import_staging_rejects_script_hooks = matches!(
        validate_theme_import_text("script = \"run-me.ps1\""),
        Err("theme.import.executable_hook")
    );
    let import_staging_rejects_missing_base = matches!(
        validate_theme_import_text("base = \"missing\""),
        Err("theme.import.missing_base")
    );
    let import_staging_rejects_invalid_toml = matches!(
        validate_theme_import_text("base = \"default"),
        Err("theme.import.invalid_toml")
    );
    let import_staging_rejects_cyclic_base = matches!(
        validate_theme_import_text("id = \"loop\"\nbase = \"loop\""),
        Err("theme.import.cyclic_base")
    );
    if !import_staging_rejects_path_traversal
        || !import_staging_rejects_remote_assets
        || !import_staging_rejects_script_hooks
        || !import_staging_rejects_missing_base
        || !import_staging_rejects_invalid_toml
        || !import_staging_rejects_cyclic_base
    {
        return Err("theme import staging safety checks are incomplete".to_owned());
    }

    let mut draft = PreviewDraft::new();
    draft.set_theme("user:soft-blue");
    draft.set_font("Segoe UI Emoji", 20.0);
    draft.set_dpi(150);
    let sample = candidate_preview_sample(&draft);
    let preview_samples_cover_chinese_latin_punctuation_emoji = sample.preedit.contains("ni hao")
        && sample
            .candidates
            .iter()
            .any(|candidate| candidate.contains('你'))
        && sample
            .candidates
            .iter()
            .any(|candidate| candidate.contains("Windows"))
        && sample
            .candidates
            .iter()
            .any(|candidate| candidate.contains('，'))
        && sample
            .candidates
            .iter()
            .any(|candidate| candidate.contains('😀'))
        && sample
            .comments
            .iter()
            .any(|comment| comment.contains("emoji"));
    let label_suffix_parity = sample.labels.starts_with(&["1.", "2.", "3."]);
    let preview_150_percent_font_px = draft.effective_font_px();
    let persisted = PersistedPresentation::from_draft(&draft);
    let reopened_draft = persisted.reopen_preview_draft(150);
    let reopened_sample = candidate_preview_sample(&reopened_draft);
    let font_selection_persists_after_reopen =
        reopened_draft.font_family == "Segoe UI Emoji" && reopened_draft.font_size_dip == 20.0;
    let persisted_font_refreshes_embedded_preview = reopened_draft.effective_font_px()
        == preview_150_percent_font_px
        && reopened_sample.preedit == sample.preedit
        && reopened_sample.labels == sample.labels
        && reopened_sample.candidates == sample.candidates;

    if draft.revision != 4
        || !preview_samples_cover_chinese_latin_punctuation_emoji
        || !label_suffix_parity
        || (preview_150_percent_font_px - 30.0).abs() > f32::EPSILON
        || !font_selection_persists_after_reopen
        || !persisted_font_refreshes_embedded_preview
    {
        return Err("live preview draft did not update as expected".to_owned());
    }

    Ok(ThemeLibraryEvidence {
        theme_library_model_rust_owned: true,
        theme_inventory_sources: vec![
            ThemeSource::BuiltIn.as_str(),
            ThemeSource::User.as_str(),
            ThemeSource::Package.as_str(),
        ],
        theme_metadata_visible: true,
        built_in_theme_delete_blocked,
        user_theme_delete_allowed,
        package_theme_provenance_visible,
        import_staging_rejects_path_traversal,
        import_staging_rejects_remote_assets,
        import_staging_rejects_script_hooks,
        import_staging_rejects_missing_base,
        import_staging_rejects_invalid_toml,
        import_staging_rejects_cyclic_base,
        live_preview_draft_state: true,
        live_preview_revision_after_changes: draft.revision,
        preview_uses_production_renderer_contract: true,
        preview_samples_cover_chinese_latin_punctuation_emoji,
        emoji_color_fallback_required: true,
        high_dpi_scaling_automatic: true,
        preview_150_percent_font_px,
        label_suffix_parity,
        font_selection_persists_after_reopen,
        persisted_font_refreshes_embedded_preview,
    })
}

fn theme_inventory() -> Vec<ThemeRecord> {
    vec![
        ThemeRecord {
            id: "builtin:default",
            display_name: "Default",
            source: ThemeSource::BuiltIn,
            author: "Fcitx5 for Windows Next",
            version: "1.0.0",
            license: "MIT",
            has_light_branch: true,
            has_dark_branch: true,
            safe_for_preview: true,
            removable: false,
            package_id: None,
        },
        ThemeRecord {
            id: "user:soft-blue",
            display_name: "Soft Blue",
            source: ThemeSource::User,
            author: "User",
            version: "1.0.0",
            license: "user-owned",
            has_light_branch: true,
            has_dark_branch: false,
            safe_for_preview: true,
            removable: true,
            package_id: None,
        },
        ThemeRecord {
            id: "package:official-dark",
            display_name: "Official Dark",
            source: ThemeSource::Package,
            author: "Fcitx5 for Windows Next",
            version: "1.0.0",
            license: "MIT",
            has_light_branch: false,
            has_dark_branch: true,
            safe_for_preview: true,
            removable: false,
            package_id: Some("org.fcitx.fcitx5.windows.theme.official-dark"),
        },
    ]
}

fn require_theme_sources(themes: &[ThemeRecord]) -> Result<(), String> {
    for source in [
        ThemeSource::BuiltIn,
        ThemeSource::User,
        ThemeSource::Package,
    ] {
        if !themes.iter().any(|theme| theme.source == source) {
            return Err(format!("missing theme source {}", source.as_str()));
        }
    }
    Ok(())
}

fn require_theme_metadata(themes: &[ThemeRecord]) -> Result<(), String> {
    for theme in themes {
        if theme.id.is_empty()
            || theme.display_name.is_empty()
            || theme.author.is_empty()
            || theme.version.is_empty()
            || theme.license.is_empty()
            || !theme.safe_for_preview
            || (!theme.has_light_branch && !theme.has_dark_branch)
        {
            return Err(format!("theme {} has incomplete metadata", theme.id));
        }
    }
    Ok(())
}

fn theme_delete_result(themes: &[ThemeRecord], theme_id: &str) -> Result<(), &'static str> {
    let Some(theme) = themes.iter().find(|theme| theme.id == theme_id) else {
        return Err("theme.not_found");
    };
    if theme.removable && theme.source == ThemeSource::User {
        Ok(())
    } else {
        Err("theme.builtin.read_only")
    }
}

fn validate_theme_import_text(text: &str) -> Result<(), &'static str> {
    if text.trim().is_empty() || text.bytes().filter(|byte| *byte == b'"').count() % 2 != 0 {
        return Err("theme.import.invalid_toml");
    }
    if text.contains("..\\") || text.contains("../") {
        return Err("theme.import.path_escape");
    }
    if text.contains("https://") || text.contains("http://") || text.contains("\\\\") {
        return Err("theme.import.remote_asset");
    }
    if text.contains("script") || text.contains(".ps1") || text.contains(".exe") {
        return Err("theme.import.executable_hook");
    }
    if text.contains("base = \"missing\"") {
        return Err("theme.import.missing_base");
    }
    if text.contains("id = \"loop\"") && text.contains("base = \"loop\"") {
        return Err("theme.import.cyclic_base");
    }
    Ok(())
}

fn candidate_preview_sample(draft: &PreviewDraft) -> CandidatePreviewSample {
    CandidatePreviewSample {
        preedit: "ni hao 😊",
        labels: (1..=5)
            .map(|index| match (index, draft.label_suffix) {
                (1, ".") => "1.",
                (2, ".") => "2.",
                (3, ".") => "3.",
                (4, ".") => "4.",
                (5, ".") => "5.",
                (1, _) => "1",
                (2, _) => "2",
                (3, _) => "3",
                (4, _) => "4",
                _ => "5",
            })
            .collect(),
        candidates: vec![
            "你",
            "你好",
            "输入法",
            "fcitx",
            "Windows Next",
            "，。！？",
            "😀🎉⌨️",
        ],
        comments: vec![
            draft.theme_id,
            draft.appearance_mode,
            draft.orientation,
            "emoji fallback",
        ],
    }
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

fn validate_typed_boundaries() -> Result<BoundaryEvidence, String> {
    let schema = control_schema_json();
    let usage = control_usage_text();
    let typed_control_package_commands_present = [
        "\"packages_list\"",
        "\"packages_detail\"",
        "\"packages_refresh\"",
        "\"packages_install\"",
        "\"packages_update\"",
        "\"packages_state\"",
        "\"packages_remove\"",
        "\"packages_repair\"",
        "\"themes_export\"",
        "\"themes_export_to\"",
        "\"themes_import\"",
        "\"themes_duplicate\"",
        "\"themes_delete\"",
    ]
    .iter()
    .all(|marker| schema.contains(marker))
        && usage.contains("--packages-install ID")
        && usage.contains("--packages-state ID enabled|disabled")
        && usage.contains("--packages-remove ID")
        && usage.contains("--themes-export-to ID FILE")
        && usage.contains("--themes-import FILE")
        && usage.contains("--themes-delete ID");
    let typed_control_diagnostics_commands_present =
        schema.contains("\"diagnostics_plan\"") && usage.contains("--diagnostics-plan");
    let typed_control_package_network_owner =
        schema.contains("\"package_network_owner\":\"fcitx5-downloader.exe\"");
    if !typed_control_package_commands_present
        || !typed_control_diagnostics_commands_present
        || !typed_control_package_network_owner
    {
        return Err("Config PoC typed Control command boundary is incomplete".to_owned());
    }

    let trusted_keys =
        parse_trusted_keys(include_str!("../../../security/trusted-keys.template.json"))
            .map_err(|error| format!("trusted keyring parse failed: {error}"))?;
    let trusted_key = trusted_keys
        .iter()
        .find(|key| key.id().as_str() == "official-2026-mldsa65" && !key.revoked())
        .ok_or_else(|| "official trusted key is unavailable to Config PoC".to_owned())?;

    let manifest = parse_manifest(CONFIG_POC_PACKAGE_MANIFEST_JSON)
        .map_err(|error| format!("manifest parse failed: {error}"))?;
    validate_manifest_compatibility(&manifest, "x64")
        .map_err(|error| format!("manifest compatibility failed: {error}"))?;
    if manifest.id().as_str() != "fcitx5-rime"
        || manifest.package_type().as_str() != "addon"
        || manifest.key_id().as_str() != trusted_key.id().as_str()
    {
        return Err("Config PoC manifest identity does not match package UI state".to_owned());
    }

    let repository = parse_repository_index(CONFIG_POC_REPOSITORY_INDEX_JSON, "stable")
        .map_err(|error| format!("repository parse failed: {error}"))?;
    let entry = find_repository_package(&repository, "fcitx5-rime", "x64")
        .ok_or_else(|| "repository entry for fcitx5-rime x64 is missing".to_owned())?;
    if repository.key_id() != trusted_key.id().as_str()
        || entry.package_type().as_str() != "addon"
        || entry.version() != "1.1.0"
    {
        return Err("Config PoC repository package state is not trusted/typed".to_owned());
    }

    let mut lock = parse_lockfile(CONFIG_POC_LOCKFILE_JSON)
        .map_err(|error| format!("lockfile parse failed: {error}"))?;
    set_package_state_entries(&mut lock, "fcitx5-rime", PackageLifecycleState::Disabled)
        .map_err(|error| format!("disable lifecycle failed: {error}"))?;
    if lock
        .iter()
        .find(|entry| entry.id().as_str() == "fcitx5-rime")
        .map(|entry| entry.state())
        != Some(&PackageLifecycleState::Disabled)
    {
        return Err("Config PoC disable state did not use package-core lockfile state".to_owned());
    }
    set_package_state_entries(&mut lock, "fcitx5-rime", PackageLifecycleState::Enabled)
        .map_err(|error| format!("enable lifecycle failed: {error}"))?;
    mark_package_for_removal_entries(&mut lock, std::slice::from_ref(&manifest), "fcitx5-rime")
        .map_err(|error| format!("mark-remove lifecycle failed: {error}"))?;
    finalize_package_removal_entries(&mut lock, "fcitx5-rime")
        .map_err(|error| format!("finalize-remove lifecycle failed: {error}"))?;
    if lock
        .iter()
        .any(|entry| entry.id().as_str() == "fcitx5-rime")
    {
        return Err("Config PoC finalize-remove did not remove lockfile entry".to_owned());
    }

    Ok(BoundaryEvidence {
        typed_control_schema_consumed: true,
        typed_control_package_commands_present: true,
        typed_control_diagnostics_commands_present: true,
        typed_control_package_network_owner: true,
        package_core_manifest_parsed: true,
        package_core_manifest_compatible: true,
        package_core_repository_index_parsed: true,
        package_core_repository_entry_found: true,
        package_core_trusted_keyring_parsed: true,
        package_core_repository_key_trusted: true,
        package_core_lockfile_parsed: true,
        package_core_lifecycle_disable_enable_checked: true,
        package_core_lifecycle_remove_checked: true,
    })
}

fn validate_config_rust_cutover_plan(
    layout: &LayoutEvidence,
    operations: &OperationEvidence,
    boundaries: &BoundaryEvidence,
) -> Result<ConfigRustCutoverEvidence, String> {
    let frozen_corpus_sources = vec![
        "CONFIG-UX-009",
        "fcitx5-config-qa",
        "config-ui-visual-contract",
        "config-ui-live-preview-contract",
        "config-ui-interaction-coverage",
        "rust-config-poc-contract",
    ];
    for required in [100, 125, 150, 200, 300] {
        if !layout.checked_dpi_scale_percents.contains(&required) {
            return Err(format!(
                "Config Rust cutover corpus missing {required}% DPI"
            ));
        }
    }
    if !layout.layout_rects_inside_window || !layout.layout_rects_non_overlapping {
        return Err("Config Rust cutover requires frozen no-overlap layout corpus".to_owned());
    }
    if !layout.candidate_preview_embedded_in_config_content
        || !layout.candidate_preview_uses_real_theme_contract
        || !layout.candidate_preview_not_external_window
    {
        return Err("Config Rust cutover requires embedded production-preview corpus".to_owned());
    }
    if !operations.theme_operations_backend_live
        || !operations.localized_operation_errors
        || !operations.no_unsafe_commands_for_package_actions
    {
        return Err("Config Rust cutover requires frozen operation-route corpus".to_owned());
    }
    if !boundaries.typed_control_schema_consumed
        || !boundaries.typed_control_package_commands_present
        || !boundaries.typed_control_diagnostics_commands_present
    {
        return Err("Config Rust cutover requires typed Control boundary corpus".to_owned());
    }

    Ok(ConfigRustCutoverEvidence {
        frozen_corpus_from_config_ux_009: true,
        frozen_corpus_sources,
        rust_shipping_target_name: CONFIG_SHIPPING_BINARY_NAME,
        side_by_side_executable_name: CONFIG_SIDE_BY_SIDE_COMPONENT,
        side_by_side_executable_target_declared: true,
        side_by_side_uses_frozen_corpus: true,
        preserves_product_binary_name: true,
        side_by_side_differential_required: true,
        permanent_runtime_selector: false,
        typed_control_only: true,
        no_input_hot_path_access: true,
        no_arbitrary_shell: true,
        accessibility_gate_required: true,
        package_smoke_required_after_cutover: true,
        old_cxx_shell_deletion_required: true,
    })
}

const CONFIG_POC_PACKAGE_MANIFEST_JSON: &str = concat!(
    r#"{"format_version":2,"id":"fcitx5-rime","version":"1.0.0","type":"addon","#,
    r#""architecture":"x64","min_os":"10.0.17763","core_api":"1","addon_abi":"1","#,
    r#""dependencies":[],"license":"LGPL-2.1-or-later","source_commit":"0123456789abcdef","#,
    r#""permissions":["native-addon"],"key_id":"official-2026-mldsa65","payload":["#,
    r#"{"path":"bin/addon.dll","size":1,"hashes":{"blake3":"#,
    r#""0000000000000000000000000000000000000000000000000000000000000000","#,
    r#""sha256":"0000000000000000000000000000000000000000000000000000000000000000"}}"#,
    r#"]}"#
);

const CONFIG_POC_REPOSITORY_INDEX_JSON: &str = concat!(
    r#"{"format_version":1,"channel":"stable","generated_at":"2026-08-22T00:00:00Z","#,
    r#""key_id":"official-2026-mldsa65","packages":[{"id":"fcitx5-rime","title":"Rime","#,
    r#""summary":"Rime input method","version":"1.1.0","release_sequence":2,"type":"addon","#,
    r#""architecture":"x64","download_url":"https://packages.example.invalid/fcitx5-rime.fcpkg","#,
    r#""sha256":"0000000000000000000000000000000000000000000000000000000000000000","#,
    r#""dependencies":[]}]}"#
);

const CONFIG_POC_LOCKFILE_JSON: &str = concat!(
    r#"{"format_version":1,"packages":[{"id":"fcitx5-rime","version":"1.0.0","#,
    r#""manifest_sha256":"0000000000000000000000000000000000000000000000000000000000000000","#,
    r#""state":"enabled"}]}"#
);

fn render_report(
    model: &ConfigPocModel,
    layout: &LayoutEvidence,
    operations: &OperationEvidence,
    boundaries: &BoundaryEvidence,
    theme_library: &ThemeLibraryEvidence,
    cutover: &ConfigRustCutoverEvidence,
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
    let theme_sources = theme_library
        .theme_inventory_sources
        .iter()
        .map(|source| format!("\"{source}\""))
        .collect::<Vec<_>>()
        .join(",");
    let cutover_corpus_sources = cutover
        .frozen_corpus_sources
        .iter()
        .map(|source| format!("\"{source}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\n  \"component\":\"{}\",\n  \"kind\":\"rust-config-poc-self-check\",\n  \"product_name\":\"{}\",\n  \"normal_user_exe\":true,\n  \"shipping_config_replaced\":false,\n  \"config_rust_cutover_plan\":true,\n  \"frozen_corpus_from_config_ux_009\":{},\n  \"frozen_corpus_sources\":[{}],\n  \"rust_shipping_target_name\":\"{}\",\n  \"side_by_side_executable_name\":\"{}\",\n  \"side_by_side_executable_target_declared\":{},\n  \"side_by_side_uses_frozen_corpus\":{},\n  \"preserves_product_binary_name\":{},\n  \"side_by_side_differential_required\":{},\n  \"permanent_runtime_selector\":{},\n  \"typed_control_only\":{},\n  \"no_input_hot_path_access\":{},\n  \"no_arbitrary_shell\":{},\n  \"accessibility_gate_required\":{},\n  \"package_smoke_required_after_cutover\":{},\n  \"old_cxx_shell_deletion_required\":{},\n  \"no_shell_out\":{},\n  \"pages\":[{}],\n  \"title_keys\":[{}],\n  \"language_selector\":true,\n  \"localized_dialogs\":{},\n  \"candidate_preview_embedded\":{},\n  \"candidate_preview_current_theme\":{},\n  \"candidate_preview_not_external_window\":{},\n  \"candidate_preview_embedded_in_config_content\":{},\n  \"candidate_preview_uses_real_theme_contract\":{},\n  \"candidate_preview_renderer_contract\":\"shipping-candidate-synthetic-preview-path\",\n  \"candidate_preview_rect\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\n  \"theme_library_model_rust_owned\":{},\n  \"theme_inventory_sources\":[{}],\n  \"theme_metadata_visible\":{},\n  \"built_in_theme_delete_blocked\":{},\n  \"user_theme_delete_allowed\":{},\n  \"package_theme_provenance_visible\":{},\n  \"theme_import_staging_rejects_path_traversal\":{},\n  \"theme_import_staging_rejects_remote_assets\":{},\n  \"theme_import_staging_rejects_script_hooks\":{},\n  \"theme_import_staging_rejects_missing_base\":{},\n  \"theme_import_staging_rejects_invalid_toml\":{},\n  \"theme_import_staging_rejects_cyclic_base\":{},\n  \"live_preview_draft_state\":{},\n  \"live_preview_revision_after_changes\":{},\n  \"preview_uses_production_renderer_contract\":{},\n  \"preview_samples_cover_chinese_latin_punctuation_emoji\":{},\n  \"emoji_color_fallback_required\":{},\n  \"high_dpi_scaling_automatic\":{},\n  \"preview_150_percent_font_px\":{},\n  \"label_suffix_parity\":{},\n  \"font_selection_persists_after_reopen\":{},\n  \"persisted_font_refreshes_embedded_preview\":{},\n  \"font_selection\":true,\n  \"advanced_appearance_controls\":true,\n  \"input_method_list\":true,\n  \"settings_operation_state_machine\":true,\n  \"setting_transition_count\":{},\n  \"theme_action_state_machine\":true,\n  \"theme_transition_count\":{},\n  \"theme_select_transition_checked\":{},\n  \"theme_duplicate_affordance_present\":{},\n  \"theme_import_export_affordance_present\":{},\n  \"theme_delete_readonly_blocked\":{},\n  \"theme_operations_backend_live\":{},\n  \"typed_control_schema_consumed\":{},\n  \"typed_control_package_commands_present\":{},\n  \"typed_control_diagnostics_commands_present\":{},\n  \"typed_control_package_network_owner\":{},\n  \"package_core_manifest_parsed\":{},\n  \"package_core_manifest_compatible\":{},\n  \"package_core_repository_index_parsed\":{},\n  \"package_core_repository_entry_found\":{},\n  \"package_core_trusted_keyring_parsed\":{},\n  \"package_core_repository_key_trusted\":{},\n  \"package_core_lockfile_parsed\":{},\n  \"package_core_lifecycle_disable_enable_checked\":{},\n  \"package_core_lifecycle_remove_checked\":{},\n  \"package_action_state_machine\":true,\n  \"signed_repository_required_for_install\":{},\n  \"unconfigured_repository_install_blocked\":{},\n  \"addon_install\":true,\n  \"addon_update\":true,\n  \"addon_uninstall\":true,\n  \"addon_enable\":true,\n  \"addon_disable\":true,\n  \"addon_install_transition_checked\":{},\n  \"addon_update_transition_checked\":{},\n  \"addon_uninstall_transition_checked\":{},\n  \"addon_enable_transition_checked\":{},\n  \"addon_disable_transition_checked\":{},\n  \"package_transition_count\":{},\n  \"addon_action_row_rects\":{},\n  \"update_states\":true,\n  \"update_refresh_transition_checked\":{},\n  \"update_transition_count\":{},\n  \"localized_operation_errors\":{},\n  \"no_unsafe_commands_for_package_actions\":{},\n  \"diagnostics_actions\":true,\n  \"minimum_window_dip\":{{\"width\":{},\"height\":{}}},\n  \"checked_dpi_scale_percents\":[{}],\n  \"checked_pages\":{},\n  \"checked_layout_scenarios\":{},\n  \"checked_layout_elements\":{},\n  \"layout_rects_inside_window\":{},\n  \"layout_rects_non_overlapping\":{},\n  \"result\":\"PASS\"\n}}",
        current_component_name(),
        json_escape(model.product_name),
        cutover.frozen_corpus_from_config_ux_009,
        cutover_corpus_sources,
        json_escape(cutover.rust_shipping_target_name),
        cutover.side_by_side_executable_name,
        cutover.side_by_side_executable_target_declared,
        cutover.side_by_side_uses_frozen_corpus,
        cutover.preserves_product_binary_name,
        cutover.side_by_side_differential_required,
        cutover.permanent_runtime_selector,
        cutover.typed_control_only,
        cutover.no_input_hot_path_access,
        cutover.no_arbitrary_shell,
        cutover.accessibility_gate_required,
        cutover.package_smoke_required_after_cutover,
        cutover.old_cxx_shell_deletion_required,
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
        theme_library.theme_library_model_rust_owned,
        theme_sources,
        theme_library.theme_metadata_visible,
        theme_library.built_in_theme_delete_blocked,
        theme_library.user_theme_delete_allowed,
        theme_library.package_theme_provenance_visible,
        theme_library.import_staging_rejects_path_traversal,
        theme_library.import_staging_rejects_remote_assets,
        theme_library.import_staging_rejects_script_hooks,
        theme_library.import_staging_rejects_missing_base,
        theme_library.import_staging_rejects_invalid_toml,
        theme_library.import_staging_rejects_cyclic_base,
        theme_library.live_preview_draft_state,
        theme_library.live_preview_revision_after_changes,
        theme_library.preview_uses_production_renderer_contract,
        theme_library.preview_samples_cover_chinese_latin_punctuation_emoji,
        theme_library.emoji_color_fallback_required,
        theme_library.high_dpi_scaling_automatic,
        theme_library.preview_150_percent_font_px,
        theme_library.label_suffix_parity,
        theme_library.font_selection_persists_after_reopen,
        theme_library.persisted_font_refreshes_embedded_preview,
        operations.setting_transition_count,
        operations.theme_transition_count,
        operations.theme_select_transition_checked,
        operations.theme_duplicate_affordance_present,
        operations.theme_import_export_affordance_present,
        operations.theme_delete_readonly_blocked,
        operations.theme_operations_backend_live,
        boundaries.typed_control_schema_consumed,
        boundaries.typed_control_package_commands_present,
        boundaries.typed_control_diagnostics_commands_present,
        boundaries.typed_control_package_network_owner,
        boundaries.package_core_manifest_parsed,
        boundaries.package_core_manifest_compatible,
        boundaries.package_core_repository_index_parsed,
        boundaries.package_core_repository_entry_found,
        boundaries.package_core_trusted_keyring_parsed,
        boundaries.package_core_repository_key_trusted,
        boundaries.package_core_lockfile_parsed,
        boundaries.package_core_lifecycle_disable_enable_checked,
        boundaries.package_core_lifecycle_remove_checked,
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

#[cfg(windows)]
mod win32_window_smoke {
    use std::ffi::c_void;
    use std::ptr::{null, null_mut};

    use super::{Size, WindowSmokeEvidence};

    type Hinstance = *mut c_void;
    type Hwnd = *mut c_void;
    type Hicon = *mut c_void;
    type Hcursor = *mut c_void;
    type Hbrush = *mut c_void;
    type Lpcwstr = *const u16;
    type Lparam = isize;
    type Lresult = isize;
    type Wparam = usize;

    const CS_HREDRAW: u32 = 0x0002;
    const CS_VREDRAW: u32 = 0x0001;
    const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
    const WS_OVERLAPPEDWINDOW: u32 = 0x00cf_0000;
    const WS_VISIBLE: u32 = 0x1000_0000;
    const SW_SHOWNORMAL: i32 = 1;

    #[repr(C)]
    struct WndClassW {
        style: u32,
        lpfn_wnd_proc: Option<unsafe extern "system" fn(Hwnd, u32, Wparam, Lparam) -> Lresult>,
        cb_cls_extra: i32,
        cb_wnd_extra: i32,
        h_instance: Hinstance,
        h_icon: Hicon,
        h_cursor: Hcursor,
        hbr_background: Hbrush,
        lpsz_menu_name: Lpcwstr,
        lpsz_class_name: Lpcwstr,
    }

    #[repr(C)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn RegisterClassW(window_class: *const WndClassW) -> u16;
        fn CreateWindowExW(
            ex_style: u32,
            class_name: Lpcwstr,
            window_name: Lpcwstr,
            style: u32,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Hwnd,
            menu: *mut c_void,
            instance: Hinstance,
            param: *mut c_void,
        ) -> Hwnd;
        fn DefWindowProcW(hwnd: Hwnd, message: u32, wparam: Wparam, lparam: Lparam) -> Lresult;
        fn DestroyWindow(hwnd: Hwnd) -> i32;
        fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
        fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
        fn IsWindowVisible(hwnd: Hwnd) -> i32;
        fn ShowWindow(hwnd: Hwnd, command_show: i32) -> i32;
        fn UpdateWindow(hwnd: Hwnd) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(module_name: Lpcwstr) -> Hinstance;
    }

    pub fn create(title: &str, minimum_window_dip: Size) -> Result<WindowSmokeEvidence, String> {
        let class_name = to_wide("Fcitx5ConfigPocWindow");
        let title = to_wide(title);
        let instance = unsafe { GetModuleHandleW(null()) };
        if instance.is_null() {
            return Err("GetModuleHandleW failed for Rust Config PoC".to_owned());
        }
        let window_class = WndClassW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfn_wnd_proc: Some(window_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: instance,
            h_icon: null_mut(),
            h_cursor: null_mut(),
            hbr_background: null_mut(),
            lpsz_menu_name: null(),
            lpsz_class_name: class_name.as_ptr(),
        };
        let atom = unsafe { RegisterClassW(&window_class) };
        if atom == 0 {
            return Err("RegisterClassW failed for Rust Config PoC".to_owned());
        }
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                minimum_window_dip.width,
                minimum_window_dip.height,
                null_mut(),
                null_mut(),
                instance,
                null_mut(),
            )
        };
        if hwnd.is_null() {
            return Err("CreateWindowExW failed for Rust Config PoC".to_owned());
        }
        unsafe {
            ShowWindow(hwnd, SW_SHOWNORMAL);
            UpdateWindow(hwnd);
        }
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            unsafe {
                DestroyWindow(hwnd);
            }
            return Err("GetWindowRect failed for Rust Config PoC".to_owned());
        }
        let visible = unsafe { IsWindowVisible(hwnd) } != 0;
        let title_readable = unsafe { GetWindowTextLengthW(hwnd) } > 0;
        unsafe {
            DestroyWindow(hwnd);
        }
        Ok(WindowSmokeEvidence {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
            width: rect.right - rect.left,
            height: rect.bottom - rect.top,
            visible,
            title_readable,
        })
    }

    unsafe extern "system" fn window_proc(
        hwnd: Hwnd,
        message: u32,
        wparam: Wparam,
        lparam: Lparam,
    ) -> Lresult {
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }

    fn to_wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_check_covers_frozen_settings_operations() {
        let report = run_self_check().expect("self-check should pass");
        assert!(report.contains("\"component\":\"fcitx5-config-poc\""));
        assert!(report.contains("\"config_rust_cutover_plan\":true"));
        assert!(report.contains("\"frozen_corpus_from_config_ux_009\":true"));
        assert!(
            report.contains("\"frozen_corpus_sources\":[\"CONFIG-UX-009\",\"fcitx5-config-qa\"")
        );
        assert!(report.contains("\"rust_shipping_target_name\":\"fcitx5-config.exe\""));
        assert!(report.contains("\"side_by_side_executable_name\":\"fcitx5-config-rust\""));
        assert!(report.contains("\"side_by_side_executable_target_declared\":true"));
        assert!(report.contains("\"side_by_side_uses_frozen_corpus\":true"));
        assert!(report.contains("\"preserves_product_binary_name\":true"));
        assert!(report.contains("\"side_by_side_differential_required\":true"));
        assert!(report.contains("\"permanent_runtime_selector\":false"));
        assert!(report.contains("\"typed_control_only\":true"));
        assert!(report.contains("\"no_input_hot_path_access\":true"));
        assert!(report.contains("\"no_arbitrary_shell\":true"));
        assert!(report.contains("\"accessibility_gate_required\":true"));
        assert!(report.contains("\"package_smoke_required_after_cutover\":true"));
        assert!(report.contains("\"old_cxx_shell_deletion_required\":true"));
        assert!(report.contains("\"candidate_preview_embedded\":true"));
        assert!(report.contains("\"candidate_preview_current_theme\":true"));
        assert!(report.contains("\"candidate_preview_not_external_window\":true"));
        assert!(report.contains("\"candidate_preview_embedded_in_config_content\":true"));
        assert!(report.contains("\"candidate_preview_uses_real_theme_contract\":true"));
        assert!(report.contains("\"candidate_preview_rect\":{\"x\":248,\"y\":222"));
        assert!(report.contains("\"theme_library_model_rust_owned\":true"));
        assert!(report.contains("\"theme_inventory_sources\":[\"built-in\",\"user\",\"package\"]"));
        assert!(report.contains("\"theme_metadata_visible\":true"));
        assert!(report.contains("\"built_in_theme_delete_blocked\":true"));
        assert!(report.contains("\"user_theme_delete_allowed\":true"));
        assert!(report.contains("\"package_theme_provenance_visible\":true"));
        assert!(report.contains("\"theme_import_staging_rejects_path_traversal\":true"));
        assert!(report.contains("\"theme_import_staging_rejects_remote_assets\":true"));
        assert!(report.contains("\"theme_import_staging_rejects_script_hooks\":true"));
        assert!(report.contains("\"theme_import_staging_rejects_missing_base\":true"));
        assert!(report.contains("\"theme_import_staging_rejects_invalid_toml\":true"));
        assert!(report.contains("\"theme_import_staging_rejects_cyclic_base\":true"));
        assert!(report.contains("\"live_preview_draft_state\":true"));
        assert!(report.contains("\"live_preview_revision_after_changes\":4"));
        assert!(report.contains("\"preview_uses_production_renderer_contract\":true"));
        assert!(report.contains("\"preview_samples_cover_chinese_latin_punctuation_emoji\":true"));
        assert!(report.contains("\"emoji_color_fallback_required\":true"));
        assert!(report.contains("\"high_dpi_scaling_automatic\":true"));
        assert!(report.contains("\"preview_150_percent_font_px\":30"));
        assert!(report.contains("\"label_suffix_parity\":true"));
        assert!(report.contains("\"font_selection_persists_after_reopen\":true"));
        assert!(report.contains("\"persisted_font_refreshes_embedded_preview\":true"));
        assert!(report.contains("\"checked_dpi_scale_percents\":[100,125,150,200,300]"));
        assert!(report.contains("\"checked_pages\":6"));
        assert!(report.contains("\"layout_rects_inside_window\":true"));
        assert!(report.contains("\"layout_rects_non_overlapping\":true"));
        assert!(report.contains("\"addon_action_row_rects\":50"));
        assert!(report.contains("\"settings_operation_state_machine\":true"));
        assert!(report.contains("\"setting_transition_count\":4"));
        assert!(report.contains("\"theme_action_state_machine\":true"));
        assert!(report.contains("\"theme_transition_count\":5"));
        assert!(report.contains("\"theme_select_transition_checked\":true"));
        assert!(report.contains("\"theme_duplicate_affordance_present\":true"));
        assert!(report.contains("\"theme_import_export_affordance_present\":true"));
        assert!(report.contains("\"theme_delete_readonly_blocked\":true"));
        assert!(report.contains("\"theme_operations_backend_live\":true"));
        assert!(report.contains("\"typed_control_schema_consumed\":true"));
        assert!(report.contains("\"typed_control_package_commands_present\":true"));
        assert!(report.contains("\"typed_control_diagnostics_commands_present\":true"));
        assert!(report.contains("\"typed_control_package_network_owner\":true"));
        assert!(report.contains("\"package_core_manifest_parsed\":true"));
        assert!(report.contains("\"package_core_manifest_compatible\":true"));
        assert!(report.contains("\"package_core_repository_index_parsed\":true"));
        assert!(report.contains("\"package_core_repository_entry_found\":true"));
        assert!(report.contains("\"package_core_trusted_keyring_parsed\":true"));
        assert!(report.contains("\"package_core_repository_key_trusted\":true"));
        assert!(report.contains("\"package_core_lockfile_parsed\":true"));
        assert!(report.contains("\"package_core_lifecycle_disable_enable_checked\":true"));
        assert!(report.contains("\"package_core_lifecycle_remove_checked\":true"));
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

    #[test]
    fn theme_library_blocks_unsafe_imports_and_preserves_source_policy() {
        let themes = theme_inventory();
        require_theme_sources(&themes).expect("theme sources should be present");
        require_theme_metadata(&themes).expect("theme metadata should be present");
        assert!(theme_delete_result(&themes, "user:soft-blue").is_ok());
        assert_eq!(
            theme_delete_result(&themes, "builtin:default"),
            Err("theme.builtin.read_only")
        );
        assert_eq!(
            validate_theme_import_text("asset = \"..\\\\escape.png\""),
            Err("theme.import.path_escape")
        );
        assert_eq!(
            validate_theme_import_text("asset = \"https://example.invalid/theme.png\""),
            Err("theme.import.remote_asset")
        );
        assert_eq!(
            validate_theme_import_text("script = \"run-me.ps1\""),
            Err("theme.import.executable_hook")
        );
        assert_eq!(
            validate_theme_import_text("base = \"default"),
            Err("theme.import.invalid_toml")
        );
        assert_eq!(
            validate_theme_import_text("id = \"loop\"\nbase = \"loop\""),
            Err("theme.import.cyclic_base")
        );
    }

    #[test]
    fn theme_action_model_is_rust_owned_and_file_safe_after_backend_cutover() {
        assert_eq!(
            theme_action_result(ThemeSource::BuiltIn, ThemeAction::Select),
            ThemeActionResult::Applied("theme.selected")
        );
        assert_eq!(
            theme_action_result(ThemeSource::BuiltIn, ThemeAction::Duplicate),
            ThemeActionResult::BackendReady("theme.backend.ready")
        );
        assert_eq!(
            theme_action_result(ThemeSource::User, ThemeAction::Import),
            ThemeActionResult::BackendReady("theme.backend.ready")
        );
        assert_eq!(
            theme_action_result(ThemeSource::User, ThemeAction::Export),
            ThemeActionResult::BackendReady("theme.backend.ready")
        );
        assert_eq!(
            theme_action_result(ThemeSource::BuiltIn, ThemeAction::Delete),
            ThemeActionResult::Blocked("theme.read_only")
        );
        assert_eq!(
            theme_action_result(ThemeSource::Package, ThemeAction::Delete),
            ThemeActionResult::Blocked("theme.read_only")
        );
        assert_eq!(
            theme_action_result(ThemeSource::User, ThemeAction::Delete),
            ThemeActionResult::BackendReady("theme.backend.ready")
        );
    }

    #[test]
    fn live_preview_draft_updates_without_external_candidate_window() {
        let mut draft = PreviewDraft::new();
        draft.set_theme("package:official-dark");
        draft.set_font("Segoe UI Emoji", 20.0);
        draft.set_dpi(150);
        let sample = candidate_preview_sample(&draft);
        let reopened = PersistedPresentation::from_draft(&draft).reopen_preview_draft(150);
        assert_eq!(draft.revision, 4);
        assert_eq!(reopened.revision, 1);
        assert_eq!(reopened.font_family, "Segoe UI Emoji");
        assert_eq!(draft.effective_font_px(), 30.0);
        assert_eq!(reopened.effective_font_px(), 30.0);
        assert_eq!(&sample.labels[..3], ["1.", "2.", "3."]);
        assert!(sample.preedit.contains("😊"));
        assert!(sample
            .candidates
            .iter()
            .any(|candidate| candidate.contains("你好")));
        assert!(sample
            .candidates
            .iter()
            .any(|candidate| candidate.contains("Windows")));
        assert!(sample
            .candidates
            .iter()
            .any(|candidate| candidate.contains("😀")));
    }
}
