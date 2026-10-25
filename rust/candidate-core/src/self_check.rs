//! Headless candidate scenarios and the JSON evidence report they produce.

#![forbid(unsafe_code)]

use super::*;

#[derive(Clone, Debug)]
pub struct SelfCheckCandidate {
    pub label: String,
    pub text: String,
    pub comment: String,
}

#[derive(Clone, Debug)]
pub struct SelfCheckScenario {
    pub name: &'static str,
    pub host: &'static str,
    pub locale: &'static str,
    pub dpi_scale: f32,
    pub popup_allowed: bool,
    pub presentation: PresentationOrientation,
    pub identity: CompositionIdentity,
    pub caret: Point,
    pub work_area: Rect,
    pub candidates: Vec<SelfCheckCandidate>,
    pub selected: usize,
    pub expected_orientation: Orientation,
}

#[derive(Clone, Debug)]
pub struct SelfCheckEvidence {
    pub name: &'static str,
    pub host: &'static str,
    pub locale: &'static str,
    pub dpi_scale: f32,
    pub popup_allowed: bool,
    pub orientation: Orientation,
    pub placement: Placement,
    pub window: Rect,
    pub visible_candidates: usize,
    pub selected: usize,
    pub accessibility_name: String,
    pub color_font_candidate_present: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidatePreviewPaintItem {
    pub text: String,
    pub bounds: Rect,
    pub selected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidatePreviewPaintPlan {
    pub dpi_scale: f32,
    pub background_color: u32,
    pub selected_background_color: u32,
    pub text_color: u32,
    pub selected_text_color: u32,
    pub items: Vec<CandidatePreviewPaintItem>,
}

pub fn run_candidate_self_check() -> Result<String, String> {
    let mut evidence = Vec::new();
    for scenario in candidate_self_check_scenarios() {
        evidence.push(check_candidate_scenario(&scenario)?);
    }
    Ok(render_candidate_self_check_report(&evidence))
}

pub fn candidate_preview_paint_plan(
    dpi_scale: f32,
    width: f32,
    height: f32,
) -> Result<CandidatePreviewPaintPlan, String> {
    if !dpi_scale.is_finite() || !(0.5..=4.0).contains(&dpi_scale) {
        return Err("candidate preview DPI scale must be finite and within 0.5..=4.0".to_owned());
    }
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err("candidate preview paint plan requires a positive finite surface".to_owned());
    }
    let candidates = [
        poc_candidate("1.", "你", ""),
        poc_candidate("2.", "好", ""),
        poc_candidate("3.", "😀", "emoji"),
    ];
    let item_sizes = candidates
        .iter()
        .map(|candidate| measure_candidate(candidate, dpi_scale))
        .collect::<Vec<_>>();
    let result = layout(&LayoutInput {
        orientation: Orientation::Horizontal,
        items: item_sizes,
        caret: Point { x: 0.0, y: 0.0 },
        caret_height: 0.0,
        work_area: Rect {
            left: 0.0,
            top: 0.0,
            right: width,
            bottom: height,
        },
        max_width: width,
        padding_x: 8.0 * dpi_scale,
        padding_y: 8.0 * dpi_scale,
        row_gap: 4.0 * dpi_scale,
        column_gap: 8.0 * dpi_scale,
        placement: Placement::Below,
        scroll_mode: false,
        scroll_columns: 6,
        scroll_visible_rows: 6,
        selected: 0,
        scroll_cell_width: 96.0 * dpi_scale,
    });
    if result.items.is_empty() {
        return Err("candidate preview paint plan produced no visible candidates".to_owned());
    }
    let mut items = Vec::with_capacity(result.items.len());
    for (visible_index, (candidate_index, bounds)) in result
        .item_indices
        .iter()
        .copied()
        .zip(result.items.iter().copied())
        .enumerate()
    {
        let candidate = candidates
            .get(candidate_index)
            .ok_or_else(|| "candidate preview paint plan produced an invalid index".to_owned())?;
        if bounds.left < 0.0 || bounds.top < 0.0 || bounds.right > width || bounds.bottom > height {
            return Err("candidate preview paint plan produced an out-of-bounds item".to_owned());
        }
        items.push(CandidatePreviewPaintItem {
            text: format!(
                "{} {} {}",
                candidate.label, candidate.text, candidate.comment
            )
            .trim()
            .to_owned(),
            bounds,
            selected: visible_index == 0,
        });
    }
    Ok(CandidatePreviewPaintPlan {
        dpi_scale,
        background_color: 0x00ee_f3f7,
        selected_background_color: theme_tokens::WECHAT_GREEN_COLORREF,
        text_color: 0x0020_2020,
        selected_text_color: theme_tokens::WHITE_COLORREF,
        items,
    })
}

pub fn candidate_self_check_scenarios() -> Vec<SelfCheckScenario> {
    let work_area = Rect {
        left: 0.0,
        top: 0.0,
        right: 1920.0,
        bottom: 1080.0,
    };
    vec![
        SelfCheckScenario {
            name: "notepad-horizontal-zh",
            host: "mock-notepad",
            locale: "zh-CN",
            dpi_scale: 1.0,
            popup_allowed: true,
            presentation: PresentationOrientation::Automatic,
            identity: CompositionIdentity {
                engine_epoch: 7,
                context_id: 11,
                composition_id: 101,
            },
            caret: Point { x: 120.0, y: 240.0 },
            work_area,
            candidates: vec![
                poc_candidate("1", "你", ""),
                poc_candidate("2", "好", ""),
                poc_candidate("3", "呢", "particle"),
            ],
            selected: 1,
            expected_orientation: Orientation::Horizontal,
        },
        SelfCheckScenario {
            name: "word-vertical-annotation",
            host: "mock-word",
            locale: "zh-CN",
            dpi_scale: 1.25,
            popup_allowed: true,
            presentation: PresentationOrientation::Automatic,
            identity: CompositionIdentity {
                engine_epoch: 7,
                context_id: 12,
                composition_id: 201,
            },
            caret: Point { x: 640.0, y: 420.0 },
            work_area,
            candidates: vec![
                poc_candidate("1", "候选", "annotation-longer-than-compact-budget"),
                poc_candidate("2", "候補", "traditional"),
                poc_candidate("3", "candidate", "latin fallback"),
            ],
            selected: 0,
            expected_orientation: Orientation::Vertical,
        },
        SelfCheckScenario {
            name: "chromium-right-edge-dpi200",
            host: "mock-chromium",
            locale: "zh-CN",
            dpi_scale: 2.0,
            popup_allowed: true,
            presentation: PresentationOrientation::Automatic,
            identity: CompositionIdentity {
                engine_epoch: 7,
                context_id: 13,
                composition_id: 301,
            },
            caret: Point {
                x: 1880.0,
                y: 700.0,
            },
            work_area,
            candidates: vec![
                poc_candidate("1", "边缘", ""),
                poc_candidate("2", "edge", ""),
            ],
            selected: 0,
            expected_orientation: Orientation::Vertical,
        },
        SelfCheckScenario {
            name: "vscode-uiless-accessibility",
            host: "mock-vscode-uiless",
            locale: "en-US",
            dpi_scale: 1.5,
            popup_allowed: false,
            presentation: PresentationOrientation::Vertical,
            identity: CompositionIdentity {
                engine_epoch: 7,
                context_id: 14,
                composition_id: 401,
            },
            caret: Point { x: 320.0, y: 300.0 },
            work_area,
            candidates: vec![
                poc_candidate("1", "hello", ""),
                poc_candidate("2", "world", ""),
            ],
            selected: 1,
            expected_orientation: Orientation::Vertical,
        },
        SelfCheckScenario {
            name: "emoji-color-font",
            host: "mock-emoji-host",
            locale: "zh-CN",
            dpi_scale: 1.0,
            popup_allowed: true,
            presentation: PresentationOrientation::Horizontal,
            identity: CompositionIdentity {
                engine_epoch: 7,
                context_id: 15,
                composition_id: 501,
            },
            caret: Point { x: 180.0, y: 360.0 },
            work_area,
            candidates: vec![
                poc_candidate("1", "😀", "emoji"),
                poc_candidate("2", "🏳️‍🌈", "zwj color glyph"),
                poc_candidate("3", "候选", "text fallback"),
            ],
            selected: 0,
            expected_orientation: Orientation::Horizontal,
        },
    ]
}

fn check_candidate_scenario(scenario: &SelfCheckScenario) -> Result<SelfCheckEvidence, String> {
    if scenario.candidates.is_empty() {
        return Err(format!("{} has no candidates", scenario.name));
    }
    if scenario.selected >= scenario.candidates.len() {
        return Err(format!("{} has invalid selected index", scenario.name));
    }

    let mut state = CompositionLayoutState::default();
    state.begin(scenario.identity);
    let orientation = state.resolve_orientation(
        scenario.presentation,
        AutomaticOrientationInput {
            candidates: &scenario
                .candidates
                .iter()
                .map(|candidate| CandidateText {
                    text: candidate.text.clone(),
                    comment: candidate.comment.clone(),
                })
                .collect::<Vec<_>>(),
            locale: scenario.locale,
            work_area: scenario.work_area,
            caret_x: scenario.caret.x,
            scale: scenario.dpi_scale,
            page_size: 9,
        },
    );
    if orientation != scenario.expected_orientation {
        return Err(format!(
            "{} orientation mismatch: got {:?}, expected {:?}",
            scenario.name, orientation, scenario.expected_orientation
        ));
    }

    let item_sizes = scenario
        .candidates
        .iter()
        .map(|candidate| measure_candidate(candidate, scenario.dpi_scale))
        .collect::<Vec<_>>();
    let result = layout(&LayoutInput {
        orientation,
        items: item_sizes,
        caret: scenario.caret,
        caret_height: 24.0 * scenario.dpi_scale,
        work_area: scenario.work_area,
        max_width: 720.0 * scenario.dpi_scale,
        padding_x: 8.0 * scenario.dpi_scale,
        padding_y: 6.0 * scenario.dpi_scale,
        row_gap: 2.0 * scenario.dpi_scale,
        column_gap: 8.0 * scenario.dpi_scale,
        selected: scenario.selected,
        ..LayoutInput::default()
    });
    if result.items.len() != scenario.candidates.len() {
        return Err(format!(
            "{} visible candidate count mismatch: got {}, expected {}",
            scenario.name,
            result.items.len(),
            scenario.candidates.len()
        ));
    }
    if !rect_inside(result.window, scenario.work_area) {
        return Err(format!("{} window is outside work area", scenario.name));
    }
    for (left_index, left) in result.items.iter().enumerate() {
        if !rect_inside(*left, result.window) {
            return Err(format!(
                "{} candidate {left_index} is outside window",
                scenario.name
            ));
        }
        for (right_index, right) in result.items.iter().enumerate().skip(left_index + 1) {
            if rects_overlap(*left, *right) {
                return Err(format!(
                    "{} candidate rectangles overlap: {left_index} and {right_index}",
                    scenario.name
                ));
            }
        }
    }

    let selected = &scenario.candidates[scenario.selected];
    let accessibility_name = format!("{} {} {}", selected.label, selected.text, selected.comment)
        .trim()
        .to_owned();
    if accessibility_name.is_empty() {
        return Err(format!(
            "{} selected candidate has no accessibility name",
            scenario.name
        ));
    }

    Ok(SelfCheckEvidence {
        name: scenario.name,
        host: scenario.host,
        locale: scenario.locale,
        dpi_scale: scenario.dpi_scale,
        popup_allowed: scenario.popup_allowed,
        orientation,
        placement: result.placement,
        window: result.window,
        visible_candidates: result.items.len(),
        selected: scenario.selected,
        accessibility_name,
        color_font_candidate_present: scenario
            .candidates
            .iter()
            .any(|candidate| contains_non_bmp_or_zwj(&candidate.text)),
    })
}

fn render_candidate_self_check_report(evidence: &[SelfCheckEvidence]) -> String {
    let mut output = String::from(
        "{\n  \"component\":\"fcitx5-candidate-self-check\",\n  \"kind\":\"rust-out-of-process-headless-self-check\",\n  \"cpp_ffi\":false,\n  \"send_input\":false,\n  \"global_hooks\":false,\n  \"process_injection\":false,\n  \"scenarios\":[\n",
    );
    for (index, item) in evidence.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        output.push_str("    {");
        push_json_field(&mut output, "name", item.name, false);
        push_json_field(&mut output, "host", item.host, true);
        push_json_field(&mut output, "locale", item.locale, true);
        push_json_number(&mut output, "dpi_scale", item.dpi_scale, true);
        push_json_bool(&mut output, "popup_allowed", item.popup_allowed, true);
        push_json_field(
            &mut output,
            "orientation",
            orientation_name(item.orientation),
            true,
        );
        push_json_field(
            &mut output,
            "placement",
            placement_name(item.placement),
            true,
        );
        push_json_number(&mut output, "window_left", item.window.left, true);
        push_json_number(&mut output, "window_top", item.window.top, true);
        push_json_number(&mut output, "window_right", item.window.right, true);
        push_json_number(&mut output, "window_bottom", item.window.bottom, true);
        push_json_usize(
            &mut output,
            "visible_candidates",
            item.visible_candidates,
            true,
        );
        push_json_usize(&mut output, "selected", item.selected, true);
        push_json_field(
            &mut output,
            "accessibility_name",
            &item.accessibility_name,
            true,
        );
        push_json_bool(
            &mut output,
            "color_font_candidate_present",
            item.color_font_candidate_present,
            true,
        );
        output.push('}');
    }
    output.push_str("\n  ],\n  \"result\":\"PASS\"\n}");
    output
}

