#![forbid(unsafe_code)]

//! Typed Config state, transaction, and recovery contract.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

const CONFIG_FORMAT_VERSION: u32 = 1;
const MAX_CONFIG_BYTES: u64 = 256 * 1024;
const COMPILED_DEFAULTS: &str = include_str!("../../../resources/config.toml");
static STAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A typed, fully resolved configuration snapshot suitable for rendering.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigSnapshot {
    format_version: u32,
    ui: UiConfig,
    appearance: AppearanceConfig,
    candidate: CandidateConfig,
    fonts: FontsConfig,
}

impl ConfigSnapshot {
    /// Returns the configured appearance settings.
    #[must_use]
    pub fn appearance(&self) -> &AppearanceConfig {
        &self.appearance
    }

    /// Returns the configured candidate presentation.
    #[must_use]
    pub fn candidate(&self) -> &CandidateConfig {
        &self.candidate
    }

    /// Returns the configured font settings.
    #[must_use]
    pub fn fonts(&self) -> &FontsConfig {
        &self.fonts
    }
}

/// Resolved UI settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiConfig {
    language: String,
}

/// Resolved appearance settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceConfig {
    mode: String,
    theme: String,
}

impl AppearanceConfig {
    /// Returns the selected appearance mode.
    #[must_use]
    pub fn mode(&self) -> &str {
        &self.mode
    }

    /// Returns the selected theme ID.
    #[must_use]
    pub fn theme(&self) -> &str {
        &self.theme
    }
}

/// Resolved candidate settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateConfig {
    orientation: String,
    page_size: u8,
    scroll_mode: bool,
    max_width_dip: f32,
    scroll_cell_width_dip: f32,
    opacity: f32,
    preedit_mode: String,
    geometry: CandidateGeometry,
    label: CandidateLabel,
    #[serde(default)]
    colors: BTreeMap<String, String>,
}

impl CandidateConfig {
    /// Returns the configured candidate orientation.
    #[must_use]
    pub fn orientation(&self) -> &str {
        &self.orientation
    }

    /// Returns the resolved candidate page size.
    #[must_use]
    pub fn page_size(&self) -> u8 {
        self.page_size
    }

    /// Returns whether the resolved candidate layout uses scroll presentation.
    #[must_use]
    pub fn scroll_mode(&self) -> bool {
        self.scroll_mode
    }

    /// Returns the configured scroll cell width in DIP.
    #[must_use]
    pub fn scroll_cell_width_dip(&self) -> f32 {
        self.scroll_cell_width_dip
    }

    /// Returns the configured preedit presentation mode.
    #[must_use]
    pub fn preedit_mode(&self) -> &str {
        &self.preedit_mode
    }
}

/// Resolved candidate geometry settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateGeometry {
    padding_x_dip: f32,
    padding_y_dip: f32,
    item_padding_x_dip: f32,
    item_padding_y_dip: f32,
    row_gap_dip: f32,
    column_gap_dip: f32,
    border_width_dip: f32,
    corner_radius_dip: f32,
    shadow: bool,
}

/// Resolved candidate label settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateLabel {
    visible: bool,
    style: String,
    sequence: Vec<String>,
    font_scale: f32,
    gap_dip: f32,
}

/// Resolved font settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontsConfig {
    ui: FontFamilies,
    candidate: CandidateFont,
    annotation: AnnotationFont,
    monospace: FontFamilies,
}

impl FontsConfig {
    /// Returns the candidate font settings.
    #[must_use]
    pub fn candidate(&self) -> &CandidateFont {
        &self.candidate
    }
}

/// A font family fallback list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontFamilies {
    families: Vec<String>,
}

/// Candidate font settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateFont {
    families: Vec<String>,
    size_dip: f32,
    weight: u16,
}

impl CandidateFont {
    /// Returns candidate font fallbacks in priority order.
    #[must_use]
    pub fn families(&self) -> &[String] {
        &self.families
    }

    /// Returns the candidate font size in DIP.
    #[must_use]
    pub fn size_dip(&self) -> f32 {
        self.size_dip
    }
}

/// Annotation font settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnnotationFont {
    families: Vec<String>,
    scale: f32,
}

/// Sparse user overrides persisted in `config.toml`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfigOverrides {
    format_version: u32,
    #[serde(default, skip_serializing_if = "UiOverrides::is_empty")]
    ui: UiOverrides,
    #[serde(default, skip_serializing_if = "AppearanceOverrides::is_empty")]
    appearance: AppearanceOverrides,
    #[serde(default, skip_serializing_if = "CandidateOverrides::is_empty")]
    candidate: CandidateOverrides,
    #[serde(default, skip_serializing_if = "FontsOverrides::is_empty")]
    fonts: FontsOverrides,
    /// Fcitx-owned and forward-compatible TOML is retained verbatim by the
    /// Windows Config transaction without becoming a second semantic owner.
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    passthrough: BTreeMap<String, toml::Value>,
}

impl ConfigOverrides {
    fn empty() -> Self {
        Self {
            format_version: CONFIG_FORMAT_VERSION,
            ..Self::default()
        }
    }

    fn clear_known_overrides(&mut self) {
        self.format_version = CONFIG_FORMAT_VERSION;
        self.ui.language = None;
        self.appearance.mode = None;
        self.appearance.theme = None;
        self.candidate.orientation = None;
        self.candidate.page_size = None;
        self.candidate.scroll_mode = None;
        self.candidate.max_width_dip = None;
        self.candidate.scroll_cell_width_dip = None;
        self.candidate.opacity = None;
        self.candidate.preedit_mode = None;
        self.candidate.geometry.clear_known_overrides();
        self.candidate.label.clear_known_overrides();
        self.candidate.colors.clear();
        self.fonts.ui.families = None;
        self.fonts.candidate.families = None;
        self.fonts.candidate.size_dip = None;
        self.fonts.candidate.weight = None;
        self.fonts.annotation.families = None;
        self.fonts.annotation.scale = None;
        self.fonts.monospace.families = None;
    }
}

/// Sparse UI overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UiOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    passthrough: BTreeMap<String, toml::Value>,
}

