#![forbid(unsafe_code)]

use crate::{
    format_candidate_label, layout, AutomaticOrientationInput, CandidateLabelStyle, CandidateModel,
    CandidatePresentationState, CandidatePresentationUpdate, CandidateSemanticSnapshot,
    CandidateText, CompositionIdentity, CompositionLayoutState, LayoutInput, Orientation,
    Placement, Point, PresentationOrientation, Rect, Size,
};

/// Color resolved for one Candidate renderer surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateUiColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl CandidateUiColor {
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

/// Fully resolved colors consumed by the native drawing adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateUiColors {
    pub background: CandidateUiColor,
    pub border: CandidateUiColor,
    pub text: CandidateUiColor,
    pub annotation: CandidateUiColor,
    pub label: CandidateUiColor,
    pub selected_background: CandidateUiColor,
    pub selected_text: CandidateUiColor,
    pub scrollbar: CandidateUiColor,
}

impl CandidateUiColors {
    const fn light() -> Self {
        Self {
            background: CandidateUiColor::rgba(255, 255, 255, 255),
            border: CandidateUiColor::rgba(215, 215, 215, 255),
            text: CandidateUiColor::rgba(32, 33, 36, 255),
            annotation: CandidateUiColor::rgba(95, 99, 104, 255),
            label: CandidateUiColor::rgba(95, 99, 104, 255),
            selected_background: CandidateUiColor::rgba(220, 235, 255, 255),
            selected_text: CandidateUiColor::rgba(23, 78, 166, 255),
            scrollbar: CandidateUiColor::rgba(128, 128, 128, 180),
        }
    }

    const fn dark() -> Self {
        Self {
            background: CandidateUiColor::rgba(32, 33, 36, 255),
            border: CandidateUiColor::rgba(90, 90, 90, 255),
            text: CandidateUiColor::rgba(255, 255, 255, 255),
            annotation: CandidateUiColor::rgba(189, 193, 198, 255),
            label: CandidateUiColor::rgba(189, 193, 198, 255),
            selected_background: CandidateUiColor::rgba(190, 220, 255, 255),
            selected_text: CandidateUiColor::rgba(32, 33, 36, 255),
            scrollbar: CandidateUiColor::rgba(189, 193, 198, 180),
        }
    }

    const fn high_contrast() -> Self {
        Self {
            background: CandidateUiColor::rgba(0, 0, 0, 255),
            border: CandidateUiColor::rgba(255, 255, 255, 255),
            text: CandidateUiColor::rgba(255, 255, 255, 255),
            annotation: CandidateUiColor::rgba(255, 255, 0, 255),
            label: CandidateUiColor::rgba(255, 255, 0, 255),
            selected_background: CandidateUiColor::rgba(0, 0, 128, 255),
            selected_text: CandidateUiColor::rgba(255, 255, 255, 255),
            scrollbar: CandidateUiColor::rgba(255, 255, 255, 255),
        }
    }
}

/// User-visible appearance mode after the system/theme resolution boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateTheme {
    Light,
    Dark,
    HighContrast,
}

/// Resolved candidate behavior and visual values. The adapter only consumes it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CandidateUiConfig {
    pub orientation: PresentationOrientation,
    pub scroll_mode: bool,
    pub page_size: u32,
    pub max_width: f32,
    pub scroll_cell_width: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub row_gap: f32,
    pub column_gap: f32,
    pub item_padding_x: f32,
    pub item_padding_y: f32,
    pub label_gap: f32,
    pub candidate_font_size: f32,
    pub theme: CandidateTheme,
    pub opacity: f32,
}

impl Default for CandidateUiConfig {
    fn default() -> Self {
        Self {
            orientation: PresentationOrientation::Automatic,
            scroll_mode: false,
            page_size: 6,
            max_width: 720.0,
            scroll_cell_width: 96.0,
            padding_x: 8.0,
            padding_y: 6.0,
            row_gap: 2.0,
            column_gap: 8.0,
            item_padding_x: 6.0,
            item_padding_y: 4.0,
            label_gap: 4.0,
            candidate_font_size: 20.0,
            theme: CandidateTheme::Light,
            opacity: 1.0,
        }
    }
}

