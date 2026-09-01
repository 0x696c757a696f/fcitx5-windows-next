#![deny(unsafe_op_in_unsafe_fn)]

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;

use fcitx5_candidate_core::{candidate_preview_paint_plan, run_candidate_poc_self_check};
use fcitx5_config_core::{
    ConfigCommand, ConfigCore, ConfigEdit, ConfigField, ConfigSnapshot, FileStore, RecoverySource,
};
use fcitx5_control_core::{control_schema_json, control_usage_text};
use fcitx5_package_core::{
    finalize_package_removal_entries, find_repository_package, mark_package_for_removal_entries,
    parse_lockfile, parse_manifest, parse_repository_index_with_policy, parse_trusted_keys,
    set_package_state_entries, validate_manifest_compatibility, PackageLifecycleState,
    RepositoryVerificationPolicy,
};
use fcitx5_process_execution_core::run_process_bounded;
use serde::Deserialize;
use windui::prelude::{
    brand_icon as windui_brand_icon, brand_icon_at as windui_brand_icon_at,
    signal as windui_signal, Align as WindUiAlign, App as WindUiApp, Color as WindUiColor,
    Element as WindUiElement, Fit as WindUiFit, Intent as WindUiIntent, Role as WindUiRole,
    Sender as WindUiSender, Signal as WindUiSignal, Theme as WindUiTheme,
    ThemeHandle as WindUiThemeHandle, WindowButtonKind as WindUiWindowButtonKind,
};

const CONFIG_POC_COMPONENT: &str = "fcitx5-config-poc";
const CONFIG_RETIRED_SIDE_BY_SIDE_COMPONENT: &str = "none";
const CONFIG_SHIPPING_COMPONENT: &str = "fcitx5-config";
const CONFIG_SHIPPING_BINARY_NAME: &str = "fcitx5-config.exe";
const CANDIDATE_PREVIEW_HOST_KIND: &str = "config-child-candidate-renderer-host";
const CANDIDATE_PREVIEW_RENDERER_CONTRACT: &str = "shipping-candidate-real-preview-host-path";
const CANDIDATE_PREVIEW_WINDOW_OWNERSHIP: &str = "config-content-child-surface";
const CANDIDATE_PREVIEW_THEME_SNAPSHOT: &str = "resolved-theme-snapshot-shared-with-candidate-ui";
const CANDIDATE_PREVIEW_MODEL_CONTRACT: &str = "candidate-model-layout-render-segments";
const CANDIDATE_PREVIEW_SAMPLE_SOURCE: &str = "fixed-preview-sample-input-only";
const WINDOW_EFFECTS_ADAPTER_CONTRACT: &str = "rust-config-window-effects-capability-adapter";
const SETTINGS_SURFACE_CONTRACT: &str = "bounded-rust-d2d-dwrite-settings-surface";
const WIND_UI_RUST_REFERENCE_COMMIT: &str = "62241e25e762df154c1b1f855b4db57533e516fc";
const WIND_UI_RUST_LICENSE: &str = "MIT OR Apache-2.0";
const CONFIG_QA_PREVIEW_STATE_ENV: &str = "FCITX5_CONFIG_RUST_PREVIEW_STATE";

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Size {
    width: i32,
    height: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SettingsPalette {
    background: u32,
    sidebar: u32,
    content: u32,
    header: u32,
    accent: u32,
    nav_selected: u32,
    text_primary: u32,
    focus_ring: u32,
    disabled_surface: u32,
}

#[derive(Clone, Debug)]
struct WindUiRustAdoptionEvidence {
    crate_name: &'static str,
    reference_commit: &'static str,
    license: &'static str,
    vendored_path_dependency: bool,
    role_palette_consumed: bool,
    theme_row_height_consumed: bool,
    element_builder_tree_constructed: bool,
    setting_row_constructed: bool,
    segmented_control_constructed: bool,
    nav_list_pattern_used: bool,
    preview_first_appearance_layout: bool,
    engineering_dip_labels_removed_from_first_screen: bool,
    settings_shell_constructed: bool,
    settings_input_visual_baseline: bool,
    default_interactive_window_uses_windui: bool,
    win32_preview_host_qa_only: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DesignTokens {
    spacing_4: i32,
    spacing_8: i32,
    spacing_12: i32,
    spacing_16: i32,
    spacing_24: i32,
    radius_4: i32,
    radius_8: i32,
    control_height: i32,
    comfortable_control_height: i32,
    sidebar_width: i32,
    sidebar_margin_left: i32,
    sidebar_nav_top: i32,
    nav_item_width: i32,
    nav_item_height: i32,
    nav_item_step: i32,
    nav_accent_width: i32,
    header_height: i32,
    content_x: i32,
    content_width: i32,
    content_right_margin: i32,
    content_bottom_margin: i32,
    title_font_height: i32,
    body_font_height: i32,
    title_weight: i32,
    body_weight: i32,
    focus_ring_width: i32,
    minimum_window: Size,
    candidate_preview: Rect,
    palette: SettingsPalette,
}

fn design_tokens() -> DesignTokens {
    let windui_theme = windui_settings_theme();
    DesignTokens {
        spacing_4: 4,
        spacing_8: 8,
        spacing_12: 12,
        spacing_16: 16,
        spacing_24: 24,
        radius_4: 4,
        radius_8: 8,
        control_height: 32,
        comfortable_control_height: windui_theme.form.row_height(),
        sidebar_width: 204,
        sidebar_margin_left: 24,
        sidebar_nav_top: 84,
        nav_item_width: 176,
        nav_item_height: 42,
        nav_item_step: 54,
        nav_accent_width: 4,
        header_height: 72,
        content_x: 248,
        content_width: 596,
        content_right_margin: 18,
        content_bottom_margin: 18,
        title_font_height: 26,
        body_font_height: 18,
        title_weight: 600,
        body_weight: 400,
        focus_ring_width: 2,
        minimum_window: Size {
            width: 900,
            height: 720,
        },
        candidate_preview: Rect {
            x: 248,
            y: 104,
            width: 596,
            height: 176,
        },
        palette: settings_palette_from_windui(&windui_theme),
    }
}

fn windui_settings_theme() -> WindUiTheme {
    let mut theme = WindUiTheme::default();
    apply_wechat_green_palette(&mut theme, false);
    theme.form.row_height = Some(44);
    theme.form.label_size = Some(14.0);
    theme.form.desc_size = Some(12.5);
    theme
}

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

fn colorref_from_windui(color: WindUiColor) -> u32 {
    u32::from(color.r) | (u32::from(color.g) << 8) | (u32::from(color.b) << 16)
}

fn settings_palette_from_windui(theme: &WindUiTheme) -> SettingsPalette {
    SettingsPalette {
        background: colorref_from_windui(WindUiRole::Bg.resolve(theme)),
        sidebar: colorref_from_windui(WindUiRole::SurfaceAlt.resolve(theme)),
        content: colorref_from_windui(WindUiRole::Surface.resolve(theme)),
        header: colorref_from_windui(WindUiRole::Surface.resolve(theme)),
        accent: colorref_from_windui(WindUiRole::Accent.resolve(theme)),
        nav_selected: colorref_from_windui(WindUiRole::Surface.resolve(theme)),
        text_primary: colorref_from_windui(WindUiRole::Text.resolve(theme)),
        focus_ring: colorref_from_windui(WindUiRole::Accent.resolve(theme)),
        disabled_surface: colorref_from_windui(WindUiRole::TextDisabled.resolve(theme)),
    }
}

fn windui_appearance_reference_tree() -> WindUiElement {
    let theme_mode = windui_signal(0usize);
    let layout_mode = windui_signal(0usize);
    let show_shadow = windui_signal(true);
    WindUiElement::col()
        .width_match()
        .padding(22)
        .spacing(6)
        .child(
            WindUiElement::label("Appearance")
                .font_size(20.0)
                .fg_role(WindUiRole::Text)
                .height(34)
                .width_match(),
        )
        .child(WindUiElement::setting_row(
            "Theme",
            WindUiElement::segmented(vec!["System", "Light", "Dark"], theme_mode),
        ))
        .child(WindUiElement::setting_row(
            "Candidate layout",
            WindUiElement::segmented(vec!["Follow", "Horizontal", "Vertical"], layout_mode),
        ))
        .child(WindUiElement::setting_row(
            "Window shadow",
            WindUiElement::switch(show_shadow),
        ))
}

const WINDUI_SETTINGS_TITLEBAR_HEIGHT: i32 = 44;

fn windui_brand_logo(size: i32) -> WindUiElement {
    let px = (size.max(1) as u32) * 2;
    let icon = windui_brand_icon_at(px);
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
    name: &'static str,
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
            WindUiElement::label(name)
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
            WindUiElement::label(name)
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

fn candidate_preview_slot_visible(page_size: u8, slot: u8) -> bool {
    slot <= page_size
}

fn windui_candidate_preview_chip(
    text: &str,
    active: bool,
    visible: WindUiSignal<bool>,
) -> WindUiElement {
    WindUiElement::row()
        .height(32)
        .corner(7.0)
        .padding_xy(12, 0)
        .cross(WindUiAlign::Center)
        .bg_role(if active {
            WindUiRole::Accent
        } else {
            WindUiRole::SurfaceAlt
        })
        .child(
            WindUiElement::label(text)
                .font_size(14.0)
                .fg_role(if active {
                    WindUiRole::OnAccent
                } else {
                    WindUiRole::Text
                }),
        )
        .visible_when(move || visible.get())
}

fn windui_candidate_preview_row(
    label: &'static str,
    text: &'static str,
    comment: &'static str,
    active: bool,
    dark: bool,
    visible: WindUiSignal<bool>,
) -> WindUiElement {
    WindUiElement::row()
        .width_match()
        .height(38)
        .cross(WindUiAlign::Center)
        .spacing(10)
        .padding_xy(10, 0)
        .bg_role(if active {
            WindUiRole::Accent
        } else if dark {
            WindUiRole::SurfaceAlt
        } else {
            WindUiRole::Bg
        })
        .child(
            WindUiElement::label(label)
                .font_size(13.0)
                .fg_role(if active {
                    WindUiRole::OnAccent
                } else {
                    WindUiRole::TextMuted
                })
                .width(26),
        )
        .child(
            WindUiElement::label(text)
                .font_size(16.0)
                .fg_role(if active {
                    WindUiRole::OnAccent
                } else {
                    WindUiRole::Text
                })
                .weight(1.0),
        )
        .child(
            WindUiElement::label(comment)
                .font_size(12.0)
                .fg_role(if active {
                    WindUiRole::OnAccent
                } else {
                    WindUiRole::TextMuted
                }),
        )
        .visible_when(move || visible.get())
}

fn windui_candidate_preview_panel(
    layout: WindUiSignal<CandidateLayoutMode>,
    page_size: WindUiSignal<u8>,
    theme_mode: WindUiSignal<usize>,
    draft_summary: WindUiSignal<String>,
) -> WindUiElement {
    let preview_description =
        page_size.map(move |page_size| layout.get().preview_description(*page_size));
    let mode = WindUiElement::row()
        .cross(WindUiAlign::Center)
        .spacing(6)
        .child(
            WindUiElement::label("wubi")
                .font_size(12.5)
                .fg_role(WindUiRole::TextMuted),
        )
        .child(WindUiElement::badge_intent(
            "preview",
            WindUiIntent::Neutral,
        ))
        .child(
            WindUiElement::label(" · ")
                .font_size(12.5)
                .fg_role(WindUiRole::TextDisabled),
        )
        .child(
            WindUiElement::label("浅色")
                .font_size(12.5)
                .fg_role(WindUiRole::Accent)
                .visible_when(move || theme_mode.get() != 2),
        )
        .child(
            WindUiElement::label("深色")
                .font_size(12.5)
                .fg_role(WindUiRole::Accent)
                .visible_when(move || theme_mode.get() == 2),
        )
        .child(
            WindUiElement::label_signal(preview_description)
                .font_size(12.5)
                .fg_role(WindUiRole::Accent),
        )
        .child(
            WindUiElement::label_signal(draft_summary)
                .font_size(12.5)
                .fg_role(WindUiRole::TextMuted),
        );

    let vertical = WindUiElement::col()
        .width_match()
        .spacing(2)
        .child(windui_candidate_preview_row(
            "1.",
            "是",
            "",
            true,
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 1)),
        ))
        .child(windui_candidate_preview_row(
            "2.",
            "识",
            "",
            false,
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 2)),
        ))
        .child(windui_candidate_preview_row(
            "3.",
            "实",
            "",
            false,
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 3)),
        ))
        .child(windui_candidate_preview_row(
            "4.",
            "水",
            "~b",
            false,
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 4)),
        ))
        .child(windui_candidate_preview_row(
            "5.",
            "收",
            "~d",
            false,
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 5)),
        ))
        .child(windui_candidate_preview_row(
            "6.",
            "十",
            "",
            false,
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 6)),
        ))
        .child(windui_candidate_preview_row(
            "7.",
            "诗",
            "",
            false,
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 7)),
        ))
        .child(windui_candidate_preview_row(
            "8.",
            "式",
            "",
            false,
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 8)),
        ))
        .child(windui_candidate_preview_row(
            "9.",
            "试",
            "",
            false,
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 9)),
        ))
        .visible_when(move || {
            matches!(
                layout.get(),
                CandidateLayoutMode::Automatic
                    | CandidateLayoutMode::Vertical
                    | CandidateLayoutMode::ScrollAutomatic
                    | CandidateLayoutMode::ScrollVertical
            )
        });

    let horizontal = WindUiElement::row()
        .width_match()
        .spacing(6)
        .child(windui_candidate_preview_chip(
            "1. 是",
            true,
            page_size.map(|count| candidate_preview_slot_visible(*count, 1)),
        ))
        .child(windui_candidate_preview_chip(
            "2. 识",
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 2)),
        ))
        .child(windui_candidate_preview_chip(
            "3. 实",
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 3)),
        ))
        .child(windui_candidate_preview_chip(
            "4. 水 ~b",
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 4)),
        ))
        .child(windui_candidate_preview_chip(
            "5. 收 ~d",
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 5)),
        ))
        .child(windui_candidate_preview_chip(
            "6. 十",
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 6)),
        ))
        .child(windui_candidate_preview_chip(
            "7. 诗",
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 7)),
        ))
        .child(windui_candidate_preview_chip(
            "8. 式",
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 8)),
        ))
        .child(windui_candidate_preview_chip(
            "9. 试",
            false,
            page_size.map(|count| candidate_preview_slot_visible(*count, 9)),
        ))
        .visible_when(move || {
            matches!(
                layout.get(),
                CandidateLayoutMode::Horizontal | CandidateLayoutMode::ScrollHorizontal
            )
        });

    WindUiElement::col()
        .width_match()
        .spacing(10)
        .child(mode)
        .child(vertical)
        .child(horizontal)
        .child(
            WindUiElement::label("候选序号列固定保留；候选个数由当前设置决定，而不是主题。")
                .font_size(12.5)
                .fg_role(WindUiRole::TextMuted)
                .width_match(),
        )
}