impl UiOverrides {
    fn is_empty(&self) -> bool {
        self.language.is_none() && self.passthrough.is_empty()
    }
}

/// Sparse appearance overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AppearanceOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    theme: Option<String>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    passthrough: BTreeMap<String, toml::Value>,
}

impl AppearanceOverrides {
    fn is_empty(&self) -> bool {
        self.mode.is_none() && self.theme.is_none() && self.passthrough.is_empty()
    }
}

/// Sparse candidate overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CandidateOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    orientation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_size: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scroll_mode: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_width_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scroll_cell_width_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    opacity: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preedit_mode: Option<String>,
    #[serde(default, skip_serializing_if = "CandidateGeometryOverrides::is_empty")]
    geometry: CandidateGeometryOverrides,
    #[serde(default, skip_serializing_if = "CandidateLabelOverrides::is_empty")]
    label: CandidateLabelOverrides,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    colors: BTreeMap<String, String>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    passthrough: BTreeMap<String, toml::Value>,
}

impl CandidateOverrides {
    fn is_empty(&self) -> bool {
        self.orientation.is_none()
            && self.page_size.is_none()
            && self.scroll_mode.is_none()
            && self.max_width_dip.is_none()
            && self.scroll_cell_width_dip.is_none()
            && self.opacity.is_none()
            && self.preedit_mode.is_none()
            && self.geometry.is_empty()
            && self.label.is_empty()
            && self.colors.is_empty()
            && self.passthrough.is_empty()
    }
}

/// Sparse candidate geometry overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CandidateGeometryOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    padding_x_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    padding_y_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    item_padding_x_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    item_padding_y_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    row_gap_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    column_gap_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    border_width_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    corner_radius_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shadow: Option<bool>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    passthrough: BTreeMap<String, toml::Value>,
}

impl CandidateGeometryOverrides {
    fn is_empty(&self) -> bool {
        self.padding_x_dip.is_none()
            && self.padding_y_dip.is_none()
            && self.item_padding_x_dip.is_none()
            && self.item_padding_y_dip.is_none()
            && self.row_gap_dip.is_none()
            && self.column_gap_dip.is_none()
            && self.border_width_dip.is_none()
            && self.corner_radius_dip.is_none()
            && self.shadow.is_none()
            && self.passthrough.is_empty()
    }

    fn clear_known_overrides(&mut self) {
        self.padding_x_dip = None;
        self.padding_y_dip = None;
        self.item_padding_x_dip = None;
        self.item_padding_y_dip = None;
        self.row_gap_dip = None;
        self.column_gap_dip = None;
        self.border_width_dip = None;
        self.corner_radius_dip = None;
        self.shadow = None;
    }
}

/// Sparse candidate label overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CandidateLabelOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    visible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    font_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gap_dip: Option<f32>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    passthrough: BTreeMap<String, toml::Value>,
}

impl CandidateLabelOverrides {
    fn is_empty(&self) -> bool {
        self.visible.is_none()
            && self.style.is_none()
            && self.sequence.is_none()
            && self.font_scale.is_none()
            && self.gap_dip.is_none()
            && self.passthrough.is_empty()
    }

    fn clear_known_overrides(&mut self) {
        self.visible = None;
        self.style = None;
        self.sequence = None;
        self.font_scale = None;
        self.gap_dip = None;
    }
}

/// Sparse font overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FontsOverrides {
    #[serde(default, skip_serializing_if = "FontFamiliesOverrides::is_empty")]
    ui: FontFamiliesOverrides,
    #[serde(default, skip_serializing_if = "CandidateFontOverrides::is_empty")]
    candidate: CandidateFontOverrides,
    #[serde(default, skip_serializing_if = "AnnotationFontOverrides::is_empty")]
    annotation: AnnotationFontOverrides,
    #[serde(default, skip_serializing_if = "FontFamiliesOverrides::is_empty")]
    monospace: FontFamiliesOverrides,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    passthrough: BTreeMap<String, toml::Value>,
}

impl FontsOverrides {
    fn is_empty(&self) -> bool {
        self.ui.is_empty()
            && self.candidate.is_empty()
            && self.annotation.is_empty()
            && self.monospace.is_empty()
            && self.passthrough.is_empty()
    }
}

/// Sparse generic font fallback overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FontFamiliesOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    families: Option<Vec<String>>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    passthrough: BTreeMap<String, toml::Value>,
}

impl FontFamiliesOverrides {
    fn is_empty(&self) -> bool {
        self.families.is_none() && self.passthrough.is_empty()
    }
}

/// Sparse candidate font overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CandidateFontOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    families: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_dip: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    weight: Option<u16>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    passthrough: BTreeMap<String, toml::Value>,
}

impl CandidateFontOverrides {
    fn is_empty(&self) -> bool {
        self.families.is_none()
            && self.size_dip.is_none()
            && self.weight.is_none()
            && self.passthrough.is_empty()
    }
}

/// Sparse annotation font overrides.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AnnotationFontOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    families: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scale: Option<f32>,
    #[serde(default, flatten, skip_serializing_if = "BTreeMap::is_empty")]
    passthrough: BTreeMap<String, toml::Value>,
}

impl AnnotationFontOverrides {
    fn is_empty(&self) -> bool {
        self.families.is_none() && self.scale.is_none() && self.passthrough.is_empty()
    }
}

/// The settings exposed for mutation through the shared Config Core.
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigEdit {
    /// Sets the UI display language.
    UiLanguage(String),
    /// Sets the appearance mode.
    AppearanceMode(String),
    /// Sets the selected theme ID.
    Theme(String),
    /// Sets the candidate orientation.
    CandidateOrientation(String),
    /// Sets the candidate page size.
    CandidatePageSize(u8),
    /// Sets candidate scroll mode.
    CandidateScrollMode(bool),
    /// Sets the candidate maximum width in DIP.
    CandidateMaxWidthDip(f32),
    /// Sets the scroll candidate cell width in DIP.
    CandidateScrollCellWidthDip(f32),
    /// Sets the candidate opacity.
    CandidateOpacity(f32),
    /// Sets the candidate preedit presentation mode.
    CandidatePreeditMode(String),
    /// Sets candidate font fallbacks.
    CandidateFontFamilies(Vec<String>),
    /// Sets the candidate font size in DIP.
    CandidateFontSizeDip(f32),
}