/// DirectWrite measurements supplied by the native font-fallback adapter.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CandidateUiMeasurement {
    pub label_width: f32,
    pub text_width: f32,
    pub comment_width: f32,
    pub height: f32,
}

/// UTF-8 text that the native DirectWrite adapter must measure for one candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateUiText {
    pub candidate_index: usize,
    pub label: String,
    pub text: String,
    pub comment: String,
}

impl CandidateUiMeasurement {
    #[must_use]
    pub const fn new(label_width: f32, text_width: f32, comment_width: f32, height: f32) -> Self {
        Self {
            label_width,
            text_width,
            comment_width,
            height,
        }
    }
}

/// Immutable state input supplied by the Rust protocol/transport owner.
#[derive(Clone, Debug)]
pub struct CandidateUiInput {
    pub snapshot: CandidateSemanticSnapshot,
    pub locale: String,
    pub caret: Point,
    pub caret_height: f32,
    pub work_area: Rect,
    pub candidate_bulk: bool,
    pub config: CandidateUiConfig,
}

/// Outcome of applying a frame to the Candidate UI state machine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateUiApplyResult {
    Applied,
    Duplicate,
    Stale,
    Invalid,
}

/// One complete item for the native renderer. Strings and geometry share one revision.
#[derive(Clone, Debug, PartialEq)]
pub struct CandidateRenderItem {
    pub candidate_index: usize,
    pub id: u64,
    pub label: String,
    pub text: String,
    pub comment: String,
    pub item_rect: Rect,
    pub label_rect: Rect,
    pub text_rect: Rect,
    pub comment_rect: Option<Rect>,
    pub selected: bool,
}

/// Immutable UI Automation projection for the same candidate revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateUiaItem {
    pub candidate_index: usize,
    pub name: String,
    pub selected: bool,
}

/// Candidate UI Automation projection. Actual providers are native adapters.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CandidateUiaPlan {
    pub live_region_name: String,
    pub items: Vec<CandidateUiaItem>,
}

/// Immutable renderer and accessibility plan for one presentation revision.
#[derive(Clone, Debug, PartialEq)]
pub struct CandidateUiPlan {
    pub popup_visible: bool,
    pub orientation: Orientation,
    pub placement: Placement,
    pub window: Rect,
    pub preedit: String,
    pub preedit_rect: Option<Rect>,
    pub colors: CandidateUiColors,
    pub opacity: f32,
    pub scrollbar_track: Option<Rect>,
    pub scrollbar_thumb: Option<Rect>,
    pub items: Vec<CandidateRenderItem>,
    pub uia: CandidateUiaPlan,
}

impl Default for CandidateUiPlan {
    fn default() -> Self {
        Self {
            popup_visible: false,
            orientation: Orientation::Vertical,
            placement: Placement::Below,
            window: Rect::default(),
            preedit: String::new(),
            preedit_rect: None,
            colors: CandidateUiColors::light(),
            opacity: 1.0,
            scrollbar_track: None,
            scrollbar_thumb: None,
            items: Vec::new(),
            uia: CandidateUiaPlan::default(),
        }
    }
}

/// Rust-owned Candidate presentation state. It has no HWND, D2D, DWrite, or UIA provider state.
#[derive(Default)]
pub struct CandidateUiState {
    model: CandidateModel,
    presentation: CandidatePresentationState,
    layout: CompositionLayoutState,
    input: Option<CandidateUiInput>,
}