fn poc_candidate(label: &str, text: &str, comment: &str) -> SelfCheckCandidate {
    SelfCheckCandidate {
        label: label.to_owned(),
        text: text.to_owned(),
        comment: comment.to_owned(),
    }
}

fn measure_candidate(candidate: &SelfCheckCandidate, scale: f32) -> Size {
    let label = text_units(&candidate.label) * 10.0 * scale;
    let text = text_units(&candidate.text) * 18.0 * scale;
    let comment = text_units(&candidate.comment) * 14.0 * scale;
    Size {
        width: (label + 14.0 * scale + text + comment + 48.0 * scale).max(64.0 * scale),
        height: 42.0 * scale,
    }
}

fn text_units(value: &str) -> f32 {
    value
        .chars()
        .map(|character| if character.len_utf8() == 1 { 0.6 } else { 1.0 })
        .sum::<f32>()
}

pub(crate) fn rect_inside(inner: Rect, outer: Rect) -> bool {
    inner.left >= outer.left - 0.01
        && inner.top >= outer.top - 0.01
        && inner.right <= outer.right + 0.01
        && inner.bottom <= outer.bottom + 0.01
        && inner.right >= inner.left
        && inner.bottom >= inner.top
}

fn rects_overlap(left: Rect, right: Rect) -> bool {
    left.left < right.right - 0.01
        && left.right > right.left + 0.01
        && left.top < right.bottom - 0.01
        && left.bottom > right.top + 0.01
}