/// Identifies an override that can be reset to inherited defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigField {
    /// Resets every mutable override.
    All,
    /// Resets the UI language.
    UiLanguage,
    /// Resets the appearance mode.
    AppearanceMode,
    /// Resets the theme.
    Theme,
    /// Resets the candidate orientation.
    CandidateOrientation,
    /// Resets the candidate page size.
    CandidatePageSize,
    /// Resets the candidate scroll mode.
    CandidateScrollMode,
    /// Resets the candidate maximum width.
    CandidateMaxWidthDip,
    /// Resets the scroll candidate cell width.
    CandidateScrollCellWidthDip,
    /// Resets the candidate opacity.
    CandidateOpacity,
    /// Resets the candidate preedit presentation mode.
    CandidatePreeditMode,
    /// Resets all candidate geometry overrides.
    CandidateGeometry,
    /// Resets all candidate label overrides.
    CandidateLabel,
    /// Resets UI font fallbacks.
    UiFontFamilies,
    /// Resets candidate font fallbacks.
    CandidateFontFamilies,
    /// Resets the candidate font size.
    CandidateFontSizeDip,
    /// Resets the candidate font weight.
    CandidateFontWeight,
    /// Resets annotation font overrides.
    AnnotationFont,
    /// Resets monospace font fallbacks.
    MonospaceFontFamilies,
}

/// A GUI or CLI action interpreted by the same Config Core contract.
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigCommand {
    /// Returns the resolved Draft snapshot.
    Get,
    /// Edits Draft without writing a file.
    Set(ConfigEdit),
    /// Validates the complete Draft.
    Validate,
    /// Returns the differences between Current and Draft.
    Diff,
    /// Removes an override from Draft.
    Reset(ConfigField),
    /// Parses an imported sparse configuration into Draft.
    Import(String),
    /// Serializes Draft without writing a file.
    Export,
    /// Inspects recovery state without modifying any file.
    Doctor,
}

/// The typed result of a Config command.
#[derive(Clone, Debug, PartialEq)]
pub enum CommandOutput {
    /// A resolved Draft snapshot.
    Snapshot(ConfigSnapshot),
    /// A validated command produced no data.
    Valid,
    /// The current Draft differences.
    Diff(Vec<ConfigDiff>),
    /// A sparse TOML export.
    Export(String),
    /// Read-only recovery diagnostics.
    Doctor(RecoverySource),
}

/// A user-visible field change between Current and Draft.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigDiff {
    /// The changed field.
    pub field: ConfigField,
}

/// A deterministic simulated failure point used by contract tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitFault {
    /// Performs the normal staged commit.
    None,
    /// Fails while writing the Current staging file.
    StagedWrite,
    /// Fails while flushing the Current staging file.
    StagedFlush,
    /// Simulates a changed staged file before reread validation.
    RereadMismatch,
    /// Fails while replacing the last-known-good record.
    LastKnownGoodReplace,
    /// Fails while replacing Current.
    CurrentReplace,
}

/// Where a valid startup configuration was recovered from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoverySource {
    /// The committed Current file validated successfully.
    Current,
    /// Current was unusable and the last-known-good record validated successfully.
    LastKnownGood,
    /// Neither persisted record was usable, so compiled safe defaults were selected.
    SafeDefaults,
}

/// A recovered Config Core and its source.
#[derive(Debug)]
pub struct Recovery {
    /// The usable Core state selected during recovery.
    pub core: ConfigCore,
    /// The selected recovery source.
    pub source: RecoverySource,
}

