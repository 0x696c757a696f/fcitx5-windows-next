#![forbid(unsafe_code)]

use super::{valid_text, MAX_CANDIDATES, MAX_CANDIDATE_TEXT_UTF8};
use crate::axis_layout::{self, CandidateLayoutOptions, OverflowBehavior, WritingMode};
use crate::renderer::{
    grapheme_clusters, horizontal_column_anchors, horizontal_run_plan_with_anchors,
    HorizontalColumnAnchors, HorizontalColumnInput,
};
use crate::{Orientation, Placement, Point, Rect, Size};

const SEED: u64 = 0x4341_4E44_5F46_555A;
const CASES: usize = 4096;

struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 7;
        self.0 ^= self.0 >> 9;
        self.0 ^= self.0 << 8;
        self.0
    }

    fn below(&mut self, upper: usize) -> usize {
        if upper == 0 {
            0
        } else {
            (self.next() as usize) % upper
        }
    }

    fn between(&mut self, low: f32, high: f32) -> f32 {
        let fraction = (self.next() as f64 / u64::MAX as f64) as f32;
        low + (high - low) * fraction
    }
}

struct CandidateFixture {
    label: String,
    text: String,
    comment: String,
}

fn candidate_fixture(rng: &mut Rng, iteration: usize, index: usize) -> CandidateFixture {
    let text = match (iteration + index + rng.below(10)) % 10 {
        0 => "alpha",
        1 => "你好",
        2 => "这是一个非常非常长的候选词条",
        3 => "中文Windows Next",
        4 => "e\u{301}",
        5 => "😀",
        6 => "👍🏽",
        7 => "🇨🇳",
        8 => "👨‍👩‍👧‍👦",
        _ => "1️⃣",
    }
    .to_owned();
    let comment = match (iteration + index) % 4 {
        0 => String::new(),
        1 => "注释".to_owned(),
        2 => "annotation 注释".to_owned(),
        _ => "这是一个很长的候选注释 annotation".to_owned(),
    };
    CandidateFixture {
        label: format!("{}.", index + 1),
        text,
        comment,
    }
}

fn deterministic_measure(value: &str, font_size: f32) -> (f32, f32) {
    let clusters = grapheme_clusters(value);
    let width_factor = if value.is_ascii() { 0.58 } else { 1.0 };
    (
        clusters.len() as f32 * font_size * width_factor,
        font_size * 1.25,
    )
}

fn measured_candidate_size(
    candidate: &CandidateFixture,
    anchors: HorizontalColumnAnchors,
    max_width: f32,
    dpi: f32,
) -> Result<Size, String> {
    check_text_clusters(&candidate.label)?;
    check_text_clusters(&candidate.text)?;
    check_text_clusters(&candidate.comment)?;
    let plan = horizontal_run_plan_with_anchors(
        &candidate.label,
        &candidate.text,
        &candidate.comment,
        max_width,
        4.0 * dpi,
        14.0 * dpi,
        18.0 * dpi,
        14.0 * dpi,
        anchors,
        deterministic_measure,
    );
    if plan
        .lines
        .iter()
        .map(|line| line.label.as_str())
        .collect::<String>()
        != candidate.label
        || plan
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<String>()
            != candidate.text
        || plan
            .lines
            .iter()
            .map(|line| line.comment.as_str())
            .collect::<String>()
            != candidate.comment
        || !plan.width.is_finite()
        || !plan.height.is_finite()
        || plan.width < 0.0
        || plan.height < 0.0
        || plan.lines.iter().any(|line| {
            (!line.label.is_empty() && line.label_x != anchors.label_x)
                || (!line.text.is_empty() && line.text_x != anchors.text_x)
                || (!line.comment.is_empty() && line.comment_x != anchors.comment_x)
        })
    {
        return Err(format!(
            "run plan lost candidate text label={:?} text={:?} comment={:?} plan={plan:?}",
            candidate.label, candidate.text, candidate.comment
        ));
    }
    Ok(Size {
        width: (plan.width + 16.0 * dpi).max(24.0 * dpi),
        height: (plan.height + 12.0 * dpi).max(24.0 * dpi),
    })
}

fn finite_rect(rect: Rect) -> bool {
    [rect.left, rect.top, rect.right, rect.bottom]
        .into_iter()
        .all(f32::is_finite)
        && rect.right >= rect.left
        && rect.bottom >= rect.top
}

fn inside(inner: Rect, outer: Rect) -> bool {
    inner.left >= outer.left
        && inner.top >= outer.top
        && inner.right <= outer.right
        && inner.bottom <= outer.bottom
}

fn overlaps(left: Rect, right: Rect) -> bool {
    left.left < right.right
        && right.left < left.right
        && left.top < right.bottom
        && right.top < left.bottom
}

