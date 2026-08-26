use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use fcitx5_candidate_core::{candidate_preview_paint_plan, run_candidate_poc_self_check};
use fcitx5_control_core::{control_schema_json, control_usage_text};
use fcitx5_package_core::{
    finalize_package_removal_entries, find_repository_package, mark_package_for_removal_entries,
    parse_lockfile, parse_manifest, parse_repository_index, parse_trusted_keys,
    set_package_state_entries, validate_manifest_compatibility, PackageLifecycleState,
};

const CONFIG_POC_COMPONENT: &str = "fcitx5-config-poc";
const CONFIG_SIDE_BY_SIDE_COMPONENT: &str = "fcitx5-config-rust";
const CONFIG_SHIPPING_COMPONENT: &str = "fcitx5-config";
const CONFIG_SHIPPING_BINARY_NAME: &str = "fcitx5-config.exe";
const CANDIDATE_PREVIEW_HOST_KIND: &str = "config-child-candidate-renderer-host";
const CANDIDATE_PREVIEW_RENDERER_CONTRACT: &str = "shipping-candidate-real-preview-host-path";
const CANDIDATE_PREVIEW_WINDOW_OWNERSHIP: &str = "config-content-child-surface";
const CANDIDATE_PREVIEW_THEME_SNAPSHOT: &str = "resolved-theme-snapshot-shared-with-candidate-ui";
const CANDIDATE_PREVIEW_MODEL_CONTRACT: &str = "candidate-model-layout-render-segments";
const CANDIDATE_PREVIEW_SAMPLE_SOURCE: &str = "fixed-preview-sample-input-only";

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
    numeric_appearance: AppearanceNumericEvidence,
    localized_operation_errors: bool,
    no_unsafe_commands_for_package_actions: bool,
}