/// Errors returned by Config Core.
#[derive(Debug)]
pub enum ConfigError {
    /// A user-provided configuration value is invalid.
    Validation { message: String },
    /// TOML input could not be parsed.
    Parse { message: String },
    /// A bounded file operation failed.
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    /// A staged reread did not reproduce the validated Draft.
    RereadMismatch,
    /// A deterministic contract-test fault was selected.
    InjectedFault { stage: &'static str },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation { message } => write!(formatter, "invalid configuration: {message}"),
            Self::Parse { message } => write!(formatter, "invalid configuration TOML: {message}"),
            Self::Io {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "failed to {operation} {}: {source}",
                path.display()
            ),
            Self::RereadMismatch => write!(formatter, "staged configuration did not match Draft"),
            Self::InjectedFault { stage } => {
                write!(formatter, "injected configuration failure at {stage}")
            }
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// The sole state authority for Config Current, Draft, and Defaults.
#[derive(Debug)]
pub struct ConfigCore {
    defaults: ConfigSnapshot,
    current: ConfigOverrides,
    draft: ConfigOverrides,
}

impl ConfigCore {
    /// Constructs a Core from the compiled, reviewed default config resource.
    #[must_use]
    pub fn compiled_defaults() -> Self {
        let defaults = toml::from_str(COMPILED_DEFAULTS)
            .expect("compiled resource config.toml must satisfy the Config Core schema");
        Self {
            defaults,
            current: ConfigOverrides::empty(),
            draft: ConfigOverrides::empty(),
        }
    }

    /// Loads an existing committed configuration without recovery fallback.
    ///
    /// # Errors
    ///
    /// Returns an error when the file cannot be read, parsed, or fully validated.
    pub fn load(store: &FileStore, path: &Path) -> Result<Self, ConfigError> {
        let overrides = store.load(path)?;
        let mut core = Self::compiled_defaults();
        core.current = overrides.clone();
        core.draft = overrides;
        core.validate()?;
        Ok(core)
    }

    /// Loads Current when it exists, otherwise starts from compiled Defaults.
    ///
    /// # Errors
    ///
    /// Returns an error when an existing Current file is not valid.
    pub fn load_or_defaults(store: &FileStore, path: &Path) -> Result<Self, ConfigError> {
        if path.is_file() {
            Self::load(store, path)
        } else {
            Ok(Self::compiled_defaults())
        }
    }

    /// Returns the immutable compiled Defaults snapshot.
    #[must_use]
    pub fn defaults(&self) -> &ConfigSnapshot {
        &self.defaults
    }

    /// Returns the resolved committed Current snapshot.
    #[must_use]
    pub fn current(&self) -> ConfigSnapshot {
        self.resolve(&self.current)
    }

    /// Returns the read-only resolved Draft snapshot for a preview renderer.
    #[must_use]
    pub fn preview(&self) -> ConfigSnapshot {
        self.resolve(&self.draft)
    }

    /// Validates the complete Draft before any write can occur.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Validation`] when Draft is not valid as a complete configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_snapshot(&self.preview())
    }

    /// Computes the user-visible differences between Current and Draft.
    #[must_use]
    pub fn diff(&self) -> Vec<ConfigDiff> {
        let current = self.current();
        let draft = self.preview();
        let mut differences = Vec::new();
        compare_field(
            &mut differences,
            ConfigField::UiLanguage,
            current.ui.language != draft.ui.language,
        );
        compare_field(
            &mut differences,
            ConfigField::AppearanceMode,
            current.appearance.mode != draft.appearance.mode,
        );
        compare_field(
            &mut differences,
            ConfigField::Theme,
            current.appearance.theme != draft.appearance.theme,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidateOrientation,
            current.candidate.orientation != draft.candidate.orientation,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidatePageSize,
            current.candidate.page_size != draft.candidate.page_size,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidateScrollMode,
            current.candidate.scroll_mode != draft.candidate.scroll_mode,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidateMaxWidthDip,
            current.candidate.max_width_dip != draft.candidate.max_width_dip,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidateScrollCellWidthDip,
            current.candidate.scroll_cell_width_dip != draft.candidate.scroll_cell_width_dip,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidateOpacity,
            current.candidate.opacity != draft.candidate.opacity,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidatePreeditMode,
            current.candidate.preedit_mode != draft.candidate.preedit_mode,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidateGeometry,
            current.candidate.geometry != draft.candidate.geometry,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidateLabel,
            current.candidate.label != draft.candidate.label,
        );
        compare_field(
            &mut differences,
            ConfigField::UiFontFamilies,
            current.fonts.ui.families != draft.fonts.ui.families,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidateFontFamilies,
            current.fonts.candidate.families != draft.fonts.candidate.families,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidateFontSizeDip,
            current.fonts.candidate.size_dip != draft.fonts.candidate.size_dip,
        );
        compare_field(
            &mut differences,
            ConfigField::CandidateFontWeight,
            current.fonts.candidate.weight != draft.fonts.candidate.weight,
        );
        compare_field(
            &mut differences,
            ConfigField::AnnotationFont,
            current.fonts.annotation != draft.fonts.annotation,
        );
        compare_field(
            &mut differences,
            ConfigField::MonospaceFontFamilies,
            current.fonts.monospace.families != draft.fonts.monospace.families,
        );
        differences
    }

    /// Discards every pending Draft edit and restores Draft from Current.
    pub fn cancel(&mut self) {
        self.draft.clone_from(&self.current);
    }

    /// Removes a Draft override so it inherits from Defaults.
    pub fn reset(&mut self, field: ConfigField) {
        match field {
            ConfigField::All => self.draft.clear_known_overrides(),
            ConfigField::UiLanguage => self.draft.ui.language = None,
            ConfigField::AppearanceMode => self.draft.appearance.mode = None,
            ConfigField::Theme => self.draft.appearance.theme = None,
            ConfigField::CandidateOrientation => self.draft.candidate.orientation = None,
            ConfigField::CandidatePageSize => self.draft.candidate.page_size = None,
            ConfigField::CandidateScrollMode => self.draft.candidate.scroll_mode = None,
            ConfigField::CandidateMaxWidthDip => self.draft.candidate.max_width_dip = None,
            ConfigField::CandidateScrollCellWidthDip => {
                self.draft.candidate.scroll_cell_width_dip = None;
            }
            ConfigField::CandidateOpacity => self.draft.candidate.opacity = None,
            ConfigField::CandidatePreeditMode => self.draft.candidate.preedit_mode = None,
            ConfigField::CandidateGeometry => {
                self.draft.candidate.geometry.clear_known_overrides();
            }
            ConfigField::CandidateLabel => {
                self.draft.candidate.label.clear_known_overrides();
            }
            ConfigField::UiFontFamilies => self.draft.fonts.ui.families = None,
            ConfigField::CandidateFontFamilies => {
                self.draft.fonts.candidate.families = None;
            }
            ConfigField::CandidateFontSizeDip => self.draft.fonts.candidate.size_dip = None,
            ConfigField::CandidateFontWeight => self.draft.fonts.candidate.weight = None,
            ConfigField::AnnotationFont => {
                self.draft.fonts.annotation.families = None;
                self.draft.fonts.annotation.scale = None;
            }
            ConfigField::MonospaceFontFamilies => {
                self.draft.fonts.monospace.families = None;
            }
        }
    }

    /// Applies a typed edit to Draft without writing a file.
    pub fn set(&mut self, edit: ConfigEdit) {
        match edit {
            ConfigEdit::UiLanguage(value) => self.draft.ui.language = Some(value),
            ConfigEdit::AppearanceMode(value) => self.draft.appearance.mode = Some(value),
            ConfigEdit::Theme(value) => self.draft.appearance.theme = Some(value),
            ConfigEdit::CandidateOrientation(value) => {
                self.draft.candidate.orientation = Some(value)
            }
            ConfigEdit::CandidatePageSize(value) => self.draft.candidate.page_size = Some(value),
            ConfigEdit::CandidateScrollMode(value) => {
                self.draft.candidate.scroll_mode = Some(value)
            }
            ConfigEdit::CandidateMaxWidthDip(value) => {
                self.draft.candidate.max_width_dip = Some(value)
            }
            ConfigEdit::CandidateScrollCellWidthDip(value) => {
                self.draft.candidate.scroll_cell_width_dip = Some(value)
            }
            ConfigEdit::CandidateOpacity(value) => self.draft.candidate.opacity = Some(value),
            ConfigEdit::CandidatePreeditMode(value) => {
                self.draft.candidate.preedit_mode = Some(value)
            }
            ConfigEdit::CandidateFontFamilies(value) => {
                self.draft.fonts.candidate.families = Some(value);
            }
            ConfigEdit::CandidateFontSizeDip(value) => {
                self.draft.fonts.candidate.size_dip = Some(value);
            }
        }
    }

    /// Imports a TOML file into Draft through the shared parser and validator.
    ///
    /// # Errors
    ///
    /// Returns an error when the import cannot be read, parsed, or validated.
    pub fn import_from_path(&mut self, store: &FileStore, path: &Path) -> Result<(), ConfigError> {
        let imported = parse_overrides(&store.read(path)?)?;
        validate_snapshot(&self.resolve(&imported))?;
        self.draft = imported;
        Ok(())
    }

    /// Exports Draft through the shared serializer and staged filesystem writer.
    ///
    /// # Errors
    ///
    /// Returns an error when Draft cannot be serialized or the destination cannot be atomically replaced.
    pub fn export_to(&self, store: &FileStore, path: &Path) -> Result<(), ConfigError> {
        let text = serialize_overrides(&self.draft)?;
        let staged = store.stage(path, text.as_bytes(), CommitFault::None)?;
        store.replace(&staged, path)
    }

    /// Executes a GUI or CLI command through the shared typed model.
    ///
    /// # Errors
    ///
    /// Returns parsing, validation, or recovery errors from the shared Core implementation.
    pub fn execute(
        &mut self,
        command: ConfigCommand,
        store: &FileStore,
        path: &Path,
    ) -> Result<CommandOutput, ConfigError> {
        match command {
            ConfigCommand::Get => Ok(CommandOutput::Snapshot(self.preview())),
            ConfigCommand::Set(edit) => {
                self.set(edit);
                Ok(CommandOutput::Snapshot(self.preview()))
            }
            ConfigCommand::Validate => {
                self.validate()?;
                Ok(CommandOutput::Valid)
            }
            ConfigCommand::Diff => Ok(CommandOutput::Diff(self.diff())),
            ConfigCommand::Reset(field) => {
                self.reset(field);
                Ok(CommandOutput::Snapshot(self.preview()))
            }
            ConfigCommand::Import(text) => {
                let imported = parse_overrides(&text)?;
                validate_snapshot(&self.resolve(&imported))?;
                self.draft = imported;
                Ok(CommandOutput::Snapshot(self.preview()))
            }
            ConfigCommand::Export => Ok(CommandOutput::Export(serialize_overrides(&self.draft)?)),
            ConfigCommand::Doctor => Ok(CommandOutput::Doctor(Self::recover(store, path)?.source)),
        }
    }

    /// Validates, stages, rereads, and atomically commits Draft and last-known-good records.
    ///
    /// # Errors
    ///
    /// Returns an error before replacing Current when validation, staging, reread, or replacement fails.
    pub fn apply(
        &mut self,
        store: &FileStore,
        path: &Path,
        fault: CommitFault,
    ) -> Result<(), ConfigError> {
        self.validate()?;
        let text = serialize_overrides(&self.draft)?;
        let current_stage = store.stage(path, text.as_bytes(), fault)?;
        let reread = store.read_staged(&current_stage, fault)?;
        let reread_overrides = parse_overrides(&reread)?;
        validate_snapshot(&self.resolve(&reread_overrides))?;
        if reread_overrides != self.draft {
            let _ = fs::remove_file(&current_stage);
            return Err(ConfigError::RereadMismatch);
        }

        let lkg_path = FileStore::last_known_good_path(path);
        let previous_lkg_stage = if lkg_path.is_file() {
            let previous_lkg = store.read(&lkg_path)?;
            Some(store.stage(&lkg_path, previous_lkg.as_bytes(), CommitFault::None)?)
        } else {
            None
        };
        // LKG tracks the previously committed Current. The first commit uses the new
        // snapshot because there is no earlier committed file to recover.
        let lkg_text = if path.is_file() {
            serialize_overrides(&self.current)?
        } else {
            text.clone()
        };
        let lkg_stage = store.stage(&lkg_path, lkg_text.as_bytes(), CommitFault::None)?;
        if fault == CommitFault::LastKnownGoodReplace {
            let _ = fs::remove_file(&current_stage);
            let _ = fs::remove_file(&lkg_stage);
            if let Some(stage) = previous_lkg_stage {
                let _ = fs::remove_file(stage);
            }
            return Err(ConfigError::InjectedFault {
                stage: "last-known-good replace",
            });
        }
        store.replace(&lkg_stage, &lkg_path)?;
        let current_result = if fault == CommitFault::CurrentReplace {
            Err(ConfigError::InjectedFault {
                stage: "current replace",
            })
        } else {
            store.replace(&current_stage, path)
        };
        if let Err(error) = current_result {
            let restore = store.restore_last_known_good(previous_lkg_stage.as_deref(), &lkg_path);
            let _ = fs::remove_file(&current_stage);
            if let Err(restore_error) = restore {
                return Err(restore_error);
            }
            return Err(error);
        }
        if let Some(stage) = previous_lkg_stage {
            let _ = fs::remove_file(stage);
        }
        self.current.clone_from(&self.draft);
        Ok(())
    }

    /// Selects Current, then last-known-good, then compiled safe defaults without modifying files.
    ///
    /// # Errors
    ///
    /// This currently cannot fail because compiled defaults are an internal invariant.
    pub fn recover(store: &FileStore, path: &Path) -> Result<Recovery, ConfigError> {
        if let Some(core) = Self::try_load(store, path) {
            return Ok(Recovery {
                core,
                source: RecoverySource::Current,
            });
        }
        if let Some(core) = Self::try_load(store, &FileStore::last_known_good_path(path)) {
            return Ok(Recovery {
                core,
                source: RecoverySource::LastKnownGood,
            });
        }
        Ok(Recovery {
            core: Self::compiled_defaults(),
            source: RecoverySource::SafeDefaults,
        })
    }

    fn try_load(store: &FileStore, path: &Path) -> Option<Self> {
        Self::load(store, path).ok()
    }

    fn resolve(&self, overrides: &ConfigOverrides) -> ConfigSnapshot {
        let mut resolved = self.defaults.clone();
        if let Some(value) = &overrides.ui.language {
            resolved.ui.language.clone_from(value);
        }
        if let Some(value) = &overrides.appearance.mode {
            resolved.appearance.mode.clone_from(value);
        }
        if let Some(value) = &overrides.appearance.theme {
            resolved.appearance.theme.clone_from(value);
        }
        if let Some(value) = &overrides.candidate.orientation {
            resolved.candidate.orientation.clone_from(value);
        }
        if let Some(value) = overrides.candidate.page_size {
            resolved.candidate.page_size = value;
        }
        if let Some(value) = overrides.candidate.scroll_mode {
            resolved.candidate.scroll_mode = value;
        }
        if let Some(value) = overrides.candidate.max_width_dip {
            resolved.candidate.max_width_dip = value;
        }
        if let Some(value) = overrides.candidate.scroll_cell_width_dip {
            resolved.candidate.scroll_cell_width_dip = value;
        }
        if let Some(value) = overrides.candidate.opacity {
            resolved.candidate.opacity = value;
        }
        if let Some(value) = &overrides.candidate.preedit_mode {
            resolved.candidate.preedit_mode.clone_from(value);
        }
        apply_geometry_overrides(
            &mut resolved.candidate.geometry,
            &overrides.candidate.geometry,
        );
        apply_label_overrides(&mut resolved.candidate.label, &overrides.candidate.label);
        resolved
            .candidate
            .colors
            .extend(overrides.candidate.colors.clone());
        if let Some(value) = &overrides.fonts.ui.families {
            resolved.fonts.ui.families.clone_from(value);
        }
        if let Some(value) = &overrides.fonts.candidate.families {
            resolved.fonts.candidate.families.clone_from(value);
        }
        if let Some(value) = overrides.fonts.candidate.size_dip {
            resolved.fonts.candidate.size_dip = value;
        }
        if let Some(value) = overrides.fonts.candidate.weight {
            resolved.fonts.candidate.weight = value;
        }
        if let Some(value) = &overrides.fonts.annotation.families {
            resolved.fonts.annotation.families.clone_from(value);
        }
        if let Some(value) = overrides.fonts.annotation.scale {
            resolved.fonts.annotation.scale = value;
        }
        if let Some(value) = &overrides.fonts.monospace.families {
            resolved.fonts.monospace.families.clone_from(value);
        }
        resolved
    }
}

fn apply_geometry_overrides(
    geometry: &mut CandidateGeometry,
    overrides: &CandidateGeometryOverrides,
) {
    if let Some(value) = overrides.padding_x_dip {
        geometry.padding_x_dip = value;
    }
    if let Some(value) = overrides.padding_y_dip {
        geometry.padding_y_dip = value;
    }
    if let Some(value) = overrides.item_padding_x_dip {
        geometry.item_padding_x_dip = value;
    }
    if let Some(value) = overrides.item_padding_y_dip {
        geometry.item_padding_y_dip = value;
    }
    if let Some(value) = overrides.row_gap_dip {
        geometry.row_gap_dip = value;
    }
    if let Some(value) = overrides.column_gap_dip {
        geometry.column_gap_dip = value;
    }
    if let Some(value) = overrides.border_width_dip {
        geometry.border_width_dip = value;
    }
    if let Some(value) = overrides.corner_radius_dip {
        geometry.corner_radius_dip = value;
    }
    if let Some(value) = overrides.shadow {
        geometry.shadow = value;
    }
}

fn apply_label_overrides(label: &mut CandidateLabel, overrides: &CandidateLabelOverrides) {
    if let Some(value) = overrides.visible {
        label.visible = value;
    }
    if let Some(value) = &overrides.style {
        label.style.clone_from(value);
    }
    if let Some(value) = &overrides.sequence {
        label.sequence.clone_from(value);
    }
    if let Some(value) = overrides.font_scale {
        label.font_scale = value;
    }
    if let Some(value) = overrides.gap_dip {
        label.gap_dip = value;
    }
}

impl ConfigEdit {
    /// Parses one CLI field/value pair into a typed Draft edit.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported field or malformed value.
    pub fn from_cli(field: &str, value: &str) -> Result<Self, ConfigError> {
        match field {
            "ui.language" => Ok(Self::UiLanguage(value.to_owned())),
            "appearance.mode" => Ok(Self::AppearanceMode(value.to_owned())),
            "appearance.theme" => Ok(Self::Theme(value.to_owned())),
            "candidate.orientation" => Ok(Self::CandidateOrientation(value.to_owned())),
            "candidate.page_size" => {
                value
                    .parse::<u8>()
                    .map(Self::CandidatePageSize)
                    .map_err(|_| ConfigError::Parse {
                        message: "candidate.page_size must be an unsigned integer".to_owned(),
                    })
            }
            "candidate.scroll_mode" => value
                .parse::<bool>()
                .map(Self::CandidateScrollMode)
                .map_err(|_| ConfigError::Parse {
                    message: "candidate.scroll_mode must be true or false".to_owned(),
                }),
            "candidate.max_width_dip" => value
                .parse::<f32>()
                .map(Self::CandidateMaxWidthDip)
                .map_err(|_| ConfigError::Parse {
                    message: "candidate.max_width_dip must be a number".to_owned(),
                }),
            "candidate.scroll_cell_width_dip" => value
                .parse::<f32>()
                .map(Self::CandidateScrollCellWidthDip)
                .map_err(|_| ConfigError::Parse {
                    message: "candidate.scroll_cell_width_dip must be a number".to_owned(),
                }),
            "candidate.opacity" => value
                .parse::<f32>()
                .map(Self::CandidateOpacity)
                .map_err(|_| ConfigError::Parse {
                    message: "candidate.opacity must be a number".to_owned(),
                }),
            "candidate.preedit_mode" => Ok(Self::CandidatePreeditMode(value.to_owned())),
            "fonts.candidate.families" => Ok(Self::CandidateFontFamilies(
                value.split(',').map(str::to_owned).collect(),
            )),
            "fonts.candidate.size_dip" => value
                .parse::<f32>()
                .map(Self::CandidateFontSizeDip)
                .map_err(|_| ConfigError::Parse {
                    message: "fonts.candidate.size_dip must be a number".to_owned(),
                }),
            _ => Err(ConfigError::Parse {
                message: format!("unsupported config field {field}"),
            }),
        }
    }
}

impl ConfigField {
    /// Parses a CLI reset field into the same typed reset target used by GUI controls.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsupported reset field.
    pub fn from_cli(field: &str) -> Result<Self, ConfigError> {
        match field {
            "all" => Ok(Self::All),
            "ui.language" => Ok(Self::UiLanguage),
            "appearance.mode" => Ok(Self::AppearanceMode),
            "appearance.theme" => Ok(Self::Theme),
            "candidate.orientation" => Ok(Self::CandidateOrientation),
            "candidate.page_size" => Ok(Self::CandidatePageSize),
            "candidate.scroll_mode" => Ok(Self::CandidateScrollMode),
            "candidate.max_width_dip" => Ok(Self::CandidateMaxWidthDip),
            "candidate.scroll_cell_width_dip" => Ok(Self::CandidateScrollCellWidthDip),
            "candidate.opacity" => Ok(Self::CandidateOpacity),
            "candidate.preedit_mode" => Ok(Self::CandidatePreeditMode),
            "fonts.candidate.families" => Ok(Self::CandidateFontFamilies),
            "fonts.candidate.size_dip" => Ok(Self::CandidateFontSizeDip),
            _ => Err(ConfigError::Parse {
                message: format!("unsupported config field {field}"),
            }),
        }
    }
}

/// Filesystem implementation used by the Core transaction API.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileStore;

impl FileStore {
    /// Creates a filesystem-backed Config store.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Returns the sole last-known-good path for a Current file.
    #[must_use]
    pub fn last_known_good_path(path: &Path) -> PathBuf {
        let mut value = path.as_os_str().to_owned();
        value.push(".lkg");
        PathBuf::from(value)
    }