fn contains_non_bmp_or_zwj(value: &str) -> bool {
    value
        .chars()
        .any(|character| character as u32 > 0xFFFF || character == '\u{200d}')
}

fn orientation_name(value: Orientation) -> &'static str {
    match value {
        Orientation::Vertical => "vertical",
        Orientation::Horizontal => "horizontal",
    }
}

fn placement_name(value: Placement) -> &'static str {
    match value {
        Placement::Unlocked => "unlocked",
        Placement::Below => "below",
        Placement::Above => "above",
    }
}

fn push_json_field(output: &mut String, name: &str, value: &str, comma: bool) {
    if comma {
        output.push(',');
    }
    output.push('"');
    output.push_str(name);
    output.push_str("\":\"");
    push_json_escaped(output, value);
    output.push('"');
}

fn push_json_bool(output: &mut String, name: &str, value: bool, comma: bool) {
    if comma {
        output.push(',');
    }
    output.push('"');
    output.push_str(name);
    output.push_str("\":");
    output.push_str(if value { "true" } else { "false" });
}

fn push_json_number(output: &mut String, name: &str, value: f32, comma: bool) {
    if comma {
        output.push(',');
    }
    output.push('"');
    output.push_str(name);
    output.push_str("\":");
    output.push_str(&format!("{value:.2}"));
}

fn push_json_usize(output: &mut String, name: &str, value: usize, comma: bool) {
    if comma {
        output.push(',');
    }
    output.push('"');
    output.push_str(name);
    output.push_str("\":");
    output.push_str(&value.to_string());
}

fn push_json_escaped(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character < ' ' => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
}