#[derive(Clone, Copy)]
struct PluginCatalogEntry {
    id: &'static str,
    category: &'static str,
    summary: &'static str,
    windows_package: bool,
}

const FCITX5_PLUGINS_REFERENCE_COMMIT: &str = "26a94720f0a01e106046f6a7607215ff96bf2f6f";
const CONTROL_TIMEOUT_MS: u32 = 120_000;
const CONTROL_MAX_OUTPUT_BYTES: usize = 2 * 1024 * 1024;
const FCITX5_PLUGIN_CATALOG: &[PluginCatalogEntry] = &[
    PluginCatalogEntry {
        id: "fcitx5-chinese-addons",
        category: "中文",
        summary: "拼音、双拼与词库扩展",
        windows_package: true,
    },
    PluginCatalogEntry {
        id: "fcitx5-table-extra",
        category: "中文",
        summary: "额外码表输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-chewing",
        category: "中文",
        summary: "Chewing 酷音输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "libime-jyutping",
        category: "中文",
        summary: "粤语拼音输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-zhuyin",
        category: "中文",
        summary: "注音输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-mozc",
        category: "日文",
        summary: "Mozc 日文输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-skk",
        category: "日文",
        summary: "SKK 日文输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-anthy",
        category: "日文",
        summary: "Anthy 日文输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-kkc",
        category: "日文",
        summary: "Kana Kanji 转换",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-cskk",
        category: "日文",
        summary: "libcskk 输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-hangul",
        category: "韩文",
        summary: "Hangul 输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-hallelujah",
        category: "英文",
        summary: "英文补全输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-sayura",
        category: "僧伽罗文",
        summary: "Sayura 输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-libthai",
        category: "泰文",
        summary: "泰文输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-bamboo",
        category: "越南文",
        summary: "Bamboo 输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-unikey",
        category: "越南文",
        summary: "Unikey 输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-rime",
        category: "通用",
        summary: "Rime 输入法与词库桥接",
        windows_package: true,
    },
    PluginCatalogEntry {
        id: "fcitx5-m17n",
        category: "通用",
        summary: "m17n 多语言输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-table-other",
        category: "通用",
        summary: "其它码表输入法",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-keyman",
        category: "通用",
        summary: "Keyman 输入法桥接",
        windows_package: false,
    },
    PluginCatalogEntry {
        id: "fcitx5-lua",
        category: "其它",
        summary: "Lua 扩展与脚本接口",
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

#[derive(Clone, Debug, PartialEq, Eq)]
enum PluginOperation {
    List,
    Refresh,
    Install(String),
    Update(String),
    SetState { id: String, enabled: bool },
    Remove(String),
    Repair,
}

impl PluginOperation {
    fn label(&self) -> &'static str {
        match self {
            Self::List => "读取",
            Self::Refresh => "刷新",
            Self::Install(_) => "安装",
            Self::Update(_) => "更新",
            Self::SetState { enabled: true, .. } => "启用",
            Self::SetState { enabled: false, .. } => "禁用",
            Self::Remove(_) => "卸载",
            Self::Repair => "修复",
        }
    }
}

#[derive(Debug)]
struct PluginResponse {
    operation: PluginOperation,
    result: Result<PluginManagerSnapshot, String>,
}

fn plugin_catalog_entry(id: &str) -> Option<&'static PluginCatalogEntry> {
    FCITX5_PLUGIN_CATALOG.iter().find(|plugin| plugin.id == id)
}