    fn load(&self, path: &Path) -> Result<ConfigOverrides, ConfigError> {
        parse_overrides(&self.read(path)?)
    }

    fn read(&self, path: &Path) -> Result<String, ConfigError> {
        let metadata = fs::metadata(path).map_err(|source| ConfigError::Io {
            operation: "inspect configuration",
            path: path.to_path_buf(),
            source,
        })?;
        if metadata.len() > MAX_CONFIG_BYTES {
            return Err(ConfigError::Validation {
                message: "config.toml exceeds the 256 KiB input limit".to_owned(),
            });
        }
        fs::read_to_string(path).map_err(|source| ConfigError::Io {
            operation: "read configuration",
            path: path.to_path_buf(),
            source,
        })
    }

    fn stage(
        &self,
        path: &Path,
        contents: &[u8],
        fault: CommitFault,
    ) -> Result<PathBuf, ConfigError> {
        if fault == CommitFault::StagedWrite {
            return Err(ConfigError::InjectedFault {
                stage: "staged write",
            });
        }
        let parent = path.parent().ok_or_else(|| ConfigError::Validation {
            message: "configuration path has no parent directory".to_owned(),
        })?;
        fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
            operation: "create configuration directory",
            path: parent.to_path_buf(),
            source,
        })?;
        let staged = stage_path(path);
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&staged)
            .map_err(|source| ConfigError::Io {
                operation: "create staged configuration",
                path: staged.clone(),
                source,
            })?;
        if let Err(error) = file.write_all(contents) {
            let _ = fs::remove_file(&staged);
            return Err(ConfigError::Io {
                operation: "write staged configuration",
                path: staged,
                source: error,
            });
        }
        if fault == CommitFault::StagedFlush {
            let _ = fs::remove_file(&staged);
            return Err(ConfigError::InjectedFault {
                stage: "staged flush",
            });
        }
        file.sync_all().map_err(|source| ConfigError::Io {
            operation: "flush staged configuration",
            path: staged.clone(),
            source,
        })?;
        Ok(staged)
    }

    fn read_staged(&self, path: &Path, fault: CommitFault) -> Result<String, ConfigError> {
        if fault == CommitFault::RereadMismatch {
            return Ok("format_version = 1\n[candidate]\npage_size = 6\n".to_owned());
        }
        self.read(path)
    }

    fn replace(&self, staged: &Path, destination: &Path) -> Result<(), ConfigError> {
        fs::rename(staged, destination).map_err(|source| ConfigError::Io {
            operation: "atomically replace configuration",
            path: destination.to_path_buf(),
            source,
        })
    }

    fn restore_last_known_good(
        &self,
        previous_stage: Option<&Path>,
        lkg_path: &Path,
    ) -> Result<(), ConfigError> {
        if let Some(stage) = previous_stage {
            return self.replace(stage, lkg_path);
        }
        match fs::remove_file(lkg_path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(ConfigError::Io {
                operation: "restore last-known-good configuration",
                path: lkg_path.to_path_buf(),
                source,
            }),
        }
    }
}