impl CandidateUiState {
    /// Applies one validated Candidate semantic snapshot.
    pub fn apply(&mut self, input: CandidateUiInput) -> CandidateUiApplyResult {
        if !valid_ui_input(&input) {
            return CandidateUiApplyResult::Invalid;
        }
        let model_result = self.model.apply_semantic_snapshot(input.snapshot.clone());
        let result = match model_result {
            0 => CandidateUiApplyResult::Applied,
            1 => CandidateUiApplyResult::Duplicate,
            2 => CandidateUiApplyResult::Stale,
            _ => CandidateUiApplyResult::Invalid,
        };
        if result != CandidateUiApplyResult::Applied {
            return result;
        }
        let snapshot = &input.snapshot;
        let presentation_result = self.presentation.apply(CandidatePresentationUpdate {
            engine_epoch: snapshot.identity.engine_epoch,
            context_id: snapshot.identity.context_id,
            composition_id: snapshot.identity.composition_id,
            revision: snapshot.identity.revision,
            selected: snapshot.selected.unwrap_or_default(),
            has_selected: u8::from(snapshot.selected.is_some()),
            candidate_count: snapshot.candidates.len(),
            page: snapshot.page,
            page_size: input.config.page_size,
            candidate_bulk: u8::from(input.candidate_bulk),
            configured_scroll_mode: u8::from(input.config.scroll_mode),
        });
        if presentation_result != 0 {
            return CandidateUiApplyResult::Invalid;
        }
        self.layout.begin(CompositionIdentity {
            engine_epoch: snapshot.identity.engine_epoch,
            context_id: snapshot.identity.context_id,
            composition_id: snapshot.identity.composition_id,
        });
        self.input = Some(input);
        CandidateUiApplyResult::Applied
    }