fn check_text_clusters(text: &str) -> Result<(), String> {
    let clusters = grapheme_clusters(text);
    if clusters.concat() != text {
        return Err(format!("grapheme concatenation lost text: {text:?}"));
    }
    for expected in ["e\u{301}", "👍🏽", "🇨🇳", "👨‍👩‍👧‍👦", "1️⃣"] {
        if text.contains(expected) && grapheme_clusters(expected) != [expected] {
            return Err(format!("known cluster split: {expected:?}"));
        }
    }
    Ok(())
}

fn check_axis_case(
    iteration: usize,
    layout_name: &str,
    options: CandidateLayoutOptions,
    items: &[Size],
    selected: usize,
    page_size: usize,
    work_area: Rect,
    max_width: f32,
    max_height: f32,
) -> Result<(), String> {
    let result = axis_layout::layout(&axis_layout::AxisLayoutInput {
        options,
        items: items.to_vec(),
        caret: Point { x: 32.0, y: 64.0 },
        caret_height: 20.0,
        work_area,
        max_width,
        max_height,
        padding_x: 8.0,
        padding_y: 6.0,
        row_gap: 2.0,
        column_gap: 8.0,
        page_size,
        selected,
        scroll_override: None,
        placement: Placement::Unlocked,
    });
    if !finite_rect(result.window)
        || !inside(result.window, work_area)
        || !result.content_size.width.is_finite()
        || !result.content_size.height.is_finite()
    {
        return Err("window/content is not finite or exceeds work area".to_owned());
    }
    for item in &result.items {
        if !finite_rect(item.rect) {
            return Err("item rect is not finite".to_owned());
        }
    }
    if options.overflow != OverflowBehavior::Scrolling {
        if options.overflow == OverflowBehavior::Paging
            && result.items.len() > page_size
            && page_size != 0
        {
            return Err(format!(
                "page has {} items over {page_size}",
                result.items.len()
            ));
        }
        for (index, left) in result.items.iter().enumerate() {
            if !inside(left.rect, result.window) {
                return Err(format!(
                    "non-scroll item {index} escapes window item={:?} window={:?}",
                    left.rect, result.window
                ));
            }
            for right in result.items.iter().skip(index + 1) {
                if overlaps(left.rect, right.rect) {
                    return Err("non-scroll items overlap".to_owned());
                }
            }
        }
    } else if !result.items[selected].visible {
        return Err("selected scroll item is not visible".to_owned());
    }
    if matches!(
        options.writing_mode,
        WritingMode::VerticalRl | WritingMode::VerticalLr
    ) && result.items.len() > 1
    {
        let first = result.items[0].rect.left;
        let last = result.items[result.items.len() - 1].rect.left;
        let ordered = match options.writing_mode {
            WritingMode::VerticalRl => first >= last,
            WritingMode::VerticalLr => first <= last,
            WritingMode::Horizontal => true,
        };
        if !ordered {
            return Err("vertical writing column order changed".to_owned());
        }
    }
    let _ = (iteration, layout_name);
    Ok(())
}