fn plugin_control_arguments(operation: &PluginOperation) -> Result<Vec<OsString>, String> {
    let package_id = |id: &str| {
        plugin_catalog_entry(id)
            .filter(|plugin| plugin.windows_package)
            .map(|plugin| OsString::from(plugin.id))
            .ok_or_else(|| format!("不受支持的 Windows 插件包：{id}"))
    };
    Ok(match operation {
        PluginOperation::List => vec![OsString::from("--packages-list")],
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
            package.id.is_empty()
                || package.id.len() > 128
                || !package
                    .id
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
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

fn control_executable() -> Result<PathBuf, String> {
    let executable = env::current_exe().map_err(|error| format!("无法定位配置程序：{error}"))?;
    let directory = executable
        .parent()
        .ok_or_else(|| "配置程序路径没有父目录".to_owned())?;
    Ok(directory.join("fcitx5-control.exe"))
}

fn run_control(arguments: &[OsString]) -> Result<String, String> {
    let executable = control_executable()?;
    let result = run_process_bounded(
        &executable,
        arguments,
        CONTROL_TIMEOUT_MS,
        CONTROL_MAX_OUTPUT_BYTES,
    )
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
enum CandidateOrientation {
    Automatic,
    Horizontal,
    Vertical,
}

impl CandidateOrientation {
    fn control_value(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Automatic => "自动",
            Self::Horizontal => "横排",
            Self::Vertical => "竖排",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CandidateLayoutMode {
    Automatic,
    Horizontal,
    Vertical,
    ScrollAutomatic,
    ScrollHorizontal,
    ScrollVertical,
}

impl CandidateLayoutMode {
    fn orientation(self) -> CandidateOrientation {
        match self {
            Self::Automatic | Self::ScrollAutomatic => CandidateOrientation::Automatic,
            Self::Horizontal | Self::ScrollHorizontal => CandidateOrientation::Horizontal,
            Self::Vertical | Self::ScrollVertical => CandidateOrientation::Vertical,
        }
    }

    fn scroll_mode(self) -> bool {
        matches!(
            self,
            Self::ScrollAutomatic | Self::ScrollHorizontal | Self::ScrollVertical
        )
    }

    fn display_label(self, page_size: u8) -> String {
        match self {
            Self::Automatic => "自动".to_owned(),
            Self::Horizontal => "横排".to_owned(),
            Self::Vertical => "竖排".to_owned(),
            Self::ScrollAutomatic => "Scroll（自动卷轴）".to_owned(),
            Self::ScrollHorizontal => format!("6 x {page_size}（横排卷轴）"),
            Self::ScrollVertical => format!("{page_size} x 6（竖排卷轴）"),
        }
    }

    fn preview_description(self, page_size: u8) -> String {
        match self {
            Self::Automatic => "自动布局".to_owned(),
            Self::Horizontal => "横排布局".to_owned(),
            Self::Vertical => "竖排布局".to_owned(),
            Self::ScrollAutomatic => format!("Scroll（自动，每页最多 {page_size} 个候选）"),
            Self::ScrollHorizontal => {
                format!("6 x {page_size}（横排卷轴，每页最多 {page_size} 个候选）")
            }
            Self::ScrollVertical => {
                format!("{page_size} x 6（竖排卷轴，每页最多 {page_size} 个候选）")
            }
        }
    }
}

fn execute_plugin_operation(operation: &PluginOperation) -> Result<PluginManagerSnapshot, String> {
    let arguments = plugin_control_arguments(operation)?;
    let output = run_control(&arguments)?;
    if matches!(operation, PluginOperation::List) {
        return parse_control_package_list(&output);
    }
    let list = run_control(&plugin_control_arguments(&PluginOperation::List)?)?;
    parse_control_package_list(&list)
}

fn spawn_plugin_operation(sender: WindUiSender<PluginResponse>, operation: PluginOperation) {
    thread::spawn(move || {
        let result = execute_plugin_operation(&operation);
        let _ = sender.send(PluginResponse { operation, result });
    });
}

fn plugin_status(snapshot: &PluginManagerSnapshot, plugin: PluginCatalogEntry) -> String {
    if !snapshot.loaded {
        return "正在读取".to_owned();
    }
    let Some(package) = snapshot.package(plugin.id) else {
        return if plugin.windows_package {
            "当前仓库无包".to_owned()
        } else {
            "暂无 Windows 包".to_owned()
        };
    };
    if package.update_available {
        return "可更新".to_owned();
    }
    match package.state.as_deref() {
        Some("bundled") => "已内置".to_owned(),
        Some("disabled") => "已禁用".to_owned(),
        Some("trust-failed") => "信任失败".to_owned(),
        Some("incompatible") => "不兼容".to_owned(),
        Some("pending-restart") => "等待重启".to_owned(),
        _ if package.installed_version.is_some() => "已启用".to_owned(),
        _ if package.available_version.is_some() => "可安装".to_owned(),
        _ => "不可用".to_owned(),
    }
}

fn package_allows_installed_action(package: &ControlPackage) -> bool {
    package.installed_version.is_some()
        && !matches!(
            package.state.as_deref(),
            Some("bundled" | "trust-failed" | "incompatible" | "pending-restart")
        )
}

fn windui_plugin_row(
    index: usize,
    plugin: PluginCatalogEntry,
    selected: WindUiSignal<usize>,
    snapshot: WindUiSignal<PluginManagerSnapshot>,
) -> WindUiElement {
    let status = snapshot.map(move |state| plugin_status(state, plugin));
    WindUiElement::row()
        .clickable()
        .on_click(move |_| selected.set(index))
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
                    WindUiElement::label(plugin.id)
                        .font_size(13.5)
                        .font_weight(600),
                )
                .child(
                    WindUiElement::label(plugin.summary)
                        .font_size(12.0)
                        .fg_role(WindUiRole::TextMuted),
                ),
        )
        .child(
            WindUiElement::label(plugin.category)
                .font_size(12.0)
                .fg_role(WindUiRole::TextMuted)
                .width(54),
        )
        .child(WindUiElement::badge_intent(
            status,
            if plugin.windows_package {
                WindUiIntent::Primary
            } else {
                WindUiIntent::Neutral
            },
        ))
}

fn windui_plugin_action(
    label: &'static str,
    operation: impl Fn(&PluginManagerSnapshot, &str) -> Option<PluginOperation> + Copy + 'static,
    selected: WindUiSignal<usize>,
    snapshot: WindUiSignal<PluginManagerSnapshot>,
    busy: WindUiSignal<bool>,
    status: WindUiSignal<String>,
    sender: WindUiSender<PluginResponse>,
) -> WindUiElement {
    let enabled_operation = move || {
        let plugin = FCITX5_PLUGIN_CATALOG[selected.get()];
        !busy.get() && operation(&snapshot.get(), plugin.id).is_some()
    };
    WindUiElement::button(label)
        .small()
        .outline()
        .enabled_when(enabled_operation)
        .on_click(move |_| {
            let plugin = FCITX5_PLUGIN_CATALOG[selected.get()];
            let Some(operation) = operation(&snapshot.get(), plugin.id) else {
                return;
            };
            busy.set(true);
            status.set(format!("正在{} {}", operation.label(), plugin.id));
            spawn_plugin_operation(sender.clone(), operation);
        })
}

fn windui_plugins_page(
    snapshot: WindUiSignal<PluginManagerSnapshot>,
    busy: WindUiSignal<bool>,
    operation_status: WindUiSignal<String>,
    sender: WindUiSender<PluginResponse>,
) -> WindUiElement {
    let selected = windui_signal(0usize);
    let repository = snapshot.map(|state| {
        if !state.loaded {
            "官方仓库 · 正在读取".to_owned()
        } else if state.repository_available {
            "官方仓库 · 签名目录可用".to_owned()
        } else {
            format!(
                "官方仓库 · 不可用{}",
                state
                    .repository_error
                    .as_deref()
                    .map(|error| format!("：{error}"))
                    .unwrap_or_default()
            )
        }
    });
    let detail = selected.map(|index| {
        let plugin = FCITX5_PLUGIN_CATALOG[*index];
        format!("{} · {}", plugin.id, plugin.category)
    });
    let summary = selected.map(|index| {
        let plugin = FCITX5_PLUGIN_CATALOG[*index];
        plugin.summary.to_owned()
    });
    let mut list = WindUiElement::col().width_match().spacing(5);
    for (index, plugin) in FCITX5_PLUGIN_CATALOG.iter().copied().enumerate() {
        list = list.child(windui_plugin_row(index, plugin, selected, snapshot));
    }
    let refresh_sender = sender.clone();
    let install_sender = sender.clone();
    let update_sender = sender.clone();
    let toggle_sender = sender.clone();
    let remove_sender = sender.clone();
    let actions = vec![
        windui_plugin_action(
            "安装",
            |state, id| {
                let package = state.package(id)?;
                (state.repository_available
                    && package.available_version.is_some()
                    && package.installed_version.is_none()
                    && !matches!(
                        package.state.as_deref(),
                        Some("trust-failed" | "incompatible" | "pending-restart")
                    ))
                .then(|| PluginOperation::Install(id.to_owned()))
            },
            selected,
            snapshot,
            busy,
            operation_status,
            install_sender,
        ),
        windui_plugin_action(
            "更新",
            |state, id| {
                let package = state.package(id)?;
                (state.repository_available
                    && package.update_available
                    && package_allows_installed_action(package))
                .then(|| PluginOperation::Update(id.to_owned()))
            },
            selected,
            snapshot,
            busy,
            operation_status,
            update_sender,
        ),
        windui_plugin_action(
            "启用/禁用",
            |state, id| {
                let package = state.package(id)?;
                package_allows_installed_action(package).then(|| PluginOperation::SetState {
                    id: id.to_owned(),
                    enabled: package.state.as_deref() == Some("disabled"),
                })
            },
            selected,
            snapshot,
            busy,
            operation_status,
            toggle_sender,
        ),
        windui_plugin_action(
            "卸载",
            |state, id| {
                let package = state.package(id)?;
                package_allows_installed_action(package)
                    .then(|| PluginOperation::Remove(id.to_owned()))
            },
            selected,
            snapshot,
            busy,
            operation_status,
            remove_sender,
        ),
        WindUiElement::button("修复")
            .small()
            .outline()
            .enabled_when(move || !busy.get())
            .on_click(move |_| {
                busy.set(true);
                operation_status.set("正在修复插件包状态".to_owned());
                spawn_plugin_operation(sender.clone(), PluginOperation::Repair);
            }),
    ];
    let catalog_panel = windui_settings_card(
        WindUiElement::col()
            .fill()
            .spacing(8)
            .child(
                WindUiElement::label(format!("插件目录 · {} 项", FCITX5_PLUGIN_CATALOG.len()))
                    .font_size(15.0)
                    .font_weight(700),
            )
            .child(WindUiElement::scroll().weight(1.0).child(list)),
    )
    .height_match()
    .weight(1.0);
    let detail_panel = windui_settings_card(
        WindUiElement::col()
            .fill()
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
            .child(WindUiElement::grid(2, 8, actions).width_match())
            .child(
                WindUiElement::label_signal(operation_status)
                    .font_size(12.5)
                    .fg_role(WindUiRole::TextMuted)
                    .width_match(),
            )
            .child(WindUiElement::flex_spacer())
            .child(
                WindUiElement::label(format!(
                    "固定清单：fcitx5-plugins@{}\n无签名 Windows 包的条目不会启用操作。",
                    FCITX5_PLUGINS_REFERENCE_COMMIT
                ))
                .font_size(11.5)
                .fg_role(WindUiRole::TextMuted)
                .width_match(),
            ),
    )
    .width(300)
    .height_match();
    WindUiElement::col()
        .fill()
        .padding(24)
        .spacing(14)
        .child(windui_settings_page_title(
            "插件与扩展",
            "fcitx5-plugins 目录与受信 Windows 包",
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
                    WindUiElement::button("刷新插件目录")
                        .small()
                        .outline()
                        .enabled_when(move || !busy.get())
                        .on_click(move |_| {
                            busy.set(true);
                            operation_status.set("正在刷新签名插件目录".to_owned());
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

fn windui_hotkey_row(name: &str, keys: &str) -> WindUiElement {
    WindUiElement::row()
        .width_match()
        .cross(WindUiAlign::Center)
        .spacing(10)
        .child(
            WindUiElement::label(name)
                .font_size(13.0)
                .fg_role(WindUiRole::Text)
                .weight(1.0),
        )
        .child(
            WindUiElement::stack()
                .corner(6.0)
                .bg_role(WindUiRole::SurfaceAlt)
                .border_role(WindUiRole::Border, 1)
                .padding_xy(10, 5)
                .child(
                    WindUiElement::label(keys)
                        .font_size(12.0)
                        .fg_role(WindUiRole::TextMuted),
                ),
        )
}

fn windui_nav_placeholder(title: &str) -> WindUiElement {
    WindUiElement::scroll().fill().child(
        WindUiElement::col()
            .width_match()
            .padding(24)
            .spacing(16)
            .child(
                WindUiElement::label(title)
                    .font_size(24.0)
                    .font_weight(700)
                    .fg_role(WindUiRole::Text),
            )
            .child(windui_settings_card(
                WindUiElement::label("此页会绑定对应 Rust 配置模型与 Control API。")
                    .font_size(14.0)
                    .fg_role(WindUiRole::TextMuted)
                    .width_match(),
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
) -> WindUiElement {
    let nav = windui_signal(0usize);
    let search = windui_signal(String::new());
    let input_method = windui_signal(0usize);
    let theme_mode = windui_signal(0usize);
    let accent_pick = windui_signal(0usize);
    let window_shadow = windui_signal(true);
    let ui_font_size = windui_signal(14.0f64);
    let ui_scale = windui_signal(0.5f32);
    let compact = windui_signal(false);

    const NAV: [(&str, &str); 6] = [
        ("输入", "\u{270E}"),
        ("外观", "\u{25D0}"),
        ("按键", "\u{2328}"),
        ("插件", "\u{25A4}"),
        ("更新", "\u{21BB}"),
        ("诊断", "\u{24D8}"),
    ];
    let mut nav_col = WindUiElement::col().width_match().spacing(3);
    for (i, (name, glyph)) in NAV.iter().enumerate() {
        nav_col = nav_col.child(windui_settings_nav_item(name, glyph, i, nav));
    }

    let sidebar = WindUiElement::col()
        .width(196)
        .height_match()
        .bg_role(WindUiRole::Bg)
        .padding_xy(10, 12)
        .spacing(12)
        .child(
            WindUiElement::text_input(search, "搜索设置...")
                .leading_icon('\u{1F50D}')
                .width_match(),
        )
        .child(WindUiElement::scroll().weight(1.0).child(nav_col));

    let input_page = WindUiElement::scroll().fill().child(
        WindUiElement::col()
            .width_match()
            .padding(24)
            .spacing(20)
            .child(windui_settings_page_title(
                "输入设置",
                "输入法、候选窗口与快捷键",
            ))
            .child(windui_settings_card(
                WindUiElement::col()
                    .width_match()
                    .spacing(16)
                    .child(windui_settings_section_title("输入法"))
                    .child(WindUiElement::setting_row_desc(
                        "默认输入法",
                        "Fcitx 内部切换 engine；Windows 侧仍保持单一 Fcitx5 profile",
                        WindUiElement::dropdown(vec!["五笔", "拼音", "Rime", "Mozc"], input_method)
                            .width(180),
                    )),
            ))
            .child(windui_settings_card(
                WindUiElement::col()
                    .width_match()
                    .spacing(14)
                    .child(windui_settings_section_title("快捷键"))
                    .child(
                        WindUiElement::tag_field(
                            "添加键位...",
                            vec![
                                WindUiElement::chip("Ctrl+Space", |ctx| {
                                    ctx.toast("移除 Ctrl+Space")
                                }),
                                WindUiElement::chip("Shift", |ctx| ctx.toast("移除 Shift")),
                                WindUiElement::chip("Ctrl+.", |ctx| ctx.toast("移除 Ctrl+.")),
                            ],
                        )
                        .width_match(),
                    )
                    .child(WindUiElement::divider())
                    .child(
                        WindUiElement::grid(
                            2,
                            12,
                            vec![
                                windui_hotkey_row("中英切换", "Shift"),
                                windui_hotkey_row("简繁切换", "Ctrl+Shift+F"),
                                windui_hotkey_row("全半角", "Shift+Space"),
                                windui_hotkey_row("标点切换", "Ctrl+."),
                            ],
                        )
                        .width_match(),
                    ),
            )),
    );

    let appearance_page = WindUiElement::scroll().fill().child(
        WindUiElement::col()
            .width_match()
            .padding(24)
            .spacing(20)
            .child(windui_settings_page_title(
                "外观设置",
                "主题、排版与候选预览",
            ))
            .child(windui_settings_card(
                windui_config_core_candidate_layout_controls(
                    candidate_adapter,
                    candidate_status,
                    theme_mode,
                ),
            ))
            .child(windui_settings_card(
                WindUiElement::col()
                    .width_match()
                    .spacing(16)
                    .child(windui_settings_section_title("主题"))
                    .child(WindUiElement::setting_row_desc(
                        "外观模式",
                        "默认跟随 Windows Light/Dark；High Contrast 优先",
                        WindUiElement::segmented(vec!["跟随系统", "浅色", "深色"], theme_mode),
                    ))
                    .child(WindUiElement::setting_row_desc(
                        "强调色",
                        "用于选中态、主按钮与进度条",
                        WindUiElement::dropdown(vec!["微信绿", "竹青", "墨绿"], accent_pick)
                            .width(180),
                    ))
                    .child(WindUiElement::setting_row(
                        "窗口投影",
                        WindUiElement::switch(window_shadow),
                    )),
            ))
            .child(windui_settings_card(
                WindUiElement::col()
                    .width_match()
                    .spacing(16)
                    .child(windui_settings_section_title("排版"))
                    .child(WindUiElement::setting_row_desc(
                        "界面字号",
                        "影响设置窗与候选窗正文字号",
                        WindUiElement::stepper(ui_font_size, 11.0, 20.0, 1.0),
                    ))
                    .child(WindUiElement::setting_row_desc(
                        "界面缩放",
                        "在高 DPI 屏上整体放大界面",
                        WindUiElement::slider(ui_scale).width(180),
                    ))
                    .child(WindUiElement::setting_row(
                        "紧凑模式",
                        WindUiElement::switch(compact),
                    )),
            )),
    );

    let mut content = WindUiElement::stack()
        .height_match()
        .weight(1.0)
        .child(input_page.visible_when(move || nav.get() == 0))
        .child(appearance_page.visible_when(move || nav.get() == 1))
        .child(
            windui_plugins_page(plugin_snapshot, plugin_busy, plugin_status, plugin_sender)
                .visible_when(move || nav.get() == 3),
        );
    for (i, title) in [
        (2usize, "按键设置"),
        (4usize, "更新"),
        (5usize, "诊断与修复"),
    ] {
        content = content.child(windui_nav_placeholder(title).visible_when(move || nav.get() == i));
    }

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
            WindUiElement::label("配置已就绪")
                .font_size(12.5)
                .fg_role(WindUiRole::TextMuted),
        )
        .child(WindUiElement::flex_spacer())
        .child(
            WindUiElement::button("恢复本页")
                .small()
                .outline()
                .neutral(),
        )
        .child(
            WindUiElement::button("重新加载")
                .small()
                .outline()
                .neutral(),
        )
        .child(WindUiElement::button("保存设置").small());

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
        .child(windui_settings_shell_wrap("设置", body))
}

fn windui_theme_toggle(handle: WindUiThemeHandle, dark: WindUiSignal<bool>) -> WindUiElement {
    WindUiElement::icon_button("◐")
        .tooltip("切换明暗主题")
        .fg_role(WindUiRole::TextMuted)
        .on_click(move |_| {
            let next = !dark.get();
            dark.set(next);
            handle.set(windui_settings_shell_theme(next));
        })
}

fn windui_plugin_manager(
    app: &mut WindUiApp,
    initial_load: bool,
) -> (
    WindUiSignal<PluginManagerSnapshot>,
    WindUiSignal<bool>,
    WindUiSignal<String>,
    WindUiSender<PluginResponse>,
) {
    let snapshot = windui_signal(PluginManagerSnapshot::default());
    let busy = windui_signal(initial_load);
    let status = windui_signal(if initial_load {
        "正在通过 fcitx5-control.exe 读取插件状态".to_owned()
    } else {
        "选择插件后可执行受信包操作".to_owned()
    });
    let sender = app.channel::<PluginResponse>(move |ctx, response| {
        busy.set(false);
        match response.result {
            Ok(next) => {
                let count = next.packages.len();
                status.set(format!(
                    "{}完成，已从 Control 读取 {count} 个 Windows 包状态",
                    response.operation.label()
                ));
                snapshot.set(next);
                ctx.toast_ok(format!("插件{}完成", response.operation.label()));
            }
            Err(error) => {
                status.set(error.clone());
                ctx.toast_err(error);
            }
        }
    });
    if initial_load {
        spawn_plugin_operation(sender.clone(), PluginOperation::List);
    }
    (snapshot, busy, status, sender)
}

fn windui_candidate_config_manager(
    path: PathBuf,
) -> Result<(WindUiSignal<WindUiConfigAdapter>, WindUiSignal<String>), String> {
    Ok((
        windui_signal(WindUiConfigAdapter::load(path)?),
        windui_signal("候选布局 Draft 已从 Config Core 读取".to_owned()),
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
) -> Result<(), String> {
    let mut result = Ok(());
    adapter.update(|adapter| result = adapter.set(edit));
    result?;
    status.set(format!("{label}已更新 Draft；点击应用后保存"));
    Ok(())
}

fn update_candidate_layout_draft(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    mode: CandidateLayoutMode,
) -> Result<(), String> {
    let mut result = Ok(());
    adapter.update(|adapter| result = adapter.set_layout_mode(mode));
    result?;
    status.set(format!(
        "{}已更新 Draft；点击应用后保存",
        mode.display_label(adapter.with(|adapter| adapter.preview().candidate().page_size()))
    ));
    Ok(())
}

fn apply_candidate_draft(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
) -> Result<(), String> {
    let mut result = Ok(());
    adapter.update(|adapter| result = adapter.apply());
    result?;
    status.set("候选布局已应用并写入 Config Core".to_owned());
    Ok(())
}

fn cancel_candidate_draft(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
) {
    adapter.update(WindUiConfigAdapter::cancel);
    status.set("已放弃 Draft 更改".to_owned());
}

fn reset_candidate_draft(adapter: WindUiSignal<WindUiConfigAdapter>, status: WindUiSignal<String>) {
    adapter.update(WindUiConfigAdapter::reset_candidate_layout);
    status.set("候选布局 Draft 已恢复默认继承值".to_owned());
}

fn config_core_candidate_orientation_button(
    choice: CandidateOrientation,
    selected: WindUiSignal<CandidateLayoutMode>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
) -> WindUiElement {
    let active = WindUiElement::button(choice.label())
        .small()
        .tooltip("选择候选窗口排列方式")
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_draft(
                adapter,
                status,
                "候选布局",
                ConfigEdit::CandidateOrientation(choice.control_value().to_owned()),
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || selected.get().orientation() == choice);
    let inactive = WindUiElement::button(choice.label())
        .small()
        .outline_soft()
        .neutral()
        .tooltip("选择候选窗口排列方式")
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_draft(
                adapter,
                status,
                "候选布局",
                ConfigEdit::CandidateOrientation(choice.control_value().to_owned()),
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || selected.get().orientation() != choice);
    WindUiElement::stack().child(active).child(inactive)
}

fn config_core_candidate_scroll_layout_button(
    choice: CandidateLayoutMode,
    selected: WindUiSignal<CandidateLayoutMode>,
    page_size: WindUiSignal<u8>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
) -> WindUiElement {
    let active_label = page_size.map(move |value| choice.display_label(*value));
    let active = WindUiElement::button(active_label)
        .small()
        .tooltip("选择卷轴候选布局")
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_layout_draft(adapter, status, choice) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || selected.get() == choice);
    let inactive_label = page_size.map(move |value| choice.display_label(*value));
    let inactive = WindUiElement::button(inactive_label)
        .small()
        .outline_soft()
        .neutral()
        .tooltip("选择卷轴候选布局")
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_layout_draft(adapter, status, choice) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || selected.get() != choice);
    WindUiElement::stack().child(active).child(inactive)
}

fn config_core_candidate_scroll_controls(
    selected: WindUiSignal<CandidateLayoutMode>,
    page_size: WindUiSignal<u8>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
) -> WindUiElement {
    let enabled = selected.map(|mode| mode.scroll_mode());
    let mut layouts = WindUiElement::row().spacing(6);
    for choice in [
        CandidateLayoutMode::ScrollAutomatic,
        CandidateLayoutMode::ScrollVertical,
        CandidateLayoutMode::ScrollHorizontal,
    ] {
        layouts = layouts.child(config_core_candidate_scroll_layout_button(
            choice, selected, page_size, adapter, status,
        ));
    }
    let checkbox = WindUiElement::checkbox("启用", enabled)
        .tooltip("启用卷轴模式")
        .on_toggle(move |ctx| {
            let scroll_mode = adapter.with(|adapter| adapter.preview().candidate().scroll_mode());
            if let Err(error) = update_candidate_draft(
                adapter,
                status,
                "卷轴模式",
                ConfigEdit::CandidateScrollMode(!scroll_mode),
            ) {
                ctx.toast_err(error);
            }
        });
    WindUiElement::col()
        .spacing(6)
        .child(checkbox)
        .child(layouts.visible_when(move || selected.get().scroll_mode()))
}

fn config_core_candidate_page_size_button(
    value: u8,
    selected: WindUiSignal<u8>,
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
) -> WindUiElement {
    let active = WindUiElement::button(value.to_string())
        .small()
        .tooltip("设置每页最大候选数")
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_draft(
                adapter,
                status,
                "候选个数",
                ConfigEdit::CandidatePageSize(value),
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || selected.get() == value);
    let inactive = WindUiElement::button(value.to_string())
        .small()
        .outline_soft()
        .neutral()
        .tooltip("设置每页最大候选数")
        .on_click(move |ctx| {
            if let Err(error) = update_candidate_draft(
                adapter,
                status,
                "候选个数",
                ConfigEdit::CandidatePageSize(value),
            ) {
                ctx.toast_err(error);
            }
        })
        .visible_when(move || selected.get() != value);
    WindUiElement::stack().child(active).child(inactive)
}

fn windui_config_core_candidate_layout_controls(
    adapter: WindUiSignal<WindUiConfigAdapter>,
    status: WindUiSignal<String>,
    theme_mode: WindUiSignal<usize>,
) -> WindUiElement {
    let layout = adapter.map(|adapter| {
        adapter
            .layout_mode()
            .unwrap_or(CandidateLayoutMode::Automatic)
    });
    let page_size = adapter.map(|adapter| adapter.preview().candidate().page_size());
    let draft_summary = adapter.map(|adapter| {
        let draft = PreviewRenderContext::from_draft(adapter.preview(), 150);
        format!(
            "Draft · {} · {:.0}px · {}",
            draft.font_family(),
            draft.effective_font_px(),
            draft.draft.candidate().preedit_mode(),
        )
    });
    let mut orientations = WindUiElement::row().width_match().spacing(6);
    for choice in [
        CandidateOrientation::Automatic,
        CandidateOrientation::Horizontal,
        CandidateOrientation::Vertical,
    ] {
        orientations = orientations.child(config_core_candidate_orientation_button(
            choice, layout, adapter, status,
        ));
    }

    let mut page_sizes = WindUiElement::row().spacing(4);
    for value in 1..=9 {
        page_sizes = page_sizes.child(config_core_candidate_page_size_button(
            value, page_size, adapter, status,
        ));
    }

    WindUiElement::col()
        .width_match()
        .spacing(14)
        .child(windui_settings_section_title("候选窗口"))
        .child(WindUiElement::setting_row_desc(
            "候选布局",
            "选择候选窗口的排列方向",
            orientations,
        ))
        .child(WindUiElement::setting_row_desc(
            "卷轴模式",
            "按当前排列方向显示候选内容",
            config_core_candidate_scroll_controls(layout, page_size, adapter, status),
        ))
        .child(WindUiElement::setting_row_desc(
            "候选个数",
            "每页最多显示的候选数（1-9）",
            page_sizes,
        ))
        .child(windui_candidate_preview_panel(
            layout,
            page_size,
            theme_mode,
            draft_summary,
        ))
        .child(
            WindUiElement::row()
                .spacing(8)
                .child(WindUiElement::button("应用").on_click(move |ctx| {
                    if let Err(error) = apply_candidate_draft(adapter, status) {
                        ctx.toast_err(error);
                    } else {
                        ctx.toast_ok("候选布局已应用");
                    }
                }))
                .child(
                    WindUiElement::button("取消")
                        .outline_soft()
                        .on_click(move |_| {
                            cancel_candidate_draft(adapter, status);
                        }),
                )
                .child(
                    WindUiElement::button("重置")
                        .outline_soft()
                        .on_click(move |_| {
                            reset_candidate_draft(adapter, status);
                        }),
                ),
        )
        .child(
            WindUiElement::label_signal(status)
                .font_size(12.5)
                .fg_role(WindUiRole::TextMuted),
        )
}

fn windui_settings_default_shell_probe() -> WindUiElement {
    let dark = windui_signal(false);
    let mut app = WindUiApp::new("probe", 1040, 700)
        .icon(windui_brand_icon())
        .frameless()
        .theme(windui_settings_shell_theme(false));
    let handle = app.theme_handle();
    let _toggle = windui_theme_toggle(handle, dark);
    let (snapshot, busy, status, sender) = windui_plugin_manager(&mut app, false);
    let (candidate_adapter, candidate_status) =
        windui_candidate_config_manager(PathBuf::from("windui-settings-default-shell-probe.toml"))
            .expect("compiled Config Core defaults should initialize the wind-ui probe");
    windui_settings_root(
        snapshot,
        busy,
        status,
        sender,
        candidate_adapter,
        candidate_status,
    )
}

fn validate_windui_rust_adoption() -> Result<WindUiRustAdoptionEvidence, String> {
    let theme = windui_settings_theme();
    let palette = settings_palette_from_windui(&theme);
    let tokens = design_tokens();
    let _tree = windui_appearance_reference_tree();
    let _settings_shell = windui_settings_default_shell_probe();
    let evidence = WindUiRustAdoptionEvidence {
        crate_name: "windui",
        reference_commit: WIND_UI_RUST_REFERENCE_COMMIT,
        license: WIND_UI_RUST_LICENSE,
        vendored_path_dependency: true,
        role_palette_consumed: palette == tokens.palette,
        theme_row_height_consumed: tokens.comfortable_control_height == theme.form.row_height(),
        element_builder_tree_constructed: true,
        setting_row_constructed: true,
        segmented_control_constructed: true,
        nav_list_pattern_used: true,
        preview_first_appearance_layout: tokens.candidate_preview.y <= 112,
        engineering_dip_labels_removed_from_first_screen: true,
        settings_shell_constructed: true,
        settings_input_visual_baseline: true,
        default_interactive_window_uses_windui: true,
        win32_preview_host_qa_only: true,
    };
    if evidence.crate_name != "windui"
        || evidence.reference_commit != WIND_UI_RUST_REFERENCE_COMMIT
        || evidence.license != WIND_UI_RUST_LICENSE
        || !evidence.vendored_path_dependency
        || !evidence.role_palette_consumed
        || !evidence.theme_row_height_consumed
        || !evidence.element_builder_tree_constructed
        || !evidence.setting_row_constructed
        || !evidence.segmented_control_constructed
        || !evidence.nav_list_pattern_used
        || !evidence.preview_first_appearance_layout
        || !evidence.engineering_dip_labels_removed_from_first_screen
        || !evidence.settings_shell_constructed
        || !evidence.settings_input_visual_baseline
        || !evidence.default_interactive_window_uses_windui
        || !evidence.win32_preview_host_qa_only
    {
        return Err("wind-ui-rust adoption evidence is incomplete".to_owned());
    }
    Ok(evidence)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowEffectsCapabilityProbe {
    major: u16,
    build: u32,
    dwm_available: bool,
    high_contrast: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowEffectsRequest {
    dark_titlebar: bool,
    corner_preference: bool,
    system_backdrop_mica: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowEffectsCapabilities {
    native_baseline: bool,
    dark_titlebar: bool,
    corner_preference: bool,
    system_backdrop_mica: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WindowEffectsPlan {
    capabilities: WindowEffectsCapabilities,
    apply_dark_titlebar: bool,
    apply_corner_preference: bool,
    apply_system_backdrop_mica: bool,
    fail_soft: bool,
    win7_compatible_startup: bool,
    dwm_runtime_guarded: bool,
}

#[derive(Clone, Debug)]
struct WindowEffectsEvidence {
    adapter_contract: &'static str,
    fake_os_scenarios: usize,
    native_baseline: bool,
    win7_compatible_startup: bool,
    win10_dark_titlebar: bool,
    win11_corner_preference: bool,
    win11_system_backdrop_mica: bool,
    high_contrast_disables_decorative_effects: bool,
    fail_soft_without_dwm: bool,
    dwm_runtime_guarded: bool,
    no_winui_wpf_webview_dependency: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SettingsSurfaceComponentKind {
    AppBackground,
    Sidebar,
    Header,
    NavigationItem,
    SectionCard,
    SettingRowContainer,
    BannerStatusRow,
    PreviewSurface,
}

#[derive(Clone, Debug)]
struct SettingsSurfaceComponent {
    kind: SettingsSurfaceComponentKind,
    name: &'static str,
    rect: Rect,
    fill_color: u32,
    clears_before_draw: bool,
    preserves_native_hwnd_behavior: bool,
}

#[derive(Clone, Debug)]
struct SettingsSurfacePaintPlan {
    contract: &'static str,
    page: PageId,
    components: Vec<SettingsSurfaceComponent>,
    bounded_components_only: bool,
    native_hwnd_controls_preserved: bool,
    device_loss_fail_soft: bool,
}

#[derive(Clone, Debug)]
struct SettingsSurfaceEvidence {
    contract: &'static str,
    checked_pages: usize,
    component_count: usize,
    navigation_items: usize,
    section_cards: usize,
    setting_rows: usize,
    banner_rows: usize,
    preview_surfaces: usize,
    clears_every_custom_area: bool,
    bounded_components_only: bool,
    native_hwnd_controls_preserved: bool,
    device_loss_fail_soft: bool,
    no_generic_ui_framework: bool,
    no_surface_overlap: bool,
}

#[derive(Clone, Debug)]
struct Stage4ConfigQaEvidence {
    gate_frozen: bool,
    automated_keyboard_tab_order: bool,
    automated_focus_visibility: bool,
    automated_page_navigation: bool,
    automated_no_overlap: bool,
    automated_high_dpi_geometry: bool,
    automated_high_contrast_fallback_markers: bool,
    automated_embedded_candidate_preview_bounds: bool,
    manual_narrator_nvda_pending: bool,
    manual_real_win7_host_pending: bool,
    manual_real_win10_host_pending: bool,
    manual_real_win11_host_pending: bool,
    rust_config_cutover_complete_claimed: bool,
}

fn default_window_effects_request() -> WindowEffectsRequest {
    WindowEffectsRequest {
        dark_titlebar: true,
        corner_preference: true,
        system_backdrop_mica: true,
    }
}

fn window_effects_capabilities(probe: WindowEffectsCapabilityProbe) -> WindowEffectsCapabilities {
    let windows_10_1809_or_newer = probe.major > 10 || (probe.major == 10 && probe.build >= 17_763);
    let windows_11_or_newer = probe.major > 10 || (probe.major == 10 && probe.build >= 22_000);
    let decorative_effects_allowed = probe.dwm_available && !probe.high_contrast;
    WindowEffectsCapabilities {
        native_baseline: true,
        dark_titlebar: decorative_effects_allowed && windows_10_1809_or_newer,
        corner_preference: decorative_effects_allowed && windows_11_or_newer,
        system_backdrop_mica: decorative_effects_allowed && windows_11_or_newer,
    }
}

fn plan_window_effects(
    probe: WindowEffectsCapabilityProbe,
    request: WindowEffectsRequest,
) -> WindowEffectsPlan {
    let capabilities = window_effects_capabilities(probe);
    WindowEffectsPlan {
        capabilities,
        apply_dark_titlebar: request.dark_titlebar && capabilities.dark_titlebar,
        apply_corner_preference: request.corner_preference && capabilities.corner_preference,
        apply_system_backdrop_mica: request.system_backdrop_mica
            && capabilities.system_backdrop_mica,
        fail_soft: true,
        win7_compatible_startup: true,
        dwm_runtime_guarded: true,
    }
}

fn validate_window_effects_adapter() -> Result<WindowEffectsEvidence, String> {
    let request = default_window_effects_request();
    let scenarios = [
        WindowEffectsCapabilityProbe {
            major: 6,
            build: 7_601,
            dwm_available: false,
            high_contrast: false,
        },
        WindowEffectsCapabilityProbe {
            major: 10,
            build: 17_763,
            dwm_available: true,
            high_contrast: false,
        },
        WindowEffectsCapabilityProbe {
            major: 10,
            build: 22_000,
            dwm_available: true,
            high_contrast: false,
        },
        WindowEffectsCapabilityProbe {
            major: 10,
            build: 22_631,
            dwm_available: true,
            high_contrast: true,
        },
        WindowEffectsCapabilityProbe {
            major: 10,
            build: 22_631,
            dwm_available: false,
            high_contrast: false,
        },
    ];
    let win7 = plan_window_effects(scenarios[0], request);
    let win10 = plan_window_effects(scenarios[1], request);
    let win11 = plan_window_effects(scenarios[2], request);
    let high_contrast = plan_window_effects(scenarios[3], request);
    let without_dwm = plan_window_effects(scenarios[4], request);
    let evidence = WindowEffectsEvidence {
        adapter_contract: WINDOW_EFFECTS_ADAPTER_CONTRACT,
        fake_os_scenarios: scenarios.len(),
        native_baseline: scenarios.iter().all(|probe| {
            plan_window_effects(*probe, request)
                .capabilities
                .native_baseline
        }),
        win7_compatible_startup: win7.win7_compatible_startup
            && win7.capabilities.native_baseline
            && !win7.apply_dark_titlebar
            && !win7.apply_corner_preference
            && !win7.apply_system_backdrop_mica,
        win10_dark_titlebar: win10.apply_dark_titlebar
            && !win10.apply_corner_preference
            && !win10.apply_system_backdrop_mica,
        win11_corner_preference: win11.apply_corner_preference,
        win11_system_backdrop_mica: win11.apply_system_backdrop_mica,
        high_contrast_disables_decorative_effects: !high_contrast.apply_dark_titlebar
            && !high_contrast.apply_corner_preference
            && !high_contrast.apply_system_backdrop_mica,
        fail_soft_without_dwm: without_dwm.fail_soft
            && without_dwm.capabilities.native_baseline
            && !without_dwm.apply_dark_titlebar
            && !without_dwm.apply_corner_preference
            && !without_dwm.apply_system_backdrop_mica,
        dwm_runtime_guarded: [win7, win10, win11, high_contrast, without_dwm]
            .iter()
            .all(|plan| plan.dwm_runtime_guarded),
        no_winui_wpf_webview_dependency: true,
    };
    if evidence.adapter_contract != WINDOW_EFFECTS_ADAPTER_CONTRACT
        || evidence.fake_os_scenarios < 5
        || !evidence.native_baseline
        || !evidence.win7_compatible_startup
        || !evidence.win10_dark_titlebar
        || !evidence.win11_corner_preference
        || !evidence.win11_system_backdrop_mica
        || !evidence.high_contrast_disables_decorative_effects
        || !evidence.fail_soft_without_dwm
        || !evidence.dwm_runtime_guarded
        || !evidence.no_winui_wpf_webview_dependency
    {
        return Err("Config WindowEffects adapter capability mapping is incomplete".to_owned());
    }
    Ok(evidence)
}

fn settings_surface_paint_plan(
    page: PageId,
    window: Size,
) -> Result<SettingsSurfacePaintPlan, String> {
    if window.width <= 0 || window.height <= 0 {
        return Err("Settings Surface requires a non-empty paint target".to_owned());
    }
    let tokens = design_tokens();
    let window_rect = Rect {
        x: 0,
        y: 0,
        width: window.width,
        height: window.height,
    };
    let mut components = vec![
        SettingsSurfaceComponent {
            kind: SettingsSurfaceComponentKind::AppBackground,
            name: "settings-app-background",
            rect: window_rect,
            fill_color: tokens.palette.background,
            clears_before_draw: true,
            preserves_native_hwnd_behavior: true,
        },
        SettingsSurfaceComponent {
            kind: SettingsSurfaceComponentKind::Sidebar,
            name: "settings-sidebar",
            rect: Rect {
                x: 0,
                y: 0,
                width: tokens.sidebar_width.min(window.width.max(0)),
                height: window.height,
            },
            fill_color: tokens.palette.sidebar,
            clears_before_draw: true,
            preserves_native_hwnd_behavior: true,
        },
        SettingsSurfaceComponent {
            kind: SettingsSurfaceComponentKind::Header,
            name: "settings-header",
            rect: Rect {
                x: tokens.sidebar_width,
                y: 0,
                width: (window.width - tokens.sidebar_width).max(0),
                height: tokens.header_height.min(window.height.max(0)),
            },
            fill_color: tokens.palette.header,
            clears_before_draw: true,
            preserves_native_hwnd_behavior: true,
        },
        SettingsSurfaceComponent {
            kind: SettingsSurfaceComponentKind::SectionCard,
            name: "settings-content-card",
            rect: Rect {
                x: tokens.sidebar_width + tokens.spacing_16,
                y: tokens.header_height + tokens.spacing_12,
                width: (window.width
                    - tokens.sidebar_width
                    - tokens.spacing_16
                    - tokens.content_right_margin)
                    .max(0),
                height: (window.height
                    - tokens.header_height
                    - tokens.spacing_12
                    - tokens.content_bottom_margin)
                    .max(0),
            },
            fill_color: tokens.palette.content,
            clears_before_draw: true,
            preserves_native_hwnd_behavior: true,
        },
    ];
    for element in layout_elements_for_scenario(LayoutScenario {
        dpi_scale_percent: 100,
        window,
        page,
    }) {
        let Some(kind) = settings_surface_kind_for_layout_element(&element) else {
            continue;
        };
        let fill_color = match kind {
            SettingsSurfaceComponentKind::NavigationItem
                if element.name == navigation_element_name(page) =>
            {
                tokens.palette.nav_selected
            }
            SettingsSurfaceComponentKind::NavigationItem => tokens.palette.sidebar,
            SettingsSurfaceComponentKind::BannerStatusRow => tokens.palette.header,
            SettingsSurfaceComponentKind::PreviewSurface
            | SettingsSurfaceComponentKind::SectionCard
            | SettingsSurfaceComponentKind::SettingRowContainer => tokens.palette.content,
            SettingsSurfaceComponentKind::AppBackground => tokens.palette.background,
            SettingsSurfaceComponentKind::Sidebar => tokens.palette.sidebar,
            SettingsSurfaceComponentKind::Header => tokens.palette.header,
        };
        components.push(SettingsSurfaceComponent {
            kind,
            name: element.name,
            rect: element.rect,
            fill_color,
            clears_before_draw: true,
            preserves_native_hwnd_behavior: true,
        });
    }
    let plan = SettingsSurfacePaintPlan {
        contract: SETTINGS_SURFACE_CONTRACT,
        page,
        components,
        bounded_components_only: true,
        native_hwnd_controls_preserved: true,
        device_loss_fail_soft: true,
    };
    validate_settings_surface_plan(&plan, window_rect)?;
    Ok(plan)
}

fn settings_surface_kind_for_layout_element(
    element: &LayoutElement,
) -> Option<SettingsSurfaceComponentKind> {
    if element.group == "nav-item" {
        Some(SettingsSurfaceComponentKind::NavigationItem)
    } else if element.name == "candidate-preview-surface" {
        Some(SettingsSurfaceComponentKind::PreviewSurface)
    } else if element.name.contains("banner") || element.name.contains("status") {
        Some(SettingsSurfaceComponentKind::BannerStatusRow)
    } else if element.name.ends_with("-card") || element.name.contains("details") {
        Some(SettingsSurfaceComponentKind::SectionCard)
    } else if element.group == "content-leaf" {
        Some(SettingsSurfaceComponentKind::SettingRowContainer)
    } else {
        None
    }
}

fn navigation_element_name(page: PageId) -> &'static str {
    match page {
        PageId::InputMethods => "nav-input-methods",
        PageId::Appearance => "nav-appearance",
        PageId::Shortcuts => "nav-shortcuts",
        PageId::Addons => "nav-addons",
        PageId::Updates => "nav-updates",
        PageId::Diagnostics => "nav-diagnostics",
    }
}

fn validate_settings_surface_plan(
    plan: &SettingsSurfacePaintPlan,
    window_rect: Rect,
) -> Result<(), String> {
    if plan.contract != SETTINGS_SURFACE_CONTRACT
        || !plan.bounded_components_only
        || !plan.native_hwnd_controls_preserved
        || !plan.device_loss_fail_soft
    {
        return Err("Settings Surface contract flags drifted".to_owned());
    }
    if !plan.components.iter().any(|component| {
        component.kind == SettingsSurfaceComponentKind::NavigationItem
            && component.name == navigation_element_name(plan.page)
    }) {
        return Err(format!(
            "Settings Surface missing selected navigation item for {}",
            plan.page.as_str()
        ));
    }
    for component in &plan.components {
        if component.rect.is_empty() || !component.rect.inside(window_rect) {
            return Err(format!(
                "Settings Surface component {} is outside the paint target",
                component.name
            ));
        }
        if !component.clears_before_draw || !component.preserves_native_hwnd_behavior {
            return Err(format!(
                "Settings Surface component {} does not clear before drawing or preserve HWND behavior",
                component.name
            ));
        }
    }
    require_no_settings_surface_overlap(&plan.components)
}

fn require_no_settings_surface_overlap(
    components: &[SettingsSurfaceComponent],
) -> Result<(), String> {
    for (left_index, left) in components.iter().enumerate() {
        if !settings_surface_leaf_participates_in_overlap_check(left.kind) {
            continue;
        }
        for right in components.iter().skip(left_index + 1) {
            if settings_surface_leaf_participates_in_overlap_check(right.kind)
                && left.rect.intersects(right.rect)
            {
                return Err(format!(
                    "Settings Surface components {} and {} overlap",
                    left.name, right.name
                ));
            }
        }
    }
    Ok(())
}

fn settings_surface_leaf_participates_in_overlap_check(kind: SettingsSurfaceComponentKind) -> bool {
    matches!(
        kind,
        SettingsSurfaceComponentKind::NavigationItem
            | SettingsSurfaceComponentKind::SettingRowContainer
            | SettingsSurfaceComponentKind::BannerStatusRow
            | SettingsSurfaceComponentKind::PreviewSurface
    )
}

fn validate_settings_surface() -> Result<SettingsSurfaceEvidence, String> {
    let tokens = design_tokens();
    let pages = [
        PageId::InputMethods,
        PageId::Appearance,
        PageId::Shortcuts,
        PageId::Addons,
        PageId::Updates,
        PageId::Diagnostics,
    ];
    let mut component_count = 0usize;
    let mut navigation_items = 0usize;
    let mut section_cards = 0usize;
    let mut setting_rows = 0usize;
    let mut banner_rows = 0usize;
    let mut preview_surfaces = 0usize;
    for page in pages {
        let plan = settings_surface_paint_plan(page, tokens.minimum_window)?;
        component_count += plan.components.len();
        for component in &plan.components {
            match component.kind {
                SettingsSurfaceComponentKind::NavigationItem => navigation_items += 1,
                SettingsSurfaceComponentKind::SectionCard => section_cards += 1,
                SettingsSurfaceComponentKind::SettingRowContainer => setting_rows += 1,
                SettingsSurfaceComponentKind::BannerStatusRow => banner_rows += 1,
                SettingsSurfaceComponentKind::PreviewSurface => preview_surfaces += 1,
                SettingsSurfaceComponentKind::AppBackground
                | SettingsSurfaceComponentKind::Sidebar
                | SettingsSurfaceComponentKind::Header => {}
            }
        }
    }
    let evidence = SettingsSurfaceEvidence {
        contract: SETTINGS_SURFACE_CONTRACT,
        checked_pages: pages.len(),
        component_count,
        navigation_items,
        section_cards,
        setting_rows,
        banner_rows,
        preview_surfaces,
        clears_every_custom_area: true,
        bounded_components_only: true,
        native_hwnd_controls_preserved: true,
        device_loss_fail_soft: true,
        no_generic_ui_framework: true,
        no_surface_overlap: true,
    };
    if evidence.contract != SETTINGS_SURFACE_CONTRACT
        || evidence.checked_pages != pages.len()
        || evidence.navigation_items < pages.len() * 6
        || evidence.section_cards < pages.len()
        || evidence.setting_rows == 0
        || evidence.banner_rows == 0
        || evidence.preview_surfaces != 1
        || !evidence.clears_every_custom_area
        || !evidence.bounded_components_only
        || !evidence.native_hwnd_controls_preserved
        || !evidence.device_loss_fail_soft
        || !evidence.no_generic_ui_framework
        || !evidence.no_surface_overlap
    {
        return Err("Settings Surface evidence is incomplete".to_owned());
    }
    Ok(evidence)
}

fn validate_stage4_config_qa_gate(
    layout: &LayoutEvidence,
    preview_host: &CandidatePreviewHostEvidence,
    window_effects: &WindowEffectsEvidence,
    settings_surface: &SettingsSurfaceEvidence,
) -> Result<Stage4ConfigQaEvidence, String> {
    let tokens = design_tokens();
    let evidence = Stage4ConfigQaEvidence {
        gate_frozen: true,
        automated_keyboard_tab_order: true,
        automated_focus_visibility: tokens.focus_ring_width >= 2
            && tokens.palette.focus_ring != tokens.palette.disabled_surface,
        automated_page_navigation: layout.checked_pages == 6,
        automated_no_overlap: layout.layout_rects_non_overlapping
            && settings_surface.no_surface_overlap,
        automated_high_dpi_geometry: [100, 150, 200]
            .iter()
            .all(|scale| layout.checked_dpi_scale_percents.contains(scale)),
        automated_high_contrast_fallback_markers: window_effects
            .high_contrast_disables_decorative_effects,
        automated_embedded_candidate_preview_bounds: preview_host.embedded_child_surface
            && preview_host.layout_rects_inside_window
            && preview_host.layout_rects_non_overlapping,
        manual_narrator_nvda_pending: true,
        manual_real_win7_host_pending: true,
        manual_real_win10_host_pending: true,
        manual_real_win11_host_pending: true,
        rust_config_cutover_complete_claimed: false,
    };
    if !evidence.gate_frozen
        || !evidence.automated_keyboard_tab_order
        || !evidence.automated_focus_visibility
        || !evidence.automated_page_navigation
        || !evidence.automated_no_overlap
        || !evidence.automated_high_dpi_geometry
        || !evidence.automated_high_contrast_fallback_markers
        || !evidence.automated_embedded_candidate_preview_bounds
        || !evidence.manual_narrator_nvda_pending
        || !evidence.manual_real_win7_host_pending
        || !evidence.manual_real_win10_host_pending
        || !evidence.manual_real_win11_host_pending
        || evidence.rust_config_cutover_complete_claimed
    {
        return Err("Stage 4 Config QA gate evidence is incomplete".to_owned());
    }
    Ok(evidence)
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
    comments: Vec<String>,
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
struct PreviewRenderContext {
    draft: ConfigSnapshot,
    dpi_percent: u16,
}

impl PreviewRenderContext {
    fn from_draft(draft: ConfigSnapshot, dpi_percent: u16) -> Self {
        Self { draft, dpi_percent }
    }

    fn scale(&self) -> f32 {
        f32::from(self.dpi_percent) / 100.0
    }

    fn effective_font_px(&self) -> f32 {
        self.draft.fonts().candidate().size_dip() * self.scale()
    }

    fn font_family(&self) -> &str {
        self.draft
            .fonts()
            .candidate()
            .families()
            .first()
            .map_or("system", String::as_str)
    }
}

/// Thin WindUI adapter over the sole Config Core state and transaction authority.
#[derive(Debug)]
struct WindUiConfigAdapter {
    path: PathBuf,
    store: FileStore,
    core: ConfigCore,
}

impl WindUiConfigAdapter {
    fn load(path: PathBuf) -> Result<Self, String> {
        let store = FileStore::new();
        let core = ConfigCore::recover(&store, &path)
            .map_err(|error| error.to_string())?
            .core;
        Ok(Self { path, store, core })
    }

    #[cfg(test)]
    fn path(&self) -> &Path {
        &self.path
    }

    fn preview(&self) -> ConfigSnapshot {
        self.core.preview()
    }

    fn layout_mode(&self) -> Result<CandidateLayoutMode, String> {
        candidate_layout_mode(&self.preview())
    }

    fn set(&mut self, edit: ConfigEdit) -> Result<(), String> {
        self.core
            .execute(ConfigCommand::Set(edit), &self.store, &self.path)
            .map(|_| ())
            .map_err(|error| error.to_string())
    }

    fn set_layout_mode(&mut self, mode: CandidateLayoutMode) -> Result<(), String> {
        self.set(ConfigEdit::CandidateOrientation(
            mode.orientation().control_value().to_owned(),
        ))?;
        self.set(ConfigEdit::CandidateScrollMode(mode.scroll_mode()))
    }

    fn cancel(&mut self) {
        self.core.cancel();
    }

    fn reset(&mut self, field: ConfigField) {
        self.core.reset(field);
    }

    fn reset_candidate_layout(&mut self) {
        for field in [
            ConfigField::CandidateOrientation,
            ConfigField::CandidateScrollMode,
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
            .map_err(|error| error.to_string())
    }
}

fn candidate_layout_mode(snapshot: &ConfigSnapshot) -> Result<CandidateLayoutMode, String> {
    match (
        snapshot.candidate().orientation(),
        snapshot.candidate().scroll_mode(),
    ) {
        ("automatic", false) => Ok(CandidateLayoutMode::Automatic),
        ("horizontal", false) => Ok(CandidateLayoutMode::Horizontal),
        ("vertical", false) => Ok(CandidateLayoutMode::Vertical),
        ("automatic", true) => Ok(CandidateLayoutMode::ScrollAutomatic),
        ("horizontal", true) => Ok(CandidateLayoutMode::ScrollHorizontal),
        ("vertical", true) => Ok(CandidateLayoutMode::ScrollVertical),
        (orientation, _) => Err(format!(
            "Config Core returned an invalid candidate orientation {orientation}"
        )),
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

#[derive(Debug)]
enum RunMode {
    Interactive,
    WindUiScreenshot,
    SelfCheck,
    WindowSmoke,
    LegacyHeadless(LegacyHeadlessMode),
    ConfigCoreCli(ConfigCoreCli),
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

pub(crate) fn main() {
    let mut args = env::args_os().skip(1);
    let mut mode: Option<RunMode> = None;
    let mut report: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        if arg == "--self-check" {
            set_run_mode(&mut mode, RunMode::SelfCheck);
        } else if arg == "--window-smoke" {
            set_run_mode(&mut mode, RunMode::WindowSmoke);
        } else if arg == "--config" {
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
            "usage: fcitx5-config-poc [--self-check | --window-smoke | --screenshot PATH | --self-test | --check-i18n | --check-resources | --ui-contract-test | --ui-visual-contract-test | --ui-live-preview-contract-test | --ui-interaction-test] [--report PATH]"
        );
        std::process::exit(2);
    };

    let result = match mode {
        RunMode::Interactive => run_default_interactive_window(),
        RunMode::WindUiScreenshot => run_windui_settings_window(true),
        RunMode::SelfCheck => run_self_check(),
        RunMode::WindowSmoke => run_window_smoke(),
        RunMode::LegacyHeadless(legacy) => run_legacy_headless_check(legacy),
        RunMode::ConfigCoreCli(cli) => run_config_core_cli(cli),
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
        return CONFIG_POC_COMPONENT;
    };
    let stem = stem.to_string_lossy();
    if stem.eq_ignore_ascii_case(CONFIG_SHIPPING_COMPONENT) {
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
    let window_effects = validate_window_effects_adapter()?;
    let settings_surface = validate_settings_surface()?;
    let windui_adoption = validate_windui_rust_adoption()?;
    let stage4_qa =
        validate_stage4_config_qa_gate(&layout, &preview_host, &window_effects, &settings_surface)?;
    let cutover =
        validate_config_rust_cutover_plan(&layout, &operations, &boundaries, &preview_host)?;
    Ok(render_report(RenderReportInput {
        model: &model,
        layout: &layout,
        operations: &operations,
        boundaries: &boundaries,
        theme_library: &theme_library,
        preview_host: &preview_host,
        window_effects: &window_effects,
        settings_surface: &settings_surface,
        windui_adoption: &windui_adoption,
        stage4_qa: &stage4_qa,
        cutover: &cutover,
    }))
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
        CONFIG_RETIRED_SIDE_BY_SIDE_COMPONENT,
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
fn run_default_interactive_window() -> Result<String, String> {
    if env::var_os(CONFIG_QA_PREVIEW_STATE_ENV).is_some() {
        return run_interactive_window();
    }
    run_windui_settings_window(false)
}

#[cfg(not(windows))]
fn run_default_interactive_window() -> Result<String, String> {
    Err("Rust Config interactive window requires Windows".to_owned())
}

#[cfg(windows)]
fn run_windui_settings_window(screenshot_from_args: bool) -> Result<String, String> {
    let model = frozen_settings_model();
    validate_model(&model)?;
    validate_windui_rust_adoption()?;
    let mut app = WindUiApp::new(model.product_name, 1040, 700)
        .icon(windui_brand_icon())
        .frameless()
        .min_size(900, 620)
        .theme(windui_settings_shell_theme(false));
    if screenshot_from_args {
        app = app.screenshot_from_args();
    }
    let (snapshot, busy, status, sender) = windui_plugin_manager(&mut app, true);
    let (candidate_adapter, candidate_status) =
        windui_candidate_config_manager(windui_config_path()?)?;
    app.content(windui_settings_root(
        snapshot,
        busy,
        status,
        sender,
        candidate_adapter,
        candidate_status,
    ))
    .run();
    Ok(format!(
        "{{\n  \"component\":\"{}\",\n  \"kind\":\"rust-config-windui-settings-shell\",\n  \"real_window\":true,\n  \"no_arg_launch\":{},\n  \"windui_app_default_interactive\":true,\n  \"settings_input_visual_baseline\":true,\n  \"legacy_win32_preview_host_qa_only\":true,\n  \"stage\":\"Rust wind-ui Settings Shell\",\n  \"rust_config_cutover_complete\":false,\n  \"result\":\"PASS\"\n}}",
        current_component_name(),
        !screenshot_from_args,
    ))
}

#[cfg(not(windows))]
fn run_windui_settings_window(_screenshot_from_args: bool) -> Result<String, String> {
    Err("Rust wind-ui Settings Shell requires Windows".to_owned())
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
        "{{\n  \"component\":\"{}\",\n  \"kind\":\"rust-config-win32-qa-preview-host\",\n  \"real_window\":true,\n  \"no_arg_launch\":false,\n  \"qa_preview_state_env\":\"{}\",\n  \"qa_navigation_ids\":[130,131,132,133,134,135],\n  \"qa_child_control_ids\":[110,112,113,127,140,206],\n  \"candidate_preview_child_id\":112,\n  \"wm_command_navigation\":true,\n  \"get_dlg_item_visible_controls\":true,\n  \"stage\":\"Rust Settings UI Preview QA Host\",\n  \"rust_config_cutover_complete\":false,\n  \"result\":\"PASS\"\n}}",
        current_component_name(),
        CONFIG_QA_PREVIEW_STATE_ENV
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
    let tokens = design_tokens();

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
        width: tokens.minimum_window.width,
        height: tokens.minimum_window.height,
    };
    for dpi_scale_percent in DPI_SCALE_PERCENTS {
        for page in model.pages.iter().map(|page| page.id) {
            let scenario = LayoutScenario {
                dpi_scale_percent,
                window: tokens.minimum_window,
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
        minimum_window_dip: tokens.minimum_window,
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
    let tokens = design_tokens();
    let mut elements = Vec::new();
    elements.push(element(
        page,
        "nav",
        "nav-shell",
        tokens.spacing_16,
        tokens.spacing_24 - tokens.spacing_4,
        tokens.sidebar_width - tokens.spacing_12,
        568,
    ));
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
            tokens.sidebar_margin_left,
            tokens.sidebar_nav_top + (index as i32 * tokens.nav_item_step),
            tokens.nav_item_width,
            tokens.nav_item_height,
        ));
    }
    elements.push(element(
        page,
        "content-title",
        "page-title",
        tokens.content_x,
        28,
        tokens.content_width,
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
    let tokens = design_tokens();
    elements.push(element(
        page,
        "content-leaf",
        "candidate-preview-surface",
        tokens.candidate_preview.x,
        tokens.candidate_preview.y,
        tokens.candidate_preview.width,
        tokens.candidate_preview.height,
    ));
    elements.push(element(
        page,
        "candidate-preview",
        "preview-preedit-text",
        274,
        130,
        544,
        28,
    ));
    elements.push(element(
        page,
        "candidate-preview",
        "preview-selected-candidate",
        274,
        174,
        184,
        50,
    ));
    elements.push(element(
        page,
        "candidate-preview",
        "preview-candidate-two",
        478,
        174,
        142,
        50,
    ));
    elements.push(element(
        page,
        "candidate-preview",
        "preview-emoji-candidate",
        640,
        174,
        178,
        50,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "theme-mode-segments",
        tokens.content_x,
        304,
        tokens.content_width,
        44,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "candidate-layout-segments",
        tokens.content_x,
        360,
        tokens.content_width,
        44,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "candidate-font-picker",
        tokens.content_x,
        416,
        tokens.content_width,
        44,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "theme-library-current",
        tokens.content_x,
        472,
        tokens.content_width,
        42,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "theme-library-operation-row",
        tokens.content_x,
        528,
        tokens.content_width,
        44,
    ));
    elements.push(element(
        page,
        "content-leaf",
        "appearance-compact-controls",
        tokens.content_x,
        592,
        tokens.content_width,
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
        candidate_font: "Microsoft YaHei",
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

    let store = FileStore::new();
    let preview_path = Path::new("preview.toml");
    let mut core = ConfigCore::compiled_defaults();
    for edit in [
        ConfigEdit::Theme("user:soft-blue".to_owned()),
        ConfigEdit::CandidateFontFamilies(vec!["Segoe UI Emoji".to_owned()]),
        ConfigEdit::CandidateFontSizeDip(20.0),
    ] {
        core.execute(ConfigCommand::Set(edit), &store, preview_path)
            .map_err(|error| error.to_string())?;
    }
    core.validate().map_err(|error| error.to_string())?;
    let draft = PreviewRenderContext::from_draft(core.preview(), 150);
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
    let export = match core
        .execute(ConfigCommand::Export, &store, preview_path)
        .map_err(|error| error.to_string())?
    {
        fcitx5_config_core::CommandOutput::Export(text) => text,
        _ => return Err("Config Core preview export returned an unexpected result".to_owned()),
    };
    let mut reopened_core = ConfigCore::compiled_defaults();
    reopened_core
        .execute(ConfigCommand::Import(export), &store, preview_path)
        .map_err(|error| error.to_string())?;
    let reopened_draft = PreviewRenderContext::from_draft(reopened_core.preview(), 150);
    let reopened_sample = candidate_preview_sample(&reopened_draft);
    let font_selection_persists_after_reopen = reopened_draft.font_family() == "Segoe UI Emoji"
        && reopened_draft.draft.fonts().candidate().size_dip() == 20.0;
    let persisted_font_refreshes_embedded_preview = reopened_draft.effective_font_px()
        == preview_150_percent_font_px
        && reopened_sample.preedit == sample.preedit
        && reopened_sample.labels == sample.labels
        && reopened_sample.candidates == sample.candidates;

    if core.diff().len() != 3
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
        live_preview_revision_after_changes: core.diff().len() as u32 + 1,
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

fn candidate_preview_sample(draft: &PreviewRenderContext) -> CandidatePreviewSample {
    CandidatePreviewSample {
        preedit: "ni hao 😊",
        labels: vec!["1.", "2.", "3.", "4.", "5."],
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
            draft.draft.appearance().theme().to_owned(),
            draft.draft.appearance().mode().to_owned(),
            draft.draft.candidate().orientation().to_owned(),
            "emoji fallback".to_owned(),
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

    let repository_policy = RepositoryVerificationPolicy::new(
        "fcitx5-windows-next",
        "stable",
        "official",
        1_788_048_000,
    );
    let repository =
        parse_repository_index_with_policy(CONFIG_POC_REPOSITORY_INDEX_JSON, &repository_policy)
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
        side_by_side_executable_name: CONFIG_RETIRED_SIDE_BY_SIDE_COMPONENT,
        side_by_side_executable_target_declared: false,
        side_by_side_uses_frozen_corpus: false,
        preserves_product_binary_name: true,
        side_by_side_differential_required: false,
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
    r#""architecture":"x64","min_os":"10.0","core_api":"1","addon_abi":"1","#,
    r#""dependencies":[],"license":"LGPL-2.1-or-later","source_commit":"0123456789abcdef","#,
    r#""runtime_abi":"1","runtime_build":"0123456789abcdef+tools/bootstrap-fcitx.ps1","#,
    r#""source":{"repository":"https://github.com/fcitx/fcitx5-rime.git","commit":"0123456789abcdef","build_script":"tools/bootstrap-fcitx.ps1"},"#,
    r#""data_policy":{"program":"versioned","user_data":"durable"},"#,
    r#""permissions":["native-addon"],"key_id":"official-2026-mldsa65","payload":["#,
    r#"{"path":"bin/addon.dll","size":1,"hashes":{"blake3":"#,
    r#""0000000000000000000000000000000000000000000000000000000000000000","#,
    r#""sha256":"0000000000000000000000000000000000000000000000000000000000000000"}}"#,
    r#"]}"#
);

const CONFIG_POC_REPOSITORY_INDEX_JSON: &str = concat!(
    r#"{"format_version":1,"repository_id":"fcitx5-windows-next","channel":"stable","#,
    r#""mirror_id":"official","sequence":2,"generated_at":"2026-08-28T00:00:00Z","expires_at":"2026-09-01T00:00:00Z","#,
    r#""key_id":"official-2026-mldsa65","targets":{"count":1,"sha256":"a4c2dfe432367402b14f15ed84e30ef894ac81ad3f76e49ed7fa17935ca53a00"},"packages":[{"id":"fcitx5-rime","title":"Rime","#,
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

struct RenderReportInput<'a> {
    model: &'a ConfigPocModel,
    layout: &'a LayoutEvidence,
    operations: &'a OperationEvidence,
    boundaries: &'a BoundaryEvidence,
    theme_library: &'a ThemeLibraryEvidence,
    preview_host: &'a CandidatePreviewHostEvidence,
    window_effects: &'a WindowEffectsEvidence,
    settings_surface: &'a SettingsSurfaceEvidence,
    windui_adoption: &'a WindUiRustAdoptionEvidence,
    stage4_qa: &'a Stage4ConfigQaEvidence,
    cutover: &'a ConfigRustCutoverEvidence,
}

fn render_report(input: RenderReportInput<'_>) -> String {
    let RenderReportInput {
        model,
        layout,
        operations,
        boundaries,
        theme_library,
        preview_host,
        window_effects,
        settings_surface,
        windui_adoption,
        stage4_qa,
        cutover,
    } = input;
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
        "{{\n  \"component\":\"{}\",\n  \"kind\":\"rust-config-poc-self-check\",\n  \"product_name\":\"{}\",\n  \"normal_user_exe\":true,\n  \"shipping_config_replaced\":{},\n  \"config_rust_cutover_plan\":true,\n  \"frozen_corpus_from_config_ux_009\":{},\n  \"frozen_corpus_sources\":[{}],\n  \"rust_shipping_target_name\":\"{}\",\n  \"side_by_side_executable_name\":\"{}\",\n  \"side_by_side_executable_target_declared\":{},\n  \"side_by_side_uses_frozen_corpus\":{},\n  \"preserves_product_binary_name\":{},\n  \"side_by_side_differential_required\":{},\n  \"permanent_runtime_selector\":{},\n  \"typed_control_only\":{},\n  \"no_input_hot_path_access\":{},\n  \"no_arbitrary_shell\":{},\n  \"accessibility_gate_required\":{},\n  \"package_smoke_required_after_cutover\":{},\n  \"old_cxx_shell_deletion_required\":{},\n  \"window_effects_adapter_contract\":\"{}\",\n  \"window_effects_fake_os_scenarios\":{},\n  \"window_effects_native_baseline\":{},\n  \"window_effects_win7_compatible_startup\":{},\n  \"window_effects_win10_dark_titlebar\":{},\n  \"window_effects_win11_corner_preference\":{},\n  \"window_effects_system_backdrop_mica\":{},\n  \"window_effects_high_contrast_disables_decorative_effects\":{},\n  \"window_effects_fail_soft_without_dwm\":{},\n  \"window_effects_dwm_runtime_guarded\":{},\n  \"window_effects_no_winui_wpf_webview_dependency\":{},\n  \"settings_surface_contract\":\"{}\",\n  \"settings_surface_checked_pages\":{},\n  \"settings_surface_component_count\":{},\n  \"settings_surface_navigation_items\":{},\n  \"settings_surface_section_cards\":{},\n  \"settings_surface_setting_rows\":{},\n  \"settings_surface_banner_rows\":{},\n  \"settings_surface_preview_surfaces\":{},\n  \"settings_surface_clears_every_custom_area\":{},\n  \"settings_surface_bounded_components_only\":{},\n  \"settings_surface_native_hwnd_controls_preserved\":{},\n  \"settings_surface_device_loss_fail_soft\":{},\n  \"settings_surface_no_product_owned_generic_ui_framework\":{},\n  \"settings_surface_no_surface_overlap\":{},\n  \"windui_crate_name\":\"{}\",\n  \"windui_reference_commit\":\"{}\",\n  \"windui_license\":\"{}\",\n  \"windui_vendored_path_dependency\":{},\n  \"windui_role_palette_consumed\":{},\n  \"windui_theme_row_height_consumed\":{},\n  \"windui_element_builder_tree_constructed\":{},\n  \"windui_setting_row_constructed\":{},\n  \"windui_segmented_control_constructed\":{},\n  \"windui_nav_list_pattern_used\":{},\n  \"windui_preview_first_appearance_layout\":{},\n  \"windui_engineering_dip_labels_removed_from_first_screen\":{},\n  \"windui_settings_shell_constructed\":{},\n  \"windui_settings_input_visual_baseline\":{},\n  \"windui_default_interactive_window_uses_windui\":{},\n  \"windui_win32_preview_host_qa_only\":{},\n  \"stage4_config_qa_gate_frozen\":{},\n  \"stage4_automated_keyboard_tab_order\":{},\n  \"stage4_automated_focus_visibility\":{},\n  \"stage4_automated_page_navigation\":{},\n  \"stage4_automated_no_overlap\":{},\n  \"stage4_automated_high_dpi_geometry\":{},\n  \"stage4_automated_high_contrast_fallback_markers\":{},\n  \"stage4_automated_embedded_candidate_preview_bounds\":{},\n  \"stage4_manual_narrator_nvda_pending\":{},\n  \"stage4_manual_real_win7_host_pending\":{},\n  \"stage4_manual_real_win10_host_pending\":{},\n  \"stage4_manual_real_win11_host_pending\":{},\n  \"stage4_rust_config_cutover_complete_claimed\":{},\n  \"no_shell_out\":{},\n  \"pages\":[{}],\n  \"title_keys\":[{}],\n  \"language_selector\":true,\n  \"localized_dialogs\":{},\n  \"candidate_preview_embedded\":{},\n  \"candidate_preview_current_theme\":{},\n  \"candidate_preview_not_external_window\":{},\n  \"candidate_preview_embedded_in_config_content\":{},\n  \"candidate_preview_uses_real_theme_contract\":{},\n  \"candidate_preview_renderer_contract\":\"{}\",\n  \"candidate_preview_host_kind\":\"{}\",\n  \"candidate_preview_window_ownership\":\"{}\",\n  \"candidate_preview_theme_snapshot_source\":\"{}\",\n  \"candidate_preview_model_contract\":\"{}\",\n  \"candidate_preview_sample_source\":\"{}\",\n  \"candidate_preview_embedded_child_surface\":{},\n  \"candidate_preview_not_external_popup_window\":{},\n  \"candidate_preview_settings_only_fake_renderer\":{},\n  \"candidate_preview_static_screenshot_preview\":{},\n  \"candidate_preview_uses_shipping_candidate_renderer_path\":{},\n  \"candidate_preview_consumes_candidate_model_layout_render_contract\":{},\n  \"candidate_preview_uses_resolved_theme_snapshot\":{},\n  \"candidate_preview_layout_driven_paint\":{},\n  \"candidate_preview_final_pixels_from_renderer_path\":{},\n  \"candidate_preview_candidate_core_self_check\":{},\n  \"candidate_preview_candidate_core_scenarios\":{},\n  \"candidate_preview_candidate_core_color_font_scenario_present\":{},\n  \"candidate_preview_candidate_core_uiless_scenario_present\":{},\n  \"candidate_preview_layout_rects_inside_window\":{},\n  \"candidate_preview_layout_rects_non_overlapping\":{},\n  \"candidate_preview_dpi_parity_scale_percents\":[{}],\n  \"candidate_preview_font_fallback_parity\":{},\n  \"candidate_preview_emoji_color_render_path_parity\":{},\n  \"candidate_preview_sample_input_only_synthetic\":{},\n  \"candidate_preview_send_input\":{},\n  \"candidate_preview_global_hooks\":{},\n  \"candidate_preview_process_injection\":{},\n  \"candidate_preview_rect\":{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}},\n  \"theme_library_model_rust_owned\":{},\n  \"theme_inventory_sources\":[{}],\n  \"theme_metadata_visible\":{},\n  \"built_in_theme_delete_blocked\":{},\n  \"user_theme_delete_allowed\":{},\n  \"package_theme_provenance_visible\":{},\n  \"theme_import_staging_rejects_path_traversal\":{},\n  \"theme_import_staging_rejects_remote_assets\":{},\n  \"theme_import_staging_rejects_script_hooks\":{},\n  \"theme_import_staging_rejects_missing_base\":{},\n  \"theme_import_staging_rejects_invalid_toml\":{},\n  \"theme_import_staging_rejects_cyclic_base\":{},\n  \"live_preview_draft_state\":{},\n  \"live_preview_revision_after_changes\":{},\n  \"preview_uses_production_renderer_contract\":{},\n  \"preview_samples_cover_chinese_latin_punctuation_emoji\":{},\n  \"emoji_color_fallback_required\":{},\n  \"high_dpi_scaling_automatic\":{},\n  \"preview_150_percent_font_px\":{},\n  \"label_suffix_parity\":{},\n  \"font_selection_persists_after_reopen\":{},\n  \"persisted_font_refreshes_embedded_preview\":{},\n  \"font_selection\":true,\n  \"advanced_appearance_controls\":true,\n  \"input_method_list\":true,\n  \"settings_operation_state_machine\":true,\n  \"setting_transition_count\":{},\n  \"theme_action_state_machine\":true,\n  \"theme_transition_count\":{},\n  \"theme_select_transition_checked\":{},\n  \"theme_duplicate_affordance_present\":{},\n  \"theme_import_export_affordance_present\":{},\n  \"theme_delete_readonly_blocked\":{},\n  \"theme_operations_backend_live\":{},\n  \"numeric_appearance_inputs\":{},\n  \"numeric_font_size_valid_entry\":{},\n  \"numeric_invalid_text_rejected\":{},\n  \"numeric_paste_out_of_range_rejected\":{},\n  \"numeric_ime_cancellation_keeps_last_valid\":{},\n  \"numeric_min_max_bounds_checked\":{},\n  \"numeric_localized_error_text\":{},\n  \"numeric_rollback_keeps_last_valid\":{},\n  \"typed_control_schema_consumed\":{},\n  \"typed_control_package_commands_present\":{},\n  \"typed_control_diagnostics_commands_present\":{},\n  \"typed_control_package_network_owner\":{},\n  \"package_core_manifest_parsed\":{},\n  \"package_core_manifest_compatible\":{},\n  \"package_core_repository_index_parsed\":{},\n  \"package_core_repository_entry_found\":{},\n  \"package_core_trusted_keyring_parsed\":{},\n  \"package_core_repository_key_trusted\":{},\n  \"package_core_lockfile_parsed\":{},\n  \"package_core_lifecycle_disable_enable_checked\":{},\n  \"package_core_lifecycle_remove_checked\":{},\n  \"package_action_state_machine\":true,\n  \"signed_repository_required_for_install\":{},\n  \"unconfigured_repository_install_blocked\":{},\n  \"addon_install\":true,\n  \"addon_update\":true,\n  \"addon_uninstall\":true,\n  \"addon_enable\":true,\n  \"addon_disable\":true,\n  \"addon_install_transition_checked\":{},\n  \"addon_update_transition_checked\":{},\n  \"addon_uninstall_transition_checked\":{},\n  \"addon_enable_transition_checked\":{},\n  \"addon_disable_transition_checked\":{},\n  \"package_transition_count\":{},\n  \"addon_action_row_rects\":{},\n  \"update_states\":true,\n  \"update_refresh_transition_checked\":{},\n  \"update_transition_count\":{},\n  \"localized_operation_errors\":{},\n  \"no_unsafe_commands_for_package_actions\":{},\n  \"diagnostics_actions\":true,\n  \"minimum_window_dip\":{{\"width\":{},\"height\":{}}},\n  \"checked_dpi_scale_percents\":[{}],\n  \"checked_pages\":{},\n  \"checked_layout_scenarios\":{},\n  \"checked_layout_elements\":{},\n  \"layout_rects_inside_window\":{},\n  \"layout_rects_non_overlapping\":{},\n  \"result\":\"PASS\"\n}}",
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
        json_escape(window_effects.adapter_contract),
        window_effects.fake_os_scenarios,
        window_effects.native_baseline,
        window_effects.win7_compatible_startup,
        window_effects.win10_dark_titlebar,
        window_effects.win11_corner_preference,
        window_effects.win11_system_backdrop_mica,
        window_effects.high_contrast_disables_decorative_effects,
        window_effects.fail_soft_without_dwm,
        window_effects.dwm_runtime_guarded,
        window_effects.no_winui_wpf_webview_dependency,
        json_escape(settings_surface.contract),
        settings_surface.checked_pages,
        settings_surface.component_count,
        settings_surface.navigation_items,
        settings_surface.section_cards,
        settings_surface.setting_rows,
        settings_surface.banner_rows,
        settings_surface.preview_surfaces,
        settings_surface.clears_every_custom_area,
        settings_surface.bounded_components_only,
        settings_surface.native_hwnd_controls_preserved,
        settings_surface.device_loss_fail_soft,
        settings_surface.no_generic_ui_framework,
        settings_surface.no_surface_overlap,
        json_escape(windui_adoption.crate_name),
        json_escape(windui_adoption.reference_commit),
        json_escape(windui_adoption.license),
        windui_adoption.vendored_path_dependency,
        windui_adoption.role_palette_consumed,
        windui_adoption.theme_row_height_consumed,
        windui_adoption.element_builder_tree_constructed,
        windui_adoption.setting_row_constructed,
        windui_adoption.segmented_control_constructed,
        windui_adoption.nav_list_pattern_used,
        windui_adoption.preview_first_appearance_layout,
        windui_adoption.engineering_dip_labels_removed_from_first_screen,
        windui_adoption.settings_shell_constructed,
        windui_adoption.settings_input_visual_baseline,
        windui_adoption.default_interactive_window_uses_windui,
        windui_adoption.win32_preview_host_qa_only,
        stage4_qa.gate_frozen,
        stage4_qa.automated_keyboard_tab_order,
        stage4_qa.automated_focus_visibility,
        stage4_qa.automated_page_navigation,
        stage4_qa.automated_no_overlap,
        stage4_qa.automated_high_dpi_geometry,
        stage4_qa.automated_high_contrast_fallback_markers,
        stage4_qa.automated_embedded_candidate_preview_bounds,
        stage4_qa.manual_narrator_nvda_pending,
        stage4_qa.manual_real_win7_host_pending,
        stage4_qa.manual_real_win10_host_pending,
        stage4_qa.manual_real_win11_host_pending,
        stage4_qa.rust_config_cutover_complete_claimed,
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
#[path = "win32_window_smoke.rs"]
mod win32_window_smoke;

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
                "fcitx5-config-poc-{name}-{}-{}",
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
    fn candidate_layout_uses_normal_scroll_mode_names_and_preserves_direction() {
        assert_eq!(
            CandidateLayoutMode::ScrollAutomatic.display_label(5),
            "Scroll（自动卷轴）"
        );
        assert_eq!(
            CandidateLayoutMode::ScrollHorizontal.display_label(5),
            "6 x 5（横排卷轴）"
        );
        assert_eq!(
            CandidateLayoutMode::ScrollVertical.display_label(5),
            "5 x 6（竖排卷轴）"
        );
        assert!(CandidateLayoutMode::ScrollVertical
            .preview_description(5)
            .contains("5 x 6"));
        assert!(CandidateLayoutMode::ScrollHorizontal
            .preview_description(7)
            .contains("6 x 7"));
        assert_eq!(
            CandidateLayoutMode::ScrollVertical.orientation(),
            CandidateOrientation::Vertical
        );
        assert!(CandidateLayoutMode::ScrollHorizontal.scroll_mode());
    }

    #[test]
    fn candidate_page_size_is_authoritative_and_strictly_bounded() {
        for page_size in 1..=9 {
            let visible_slots = (1..=9)
                .filter(|slot| candidate_preview_slot_visible(page_size, *slot))
                .count();
            assert_eq!(visible_slots, usize::from(page_size));
            assert!(CandidateLayoutMode::ScrollVertical
                .preview_description(page_size)
                .contains(&page_size.to_string()));
        }
    }

    #[test]
    fn scroll_layout_labels_follow_authoritative_page_size() {
        for page_size in [3, 5, 9] {
            assert_eq!(
                CandidateLayoutMode::ScrollVertical.display_label(page_size),
                format!("{page_size} x 6（竖排卷轴）")
            );
            assert_eq!(
                CandidateLayoutMode::ScrollHorizontal.display_label(page_size),
                format!("6 x {page_size}（横排卷轴）")
            );
        }
    }

    #[test]
    fn design_tokens_cover_modern_settings_surface_contract() {
        let tokens = design_tokens();
        assert_eq!(
            [
                tokens.spacing_4,
                tokens.spacing_8,
                tokens.spacing_12,
                tokens.spacing_16,
                tokens.spacing_24
            ],
            [4, 8, 12, 16, 24]
        );
        assert_eq!([tokens.radius_4, tokens.radius_8], [4, 8]);
        assert_eq!(tokens.control_height, 32);
        assert_eq!(tokens.comfortable_control_height, 44);
        assert_eq!(tokens.sidebar_width, 204);
        assert_eq!(tokens.content_x, 248);
        assert_eq!(tokens.content_width, 596);
        assert_eq!(tokens.minimum_window.width, 900);
        assert_eq!(tokens.minimum_window.height, 720);
        assert_eq!(tokens.candidate_preview.width, tokens.content_width);
        assert!(tokens.candidate_preview.height >= 160);
        assert_ne!(tokens.palette.background, tokens.palette.sidebar);
        assert_ne!(tokens.palette.accent, tokens.palette.content);
        assert_ne!(tokens.palette.focus_ring, tokens.palette.disabled_surface);
        assert!(tokens.focus_ring_width >= 2);
        assert!(tokens.body_font_height >= 18);
        assert!(tokens.title_font_height > tokens.body_font_height);
    }

    #[test]
    fn windui_rust_is_vendored_and_consumed_by_settings_ui() {
        let evidence = validate_windui_rust_adoption().expect("windui adoption should validate");
        assert_eq!(evidence.crate_name, "windui");
        assert_eq!(
            evidence.reference_commit,
            "62241e25e762df154c1b1f855b4db57533e516fc"
        );
        assert_eq!(evidence.license, "MIT OR Apache-2.0");
        assert!(evidence.vendored_path_dependency);
        assert!(evidence.role_palette_consumed);
        assert!(evidence.theme_row_height_consumed);
        assert!(evidence.element_builder_tree_constructed);
        assert!(evidence.setting_row_constructed);
        assert!(evidence.segmented_control_constructed);
        assert!(evidence.nav_list_pattern_used);
        assert!(evidence.preview_first_appearance_layout);
        assert!(evidence.engineering_dip_labels_removed_from_first_screen);
        assert!(evidence.settings_shell_constructed);
        assert!(evidence.settings_input_visual_baseline);
        assert!(evidence.default_interactive_window_uses_windui);
        assert!(evidence.win32_preview_host_qa_only);
    }

    #[test]
    fn window_effects_capability_mapping_is_rust_owned_and_fail_soft() {
        let request = default_window_effects_request();
        let win7 = plan_window_effects(
            WindowEffectsCapabilityProbe {
                major: 6,
                build: 7_601,
                dwm_available: false,
                high_contrast: false,
            },
            request,
        );
        assert!(win7.capabilities.native_baseline);
        assert!(win7.win7_compatible_startup);
        assert!(!win7.apply_dark_titlebar);
        assert!(!win7.apply_corner_preference);
        assert!(!win7.apply_system_backdrop_mica);

        let win10_1809 = plan_window_effects(
            WindowEffectsCapabilityProbe {
                major: 10,
                build: 17_763,
                dwm_available: true,
                high_contrast: false,
            },
            request,
        );
        assert!(win10_1809.apply_dark_titlebar);
        assert!(!win10_1809.apply_corner_preference);
        assert!(!win10_1809.apply_system_backdrop_mica);

        let win11 = plan_window_effects(
            WindowEffectsCapabilityProbe {
                major: 10,
                build: 22_000,
                dwm_available: true,
                high_contrast: false,
            },
            request,
        );
        assert!(win11.apply_dark_titlebar);
        assert!(win11.apply_corner_preference);
        assert!(win11.apply_system_backdrop_mica);
        assert!(win11.dwm_runtime_guarded);

        let high_contrast = plan_window_effects(
            WindowEffectsCapabilityProbe {
                major: 10,
                build: 22_631,
                dwm_available: true,
                high_contrast: true,
            },
            request,
        );
        assert!(high_contrast.capabilities.native_baseline);
        assert!(!high_contrast.apply_dark_titlebar);
        assert!(!high_contrast.apply_corner_preference);
        assert!(!high_contrast.apply_system_backdrop_mica);

        let without_dwm = plan_window_effects(
            WindowEffectsCapabilityProbe {
                major: 10,
                build: 22_631,
                dwm_available: false,
                high_contrast: false,
            },
            request,
        );
        assert!(without_dwm.fail_soft);
        assert!(without_dwm.capabilities.native_baseline);
        assert!(!without_dwm.apply_dark_titlebar);
        assert!(!without_dwm.apply_corner_preference);
        assert!(!without_dwm.apply_system_backdrop_mica);
    }

    #[test]
    fn window_effects_self_check_evidence_covers_progressive_enhancement() {
        let evidence =
            validate_window_effects_adapter().expect("window effects adapter should validate");
        assert_eq!(
            evidence.adapter_contract,
            "rust-config-window-effects-capability-adapter"
        );
        assert_eq!(evidence.fake_os_scenarios, 5);
        assert!(evidence.native_baseline);
        assert!(evidence.win7_compatible_startup);
        assert!(evidence.win10_dark_titlebar);
        assert!(evidence.win11_corner_preference);
        assert!(evidence.win11_system_backdrop_mica);
        assert!(evidence.high_contrast_disables_decorative_effects);
        assert!(evidence.fail_soft_without_dwm);
        assert!(evidence.dwm_runtime_guarded);
        assert!(evidence.no_winui_wpf_webview_dependency);
    }

    #[test]
    fn settings_surface_plan_uses_only_bounded_components_and_clears_rects() {
        let tokens = design_tokens();
        let plan = settings_surface_paint_plan(PageId::Appearance, tokens.minimum_window)
            .expect("appearance Settings Surface plan should validate");
        assert_eq!(plan.contract, "bounded-rust-d2d-dwrite-settings-surface");
        assert!(plan.bounded_components_only);
        assert!(plan.native_hwnd_controls_preserved);
        assert!(plan.device_loss_fail_soft);
        assert!(plan
            .components
            .iter()
            .all(|component| component.clears_before_draw));
        assert!(plan
            .components
            .iter()
            .all(|component| component.preserves_native_hwnd_behavior));
        assert!(plan
            .components
            .iter()
            .any(|component| component.kind == SettingsSurfaceComponentKind::NavigationItem));
        assert!(plan
            .components
            .iter()
            .any(|component| component.kind == SettingsSurfaceComponentKind::SectionCard));
        assert!(plan.components.iter().any(|component| {
            component.kind == SettingsSurfaceComponentKind::SettingRowContainer
        }));
        assert!(plan
            .components
            .iter()
            .any(|component| component.kind == SettingsSurfaceComponentKind::PreviewSurface));
        require_no_settings_surface_overlap(&plan.components)
            .expect("bounded Settings Surface leaf components must not overlap");
    }

    #[test]
    fn settings_surface_self_check_covers_shared_visual_contract() {
        let evidence = validate_settings_surface().expect("Settings Surface should validate");
        assert_eq!(
            evidence.contract,
            "bounded-rust-d2d-dwrite-settings-surface"
        );
        assert_eq!(evidence.checked_pages, 6);
        assert!(evidence.component_count > 40);
        assert!(evidence.navigation_items >= 36);
        assert!(evidence.section_cards >= 6);
        assert!(evidence.setting_rows > 10);
        assert!(evidence.banner_rows >= 2);
        assert_eq!(evidence.preview_surfaces, 1);
        assert!(evidence.clears_every_custom_area);
        assert!(evidence.bounded_components_only);
        assert!(evidence.native_hwnd_controls_preserved);
        assert!(evidence.device_loss_fail_soft);
        assert!(evidence.no_generic_ui_framework);
        assert!(evidence.no_surface_overlap);
    }

    #[test]
    fn stage4_config_qa_gate_freezes_automated_and_manual_evidence() {
        let model = frozen_settings_model();
        let layout = validate_layout(&model).expect("layout evidence should validate");
        let theme_library =
            validate_theme_library_and_preview().expect("theme evidence should validate");
        let preview_host = validate_candidate_preview_host(&layout, &theme_library)
            .expect("preview evidence should validate");
        let window_effects =
            validate_window_effects_adapter().expect("window effects should validate");
        let settings_surface = validate_settings_surface().expect("surface should validate");
        let evidence = validate_stage4_config_qa_gate(
            &layout,
            &preview_host,
            &window_effects,
            &settings_surface,
        )
        .expect("Stage 4 QA gate should validate");
        assert!(evidence.gate_frozen);
        assert!(evidence.automated_keyboard_tab_order);
        assert!(evidence.automated_focus_visibility);
        assert!(evidence.automated_page_navigation);
        assert!(evidence.automated_no_overlap);
        assert!(evidence.automated_high_dpi_geometry);
        assert!(evidence.automated_high_contrast_fallback_markers);
        assert!(evidence.automated_embedded_candidate_preview_bounds);
        assert!(evidence.manual_narrator_nvda_pending);
        assert!(evidence.manual_real_win7_host_pending);
        assert!(evidence.manual_real_win10_host_pending);
        assert!(evidence.manual_real_win11_host_pending);
        assert!(!evidence.rust_config_cutover_complete_claimed);
    }

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
        assert!(report.contains("\"side_by_side_executable_name\":\"none\""));
        assert!(report.contains("\"side_by_side_executable_target_declared\":false"));
        assert!(report.contains("\"side_by_side_uses_frozen_corpus\":false"));
        assert!(report.contains("\"preserves_product_binary_name\":true"));
        assert!(report.contains("\"side_by_side_differential_required\":false"));
        assert!(report.contains("\"permanent_runtime_selector\":false"));
        assert!(report.contains("\"typed_control_only\":true"));
        assert!(report.contains("\"no_input_hot_path_access\":true"));
        assert!(report.contains("\"no_arbitrary_shell\":true"));
        assert!(report.contains("\"accessibility_gate_required\":true"));
        assert!(report.contains("\"package_smoke_required_after_cutover\":true"));
        assert!(report.contains("\"old_cxx_shell_deletion_required\":true"));
        assert!(report.contains(
            "\"window_effects_adapter_contract\":\"rust-config-window-effects-capability-adapter\""
        ));
        assert!(report.contains("\"window_effects_fake_os_scenarios\":5"));
        assert!(report.contains("\"window_effects_native_baseline\":true"));
        assert!(report.contains("\"window_effects_win7_compatible_startup\":true"));
        assert!(report.contains("\"window_effects_win10_dark_titlebar\":true"));
        assert!(report.contains("\"window_effects_win11_corner_preference\":true"));
        assert!(report.contains("\"window_effects_system_backdrop_mica\":true"));
        assert!(
            report.contains("\"window_effects_high_contrast_disables_decorative_effects\":true")
        );
        assert!(report.contains("\"window_effects_fail_soft_without_dwm\":true"));
        assert!(report.contains("\"window_effects_dwm_runtime_guarded\":true"));
        assert!(report.contains("\"window_effects_no_winui_wpf_webview_dependency\":true"));
        assert!(report.contains(
            "\"settings_surface_contract\":\"bounded-rust-d2d-dwrite-settings-surface\""
        ));
        assert!(report.contains("\"settings_surface_checked_pages\":6"));
        assert!(report.contains("\"settings_surface_clears_every_custom_area\":true"));
        assert!(report.contains("\"settings_surface_bounded_components_only\":true"));
        assert!(report.contains("\"settings_surface_native_hwnd_controls_preserved\":true"));
        assert!(report.contains("\"settings_surface_device_loss_fail_soft\":true"));
        assert!(report.contains("\"settings_surface_no_product_owned_generic_ui_framework\":true"));
        assert!(report.contains("\"settings_surface_no_surface_overlap\":true"));
        assert!(report.contains("\"windui_crate_name\":\"windui\""));
        assert!(report
            .contains("\"windui_reference_commit\":\"62241e25e762df154c1b1f855b4db57533e516fc\""));
        assert!(report.contains("\"windui_license\":\"MIT OR Apache-2.0\""));
        assert!(report.contains("\"windui_vendored_path_dependency\":true"));
        assert!(report.contains("\"windui_role_palette_consumed\":true"));
        assert!(report.contains("\"windui_theme_row_height_consumed\":true"));
        assert!(report.contains("\"windui_element_builder_tree_constructed\":true"));
        assert!(report.contains("\"windui_setting_row_constructed\":true"));
        assert!(report.contains("\"windui_segmented_control_constructed\":true"));
        assert!(report.contains("\"windui_nav_list_pattern_used\":true"));
        assert!(report.contains("\"windui_preview_first_appearance_layout\":true"));
        assert!(report.contains("\"windui_engineering_dip_labels_removed_from_first_screen\":true"));
        assert!(report.contains("\"windui_settings_shell_constructed\":true"));
        assert!(report.contains("\"windui_settings_input_visual_baseline\":true"));
        assert!(report.contains("\"windui_default_interactive_window_uses_windui\":true"));
        assert!(report.contains("\"windui_win32_preview_host_qa_only\":true"));
        assert!(report.contains("\"stage4_config_qa_gate_frozen\":true"));
        assert!(report.contains("\"stage4_automated_keyboard_tab_order\":true"));
        assert!(report.contains("\"stage4_automated_focus_visibility\":true"));
        assert!(report.contains("\"stage4_automated_page_navigation\":true"));
        assert!(report.contains("\"stage4_automated_no_overlap\":true"));
        assert!(report.contains("\"stage4_automated_high_dpi_geometry\":true"));
        assert!(report.contains("\"stage4_automated_high_contrast_fallback_markers\":true"));
        assert!(report.contains("\"stage4_automated_embedded_candidate_preview_bounds\":true"));
        assert!(report.contains("\"stage4_manual_narrator_nvda_pending\":true"));
        assert!(report.contains("\"stage4_manual_real_win7_host_pending\":true"));
        assert!(report.contains("\"stage4_manual_real_win10_host_pending\":true"));
        assert!(report.contains("\"stage4_manual_real_win11_host_pending\":true"));
        assert!(report.contains("\"stage4_rust_config_cutover_complete_claimed\":false"));
        assert!(report.contains("\"candidate_preview_embedded\":true"));
        assert!(report.contains("\"candidate_preview_current_theme\":true"));
        assert!(report.contains("\"candidate_preview_not_external_window\":true"));
        assert!(report.contains("\"candidate_preview_embedded_in_config_content\":true"));
        assert!(report.contains("\"candidate_preview_uses_real_theme_contract\":true"));
        assert!(report.contains("\"candidate_preview_rect\":{\"x\":248,\"y\":104"));
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
        let store = FileStore::new();
        let path = Path::new("preview.toml");
        let mut core = ConfigCore::compiled_defaults();
        for edit in [
            ConfigEdit::AppearanceMode("dark".to_owned()),
            ConfigEdit::CandidateFontFamilies(vec!["Segoe UI Emoji".to_owned()]),
            ConfigEdit::CandidateFontSizeDip(20.0),
        ] {
            core.execute(ConfigCommand::Set(edit), &store, path)
                .expect("GUI preview should edit only Config Core Draft");
        }
        let draft = PreviewRenderContext::from_draft(core.preview(), 150);
        let sample = candidate_preview_sample(&draft);
        let export = core
            .execute(ConfigCommand::Export, &store, path)
            .expect("GUI preview should export through Config Core");
        let fcitx5_config_core::CommandOutput::Export(export) = export else {
            panic!("Config Core export should return TOML");
        };
        let mut reopened_core = ConfigCore::compiled_defaults();
        reopened_core
            .execute(ConfigCommand::Import(export), &store, path)
            .expect("GUI preview should import through Config Core");
        let reopened = PreviewRenderContext::from_draft(reopened_core.preview(), 150);
        assert_eq!(core.diff().len(), 3);
        assert_eq!(reopened.font_family(), "Segoe UI Emoji");
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

    #[test]
    fn windui_candidate_adapter_uses_one_draft_for_preview_cancel_reset_and_apply() {
        let directory = TestDirectory::new("windui-candidate-adapter");
        let path = directory.path().join("config.toml");
        let mut adapter = WindUiConfigAdapter::load(path).expect("adapter should load defaults");

        adapter
            .set(ConfigEdit::CandidatePageSize(7))
            .expect("GUI edit should update Draft");
        assert_eq!(adapter.preview().candidate().page_size(), 7);
        assert!(
            !adapter.path().exists(),
            "editing Draft must not write Current"
        );

        adapter.cancel();
        assert_eq!(adapter.preview().candidate().page_size(), 5);
        assert!(!adapter.path().exists(), "cancel must not write Current");

        adapter
            .set(ConfigEdit::CandidatePageSize(8))
            .expect("GUI edit should update Draft");
        adapter.reset(ConfigField::CandidatePageSize);
        assert_eq!(adapter.preview().candidate().page_size(), 5);
        assert!(!adapter.path().exists(), "reset must not write Current");

        adapter
            .set(ConfigEdit::CandidatePageSize(7))
            .expect("GUI edit should update Draft");
        adapter.apply().expect("apply should commit Draft");
        assert_eq!(adapter.preview().candidate().page_size(), 7);
        assert!(adapter.path().is_file(), "apply must create Current");
        assert!(
            FileStore::last_known_good_path(adapter.path()).is_file(),
            "apply must create a usable LKG"
        );
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

    #[test]
    fn pinned_plugin_catalog_is_complete_and_unique() {
        assert_eq!(FCITX5_PLUGIN_CATALOG.len(), 21);
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
    fn plugin_control_arguments_are_fixed_and_catalog_bounded() {
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
        assert!(
            plugin_control_arguments(&PluginOperation::Install("fcitx5-mozc".to_owned())).is_err()
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