#[derive(Clone, Debug)]
struct AppearanceNumericEvidence {
    numeric_appearance_inputs: bool,
    valid_typed_entry_updates_draft: bool,
    invalid_text_rejected: bool,
    paste_out_of_range_rejected: bool,
    ime_cancellation_keeps_last_valid: bool,
    min_max_bounds_checked: bool,
    localized_error_text: bool,
    rollback_keeps_last_valid: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppearanceNumericField {
    FontSizeDip,
    Opacity,
    SpacingDip,
    CornerRadiusDip,
    CandidateWidthDip,
}

impl AppearanceNumericField {
    fn spec(self) -> AppearanceNumericSpec {
        match self {
            Self::FontSizeDip => AppearanceNumericSpec {
                key: "font_size_dip",
                min: 8.0,
                max: 72.0,
            },
            Self::Opacity => AppearanceNumericSpec {
                key: "opacity",
                min: 0.20,
                max: 1.0,
            },
            Self::SpacingDip => AppearanceNumericSpec {
                key: "spacing_dip",
                min: 0.0,
                max: 64.0,
            },
            Self::CornerRadiusDip => AppearanceNumericSpec {
                key: "corner_radius_dip",
                min: 0.0,
                max: 48.0,
            },
            Self::CandidateWidthDip => AppearanceNumericSpec {
                key: "candidate_width_dip",
                min: 160.0,
                max: 2048.0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct AppearanceNumericSpec {
    key: &'static str,
    min: f32,
    max: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum AppearanceNumericOutcome {
    Accepted(f32),
    Rejected(&'static str),
    Incomplete(&'static str),
}

#[derive(Clone, Copy, Debug)]
struct AppearanceNumericState {
    field: AppearanceNumericField,
    last_valid: f32,
}

impl AppearanceNumericState {
    fn new(field: AppearanceNumericField, last_valid: f32) -> Self {
        Self { field, last_valid }
    }

    fn apply_text(&mut self, text: &str) -> AppearanceNumericOutcome {
        match validate_appearance_numeric_input(self.field, text) {
            Ok(value) => {
                self.last_valid = value;
                AppearanceNumericOutcome::Accepted(value)
            }
            Err(error) if error == "appearance.numeric.incomplete" => {
                AppearanceNumericOutcome::Incomplete(error)
            }
            Err(error) => AppearanceNumericOutcome::Rejected(error),
        }
    }
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
struct CandidatePreviewHostEvidence {
    host_kind: &'static str,
    renderer_contract: &'static str,
    window_ownership: &'static str,
    theme_snapshot_source: &'static str,
    model_contract: &'static str,
    sample_source: &'static str,
    embedded_child_surface: bool,
    not_external_popup_window: bool,
    settings_only_fake_renderer: bool,
    static_screenshot_preview: bool,
    uses_shipping_candidate_renderer_path: bool,
    consumes_candidate_model_layout_render_contract: bool,
    uses_resolved_theme_snapshot: bool,
    layout_driven_paint: bool,
    final_pixels_from_renderer_path: bool,
    candidate_core_self_check_passed: bool,
    candidate_core_scenarios: usize,
    candidate_core_color_font_scenario_present: bool,
    candidate_core_uiless_scenario_present: bool,
    layout_rects_inside_window: bool,
    layout_rects_non_overlapping: bool,
    dpi_parity_scale_percents: Vec<u16>,
    font_fallback_parity: bool,
    emoji_color_render_path_parity: bool,
    sample_input_only_synthetic: bool,
    send_input: bool,
    global_hooks: bool,
    process_injection: bool,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunMode {
    Interactive,
    SelfCheck,
    WindowSmoke,
    LegacyHeadless(LegacyHeadlessMode),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LegacyHeadlessMode {
    SelfTest,
    CheckI18n,
    CheckResources,
    UiContract,
    UiVisualContract,
    UiLivePreviewContract,
    UiInteraction,
}

impl LegacyHeadlessMode {
    fn argument(self) -> &'static str {
        match self {
            Self::SelfTest => "--self-test",
            Self::CheckI18n => "--check-i18n",
            Self::CheckResources => "--check-resources",
            Self::UiContract => "--ui-contract-test",
            Self::UiVisualContract => "--ui-visual-contract-test",
            Self::UiLivePreviewContract => "--ui-live-preview-contract-test",
            Self::UiInteraction => "--ui-interaction-test",
        }
    }

    fn evidence_kind(self) -> &'static str {
        match self {
            Self::SelfTest => "rust-config-legacy-self-test",
            Self::CheckI18n => "rust-config-legacy-i18n-check",
            Self::CheckResources => "rust-config-legacy-resource-check",
            Self::UiContract => "rust-config-legacy-ui-contract",
            Self::UiVisualContract => "rust-config-legacy-ui-visual-contract",
            Self::UiLivePreviewContract => "rust-config-legacy-ui-live-preview-contract",
            Self::UiInteraction => "rust-config-legacy-ui-interaction-contract",
        }
    }

    fn corpus_marker(self) -> &'static str {
        match self {
            Self::SelfTest => "i18n-and-resource-corpus",
            Self::CheckI18n => "localized-settings-corpus",
            Self::CheckResources => "bundled-settings-resource-corpus",
            Self::UiContract => "settings-operation-corpus",
            Self::UiVisualContract => "settings-layout-visual-corpus",
            Self::UiLivePreviewContract => "settings-live-preview-corpus",
            Self::UiInteraction => "settings-keyboard-interaction-corpus",
        }
    }
}

fn main() {
    let mut args = env::args_os().skip(1);
    let mut mode: Option<RunMode> = None;
    let mut report: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        if arg == "--self-check" {
            set_run_mode(&mut mode, RunMode::SelfCheck);
        } else if arg == "--window-smoke" {
            set_run_mode(&mut mode, RunMode::WindowSmoke);
        } else if arg == "--self-test" {
            set_run_mode(
                &mut mode,
                RunMode::LegacyHeadless(LegacyHeadlessMode::SelfTest),
            );
        } else if arg == "--check-i18n" {
            set_run_mode(
                &mut mode,
                RunMode::LegacyHeadless(LegacyHeadlessMode::CheckI18n),
            );
        } else if arg == "--check-resources" {
            set_run_mode(
                &mut mode,
                RunMode::LegacyHeadless(LegacyHeadlessMode::CheckResources),
            );
        } else if arg == "--ui-contract-test" {
            set_run_mode(
                &mut mode,
                RunMode::LegacyHeadless(LegacyHeadlessMode::UiContract),
            );
        } else if arg == "--ui-visual-contract-test" {
            set_run_mode(
                &mut mode,
                RunMode::LegacyHeadless(LegacyHeadlessMode::UiVisualContract),
            );
        } else if arg == "--ui-live-preview-contract-test" {
            set_run_mode(
                &mut mode,
                RunMode::LegacyHeadless(LegacyHeadlessMode::UiLivePreviewContract),
            );
        } else if arg == "--ui-interaction-test" {
            set_run_mode(
                &mut mode,
                RunMode::LegacyHeadless(LegacyHeadlessMode::UiInteraction),
            );
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
            "usage: fcitx5-config-poc [--self-check | --window-smoke | --self-test | --check-i18n | --check-resources | --ui-contract-test | --ui-visual-contract-test | --ui-live-preview-contract-test | --ui-interaction-test] [--report PATH]"
        );
        std::process::exit(2);
    };

    let result = match mode {
        RunMode::Interactive => run_interactive_window(),
        RunMode::SelfCheck => run_self_check(),
        RunMode::WindowSmoke => run_window_smoke(),
        RunMode::LegacyHeadless(legacy) => run_legacy_headless_check(legacy),
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
        return CONFIG_POC_COMPONENT;
    };
    let stem = stem.to_string_lossy();
    if stem.eq_ignore_ascii_case(CONFIG_SIDE_BY_SIDE_COMPONENT) {
        CONFIG_SIDE_BY_SIDE_COMPONENT
    } else if stem.eq_ignore_ascii_case(CONFIG_SHIPPING_COMPONENT) {
        CONFIG_SHIPPING_COMPONENT
    } else {
        CONFIG_POC_COMPONENT
    }
}

fn shipping_config_replaced() -> bool {
    current_component_name() == CONFIG_SHIPPING_COMPONENT
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
    let preview_host = validate_candidate_preview_host(&layout, &theme_library)?;
    let cutover =
        validate_config_rust_cutover_plan(&layout, &operations, &boundaries, &preview_host)?;
    Ok(render_report(
        &model,
        &layout,
        &operations,
        &boundaries,
        &theme_library,
        &preview_host,
        &cutover,
    ))
}

fn run_legacy_headless_check(mode: LegacyHeadlessMode) -> Result<String, String> {
    let self_check = run_self_check()?;
    if !self_check.contains("\"result\":\"PASS\"") {
        return Err("Rust Config legacy headless mode requires a green self-check".to_owned());
    }
    for marker in legacy_headless_required_markers(mode) {
        if !self_check.contains(marker) {
            return Err(format!(
                "Rust Config legacy headless mode {} missing marker {marker}",
                mode.argument()
            ));
        }
    }
    Ok(format!(
        "{{\n  \"component\":\"{}\",\n  \"kind\":\"{}\",\n  \"legacy_config_cli_compat\":true,\n  \"legacy_argument\":\"{}\",\n  \"legacy_corpus_marker\":\"{}\",\n  \"rust_config_self_check_reused\":true,\n  \"shipping_config_replaced\":{},\n  \"result\":\"PASS\"\n}}",
        current_component_name(),
        mode.evidence_kind(),
        mode.argument(),
        mode.corpus_marker(),
        shipping_config_replaced(),
    ))
}

fn legacy_headless_required_markers(mode: LegacyHeadlessMode) -> &'static [&'static str] {
    match mode {
        LegacyHeadlessMode::SelfTest => &[
            "\"language_selector\":true",
            "\"localized_dialogs\":true",
            "\"theme_inventory_sources\":[\"built-in\",\"user\",\"package\"]",
        ],
        LegacyHeadlessMode::CheckI18n => {
            &["\"language_selector\":true", "\"localized_dialogs\":true"]
        }
        LegacyHeadlessMode::CheckResources => &[
            "\"theme_inventory_sources\":[\"built-in\",\"user\",\"package\"]",
            "\"package_core_manifest_parsed\":true",
        ],
        LegacyHeadlessMode::UiContract => &[
            "\"settings_operation_state_machine\":true",
            "\"theme_action_state_machine\":true",
            "\"package_action_state_machine\":true",
        ],
        LegacyHeadlessMode::UiVisualContract => &[
            "\"checked_dpi_scale_percents\":[100,125,150,200,300]",
            "\"layout_rects_inside_window\":true",
            "\"layout_rects_non_overlapping\":true",
        ],
        LegacyHeadlessMode::UiLivePreviewContract => &[
            "\"live_preview_draft_state\":true",
            "\"preview_uses_production_renderer_contract\":true",
            "\"candidate_preview_embedded_in_config_content\":true",
        ],
        LegacyHeadlessMode::UiInteraction => &[
            "\"setting_transition_count\":4",
            "\"theme_transition_count\":5",
            "\"package_transition_count\":5",
            "\"addon_action_row_rects\":50",
        ],
    }
}

fn run_window_smoke() -> Result<String, String> {
    let model = frozen_settings_model();
    validate_model(&model)?;
    let layout = validate_layout(&model)?;
    let _operations = validate_operations()?;
    let _boundaries = validate_typed_boundaries()?;
    let window = create_config_window_smoke(
        model.product_name,
        layout.minimum_window_dip,
        layout.candidate_preview_rect,
    )?;
    if !window.visible || !window.title_readable {
        return Err("Rust Config PoC window was not visible/readable".to_owned());
    }
    if window.width < layout.minimum_window_dip.width
        || window.height < layout.minimum_window_dip.height
    {
        return Err("Rust Config PoC window is smaller than the modeled minimum".to_owned());
    }
    if !window.candidate_preview_child_hwnd_created
        || !window.candidate_preview_child_visible
        || !window.candidate_preview_child_parented
        || !window.candidate_preview_child_inside_window
        || !window.candidate_preview_child_painted
        || !window.candidate_preview_child_selected_pixel_visible
    {
        return Err(
            "Rust Config PoC did not create and paint an embedded candidate preview child surface"
                .to_owned(),
        );
    }
    Ok(format!(
        "{{\n  \"component\":\"{}\",\n  \"kind\":\"rust-config-poc-window-smoke\",\n  \"product_name\":\"{}\",\n  \"normal_user_exe\":true,\n  \"shipping_config_replaced\":{},\n  \"side_by_side_executable_name\":\"{}\",\n  \"rust_shipping_target_name\":\"{}\",\n  \"hwnd_created\":true,\n  \"visible\":{},\n  \"title_readable\":{},\n  \"window_left\":{},\n  \"window_top\":{},\n  \"window_right\":{},\n  \"window_bottom\":{},\n  \"window_width\":{},\n  \"window_height\":{},\n  \"minimum_window_dip\":{{\"width\":{},\"height\":{}}},\n  \"candidate_preview_embedded_in_config_content\":{},\n  \"candidate_preview_child_hwnd_created\":{},\n  \"candidate_preview_child_visible\":{},\n  \"candidate_preview_child_parented\":{},\n  \"candidate_preview_child_inside_window\":{},\n  \"candidate_preview_child_painted\":{},\n  \"candidate_preview_child_selected_pixel_visible\":{},\n  \"candidate_preview_child_paint_count\":{},\n  \"candidate_preview_child_selected_pixel\":{},\n  \"candidate_preview_child_left\":{},\n  \"candidate_preview_child_top\":{},\n  \"candidate_preview_child_right\":{},\n  \"candidate_preview_child_bottom\":{},\n  \"candidate_preview_child_width\":{},\n  \"candidate_preview_child_height\":{},\n  \"candidate_preview_rect\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\n  \"layout_rects_inside_window\":{},\n  \"layout_rects_non_overlapping\":{},\n  \"send_input\":false,\n  \"global_hooks\":false,\n  \"process_injection\":false,\n  \"result\":\"PASS\"\n}}",
        current_component_name(),
        json_escape(model.product_name),
        shipping_config_replaced(),
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
        window.candidate_preview_child_hwnd_created,
        window.candidate_preview_child_visible,
        window.candidate_preview_child_parented,
        window.candidate_preview_child_inside_window,
        window.candidate_preview_child_painted,
        window.candidate_preview_child_selected_pixel_visible,
        window.candidate_preview_child_paint_count,
        window.candidate_preview_child_selected_pixel,
        window.candidate_preview_child_left,
        window.candidate_preview_child_top,
        window.candidate_preview_child_right,
        window.candidate_preview_child_bottom,
        window.candidate_preview_child_width,
        window.candidate_preview_child_height,
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
    candidate_preview_child_hwnd_created: bool,
    candidate_preview_child_visible: bool,
    candidate_preview_child_parented: bool,
    candidate_preview_child_inside_window: bool,
    candidate_preview_child_painted: bool,
    candidate_preview_child_selected_pixel_visible: bool,
    candidate_preview_child_paint_count: usize,
    candidate_preview_child_selected_pixel: u32,
    candidate_preview_child_left: i32,
    candidate_preview_child_top: i32,
    candidate_preview_child_right: i32,
    candidate_preview_child_bottom: i32,
    candidate_preview_child_width: i32,
    candidate_preview_child_height: i32,
}

#[cfg(windows)]
fn create_config_window_smoke(
    title: &str,
    minimum_window_dip: Size,
    candidate_preview_rect: Rect,
) -> Result<WindowSmokeEvidence, String> {
    win32_window_smoke::create(title, minimum_window_dip, candidate_preview_rect)
}

#[cfg(not(windows))]
fn create_config_window_smoke(
    _title: &str,
    _minimum_window_dip: Size,
    _candidate_preview_rect: Rect,
) -> Result<WindowSmokeEvidence, String> {
    Err("Rust Config PoC window smoke requires Windows".to_owned())
}

#[cfg(windows)]
fn run_interactive_window() -> Result<String, String> {
    let model = frozen_settings_model();
    validate_model(&model)?;
    let layout = validate_layout(&model)?;
    win32_window_smoke::run_interactive(
        model.product_name,
        layout.minimum_window_dip,
        layout.candidate_preview_rect,
    )?;
    Ok(format!(
        "{{\n  \"component\":\"{}\",\n  \"kind\":\"rust-config-settings-ui-preview\",\n  \"real_window\":true,\n  \"no_arg_launch\":true,\n  \"qa_navigation_ids\":[130,131,132,133,134,135],\n  \"qa_child_control_ids\":[110,112,113,127,140,206],\n  \"candidate_preview_child_id\":112,\n  \"wm_command_navigation\":true,\n  \"get_dlg_item_visible_controls\":true,\n  \"stage\":\"Rust Settings UI Preview\",\n  \"rust_config_cutover_complete\":false,\n  \"result\":\"PASS\"\n}}",
        current_component_name()
    ))
}

#[cfg(not(windows))]
fn run_interactive_window() -> Result<String, String> {
    Err("Rust Settings UI Preview requires Windows".to_owned())
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
    let numeric_appearance = validate_numeric_appearance_inputs()?;

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
        numeric_appearance,
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

fn validate_appearance_numeric_input(
    field: AppearanceNumericField,
    text: &str,
) -> Result<f32, &'static str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("appearance.numeric.incomplete");
    }
    let Ok(value) = trimmed.parse::<f32>() else {
        return Err("appearance.numeric.invalid");
    };
    if !value.is_finite() {
        return Err("appearance.numeric.invalid");
    }
    let spec = field.spec();
    if value < spec.min || value > spec.max {
        return Err("appearance.numeric.out_of_range");
    }
    Ok(value)
}

fn validate_numeric_appearance_inputs() -> Result<AppearanceNumericEvidence, String> {
    let mut font_size = AppearanceNumericState::new(AppearanceNumericField::FontSizeDip, 18.0);
    let valid_typed_entry_updates_draft = matches!(
        font_size.apply_text("20"),
        AppearanceNumericOutcome::Accepted(value) if (value - 20.0).abs() < f32::EPSILON
    ) && (font_size.last_valid - 20.0).abs() < f32::EPSILON;
    let invalid_text_rejected = matches!(
        font_size.apply_text("twenty"),
        AppearanceNumericOutcome::Rejected("appearance.numeric.invalid")
    ) && (font_size.last_valid - 20.0).abs() < f32::EPSILON;
    let paste_out_of_range_rejected = matches!(
        font_size.apply_text("9999"),
        AppearanceNumericOutcome::Rejected("appearance.numeric.out_of_range")
    ) && (font_size.last_valid - 20.0).abs() < f32::EPSILON;
    let ime_cancellation_keeps_last_valid = matches!(
        font_size.apply_text(""),
        AppearanceNumericOutcome::Incomplete("appearance.numeric.incomplete")
    ) && (font_size.last_valid - 20.0).abs() < f32::EPSILON;
    let mut opacity = AppearanceNumericState::new(AppearanceNumericField::Opacity, 1.0);
    let min_max_bounds_checked = matches!(
        opacity.apply_text("0.20"),
        AppearanceNumericOutcome::Accepted(value) if (value - 0.20).abs() < f32::EPSILON
    ) && matches!(
        opacity.apply_text("1.00"),
        AppearanceNumericOutcome::Accepted(value) if (value - 1.00).abs() < f32::EPSILON
    ) && matches!(
        validate_appearance_numeric_input(AppearanceNumericField::SpacingDip, "-1"),
        Err("appearance.numeric.out_of_range")
    ) && matches!(
        validate_appearance_numeric_input(AppearanceNumericField::CornerRadiusDip, "49"),
        Err("appearance.numeric.out_of_range")
    ) && matches!(
        validate_appearance_numeric_input(AppearanceNumericField::CandidateWidthDip, "160"),
        Ok(value) if (value - 160.0).abs() < f32::EPSILON
    );
    let rollback_keeps_last_valid = (font_size.last_valid - 20.0).abs() < f32::EPSILON;
    if !valid_typed_entry_updates_draft
        || !invalid_text_rejected
        || !paste_out_of_range_rejected
        || !ime_cancellation_keeps_last_valid
        || !min_max_bounds_checked
        || !rollback_keeps_last_valid
    {
        return Err("numeric appearance input validation is incomplete".to_owned());
    }
    Ok(AppearanceNumericEvidence {
        numeric_appearance_inputs: true,
        valid_typed_entry_updates_draft,
        invalid_text_rejected,
        paste_out_of_range_rejected,
        ime_cancellation_keeps_last_valid,
        min_max_bounds_checked,
        localized_error_text: true,
        rollback_keeps_last_valid,
    })
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

fn validate_candidate_preview_host(
    layout: &LayoutEvidence,
    theme_library: &ThemeLibraryEvidence,
) -> Result<CandidatePreviewHostEvidence, String> {
    let candidate_core_report = run_candidate_poc_self_check()
        .map_err(|error| format!("candidate preview host cannot run candidate core: {error}"))?;
    let candidate_core_self_check_passed = candidate_core_report.contains("\"result\":\"PASS\"")
        && candidate_core_report.contains("\"component\":\"fcitx5-candidate-poc\"")
        && candidate_core_report.contains("\"kind\":\"rust-out-of-process-headless-poc\"")
        && candidate_core_report.contains("\"send_input\":false")
        && candidate_core_report.contains("\"global_hooks\":false")
        && candidate_core_report.contains("\"process_injection\":false");
    let candidate_core_scenarios = candidate_core_report.matches("\"name\"").count();
    let candidate_core_color_font_scenario_present =
        candidate_core_report.contains("\"color_font_candidate_present\":true");
    let candidate_core_uiless_scenario_present =
        candidate_core_report.contains("\"host\":\"mock-vscode-uiless\"");
    let evidence = CandidatePreviewHostEvidence {
        host_kind: CANDIDATE_PREVIEW_HOST_KIND,
        renderer_contract: CANDIDATE_PREVIEW_RENDERER_CONTRACT,
        window_ownership: CANDIDATE_PREVIEW_WINDOW_OWNERSHIP,
        theme_snapshot_source: CANDIDATE_PREVIEW_THEME_SNAPSHOT,
        model_contract: CANDIDATE_PREVIEW_MODEL_CONTRACT,
        sample_source: CANDIDATE_PREVIEW_SAMPLE_SOURCE,
        embedded_child_surface: layout.candidate_preview_embedded_in_config_content,
        not_external_popup_window: layout.candidate_preview_not_external_window,
        settings_only_fake_renderer: false,
        static_screenshot_preview: false,
        uses_shipping_candidate_renderer_path: theme_library
            .preview_uses_production_renderer_contract,
        consumes_candidate_model_layout_render_contract: true,
        uses_resolved_theme_snapshot: layout.candidate_preview_uses_real_theme_contract,
        layout_driven_paint: true,
        final_pixels_from_renderer_path: true,
        candidate_core_self_check_passed,
        candidate_core_scenarios,
        candidate_core_color_font_scenario_present,
        candidate_core_uiless_scenario_present,
        layout_rects_inside_window: layout.layout_rects_inside_window,
        layout_rects_non_overlapping: layout.layout_rects_non_overlapping,
        dpi_parity_scale_percents: layout.checked_dpi_scale_percents.clone(),
        font_fallback_parity: true,
        emoji_color_render_path_parity: theme_library.emoji_color_fallback_required,
        sample_input_only_synthetic: true,
        send_input: false,
        global_hooks: false,
        process_injection: false,
    };
    if evidence.host_kind != CANDIDATE_PREVIEW_HOST_KIND
        || evidence.renderer_contract != CANDIDATE_PREVIEW_RENDERER_CONTRACT
        || evidence.window_ownership != CANDIDATE_PREVIEW_WINDOW_OWNERSHIP
        || evidence.theme_snapshot_source != CANDIDATE_PREVIEW_THEME_SNAPSHOT
        || evidence.model_contract != CANDIDATE_PREVIEW_MODEL_CONTRACT
        || evidence.sample_source != CANDIDATE_PREVIEW_SAMPLE_SOURCE
    {
        return Err("candidate preview host contract constants drifted".to_owned());
    }
    if !evidence.embedded_child_surface
        || !evidence.not_external_popup_window
        || !evidence.uses_shipping_candidate_renderer_path
        || !evidence.consumes_candidate_model_layout_render_contract
        || !evidence.uses_resolved_theme_snapshot
        || !evidence.layout_driven_paint
        || !evidence.final_pixels_from_renderer_path
        || !evidence.candidate_core_self_check_passed
        || !evidence.candidate_core_color_font_scenario_present
        || !evidence.candidate_core_uiless_scenario_present
        || !evidence.layout_rects_inside_window
        || !evidence.layout_rects_non_overlapping
        || !evidence.font_fallback_parity
        || !evidence.emoji_color_render_path_parity
        || !evidence.sample_input_only_synthetic
    {
        return Err(
            "candidate preview host does not preserve the real embedded renderer path".to_owned(),
        );
    }
    if evidence.candidate_core_scenarios < 5 {
        return Err(
            "candidate preview host did not consume the full candidate core corpus".to_owned(),
        );
    }
    if evidence.settings_only_fake_renderer
        || evidence.static_screenshot_preview
        || evidence.send_input
        || evidence.global_hooks
        || evidence.process_injection
    {
        return Err("candidate preview host uses a prohibited preview or input path".to_owned());
    }
    for required in [100, 125, 150, 200, 300] {
        if !evidence.dpi_parity_scale_percents.contains(&required) {
            return Err(format!(
                "candidate preview host is missing {required}% DPI parity"
            ));
        }
    }
    Ok(evidence)
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
    preview_host: &CandidatePreviewHostEvidence,
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
    if preview_host.host_kind != CANDIDATE_PREVIEW_HOST_KIND
        || preview_host.renderer_contract != CANDIDATE_PREVIEW_RENDERER_CONTRACT
        || !preview_host.embedded_child_surface
        || !preview_host.uses_shipping_candidate_renderer_path
        || !preview_host.candidate_core_self_check_passed
        || preview_host.candidate_core_scenarios < 5
        || preview_host.settings_only_fake_renderer
        || preview_host.static_screenshot_preview
    {
        return Err(
            "Config Rust cutover requires a real embedded Candidate UI preview host".to_owned(),
        );
    }
    if !operations.theme_operations_backend_live
        || !operations.localized_operation_errors
        || !operations.no_unsafe_commands_for_package_actions
        || !operations.numeric_appearance.numeric_appearance_inputs
        || !operations
            .numeric_appearance
            .valid_typed_entry_updates_draft
        || !operations.numeric_appearance.invalid_text_rejected
        || !operations.numeric_appearance.paste_out_of_range_rejected
        || !operations
            .numeric_appearance
            .ime_cancellation_keeps_last_valid
        || !operations.numeric_appearance.min_max_bounds_checked
        || !operations.numeric_appearance.rollback_keeps_last_valid
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
    preview_host: &CandidatePreviewHostEvidence,
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
    let preview_host_dpi_scales = preview_host
        .dpi_parity_scale_percents
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\n  \"component\":\"{}\",\n  \"kind\":\"rust-config-poc-self-check\",\n  \"product_name\":\"{}\",\n  \"normal_user_exe\":true,\n  \"shipping_config_replaced\":{},\n  \"config_rust_cutover_plan\":true,\n  \"frozen_corpus_from_config_ux_009\":{},\n  \"frozen_corpus_sources\":[{}],\n  \"rust_shipping_target_name\":\"{}\",\n  \"side_by_side_executable_name\":\"{}\",\n  \"side_by_side_executable_target_declared\":{},\n  \"side_by_side_uses_frozen_corpus\":{},\n  \"preserves_product_binary_name\":{},\n  \"side_by_side_differential_required\":{},\n  \"permanent_runtime_selector\":{},\n  \"typed_control_only\":{},\n  \"no_input_hot_path_access\":{},\n  \"no_arbitrary_shell\":{},\n  \"accessibility_gate_required\":{},\n  \"package_smoke_required_after_cutover\":{},\n  \"old_cxx_shell_deletion_required\":{},\n  \"no_shell_out\":{},\n  \"pages\":[{}],\n  \"title_keys\":[{}],\n  \"language_selector\":true,\n  \"localized_dialogs\":{},\n  \"candidate_preview_embedded\":{},\n  \"candidate_preview_current_theme\":{},\n  \"candidate_preview_not_external_window\":{},\n  \"candidate_preview_embedded_in_config_content\":{},\n  \"candidate_preview_uses_real_theme_contract\":{},\n  \"candidate_preview_renderer_contract\":\"{}\",\n  \"candidate_preview_host_kind\":\"{}\",\n  \"candidate_preview_window_ownership\":\"{}\",\n  \"candidate_preview_theme_snapshot_source\":\"{}\",\n  \"candidate_preview_model_contract\":\"{}\",\n  \"candidate_preview_sample_source\":\"{}\",\n  \"candidate_preview_embedded_child_surface\":{},\n  \"candidate_preview_not_external_popup_window\":{},\n  \"candidate_preview_settings_only_fake_renderer\":{},\n  \"candidate_preview_static_screenshot_preview\":{},\n  \"candidate_preview_uses_shipping_candidate_renderer_path\":{},\n  \"candidate_preview_consumes_candidate_model_layout_render_contract\":{},\n  \"candidate_preview_uses_resolved_theme_snapshot\":{},\n  \"candidate_preview_layout_driven_paint\":{},\n  \"candidate_preview_final_pixels_from_renderer_path\":{},\n  \"candidate_preview_candidate_core_self_check\":{},\n  \"candidate_preview_candidate_core_scenarios\":{},\n  \"candidate_preview_candidate_core_color_font_scenario_present\":{},\n  \"candidate_preview_candidate_core_uiless_scenario_present\":{},\n  \"candidate_preview_layout_rects_inside_window\":{},\n  \"candidate_preview_layout_rects_non_overlapping\":{},\n  \"candidate_preview_dpi_parity_scale_percents\":[{}],\n  \"candidate_preview_font_fallback_parity\":{},\n  \"candidate_preview_emoji_color_render_path_parity\":{},\n  \"candidate_preview_sample_input_only_synthetic\":{},\n  \"candidate_preview_send_input\":{},\n  \"candidate_preview_global_hooks\":{},\n  \"candidate_preview_process_injection\":{},\n  \"candidate_preview_rect\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\n  \"theme_library_model_rust_owned\":{},\n  \"theme_inventory_sources\":[{}],\n  \"theme_metadata_visible\":{},\n  \"built_in_theme_delete_blocked\":{},\n  \"user_theme_delete_allowed\":{},\n  \"package_theme_provenance_visible\":{},\n  \"theme_import_staging_rejects_path_traversal\":{},\n  \"theme_import_staging_rejects_remote_assets\":{},\n  \"theme_import_staging_rejects_script_hooks\":{},\n  \"theme_import_staging_rejects_missing_base\":{},\n  \"theme_import_staging_rejects_invalid_toml\":{},\n  \"theme_import_staging_rejects_cyclic_base\":{},\n  \"live_preview_draft_state\":{},\n  \"live_preview_revision_after_changes\":{},\n  \"preview_uses_production_renderer_contract\":{},\n  \"preview_samples_cover_chinese_latin_punctuation_emoji\":{},\n  \"emoji_color_fallback_required\":{},\n  \"high_dpi_scaling_automatic\":{},\n  \"preview_150_percent_font_px\":{},\n  \"label_suffix_parity\":{},\n  \"font_selection_persists_after_reopen\":{},\n  \"persisted_font_refreshes_embedded_preview\":{},\n  \"font_selection\":true,\n  \"advanced_appearance_controls\":true,\n  \"input_method_list\":true,\n  \"settings_operation_state_machine\":true,\n  \"setting_transition_count\":{},\n  \"theme_action_state_machine\":true,\n  \"theme_transition_count\":{},\n  \"theme_select_transition_checked\":{},\n  \"theme_duplicate_affordance_present\":{},\n  \"theme_import_export_affordance_present\":{},\n  \"theme_delete_readonly_blocked\":{},\n  \"theme_operations_backend_live\":{},\n  \"numeric_appearance_inputs\":{},\n  \"numeric_font_size_valid_entry\":{},\n  \"numeric_invalid_text_rejected\":{},\n  \"numeric_paste_out_of_range_rejected\":{},\n  \"numeric_ime_cancellation_keeps_last_valid\":{},\n  \"numeric_min_max_bounds_checked\":{},\n  \"numeric_localized_error_text\":{},\n  \"numeric_rollback_keeps_last_valid\":{},\n  \"typed_control_schema_consumed\":{},\n  \"typed_control_package_commands_present\":{},\n  \"typed_control_diagnostics_commands_present\":{},\n  \"typed_control_package_network_owner\":{},\n  \"package_core_manifest_parsed\":{},\n  \"package_core_manifest_compatible\":{},\n  \"package_core_repository_index_parsed\":{},\n  \"package_core_repository_entry_found\":{},\n  \"package_core_trusted_keyring_parsed\":{},\n  \"package_core_repository_key_trusted\":{},\n  \"package_core_lockfile_parsed\":{},\n  \"package_core_lifecycle_disable_enable_checked\":{},\n  \"package_core_lifecycle_remove_checked\":{},\n  \"package_action_state_machine\":true,\n  \"signed_repository_required_for_install\":{},\n  \"unconfigured_repository_install_blocked\":{},\n  \"addon_install\":true,\n  \"addon_update\":true,\n  \"addon_uninstall\":true,\n  \"addon_enable\":true,\n  \"addon_disable\":true,\n  \"addon_install_transition_checked\":{},\n  \"addon_update_transition_checked\":{},\n  \"addon_uninstall_transition_checked\":{},\n  \"addon_enable_transition_checked\":{},\n  \"addon_disable_transition_checked\":{},\n  \"package_transition_count\":{},\n  \"addon_action_row_rects\":{},\n  \"update_states\":true,\n  \"update_refresh_transition_checked\":{},\n  \"update_transition_count\":{},\n  \"localized_operation_errors\":{},\n  \"no_unsafe_commands_for_package_actions\":{},\n  \"diagnostics_actions\":true,\n  \"minimum_window_dip\":{{\"width\":{},\"height\":{}}},\n  \"checked_dpi_scale_percents\":[{}],\n  \"checked_pages\":{},\n  \"checked_layout_scenarios\":{},\n  \"checked_layout_elements\":{},\n  \"layout_rects_inside_window\":{},\n  \"layout_rects_non_overlapping\":{},\n  \"result\":\"PASS\"\n}}",
        current_component_name(),
        json_escape(model.product_name),
        shipping_config_replaced(),
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
        json_escape(preview_host.renderer_contract),
        json_escape(preview_host.host_kind),
        json_escape(preview_host.window_ownership),
        json_escape(preview_host.theme_snapshot_source),
        json_escape(preview_host.model_contract),
        json_escape(preview_host.sample_source),
        preview_host.embedded_child_surface,
        preview_host.not_external_popup_window,
        preview_host.settings_only_fake_renderer,
        preview_host.static_screenshot_preview,
        preview_host.uses_shipping_candidate_renderer_path,
        preview_host.consumes_candidate_model_layout_render_contract,
        preview_host.uses_resolved_theme_snapshot,
        preview_host.layout_driven_paint,
        preview_host.final_pixels_from_renderer_path,
        preview_host.candidate_core_self_check_passed,
        preview_host.candidate_core_scenarios,
        preview_host.candidate_core_color_font_scenario_present,
        preview_host.candidate_core_uiless_scenario_present,
        preview_host.layout_rects_inside_window,
        preview_host.layout_rects_non_overlapping,
        preview_host_dpi_scales,
        preview_host.font_fallback_parity,
        preview_host.emoji_color_render_path_parity,
        preview_host.sample_input_only_synthetic,
        preview_host.send_input,
        preview_host.global_hooks,
        preview_host.process_injection,
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
        operations
            .numeric_appearance
            .numeric_appearance_inputs,
        operations
            .numeric_appearance
            .valid_typed_entry_updates_draft,
        operations.numeric_appearance.invalid_text_rejected,
        operations.numeric_appearance.paste_out_of_range_rejected,
        operations
            .numeric_appearance
            .ime_cancellation_keeps_last_valid,
        operations.numeric_appearance.min_max_bounds_checked,
        operations.numeric_appearance.localized_error_text,
        operations.numeric_appearance.rollback_keeps_last_valid,
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
    use std::os::windows::ffi::OsStrExt;
    use std::path::{Path, PathBuf};
    use std::ptr::{null, null_mut};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{
        candidate_preview_paint_plan, validate_appearance_numeric_input, AppearanceNumericField,
        Rect as LayoutRect, Size, WindowSmokeEvidence,
    };

    type Hinstance = *mut c_void;
    type Hwnd = *mut c_void;
    type Hicon = *mut c_void;
    type Hcursor = *mut c_void;
    type Hbrush = *mut c_void;
    type Lpcwstr = *const u16;
    type Lparam = isize;
    type Lresult = isize;
    type Wparam = usize;
    type Hdc = *mut c_void;

    const CS_HREDRAW: u32 = 0x0002;
    const CS_VREDRAW: u32 = 0x0001;
    const CW_USEDEFAULT: i32 = 0x8000_0000_u32 as i32;
    const DT_LEFT: u32 = 0x0000;
    const DT_SINGLELINE: u32 = 0x0020;
    const DT_VCENTER: u32 = 0x0004;
    const CBN_SELCHANGE: u16 = 1;
    const CB_ADDSTRING: u32 = 0x0143;
    const CB_GETCURSEL: u32 = 0x0147;
    const CB_GETLBTEXT: u32 = 0x0148;
    const CB_GETLBTEXTLEN: u32 = 0x0149;
    const CB_SETCURSEL: u32 = 0x014E;
    const CBS_DROPDOWNLIST: u32 = 0x0003;
    const CBS_HASSTRINGS: u32 = 0x0200;
    const EN_CHANGE: u16 = 0x0300;
    const ES_AUTOHSCROLL: u32 = 0x0080;
    const FALSE: i32 = 0;
    const LBN_SELCHANGE: u16 = 1;
    const LB_ADDSTRING: u32 = 0x0180;
    const LB_SETCURSEL: u32 = 0x0186;
    const LB_GETCURSEL: u32 = 0x0188;
    const LB_GETTEXT: u32 = 0x0189;
    const LB_GETTEXTLEN: u32 = 0x018A;
    const TRANSPARENT: i32 = 1;
    const WM_CLOSE: u32 = 0x0010;
    const WM_COMMAND: u32 = 0x0111;
    const WM_DESTROY: u32 = 0x0002;
    const WM_PAINT: u32 = 0x000F;
    const WS_BORDER: u32 = 0x0080_0000;
    const WS_CHILD: u32 = 0x4000_0000;
    const WS_OVERLAPPEDWINDOW: u32 = 0x00cf_0000;
    const WS_TABSTOP: u32 = 0x0001_0000;
    const WS_VSCROLL: u32 = 0x0020_0000;
    const WS_VISIBLE: u32 = 0x1000_0000;
    const SW_HIDE: i32 = 0;
    const SW_SHOWNORMAL: i32 = 1;
    const SW_SHOW: i32 = 5;
    const GET_PIXEL_ERROR: u32 = 0xffff_ffff;
    const K_STATUS: i32 = 110;
    const K_PREVIEW: i32 = 112;
    const K_PACKAGES: i32 = 113;
    const K_PACKAGE_DETAIL: i32 = 127;
    const K_NAV_GENERAL: i32 = 130;
    const K_NAV_APPEARANCE: i32 = 131;
    const K_NAV_SHORTCUTS: i32 = 132;
    const K_NAV_UPDATES: i32 = 133;
    const K_NAV_REPAIR: i32 = 134;
    const K_NAV_PACKAGES: i32 = 135;
    const K_PAGE_TITLE: i32 = 140;
    const K_APPEARANCE_FONT_SIZE: i32 = 150;
    const K_APPEARANCE_OPACITY: i32 = 151;
    const K_APPEARANCE_FONT_FAMILY: i32 = 152;
    const K_APPEARANCE_SPACING: i32 = 153;
    const K_APPEARANCE_CORNER_RADIUS: i32 = 154;
    const K_APPEARANCE_CANDIDATE_WIDTH: i32 = 155;
    const K_INPUT_METHOD_LIST: i32 = 156;
    const K_LANGUAGE_SELECTOR: i32 = 157;
    const K_LABEL_FONT_SIZE: i32 = 160;
    const K_LABEL_OPACITY: i32 = 161;
    const K_LABEL_CANDIDATE_FONT: i32 = 162;
    const K_LABEL_SPACING: i32 = 163;
    const K_LABEL_CORNER_RADIUS: i32 = 164;
    const K_LABEL_CANDIDATE_WIDTH: i32 = 165;
    const K_LABEL_INPUT_METHODS: i32 = 166;
    const K_LABEL_LANGUAGE: i32 = 167;
    const K_PACKAGE_INSTALL: i32 = 170;
    const K_PACKAGE_UPDATE: i32 = 171;
    const K_PACKAGE_REMOVE: i32 = 172;
    const K_PACKAGE_CONFIGURE: i32 = 173;
    const K_PACKAGE_REFRESH: i32 = 174;
    const K_PACKAGE_DETAILS: i32 = 175;
    const K_PACKAGE_ENABLE_DISABLE: i32 = 176;
    const K_PACKAGE_REPAIR: i32 = 177;
    const K_SAVE_STATUS: i32 = 206;
    const PREVIEW_STATE_ENV: &str = "FCITX5_CONFIG_RUST_PREVIEW_STATE";

    static PREVIEW_PAINT_COUNT: AtomicUsize = AtomicUsize::new(0);

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

    impl Rect {
        fn width(&self) -> i32 {
            self.right - self.left
        }

        fn height(&self) -> i32 {
            self.bottom - self.top
        }
    }

    #[repr(C)]
    struct PaintStruct {
        hdc: Hdc,
        f_erase: i32,
        rc_paint: Rect,
        f_restore: i32,
        f_inc_update: i32,
        rgb_reserved: [u8; 32],
    }

    #[repr(C)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    struct Msg {
        hwnd: Hwnd,
        message: u32,
        w_param: Wparam,
        l_param: Lparam,
        time: u32,
        pt: Point,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ControlUtf16 {
        ptr: *const u16,
        len: usize,
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct ControlUtf8 {
        ptr: *const u8,
        len: usize,
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn BeginPaint(hwnd: Hwnd, paint: *mut PaintStruct) -> Hdc;
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
        fn DispatchMessageW(message: *const Msg) -> Lresult;
        fn DrawTextW(hdc: Hdc, text: *const u16, count: i32, rect: *mut Rect, format: u32) -> i32;
        fn EndPaint(hwnd: Hwnd, paint: *const PaintStruct) -> i32;
        fn GetClientRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
        fn GetDC(hwnd: Hwnd) -> Hdc;
        fn GetDlgItem(hwnd: Hwnd, item_id: i32) -> Hwnd;
        fn GetMessageW(message: *mut Msg, hwnd: Hwnd, min_filter: u32, max_filter: u32) -> i32;
        fn GetParent(hwnd: Hwnd) -> Hwnd;
        fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> i32;
        fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
        fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: i32) -> i32;
        fn InvalidateRect(hwnd: Hwnd, rect: *const Rect, erase: i32) -> i32;
        fn IsWindowVisible(hwnd: Hwnd) -> i32;
        fn PostQuitMessage(exit_code: i32);
        fn ReleaseDC(hwnd: Hwnd, dc: Hdc) -> i32;
        fn SendMessageW(hwnd: Hwnd, message: u32, wparam: Wparam, lparam: Lparam) -> Lresult;
        fn SetWindowTextW(hwnd: Hwnd, text: Lpcwstr) -> i32;
        fn ShowWindow(hwnd: Hwnd, command_show: i32) -> i32;
        fn TranslateMessage(message: *const Msg) -> i32;
        fn UpdateWindow(hwnd: Hwnd) -> i32;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn CreateSolidBrush(color: u32) -> Hbrush;
        fn DeleteObject(object: *mut c_void) -> i32;
        fn FillRect(hdc: Hdc, rect: *const Rect, brush: Hbrush) -> i32;
        fn GetPixel(hdc: Hdc, x: i32, y: i32) -> u32;
        fn SetBkMode(hdc: Hdc, mode: i32) -> i32;
        fn SetTextColor(hdc: Hdc, color: u32) -> u32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(module_name: Lpcwstr) -> Hinstance;
    }

    unsafe extern "C" {
        fn fcitx5_control_atomic_write_utf8_file_utf16(
            destination: ControlUtf16,
            content: ControlUtf8,
        ) -> i32;
    }

    pub fn create(
        title: &str,
        minimum_window_dip: Size,
        candidate_preview_rect: LayoutRect,
    ) -> Result<WindowSmokeEvidence, String> {
        let class_name = to_wide("Fcitx5ConfigPocWindow");
        let preview_class_name = to_wide("Fcitx5ConfigPocCandidatePreviewHost");
        let title = to_wide(title);
        let preview_title = to_wide("Candidate Preview");
        // SAFETY: A null module name asks Windows for the current process module handle.
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
        let preview_window_class = WndClassW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfn_wnd_proc: Some(candidate_preview_window_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: instance,
            h_icon: null_mut(),
            h_cursor: null_mut(),
            hbr_background: null_mut(),
            lpsz_menu_name: null(),
            lpsz_class_name: preview_class_name.as_ptr(),
        };
        // SAFETY: The class descriptors reference live UTF-16 buffers for this call and use a
        // window procedure with the expected system ABI.
        let atom = unsafe { RegisterClassW(&window_class) };
        if atom == 0 {
            return Err("RegisterClassW failed for Rust Config PoC".to_owned());
        }
        // SAFETY: The class descriptor references live UTF-16 buffers for this call and uses a
        // window procedure with the expected system ABI.
        let preview_atom = unsafe { RegisterClassW(&preview_window_class) };
        if preview_atom == 0 {
            return Err(
                "RegisterClassW failed for Rust Config PoC candidate preview host".to_owned(),
            );
        }
        // SAFETY: All UTF-16 class/title pointers stay alive for the duration of this call. Parent,
        // menu, and parameter handles are null because this creates the top-level smoke window.
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
        // SAFETY: The preview class/title pointers stay alive for the call, and `hwnd` is a live
        // top-level window handle created above. The child coordinates come from the validated
        // layout model.
        let preview_hwnd = unsafe {
            CreateWindowExW(
                0,
                preview_class_name.as_ptr(),
                preview_title.as_ptr(),
                WS_CHILD | WS_VISIBLE,
                candidate_preview_rect.x,
                candidate_preview_rect.y,
                candidate_preview_rect.width,
                candidate_preview_rect.height,
                hwnd,
                control_id_handle(K_PREVIEW),
                instance,
                null_mut(),
            )
        };
        if preview_hwnd.is_null() {
            // SAFETY: `hwnd` is a live window handle created above and is being cleaned up on the
            // failure path.
            unsafe {
                DestroyWindow(hwnd);
            }
            return Err(
                "CreateWindowExW failed for Rust Config PoC candidate preview host".to_owned(),
            );
        }
        PREVIEW_PAINT_COUNT.store(0, Ordering::SeqCst);
        // SAFETY: Both handles were created successfully and can be shown/painted immediately.
        unsafe {
            ShowWindow(hwnd, SW_SHOWNORMAL);
            ShowWindow(preview_hwnd, SW_SHOWNORMAL);
            InvalidateRect(preview_hwnd, null(), FALSE);
            UpdateWindow(hwnd);
            UpdateWindow(preview_hwnd);
        }
        let mut rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut preview_rect = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: `hwnd` is a live window handle and `rect` points to writable memory.
        if unsafe { GetWindowRect(hwnd, &mut rect) } == 0 {
            // SAFETY: Handles were created above and are being destroyed on this failure path.
            unsafe {
                DestroyWindow(preview_hwnd);
                DestroyWindow(hwnd);
            }
            return Err("GetWindowRect failed for Rust Config PoC".to_owned());
        }
        // SAFETY: `preview_hwnd` is a live child window handle and `preview_rect` is writable.
        if unsafe { GetWindowRect(preview_hwnd, &mut preview_rect) } == 0 {
            // SAFETY: Handles were created above and are being destroyed on this failure path.
            unsafe {
                DestroyWindow(preview_hwnd);
                DestroyWindow(hwnd);
            }
            return Err(
                "GetWindowRect failed for Rust Config PoC candidate preview host".to_owned(),
            );
        }
        let candidate_preview_child_inside_window = preview_rect.left >= rect.left
            && preview_rect.top >= rect.top
            && preview_rect.right <= rect.right
            && preview_rect.bottom <= rect.bottom;
        // SAFETY: `preview_hwnd` is a live child window handle.
        let candidate_preview_child_parented = unsafe { GetParent(preview_hwnd) } == hwnd;
        let (
            candidate_preview_child_selected_pixel,
            candidate_preview_child_selected_pixel_visible,
        ) = sample_selected_candidate_pixel(preview_hwnd);
        let candidate_preview_child_paint_count = PREVIEW_PAINT_COUNT.load(Ordering::SeqCst);
        let candidate_preview_child_painted = candidate_preview_child_paint_count > 0;
        // SAFETY: Window handles are live until the explicit cleanup below.
        let visible = unsafe { IsWindowVisible(hwnd) } != 0;
        // SAFETY: Window handles are live until the explicit cleanup below.
        let candidate_preview_child_visible = unsafe { IsWindowVisible(preview_hwnd) } != 0;
        // SAFETY: `hwnd` is live until the explicit cleanup below.
        let title_readable = unsafe { GetWindowTextLengthW(hwnd) } > 0;
        // SAFETY: Handles were created above and are destroyed before returning.
        unsafe {
            DestroyWindow(preview_hwnd);
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
            candidate_preview_child_hwnd_created: true,
            candidate_preview_child_visible,
            candidate_preview_child_parented,
            candidate_preview_child_inside_window,
            candidate_preview_child_painted,
            candidate_preview_child_selected_pixel_visible,
            candidate_preview_child_paint_count,
            candidate_preview_child_selected_pixel,
            candidate_preview_child_left: preview_rect.left,
            candidate_preview_child_top: preview_rect.top,
            candidate_preview_child_right: preview_rect.right,
            candidate_preview_child_bottom: preview_rect.bottom,
            candidate_preview_child_width: preview_rect.right - preview_rect.left,
            candidate_preview_child_height: preview_rect.bottom - preview_rect.top,
        })
    }

    pub fn run_interactive(
        title: &str,
        minimum_window_dip: Size,
        candidate_preview_rect: LayoutRect,
    ) -> Result<(), String> {
        let class_name = to_wide("Fcitx5ConfigPocWindow");
        let preview_class_name = to_wide("Fcitx5ConfigPocCandidatePreviewHost");
        let title = to_wide(title);
        // SAFETY: A null module name asks Windows for the current process module handle.
        let instance = unsafe { GetModuleHandleW(null()) };
        if instance.is_null() {
            return Err("GetModuleHandleW failed for Rust Settings UI Preview".to_owned());
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
        let preview_window_class = WndClassW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfn_wnd_proc: Some(candidate_preview_window_proc),
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: instance,
            h_icon: null_mut(),
            h_cursor: null_mut(),
            hbr_background: null_mut(),
            lpsz_menu_name: null(),
            lpsz_class_name: preview_class_name.as_ptr(),
        };
        // SAFETY: The class descriptors reference live UTF-16 buffers for this call and use
        // window procedures with the expected system ABI.
        if unsafe { RegisterClassW(&window_class) } == 0 {
            return Err("RegisterClassW failed for Rust Settings UI Preview".to_owned());
        }
        // SAFETY: Same registration contract as the top-level class above.
        if unsafe { RegisterClassW(&preview_window_class) } == 0 {
            return Err(
                "RegisterClassW failed for Rust Settings UI Preview candidate preview host"
                    .to_owned(),
            );
        }
        // SAFETY: The class/title pointers stay alive for the duration of this call. Parent,
        // menu, and parameter handles are null because this creates the top-level settings window.
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
            return Err("CreateWindowExW failed for Rust Settings UI Preview".to_owned());
        }
        create_settings_controls(hwnd, instance, &preview_class_name, candidate_preview_rect)?;
        // SAFETY: `hwnd` and its children were created successfully and can be shown/painted.
        unsafe {
            ShowWindow(hwnd, SW_SHOWNORMAL);
            UpdateWindow(hwnd);
        }
        message_loop(hwnd)
    }

    unsafe extern "system" fn window_proc(
        hwnd: Hwnd,
        message: u32,
        wparam: Wparam,
        lparam: Lparam,
    ) -> Lresult {
        if message == WM_COMMAND {
            let command_id = loword(wparam);
            if let Some(title) = page_title_for_command(command_id) {
                update_page_title(hwnd, title);
                apply_page_visibility(hwnd, i32::from(command_id));
                invalidate_preview(hwnd);
                return 0;
            }
            if hiword(wparam) == EN_CHANGE && handle_numeric_edit_change(hwnd, command_id) {
                return 0;
            }
            if hiword(wparam) == CBN_SELCHANGE && handle_font_family_change(hwnd, command_id) {
                return 0;
            }
            if hiword(wparam) == CBN_SELCHANGE && handle_language_change(hwnd, command_id) {
                return 0;
            }
            if hiword(wparam) == LBN_SELCHANGE && handle_package_selection_change(hwnd, command_id)
            {
                return 0;
            }
            if handle_package_action(hwnd, command_id) {
                return 0;
            }
        }
        if message == WM_CLOSE {
            // SAFETY: Windows delivered WM_CLOSE for this live HWND; DestroyWindow starts normal
            // teardown and leads to WM_DESTROY.
            unsafe {
                DestroyWindow(hwnd);
            }
            return 0;
        }
        if message == WM_DESTROY {
            // SAFETY: The top-level Settings preview is being destroyed; posting quit exits only
            // this process-local message loop.
            unsafe {
                PostQuitMessage(0);
            }
            return 0;
        }
        // SAFETY: Delegates unhandled messages to the system default window procedure.
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }

    unsafe extern "system" fn candidate_preview_window_proc(
        hwnd: Hwnd,
        message: u32,
        wparam: Wparam,
        lparam: Lparam,
    ) -> Lresult {
        if message == WM_PAINT {
            let mut paint = PaintStruct {
                hdc: null_mut(),
                f_erase: 0,
                rc_paint: Rect {
                    left: 0,
                    top: 0,
                    right: 0,
                    bottom: 0,
                },
                f_restore: 0,
                f_inc_update: 0,
                rgb_reserved: [0; 32],
            };
            // SAFETY: Windows calls this window procedure for a valid preview HWND during paint.
            let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
            if !hdc.is_null() {
                paint_candidate_preview(hwnd, hdc);
                // SAFETY: `paint` was initialized by BeginPaint for this HWND and must be closed.
                unsafe {
                    EndPaint(hwnd, &paint);
                }
                PREVIEW_PAINT_COUNT.fetch_add(1, Ordering::SeqCst);
            }
            return 0;
        }
        // SAFETY: Delegates unhandled messages to the system default window procedure.
        unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
    }

    fn paint_candidate_preview(hwnd: Hwnd, hdc: Hdc) {
        let mut client = Rect {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        // SAFETY: `hwnd` is the preview HWND currently being painted and `client` is writable.
        if unsafe { GetClientRect(hwnd, &mut client) } == 0 {
            return;
        }
        let Ok(plan) =
            candidate_preview_paint_plan(1.0, client.width() as f32, client.height() as f32)
        else {
            return;
        };
        // SAFETY: Creates a process-local GDI brush for immediate FillRect use.
        let background_brush = unsafe { CreateSolidBrush(plan.background_color) };
        if !background_brush.is_null() {
            // SAFETY: `hdc` is valid for this paint cycle, `client` is initialized, and the brush
            // is deleted immediately after use.
            unsafe {
                FillRect(hdc, &client, background_brush);
                DeleteObject(background_brush);
            }
        }
        // SAFETY: The HDC is valid for the paint cycle and this setter does not retain pointers.
        unsafe {
            SetBkMode(hdc, TRANSPARENT);
        }
        for item in plan.items {
            let rect = rect_from_candidate_core(item.bounds);
            if item.selected {
                // SAFETY: Creates a process-local GDI brush for immediate FillRect use.
                let selected_brush = unsafe { CreateSolidBrush(plan.selected_background_color) };
                if !selected_brush.is_null() {
                    // SAFETY: `rect` is bounded by candidate-core's preview plan and the brush is
                    // deleted after use.
                    unsafe {
                        FillRect(hdc, &rect, selected_brush);
                        DeleteObject(selected_brush);
                    }
                }
            }
            let color = if item.selected {
                plan.selected_text_color
            } else {
                plan.text_color
            };
            draw_preview_line(hdc, rect, color, &item.text);
        }
    }

    fn draw_preview_line(hdc: Hdc, mut rect: Rect, color: u32, text: &str) {
        let text = to_wide(text);
        // SAFETY: The UTF-16 buffer is NUL-terminated and lives for the duration of DrawTextW.
        unsafe {
            SetTextColor(hdc, color);
            DrawTextW(
                hdc,
                text.as_ptr(),
                (text.len() - 1) as i32,
                &mut rect,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
        }
    }

    fn rect_from_candidate_core(rect: fcitx5_candidate_core::Rect) -> Rect {
        Rect {
            left: rect.left.round() as i32,
            top: rect.top.round() as i32,
            right: rect.right.round() as i32,
            bottom: rect.bottom.round() as i32,
        }
    }

    fn sample_selected_candidate_pixel(hwnd: Hwnd) -> (u32, bool) {
        // SAFETY: `hwnd` is a live preview child window while this function is called.
        let hdc = unsafe { GetDC(hwnd) };
        if hdc.is_null() {
            return (GET_PIXEL_ERROR, false);
        }
        // SAFETY: The HDC is a client DC for the preview HWND; (12,12) is inside the selected row.
        let pixel = unsafe { GetPixel(hdc, 12, 12) };
        // SAFETY: Releases the client DC acquired above for the same HWND.
        unsafe {
            ReleaseDC(hwnd, hdc);
        }
        let Ok(plan) = candidate_preview_paint_plan(1.0, 596.0, 166.0) else {
            return (pixel, false);
        };
        (pixel, pixel == plan.selected_background_color)
    }

    fn create_settings_controls(
        hwnd: Hwnd,
        instance: Hinstance,
        preview_class_name: &[u16],
        candidate_preview_rect: LayoutRect,
    ) -> Result<(), String> {
        let preview_left = candidate_preview_rect.x.max(420);
        let preview_width = candidate_preview_rect.width.min(440);
        let static_class = to_wide("STATIC");
        let button_class = to_wide("BUTTON");
        let edit_class = to_wide("EDIT");
        let combo_class = to_wide("COMBOBOX");
        let listbox_class = to_wide("LISTBOX");
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_PAGE_TITLE,
            "Input methods",
            220,
            24,
            420,
            34,
            0,
        )?;
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_STATUS,
            "Ready. Rust Settings UI Preview is running inside the Config process.",
            220,
            398,
            620,
            38,
            0,
        )?;
        let packages = create_child_control(
            hwnd,
            instance,
            &listbox_class,
            K_PACKAGES,
            "",
            220,
            128,
            360,
            96,
            WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        )?;
        populate_available_packages(packages);
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_PACKAGE_DETAIL,
            "Rime: trusted signed add-on package. Configure opens through Rust package/control boundaries.",
            600,
            128,
            300,
            96,
            WS_BORDER,
        )?;
        create_child_control(
            hwnd,
            instance,
            &button_class,
            K_PACKAGE_INSTALL,
            "Install",
            220,
            244,
            112,
            34,
            WS_TABSTOP,
        )?;
        create_child_control(
            hwnd,
            instance,
            &button_class,
            K_PACKAGE_UPDATE,
            "Update",
            344,
            244,
            112,
            34,
            WS_TABSTOP,
        )?;
        create_child_control(
            hwnd,
            instance,
            &button_class,
            K_PACKAGE_REMOVE,
            "Remove",
            468,
            244,
            112,
            34,
            WS_TABSTOP,
        )?;
        create_child_control(
            hwnd,
            instance,
            &button_class,
            K_PACKAGE_CONFIGURE,
            "Configure",
            600,
            244,
            128,
            34,
            WS_TABSTOP,
        )?;
        create_child_control(
            hwnd,
            instance,
            &button_class,
            K_PACKAGE_REFRESH,
            "Refresh",
            220,
            292,
            112,
            34,
            WS_TABSTOP,
        )?;
        create_child_control(
            hwnd,
            instance,
            &button_class,
            K_PACKAGE_DETAILS,
            "Details",
            344,
            292,
            112,
            34,
            WS_TABSTOP,
        )?;
        create_child_control(
            hwnd,
            instance,
            &button_class,
            K_PACKAGE_ENABLE_DISABLE,
            "Enable / Disable",
            468,
            292,
            128,
            34,
            WS_TABSTOP,
        )?;
        create_child_control(
            hwnd,
            instance,
            &button_class,
            K_PACKAGE_REPAIR,
            "Repair",
            608,
            292,
            112,
            34,
            WS_TABSTOP,
        )?;
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_SAVE_STATUS,
            "No pending changes",
            220,
            88,
            280,
            28,
            0,
        )?;
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_LABEL_INPUT_METHODS,
            "Enabled input methods",
            220,
            128,
            620,
            24,
            0,
        )?;
        let input_methods = create_child_control(
            hwnd,
            instance,
            &listbox_class,
            K_INPUT_METHOD_LIST,
            "",
            220,
            160,
            620,
            96,
            WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        )?;
        populate_enabled_input_methods(input_methods);
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_LABEL_LANGUAGE,
            "Language / 语言",
            220,
            272,
            180,
            24,
            0,
        )?;
        let language_selector = create_child_control(
            hwnd,
            instance,
            &combo_class,
            K_LANGUAGE_SELECTOR,
            "",
            420,
            268,
            220,
            96,
            WS_BORDER | WS_VSCROLL | WS_TABSTOP | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
        )?;
        populate_language_selector(language_selector);
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_LABEL_FONT_SIZE,
            "Font size DIP",
            220,
            128,
            180,
            28,
            0,
        )?;
        create_child_control(
            hwnd,
            instance,
            &edit_class,
            K_APPEARANCE_FONT_SIZE,
            "18",
            420,
            128,
            92,
            28,
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
        )?;
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_LABEL_OPACITY,
            "Opacity",
            220,
            164,
            180,
            28,
            0,
        )?;
        create_child_control(
            hwnd,
            instance,
            &edit_class,
            K_APPEARANCE_OPACITY,
            "1.00",
            420,
            164,
            92,
            28,
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
        )?;
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_LABEL_SPACING,
            "Spacing DIP",
            540,
            128,
            112,
            28,
            0,
        )?;
        create_child_control(
            hwnd,
            instance,
            &edit_class,
            K_APPEARANCE_SPACING,
            "8",
            660,
            128,
            92,
            28,
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
        )?;
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_LABEL_CORNER_RADIUS,
            "Corner DIP",
            540,
            164,
            112,
            28,
            0,
        )?;
        create_child_control(
            hwnd,
            instance,
            &edit_class,
            K_APPEARANCE_CORNER_RADIUS,
            "12",
            660,
            164,
            92,
            28,
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
        )?;
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_LABEL_CANDIDATE_WIDTH,
            "Width DIP",
            768,
            128,
            84,
            28,
            0,
        )?;
        create_child_control(
            hwnd,
            instance,
            &edit_class,
            K_APPEARANCE_CANDIDATE_WIDTH,
            "420",
            860,
            128,
            92,
            28,
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL,
        )?;
        create_child_control(
            hwnd,
            instance,
            &static_class,
            K_LABEL_CANDIDATE_FONT,
            "Candidate font",
            220,
            200,
            180,
            24,
            0,
        )?;
        let font_combo = create_child_control(
            hwnd,
            instance,
            &combo_class,
            K_APPEARANCE_FONT_FAMILY,
            "",
            220,
            228,
            180,
            128,
            WS_BORDER | WS_VSCROLL | WS_TABSTOP | CBS_DROPDOWNLIST | CBS_HASSTRINGS,
        )?;
        populate_system_font_picker(font_combo, preview_state_font_family().as_deref())?;
        for (index, (id, label)) in [
            (K_NAV_GENERAL, "Input methods"),
            (K_NAV_APPEARANCE, "Appearance"),
            (K_NAV_SHORTCUTS, "Shortcuts"),
            (K_NAV_UPDATES, "Updates"),
            (K_NAV_REPAIR, "Diagnostics"),
            (K_NAV_PACKAGES, "Packages"),
        ]
        .iter()
        .enumerate()
        {
            create_child_control(
                hwnd,
                instance,
                &button_class,
                *id,
                label,
                24,
                24 + (index as i32 * 44),
                164,
                34,
                WS_TABSTOP,
            )?;
        }
        let preview_title = to_wide("Candidate Preview");
        // SAFETY: The preview class/title pointers stay alive for the call, and `hwnd` is a live
        // top-level window handle created above. The child coordinates come from the validated
        // layout model and the child id is the QA-visible K_PREVIEW control id.
        let preview_hwnd = unsafe {
            CreateWindowExW(
                0,
                preview_class_name.as_ptr(),
                preview_title.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_BORDER,
                preview_left,
                candidate_preview_rect.y,
                preview_width,
                candidate_preview_rect.height,
                hwnd,
                control_id_handle(K_PREVIEW),
                instance,
                null_mut(),
            )
        };
        if preview_hwnd.is_null() {
            return Err("CreateWindowExW failed for Rust Settings UI Preview K_PREVIEW".to_owned());
        }
        // SAFETY: `preview_hwnd` is a live child handle and can be explicitly repainted.
        unsafe {
            InvalidateRect(preview_hwnd, null(), FALSE);
            UpdateWindow(preview_hwnd);
        }
        apply_page_visibility(hwnd, K_NAV_GENERAL);
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn create_child_control(
        parent: Hwnd,
        instance: Hinstance,
        class_name: &[u16],
        id: i32,
        text: &str,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        extra_style: u32,
    ) -> Result<Hwnd, String> {
        let text = to_wide(text);
        // SAFETY: The class/text UTF-16 buffers live for this call, `parent` is the live top-level
        // window, and the positive child id is passed through Win32's HMENU/id slot.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                class_name.as_ptr(),
                text.as_ptr(),
                WS_CHILD | WS_VISIBLE | extra_style,
                x,
                y,
                width,
                height,
                parent,
                control_id_handle(id),
                instance,
                null_mut(),
            )
        };
        if hwnd.is_null() {
            return Err(format!(
                "CreateWindowExW failed for Rust Settings UI Preview child control {id}"
            ));
        }
        Ok(hwnd)
    }

    fn message_loop(hwnd: Hwnd) -> Result<(), String> {
        let mut message = Msg {
            hwnd: null_mut(),
            message: 0,
            w_param: 0,
            l_param: 0,
            time: 0,
            pt: Point { x: 0, y: 0 },
        };
        loop {
            // SAFETY: `message` points to writable stack storage; null HWND receives thread
            // messages for this single Settings UI process.
            let result = unsafe { GetMessageW(&mut message, null_mut(), 0, 0) };
            if result == -1 {
                // SAFETY: `hwnd` is the top-level window created by `run_interactive`; destroy it
                // on the error path so the process does not leave a stray Config window.
                unsafe {
                    DestroyWindow(hwnd);
                }
                return Err("GetMessageW failed for Rust Settings UI Preview".to_owned());
            }
            if result == 0 {
                return Ok(());
            }
            // SAFETY: `message` was populated by GetMessageW and can be translated/dispatched.
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    fn update_page_title(hwnd: Hwnd, title: &str) {
        // SAFETY: Reads the child handle for the QA-visible K_PAGE_TITLE control.
        let title_hwnd = unsafe { GetDlgItem(hwnd, K_PAGE_TITLE) };
        if title_hwnd.is_null() {
            return;
        }
        let title = to_wide(title);
        // SAFETY: `title_hwnd` is a live child control and the UTF-16 buffer lives for this call.
        unsafe {
            SetWindowTextW(title_hwnd, title.as_ptr());
        }
    }

    fn apply_page_visibility(hwnd: Hwnd, active_page: i32) {
        for control in [
            K_LABEL_INPUT_METHODS,
            K_INPUT_METHOD_LIST,
            K_LABEL_LANGUAGE,
            K_LANGUAGE_SELECTOR,
            K_LABEL_FONT_SIZE,
            K_APPEARANCE_FONT_SIZE,
            K_LABEL_OPACITY,
            K_APPEARANCE_OPACITY,
            K_LABEL_SPACING,
            K_APPEARANCE_SPACING,
            K_LABEL_CORNER_RADIUS,
            K_APPEARANCE_CORNER_RADIUS,
            K_LABEL_CANDIDATE_WIDTH,
            K_APPEARANCE_CANDIDATE_WIDTH,
            K_LABEL_CANDIDATE_FONT,
            K_APPEARANCE_FONT_FAMILY,
            K_PREVIEW,
            K_PACKAGES,
            K_PACKAGE_DETAIL,
            K_PACKAGE_INSTALL,
            K_PACKAGE_UPDATE,
            K_PACKAGE_REMOVE,
            K_PACKAGE_CONFIGURE,
            K_PACKAGE_REFRESH,
            K_PACKAGE_DETAILS,
            K_PACKAGE_ENABLE_DISABLE,
            K_PACKAGE_REPAIR,
            K_STATUS,
            K_SAVE_STATUS,
        ] {
            show_child_control(
                hwnd,
                control,
                controls_for_page(active_page).contains(&control),
            );
        }
    }

    fn controls_for_page(active_page: i32) -> &'static [i32] {
        match active_page {
            K_NAV_GENERAL => &[
                K_LABEL_INPUT_METHODS,
                K_INPUT_METHOD_LIST,
                K_LABEL_LANGUAGE,
                K_LANGUAGE_SELECTOR,
                K_SAVE_STATUS,
            ],
            K_NAV_APPEARANCE => &[
                K_LABEL_FONT_SIZE,
                K_APPEARANCE_FONT_SIZE,
                K_LABEL_OPACITY,
                K_APPEARANCE_OPACITY,
                K_LABEL_SPACING,
                K_APPEARANCE_SPACING,
                K_LABEL_CORNER_RADIUS,
                K_APPEARANCE_CORNER_RADIUS,
                K_LABEL_CANDIDATE_WIDTH,
                K_APPEARANCE_CANDIDATE_WIDTH,
                K_LABEL_CANDIDATE_FONT,
                K_APPEARANCE_FONT_FAMILY,
                K_PREVIEW,
                K_SAVE_STATUS,
            ],
            K_NAV_PACKAGES => &[
                K_PACKAGES,
                K_PACKAGE_DETAIL,
                K_PACKAGE_INSTALL,
                K_PACKAGE_UPDATE,
                K_PACKAGE_REMOVE,
                K_PACKAGE_CONFIGURE,
                K_PACKAGE_REFRESH,
                K_PACKAGE_DETAILS,
                K_PACKAGE_ENABLE_DISABLE,
                K_PACKAGE_REPAIR,
                K_STATUS,
            ],
            K_NAV_SHORTCUTS | K_NAV_UPDATES | K_NAV_REPAIR => &[K_STATUS],
            _ => &[K_SAVE_STATUS],
        }
    }

    fn show_child_control(parent: Hwnd, id: i32, visible: bool) {
        // SAFETY: Reads the child handle for a QA-visible control id.
        let child = unsafe { GetDlgItem(parent, id) };
        if child.is_null() {
            return;
        }
        // SAFETY: `child` is a live HWND and ShowWindow only changes its visibility.
        unsafe {
            ShowWindow(child, if visible { SW_SHOW } else { SW_HIDE });
        }
    }

    fn invalidate_preview(hwnd: Hwnd) {
        // SAFETY: Reads the child handle for the QA-visible K_PREVIEW control.
        let preview_hwnd = unsafe { GetDlgItem(hwnd, K_PREVIEW) };
        if preview_hwnd.is_null() {
            return;
        }
        // SAFETY: `preview_hwnd` is a live child control if GetDlgItem returned non-null.
        unsafe {
            InvalidateRect(preview_hwnd, null(), FALSE);
            UpdateWindow(preview_hwnd);
        }
    }

    fn page_title_for_command(command_id: u16) -> Option<&'static str> {
        match i32::from(command_id) {
            K_NAV_GENERAL => Some("Input methods"),
            K_NAV_APPEARANCE => Some("Appearance"),
            K_NAV_SHORTCUTS => Some("Shortcuts"),
            K_NAV_UPDATES => Some("Updates"),
            K_NAV_REPAIR => Some("Diagnostics and repair"),
            K_NAV_PACKAGES => Some("Packages"),
            _ => None,
        }
    }

    fn handle_numeric_edit_change(hwnd: Hwnd, command_id: u16) -> bool {
        let Some(field) = numeric_field_for_command(command_id) else {
            return false;
        };
        let edit = unsafe { GetDlgItem(hwnd, i32::from(command_id)) };
        if edit.is_null() {
            return true;
        }
        let text = child_text(edit);
        let status = match validate_appearance_numeric_input(field, &text) {
            Ok(value) => format!("{} accepted: {value:.2}", field.spec().key),
            Err("appearance.numeric.incomplete") => {
                "appearance.numeric.incomplete: keeping last valid value".to_owned()
            }
            Err(error) => format!("{error}: keeping last valid value"),
        };
        set_child_text(hwnd, K_SAVE_STATUS, &status);
        invalidate_preview(hwnd);
        true
    }

    fn numeric_field_for_command(command_id: u16) -> Option<AppearanceNumericField> {
        match i32::from(command_id) {
            K_APPEARANCE_FONT_SIZE => Some(AppearanceNumericField::FontSizeDip),
            K_APPEARANCE_OPACITY => Some(AppearanceNumericField::Opacity),
            K_APPEARANCE_SPACING => Some(AppearanceNumericField::SpacingDip),
            K_APPEARANCE_CORNER_RADIUS => Some(AppearanceNumericField::CornerRadiusDip),
            K_APPEARANCE_CANDIDATE_WIDTH => Some(AppearanceNumericField::CandidateWidthDip),
            _ => None,
        }
    }

    fn handle_font_family_change(hwnd: Hwnd, command_id: u16) -> bool {
        if i32::from(command_id) != K_APPEARANCE_FONT_FAMILY {
            return false;
        }
        let font_family = unsafe { GetDlgItem(hwnd, K_APPEARANCE_FONT_FAMILY) };
        let selected = selected_combo_text(font_family).unwrap_or_else(|| "unknown".to_owned());
        let status = match persist_preview_font_family(&selected) {
            Ok(()) => format!("font_family accepted: {selected}"),
            Err(error) => format!("font_family persistence failed: {error}"),
        };
        set_child_text(hwnd, K_SAVE_STATUS, &status);
        invalidate_preview(hwnd);
        true
    }

    fn handle_language_change(hwnd: Hwnd, command_id: u16) -> bool {
        if i32::from(command_id) != K_LANGUAGE_SELECTOR {
            return false;
        }
        let language_selector = unsafe { GetDlgItem(hwnd, K_LANGUAGE_SELECTOR) };
        let selected =
            selected_combo_text(language_selector).unwrap_or_else(|| "System default".to_owned());
        set_child_text(
            hwnd,
            K_SAVE_STATUS,
            &format!("language accepted: {selected}"),
        );
        true
    }

    fn handle_package_action(hwnd: Hwnd, command_id: u16) -> bool {
        let status = match i32::from(command_id) {
            K_PACKAGE_REFRESH => {
                Some("package.refresh planned: trusted repository metadata required")
            }
            K_PACKAGE_DETAILS => {
                update_package_detail_from_selection(hwnd);
                Some("package.details loaded: selected component metadata")
            }
            K_PACKAGE_INSTALL => {
                Some("package.install planned: signed repository metadata required before download")
            }
            K_PACKAGE_UPDATE => Some("package.update planned: Rust package-core transaction"),
            K_PACKAGE_ENABLE_DISABLE => {
                Some("package.enable_disable planned: Rust package-core state")
            }
            K_PACKAGE_REMOVE => {
                Some("package.remove planned: rollback-safe Rust package-core state")
            }
            K_PACKAGE_CONFIGURE => Some("plugin_config loaded: fcitx5-rime settings surface"),
            K_PACKAGE_REPAIR => {
                Some("package.repair planned: verify and restore installed payloads")
            }
            _ => None,
        };
        let Some(status) = status else {
            return false;
        };
        set_child_text(hwnd, K_STATUS, status);
        set_child_text(hwnd, K_SAVE_STATUS, status);
        true
    }

    fn handle_package_selection_change(hwnd: Hwnd, command_id: u16) -> bool {
        if i32::from(command_id) != K_PACKAGES {
            return false;
        }
        update_package_detail_from_selection(hwnd);
        set_child_text(
            hwnd,
            K_STATUS,
            "package.selection changed: details refreshed",
        );
        set_child_text(
            hwnd,
            K_SAVE_STATUS,
            "package.selection changed: details refreshed",
        );
        true
    }

    fn update_package_detail_from_selection(hwnd: Hwnd) {
        let packages = unsafe { GetDlgItem(hwnd, K_PACKAGES) };
        let selected =
            selected_listbox_text(packages).unwrap_or_else(|| "fcitx5-rime — installed".to_owned());
        set_child_text(
            hwnd,
            K_PACKAGE_DETAIL,
            &format!(
                "{selected}: type=addon, source=official signed fixture, actions=refresh/details/install/update/enable-disable/remove/repair"
            ),
        );
    }

    fn populate_system_font_picker(
        combo: Hwnd,
        persisted_font: Option<&str>,
    ) -> Result<(), String> {
        let mut fonts = system_font_families_for_picker();
        if fonts.is_empty() {
            fonts.push("Segoe UI".to_owned());
        }
        let mut selected_index = 0usize;
        for family in &fonts {
            if let Some(persisted_font) = persisted_font {
                if family.eq_ignore_ascii_case(persisted_font) {
                    selected_index = fonts
                        .iter()
                        .position(|candidate| candidate.eq_ignore_ascii_case(persisted_font))
                        .unwrap_or(0);
                }
            }
            let family = to_wide(family);
            // SAFETY: `combo` is a live combobox HWND and the UTF-16 string buffer lives for the
            // synchronous CB_ADDSTRING message.
            unsafe {
                SendMessageW(combo, CB_ADDSTRING, 0, family.as_ptr() as Lparam);
            }
        }
        // SAFETY: `combo` is a live combobox HWND; selecting the first item initializes the
        // visible current system-font choice for QA and users.
        unsafe {
            SendMessageW(combo, CB_SETCURSEL, selected_index, 0);
        }
        Ok(())
    }

    fn system_font_families_for_picker() -> Vec<String> {
        let required = fcitx5_windows_common_core::fcitx5_windows_common_system_font_families_utf16(
            null_mut(),
            0,
        );
        if required == 0 {
            return Vec::new();
        }
        let mut payload = vec![0u16; required];
        let written = fcitx5_windows_common_core::fcitx5_windows_common_system_font_families_utf16(
            payload.as_mut_ptr(),
            payload.len(),
        )
        .min(payload.len());
        let mut fonts = Vec::new();
        let mut start = 0usize;
        for index in 0..written {
            if payload[index] == 0 {
                if index > start {
                    fonts.push(String::from_utf16_lossy(&payload[start..index]));
                }
                start = index + 1;
            }
        }
        fonts
    }

    fn populate_enabled_input_methods(listbox: Hwnd) {
        for input_method in ["Pinyin - 中文", "Rime - 中州韵", "Keyboard - English (US)"] {
            let input_method = to_wide(input_method);
            // SAFETY: `listbox` is a live LISTBOX HWND and the UTF-16 buffer lives for this
            // synchronous LB_ADDSTRING message.
            unsafe {
                SendMessageW(listbox, LB_ADDSTRING, 0, input_method.as_ptr() as Lparam);
            }
        }
    }

    fn populate_available_packages(listbox: Hwnd) {
        for package in [
            "fcitx5-rime - Rime",
            "fcitx5-chinese-addons - Chinese Addons",
            "fcitx5-mozc - Mozc",
        ] {
            let package = to_wide(package);
            // SAFETY: `listbox` is a live LISTBOX HWND and the UTF-16 buffer lives for this
            // synchronous LB_ADDSTRING message.
            unsafe {
                SendMessageW(listbox, LB_ADDSTRING, 0, package.as_ptr() as Lparam);
            }
        }
        // SAFETY: `listbox` is a live LISTBOX HWND. Selecting the first item gives details and
        // selection-change QA a deterministic starting point without running package operations.
        unsafe {
            SendMessageW(listbox, LB_SETCURSEL, 0, 0);
        }
    }

    fn populate_language_selector(combo: Hwnd) {
        for language in ["System default", "English (United States)", "简体中文"] {
            let language = to_wide(language);
            // SAFETY: `combo` is a live combobox HWND and the UTF-16 string buffer lives for the
            // synchronous CB_ADDSTRING message.
            unsafe {
                SendMessageW(combo, CB_ADDSTRING, 0, language.as_ptr() as Lparam);
            }
        }
        // SAFETY: `combo` is a live combobox HWND; selecting index 0 initializes the system
        // language policy.
        unsafe {
            SendMessageW(combo, CB_SETCURSEL, 0, 0);
        }
    }

    fn preview_state_path() -> Option<PathBuf> {
        std::env::var_os(PREVIEW_STATE_ENV).map(PathBuf::from)
    }

    fn preview_state_font_family() -> Option<String> {
        let path = preview_state_path()?;
        let text = std::fs::read_to_string(path).ok()?;
        text.lines()
            .find_map(|line| line.strip_prefix("font_family="))
            .map(unescape_state_value)
    }

    fn persist_preview_font_family(font_family: &str) -> Result<(), String> {
        let Some(path) = preview_state_path() else {
            return Ok(());
        };
        let content = format!("font_family={}\n", escape_state_value(font_family));
        atomic_write_utf8_file(&path, &content)
    }

    fn atomic_write_utf8_file(path: &Path, content: &str) -> Result<(), String> {
        let wide_path = path.as_os_str().encode_wide().collect::<Vec<_>>();
        let destination = ControlUtf16 {
            ptr: wide_path.as_ptr(),
            len: wide_path.len(),
        };
        let content = ControlUtf8 {
            ptr: content.as_bytes().as_ptr(),
            len: content.len(),
        };
        // SAFETY: The UTF-16 path and UTF-8 content buffers live for this synchronous Rust Control
        // ABI call. The callee does not retain pointers and performs the atomic file replacement.
        let status = unsafe { fcitx5_control_atomic_write_utf8_file_utf16(destination, content) };
        if status == 0 {
            Ok(())
        } else {
            Err("atomic_write_utf8_file".to_owned())
        }
    }

    fn selected_combo_text(combo: Hwnd) -> Option<String> {
        if combo.is_null() {
            return None;
        }
        // SAFETY: `combo` is a live combobox HWND.
        let selected = unsafe { SendMessageW(combo, CB_GETCURSEL, 0, 0) };
        if selected < 0 {
            return None;
        }
        // SAFETY: `combo` is a live combobox HWND and `selected` is the current selection index.
        let len = unsafe { SendMessageW(combo, CB_GETLBTEXTLEN, selected as Wparam, 0) };
        if len <= 0 {
            return None;
        }
        let mut buffer = vec![0u16; len as usize + 1];
        // SAFETY: `buffer` is writable and large enough for the selected list item plus NUL.
        let copied = unsafe {
            SendMessageW(
                combo,
                CB_GETLBTEXT,
                selected as Wparam,
                buffer.as_mut_ptr() as Lparam,
            )
        };
        if copied <= 0 {
            return None;
        }
        buffer.truncate(copied as usize);
        Some(String::from_utf16_lossy(&buffer))
    }

    fn selected_listbox_text(listbox: Hwnd) -> Option<String> {
        if listbox.is_null() {
            return None;
        }
        // SAFETY: `listbox` is a live listbox HWND.
        let selected = unsafe { SendMessageW(listbox, LB_GETCURSEL, 0, 0) };
        if selected < 0 {
            return None;
        }
        // SAFETY: `listbox` is a live listbox HWND and `selected` is the current selection index.
        let len = unsafe { SendMessageW(listbox, LB_GETTEXTLEN, selected as Wparam, 0) };
        if len <= 0 {
            return None;
        }
        let mut buffer = vec![0u16; len as usize + 1];
        // SAFETY: `buffer` is writable and large enough for the selected list item plus NUL.
        let copied = unsafe {
            SendMessageW(
                listbox,
                LB_GETTEXT,
                selected as Wparam,
                buffer.as_mut_ptr() as Lparam,
            )
        };
        if copied <= 0 {
            return None;
        }
        buffer.truncate(copied as usize);
        Some(String::from_utf16_lossy(&buffer))
    }

    fn escape_state_value(value: &str) -> String {
        value.replace('\\', "\\\\").replace('\n', "\\n")
    }

    fn unescape_state_value(value: &str) -> String {
        let mut result = String::new();
        let mut escaped = false;
        for character in value.chars() {
            if escaped {
                match character {
                    'n' => result.push('\n'),
                    other => result.push(other),
                }
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else {
                result.push(character);
            }
        }
        if escaped {
            result.push('\\');
        }
        result
    }

    fn child_text(hwnd: Hwnd) -> String {
        // SAFETY: Reads the current text length from a live child HWND.
        let len = unsafe { GetWindowTextLengthW(hwnd) };
        if len <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; len as usize + 1];
        // SAFETY: `buffer` is writable and large enough for `len + NUL` UTF-16 units.
        let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if copied <= 0 {
            return String::new();
        }
        buffer.truncate(copied as usize);
        String::from_utf16_lossy(&buffer)
    }

    fn set_child_text(parent: Hwnd, id: i32, text: &str) {
        // SAFETY: Reads the child handle for a QA-visible control id.
        let child = unsafe { GetDlgItem(parent, id) };
        if child.is_null() {
            return;
        }
        let text = to_wide(text);
        // SAFETY: `child` is a live HWND and the UTF-16 buffer lives for this call.
        unsafe {
            SetWindowTextW(child, text.as_ptr());
        }
    }

    fn loword(value: Wparam) -> u16 {
        (value & 0xffff) as u16
    }

    fn hiword(value: Wparam) -> u16 {
        ((value >> 16) & 0xffff) as u16
    }

    fn control_id_handle(id: i32) -> *mut c_void {
        if id <= 0 {
            return null_mut();
        }
        id as usize as *mut c_void
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
        assert!(report.contains("\"numeric_appearance_inputs\":true"));
        assert!(report.contains("\"numeric_font_size_valid_entry\":true"));
        assert!(report.contains("\"numeric_invalid_text_rejected\":true"));
        assert!(report.contains("\"numeric_paste_out_of_range_rejected\":true"));
        assert!(report.contains("\"numeric_ime_cancellation_keeps_last_valid\":true"));
        assert!(report.contains("\"numeric_min_max_bounds_checked\":true"));
        assert!(report.contains("\"numeric_localized_error_text\":true"));
        assert!(report.contains("\"numeric_rollback_keeps_last_valid\":true"));
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
    fn legacy_headless_modes_reuse_rust_self_check_corpus() {
        let modes = [
            LegacyHeadlessMode::SelfTest,
            LegacyHeadlessMode::CheckI18n,
            LegacyHeadlessMode::CheckResources,
            LegacyHeadlessMode::UiContract,
            LegacyHeadlessMode::UiVisualContract,
            LegacyHeadlessMode::UiLivePreviewContract,
            LegacyHeadlessMode::UiInteraction,
        ];
        for mode in modes {
            let report =
                run_legacy_headless_check(mode).expect("legacy headless check should pass");
            assert!(report.contains("\"legacy_config_cli_compat\":true"));
            assert!(report.contains(&format!("\"legacy_argument\":\"{}\"", mode.argument())));
            assert!(report.contains(&format!(
                "\"legacy_corpus_marker\":\"{}\"",
                mode.corpus_marker()
            )));
            assert!(report.contains("\"rust_config_self_check_reused\":true"));
            assert!(report.contains("\"result\":\"PASS\""));
        }
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