#[test]
fn candidate_text_layout_property_fuzz_smoke() {
    assert!(valid_text(&vec![b'a'; MAX_CANDIDATE_TEXT_UTF8]));
    assert!(!valid_text(&vec![b'a'; MAX_CANDIDATE_TEXT_UTF8 + 1]));

    let mut rng = Rng::new(SEED);
    for iteration in 0..CASES {
        let count = if iteration % 127 == 0 {
            MAX_CANDIDATES
        } else {
            1 + rng.below(24)
        };
        let selected = rng.below(count);
        let page_size = 1 + rng.below(9);
        let dpi = [1.0, 1.25, 1.5, 2.0][rng.below(4)];
        let candidate_budget = rng.between(120.0, 360.0) * dpi;
        let candidate_inputs: Vec<_> = (0..count)
            .map(|index| candidate_fixture(&mut rng, iteration, index))
            .collect();
        let column_inputs: Vec<_> = candidate_inputs
            .iter()
            .map(|candidate| HorizontalColumnInput {
                label: &candidate.label,
                reserved_label: &candidate.label,
                text: &candidate.text,
                comment: &candidate.comment,
            })
            .collect();
        let anchors = horizontal_column_anchors(
            &column_inputs,
            24.0 * dpi,
            candidate_budget,
            4.0 * dpi,
            14.0 * dpi,
            18.0 * dpi,
            14.0 * dpi,
            deterministic_measure,
        );
        let mut reordered = column_inputs.clone();
        reordered.reverse();
        assert_eq!(
            anchors,
            horizontal_column_anchors(
                &reordered,
                24.0 * dpi,
                candidate_budget,
                4.0 * dpi,
                14.0 * dpi,
                18.0 * dpi,
                14.0 * dpi,
                deterministic_measure,
            ),
            "candidate order changed horizontal anchors"
        );
        if ![
            anchors.label_x,
            anchors.text_x,
            anchors.comment_x,
            anchors.label_width,
            anchors.text_width,
            anchors.comment_width,
        ]
        .into_iter()
        .all(f32::is_finite)
            || !(anchors.label_x <= anchors.text_x && anchors.text_x <= anchors.comment_x)
        {
            panic!(
                "seed=0x{SEED:016x} iteration={iteration}: invalid horizontal anchors {anchors:?}"
            );
        }
        let items: Vec<Size> = candidate_inputs
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                measured_candidate_size(candidate, anchors, candidate_budget, dpi).unwrap_or_else(
                    |error| {
                        panic!(
                            "seed=0x{SEED:016x} iteration={iteration} candidate={index} \
                         text={:?} comment={:?}: {error}",
                            candidate.text, candidate.comment
                        )
                    },
                )
            })
            .collect();
        let widest_item = items.iter().map(|item| item.width).fold(0.0_f32, f32::max);
        let tallest_item = items.iter().map(|item| item.height).fold(0.0_f32, f32::max);
        let work_area = Rect {
            left: 0.0,
            top: 0.0,
            right: (widest_item + rng.between(280.0, 980.0)).max(640.0),
            bottom: (tallest_item + rng.between(220.0, 720.0)).max(420.0),
        };
        let layout_case = iteration % 7;
        // Every generated candidate must fit individually; overflow pressure
        // comes from the number of candidates, not an impossible split-glyph
        // input. This lets the bounds assertions prove full-candidate safety.
        let minimum_width = items.iter().map(|item| item.width).fold(0.0_f32, f32::max) + 16.0;
        let minimum_height = items.iter().map(|item| item.height).fold(0.0_f32, f32::max) + 12.0;
        let max_width = rng.between(minimum_width.min(work_area.right), work_area.right);
        let max_height = rng.between(minimum_height.min(work_area.bottom), work_area.bottom);
        let layout_name = match layout_case {
            0 => "horizontal-paging",
            1 => "vertical-paging",
            2 => "horizontal-scrolling",
            3 => "vertical-scrolling",
            4 => "horizontal-wrapping",
            5 => "vertical-writing-rl",
            _ => "vertical-writing-lr",
        };

        let result = match layout_case {
            0 => check_axis_case(
                iteration,
                layout_name,
                CandidateLayoutOptions {
                    orientation: Orientation::Horizontal,
                    overflow: OverflowBehavior::Paging,
                    writing_mode: WritingMode::Horizontal,
                },
                &items,
                selected,
                page_size,
                work_area,
                max_width,
                max_height,
            ),
            1 => check_axis_case(
                iteration,
                layout_name,
                CandidateLayoutOptions {
                    orientation: Orientation::Vertical,
                    overflow: OverflowBehavior::Paging,
                    writing_mode: WritingMode::Horizontal,
                },
                &items,
                selected,
                page_size,
                work_area,
                max_width,
                max_height,
            ),
            2 => check_axis_case(
                iteration,
                layout_name,
                CandidateLayoutOptions {
                    orientation: Orientation::Horizontal,
                    overflow: OverflowBehavior::Scrolling,
                    writing_mode: WritingMode::Horizontal,
                },
                &items,
                selected,
                page_size,
                work_area,
                max_width,
                max_height,
            ),
            3 => check_axis_case(
                iteration,
                layout_name,
                CandidateLayoutOptions {
                    orientation: Orientation::Vertical,
                    overflow: OverflowBehavior::Scrolling,
                    writing_mode: WritingMode::Horizontal,
                },
                &items,
                selected,
                page_size,
                work_area,
                max_width,
                max_height,
            ),
            4 => check_axis_case(
                iteration,
                layout_name,
                CandidateLayoutOptions {
                    orientation: Orientation::Horizontal,
                    overflow: OverflowBehavior::Wrapping,
                    writing_mode: WritingMode::Horizontal,
                },
                &items,
                selected,
                page_size,
                work_area,
                max_width,
                max_height,
            ),
            5 | 6 => check_axis_case(
                iteration,
                layout_name,
                CandidateLayoutOptions {
                    orientation: Orientation::Vertical,
                    overflow: OverflowBehavior::Paging,
                    writing_mode: if layout_case == 5 {
                        WritingMode::VerticalRl
                    } else {
                        WritingMode::VerticalLr
                    },
                },
                &items,
                selected,
                page_size,
                work_area,
                max_width,
                max_height,
            ),
            _ => unreachable!(),
        };
        if let Err(error) = result {
            panic!(
                "seed=0x{SEED:016x} iteration={iteration} layout={layout_name} \
                 page_size={page_size} selected={selected}: {error}"
            );
        }
    }
}