fn stage_path(path: &Path) -> PathBuf {
    let sequence = STAGE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut value = path.as_os_str().to_owned();
    value.push(format!(".stage-{}-{sequence}", std::process::id()));
    PathBuf::from(value)
}

fn parse_overrides(text: &str) -> Result<ConfigOverrides, ConfigError> {
    let parsed = toml::from_str(text).map_err(|error| ConfigError::Parse {
        message: error.to_string(),
    })?;
    Ok(parsed)
}

fn serialize_overrides(overrides: &ConfigOverrides) -> Result<String, ConfigError> {
    toml::to_string(overrides).map_err(|error| ConfigError::Parse {
        message: error.to_string(),
    })
}

fn compare_field(differences: &mut Vec<ConfigDiff>, field: ConfigField, changed: bool) {
    if changed {
        differences.push(ConfigDiff { field });
    }
}

fn validate_snapshot(snapshot: &ConfigSnapshot) -> Result<(), ConfigError> {
    if snapshot.format_version != CONFIG_FORMAT_VERSION {
        return invalid("format_version must be 1");
    }
    validate_one_of(
        "ui.language",
        &snapshot.ui.language,
        &["system", "en-US", "zh-CN"],
    )?;
    validate_one_of(
        "appearance.mode",
        &snapshot.appearance.mode,
        &["system", "light", "dark"],
    )?;
    validate_id("appearance.theme", &snapshot.appearance.theme, true)?;
    validate_one_of(
        "candidate.orientation",
        &snapshot.candidate.orientation,
        &["automatic", "horizontal", "vertical"],
    )?;
    if !(1..=9).contains(&snapshot.candidate.page_size) {
        return invalid("candidate.page_size must be between 1 and 9");
    }
    validate_finite_range(
        "candidate.max_width_dip",
        snapshot.candidate.max_width_dip,
        160.0,
        2048.0,
    )?;
    validate_finite_range(
        "candidate.scroll_cell_width_dip",
        snapshot.candidate.scroll_cell_width_dip,
        40.0,
        160.0,
    )?;
    validate_finite_range("candidate.opacity", snapshot.candidate.opacity, 0.2, 1.0)?;
    validate_finite_range(
        "candidate.geometry.padding_x_dip",
        snapshot.candidate.geometry.padding_x_dip,
        0.0,
        64.0,
    )?;
    validate_finite_range(
        "candidate.geometry.padding_y_dip",
        snapshot.candidate.geometry.padding_y_dip,
        0.0,
        64.0,
    )?;
    validate_finite_range(
        "candidate.geometry.item_padding_x_dip",
        snapshot.candidate.geometry.item_padding_x_dip,
        0.0,
        64.0,
    )?;
    validate_finite_range(
        "candidate.geometry.item_padding_y_dip",
        snapshot.candidate.geometry.item_padding_y_dip,
        0.0,
        64.0,
    )?;
    validate_finite_range(
        "candidate.geometry.row_gap_dip",
        snapshot.candidate.geometry.row_gap_dip,
        0.0,
        64.0,
    )?;
    validate_finite_range(
        "candidate.geometry.column_gap_dip",
        snapshot.candidate.geometry.column_gap_dip,
        0.0,
        64.0,
    )?;
    validate_finite_range(
        "candidate.geometry.border_width_dip",
        snapshot.candidate.geometry.border_width_dip,
        0.0,
        8.0,
    )?;
    validate_finite_range(
        "candidate.geometry.corner_radius_dip",
        snapshot.candidate.geometry.corner_radius_dip,
        0.0,
        64.0,
    )?;
    validate_one_of(
        "candidate.label.style",
        &snapshot.candidate.label.style,
        &["plain", "dot", "paren", "bracket", "circled"],
    )?;
    validate_finite_range(
        "candidate.label.font_scale",
        snapshot.candidate.label.font_scale,
        0.5,
        1.5,
    )?;
    validate_finite_range(
        "candidate.label.gap_dip",
        snapshot.candidate.label.gap_dip,
        0.0,
        64.0,
    )?;
    validate_label_sequence(&snapshot.candidate.label.sequence)?;
    validate_one_of(
        "candidate.preedit_mode",
        &snapshot.candidate.preedit_mode,
        &["inline", "panel"],
    )?;
    for (name, color) in &snapshot.candidate.colors {
        validate_color(name, color)?;
    }
    validate_font_families("fonts.ui.families", &snapshot.fonts.ui.families)?;
    validate_font_families(
        "fonts.candidate.families",
        &snapshot.fonts.candidate.families,
    )?;
    validate_finite_range(
        "fonts.candidate.size_dip",
        snapshot.fonts.candidate.size_dip,
        8.0,
        72.0,
    )?;
    if !(100..=900).contains(&snapshot.fonts.candidate.weight) {
        return invalid("fonts.candidate.weight must be between 100 and 900");
    }
    validate_font_families(
        "fonts.annotation.families",
        &snapshot.fonts.annotation.families,
    )?;
    validate_finite_range(
        "fonts.annotation.scale",
        snapshot.fonts.annotation.scale,
        0.5,
        2.0,
    )?;
    validate_font_families(
        "fonts.monospace.families",
        &snapshot.fonts.monospace.families,
    )
}