    /// Returns the text owned by the current Rust semantic snapshot for DWrite measurement.
    #[must_use]
    pub fn measurement_texts(&self) -> Vec<CandidateUiText> {
        self.model
            .semantic_snapshot()
            .map(|snapshot| {
                snapshot
                    .candidates
                    .iter()
                    .enumerate()
                    .map(|(candidate_index, candidate)| CandidateUiText {
                        candidate_index,
                        label: format_candidate_label(
                            (candidate_index + 1) as u32,
                            &candidate.label,
                            CandidateLabelStyle::Dot,
                            "",
                            "",
                        ),
                        text: candidate.text.clone(),
                        comment: candidate.comment.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Builds an immutable renderer/UIA plan using native text measurements.
    #[must_use]
    pub fn render_plan(&mut self, measurements: &[CandidateUiMeasurement]) -> CandidateUiPlan {
        let Some(input) = self.input.as_ref() else {
            return CandidateUiPlan::default();
        };
        let Some(snapshot) = self.model.semantic_snapshot() else {
            return CandidateUiPlan::default();
        };
        let config = input.config;
        let colors = colors_for(config.theme);
        let popup_visible = snapshot.visibility != 0
            && snapshot.popup_allowed
            && !snapshot.candidates.is_empty()
            && input.caret_height > 0.0;
        if !popup_visible {
            return CandidateUiPlan {
                colors,
                opacity: config.opacity.clamp(0.2, 1.0),
                uia: uia_plan(&snapshot, &[]),
                ..CandidateUiPlan::default()
            };
        }
        let candidate_text = snapshot
            .candidates
            .iter()
            .map(|candidate| CandidateText {
                text: candidate.text.clone(),
                comment: candidate.comment.clone(),
            })
            .collect::<Vec<_>>();
        let orientation = self.layout.resolve_orientation(
            config.orientation,
            AutomaticOrientationInput {
                candidates: &candidate_text,
                locale: &input.locale,
                work_area: input.work_area,
                caret_x: input.caret.x,
                scale: 1.0,
                page_size: config.page_size,
            },
        );
        let presentation = self.presentation.output();
        let render_indices = visible_indices(snapshot.candidates.len(), presentation);
        let item_sizes = render_indices
            .iter()
            .map(|index| {
                item_size(
                    measurements.get(*index).copied().unwrap_or_default(),
                    config,
                )
            })
            .collect::<Vec<_>>();
        let selected = if presentation.has_selected != 0 {
            presentation.selected
        } else {
            0
        };
        let layout_result = layout(&LayoutInput {
            orientation,
            items: item_sizes,
            caret: input.caret,
            caret_height: input.caret_height,
            work_area: input.work_area,
            max_width: config.max_width.max(0.0),
            padding_x: config.padding_x.max(0.0),
            padding_y: config.padding_y.max(0.0),
            row_gap: config.row_gap.max(0.0),
            column_gap: config.column_gap.max(0.0),
            placement: placement_from_output(presentation.placement),
            scroll_mode: presentation.scroll_mode != 0,
            scroll_columns: presentation.scroll_columns,
            scroll_visible_rows: 6,
            selected: selected.min(render_indices.len().saturating_sub(1)),
            scroll_cell_width: config.scroll_cell_width.max(40.0),
        });
        self.presentation.set_placement(layout_result.placement);
        let width = self.presentation.stable_window_width(
            layout_result.window.right - layout_result.window.left,
            (input.work_area.right - input.work_area.left).min(config.max_width),
        );
        let window = Rect {
            right: layout_result.window.left + width.max(0.0),
            ..layout_result.window
        };
        let mut items = Vec::with_capacity(layout_result.items.len());
        for (local_index, bounds) in layout_result.items.iter().enumerate() {
            let Some(candidate_index) = layout_result
                .item_indices
                .get(local_index)
                .and_then(|visible| render_indices.get(*visible))
                .copied()
            else {
                continue;
            };
            let Some(candidate) = snapshot.candidates.get(candidate_index) else {
                continue;
            };
            let measurement = measurements
                .get(candidate_index)
                .copied()
                .unwrap_or_default();
            items.push(render_item(
                candidate_index,
                candidate,
                measurement,
                *bounds,
                window,
                config,
                snapshot.selected == Some(candidate_index),
            ));
        }
        let uia = uia_plan(&snapshot, &items);
        CandidateUiPlan {
            popup_visible: true,
            orientation,
            placement: layout_result.placement,
            window,
            preedit: snapshot.preedit.clone(),
            preedit_rect: (!snapshot.preedit.is_empty()).then_some(Rect {
                left: config.padding_x,
                top: config.padding_y,
                right: width - config.padding_x,
                bottom: config.padding_y + config.candidate_font_size + config.item_padding_y * 2.0,
            }),
            colors,
            opacity: config.opacity.clamp(0.2, 1.0),
            scrollbar_track: layout_result
                .has_scrollbar
                .then_some(translate_to_window(layout_result.scrollbar_track, window)),
            scrollbar_thumb: layout_result
                .has_scrollbar
                .then_some(translate_to_window(layout_result.scrollbar_thumb, window)),
            items,
            uia,
        }
    }
}

fn valid_ui_input(input: &CandidateUiInput) -> bool {
    let config = input.config;
    input.caret.x.is_finite()
        && input.caret.y.is_finite()
        && input.caret_height.is_finite()
        && input.caret_height >= 0.0
        && input.work_area.left.is_finite()
        && input.work_area.top.is_finite()
        && input.work_area.right.is_finite()
        && input.work_area.bottom.is_finite()
        && input.work_area.right > input.work_area.left
        && input.work_area.bottom > input.work_area.top
        && config.page_size != 0
        && config.max_width.is_finite()
        && config.max_width >= 0.0
        && config.scroll_cell_width.is_finite()
        && config.scroll_cell_width >= 0.0
        && config.padding_x.is_finite()
        && config.padding_x >= 0.0
        && config.padding_y.is_finite()
        && config.padding_y >= 0.0
        && config.row_gap.is_finite()
        && config.row_gap >= 0.0
        && config.column_gap.is_finite()
        && config.column_gap >= 0.0
        && config.item_padding_x.is_finite()
        && config.item_padding_x >= 0.0
        && config.item_padding_y.is_finite()
        && config.item_padding_y >= 0.0
        && config.label_gap.is_finite()
        && config.label_gap >= 0.0
        && config.candidate_font_size.is_finite()
        && config.candidate_font_size >= 0.0
        && config.opacity.is_finite()
        && config.opacity >= 0.0
}

fn colors_for(theme: CandidateTheme) -> CandidateUiColors {
    match theme {
        CandidateTheme::Light => CandidateUiColors::light(),
        CandidateTheme::Dark => CandidateUiColors::dark(),
        CandidateTheme::HighContrast => CandidateUiColors::high_contrast(),
    }
}

fn visible_indices(count: usize, presentation: crate::CandidatePresentationOutput) -> Vec<usize> {
    if presentation.scroll_mode != 0 {
        return (0..count).collect();
    }
    let end = presentation
        .ordinary_start
        .saturating_add(presentation.ordinary_count)
        .min(count);
    (presentation.ordinary_start..end).collect()
}

fn item_size(measurement: CandidateUiMeasurement, config: CandidateUiConfig) -> Size {
    let label = measurement.label_width.max(0.0);
    let text = measurement
        .text_width
        .max(config.candidate_font_size.max(0.0) + 2.0);
    let comment = measurement.comment_width.max(0.0);
    Size {
        width: config.item_padding_x.max(0.0) * 2.0
            + label
            + if label > 0.0 {
                config.label_gap.max(0.0)
            } else {
                0.0
            }
            + text
            + if comment > 0.0 {
                config.label_gap.max(0.0) + comment
            } else {
                0.0
            },
        height: measurement
            .height
            .max(config.candidate_font_size.max(0.0) + 2.0)
            + config.item_padding_y.max(0.0) * 2.0,
    }
}

fn render_item(
    candidate_index: usize,
    candidate: &crate::CandidateSemanticItem,
    measurement: CandidateUiMeasurement,
    bounds: Rect,
    window: Rect,
    config: CandidateUiConfig,
    selected: bool,
) -> CandidateRenderItem {
    let item_rect = translate_to_window(bounds, window);
    let content_left = item_rect.left + config.item_padding_x.max(0.0);
    let content_top = item_rect.top + config.item_padding_y.max(0.0);
    let content_bottom = item_rect.bottom - config.item_padding_y.max(0.0);
    let label = format_candidate_label(
        (candidate_index + 1) as u32,
        &candidate.label,
        CandidateLabelStyle::Dot,
        "",
        "",
    );
    let label_width = measurement.label_width.max(0.0);
    let text_width = measurement
        .text_width
        .max(config.candidate_font_size.max(0.0) + 2.0);
    let label_rect = Rect {
        left: content_left,
        top: content_top,
        right: content_left + label_width,
        bottom: content_bottom,
    };
    let text_left = if label_width > 0.0 {
        label_rect.right + config.label_gap.max(0.0)
    } else {
        content_left
    };
    let text_rect = Rect {
        left: text_left,
        top: content_top,
        right: text_left + text_width,
        bottom: content_bottom,
    };
    let comment_rect = (!candidate.comment.is_empty()).then_some(Rect {
        left: text_rect.right + config.label_gap.max(0.0),
        top: content_top,
        right: text_rect.right + config.label_gap.max(0.0) + measurement.comment_width.max(0.0),
        bottom: content_bottom,
    });
    CandidateRenderItem {
        candidate_index,
        id: candidate.id,
        label,
        text: candidate.text.clone(),
        comment: candidate.comment.clone(),
        item_rect,
        label_rect,
        text_rect,
        comment_rect,
        selected,
    }
}

fn translate_to_window(rect: Rect, window: Rect) -> Rect {
    Rect {
        left: rect.left - window.left,
        top: rect.top - window.top,
        right: rect.right - window.left,
        bottom: rect.bottom - window.top,
    }
}

fn placement_from_output(value: u32) -> Placement {
    match value {
        2 => Placement::Above,
        1 => Placement::Below,
        _ => Placement::Unlocked,
    }
}

fn uia_plan(
    snapshot: &CandidateSemanticSnapshot,
    items: &[CandidateRenderItem],
) -> CandidateUiaPlan {
    let items = items
        .iter()
        .map(|item| CandidateUiaItem {
            candidate_index: item.candidate_index,
            name: [
                item.label.as_str(),
                item.text.as_str(),
                item.comment.as_str(),
            ]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
            selected: item.selected,
        })
        .collect();
    CandidateUiaPlan {
        live_region_name: snapshot.preedit.clone(),
        items,
    }
}