fn validate_one_of(field: &str, value: &str, values: &[&str]) -> Result<(), ConfigError> {
    if values.contains(&value) {
        Ok(())
    } else {
        invalid(&format!("{field} has an unsupported value"))
    }
}

fn validate_id(field: &str, value: &str, allow_builtin: bool) -> Result<(), ConfigError> {
    let valid = !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-' | b':')
        })
        && (allow_builtin || !value.starts_with("builtin:"));
    if valid {
        Ok(())
    } else {
        invalid(&format!("{field} must be a stable theme ID"))
    }
}

fn validate_finite_range(
    field: &str,
    value: f32,
    minimum: f32,
    maximum: f32,
) -> Result<(), ConfigError> {
    if value.is_finite() && (minimum..=maximum).contains(&value) {
        Ok(())
    } else {
        invalid(&format!("{field} must be between {minimum} and {maximum}"))
    }
}

fn validate_label_sequence(values: &[String]) -> Result<(), ConfigError> {
    if values.is_empty()
        || values.len() > 9
        || values
            .iter()
            .any(|value| value.is_empty() || value.len() > 16)
    {
        return invalid(
            "candidate.label.sequence must contain 1 through 9 labels of at most 16 bytes",
        );
    }
    Ok(())
}

fn validate_color(name: &str, color: &str) -> Result<(), ConfigError> {
    let bytes = color.as_bytes();
    if matches!(bytes.len(), 7 | 9)
        && bytes.first() == Some(&b'#')
        && bytes[1..].iter().all(u8::is_ascii_hexdigit)
    {
        Ok(())
    } else {
        invalid(&format!(
            "candidate.colors.{name} must be #RRGGBB or #RRGGBBAA"
        ))
    }
}

fn validate_font_families(field: &str, values: &[String]) -> Result<(), ConfigError> {
    if values.is_empty()
        || values.len() > 8
        || values
            .iter()
            .any(|value| value.is_empty() || value.len() > 128)
    {
        return invalid(&format!(
            "{field} must contain 1 through 8 non-empty family names"
        ));
    }
    Ok(())
}

fn invalid(message: &str) -> Result<(), ConfigError> {
    Err(ConfigError::Validation {
        message: message.to_owned(),
    })
}
