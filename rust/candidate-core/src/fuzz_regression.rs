#![forbid(unsafe_code)]

use super::{valid_text, MAX_CANDIDATES, MAX_CANDIDATE_TEXT_UTF8};
use crate::axis_layout::{self, CandidateLayoutOptions, OverflowBehavior, WritingMode};
use crate::qingfeng::{
    qingfeng_candidate_visual_plan, QingfengCandidateVisualInput, QingfengOrientation,
    QingfengThemeMode,
};
use crate::renderer::grapheme_clusters;
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

const TEXT_CORPUS: &[&str] = &[
    "hello",
    "你好世界",
    "中文Windows Next",
    "e\u{301}",
    "😀",
    "👍🏽",
    "🇨🇳",
    "👨‍👩‍👧‍👦",
    "1️⃣",
    "候选·注释·Rime",
];

fn corpus_value(rng: &mut Rng, long: bool) -> String {
    let base = TEXT_CORPUS[rng.below(TEXT_CORPUS.len())];
    let repeats = if long {
        8 + rng.below(16)
    } else {
        1 + rng.below(3)
    };
    base.repeat(repeats)
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

fn check_qingfeng_case(
    orientation: QingfengOrientation,
    inputs: &[QingfengCandidateVisualInput],
    dpi_scale: f32,
) -> Result<(), String> {
    let plan = qingfeng_candidate_visual_plan(
        orientation,
        QingfengThemeMode::Light,
        inputs,
        24.0 * dpi_scale,
        dpi_scale,
    );
    if !finite_rect(Rect {
        left: plan.window.left,
        top: plan.window.top,
        right: plan.window.right,
        bottom: plan.window.bottom,
    }) || plan.items.len() != inputs.len()
    {
        return Err("Qingfeng plan has invalid window or item count".to_owned());
    }
    for item in &plan.items {
        let item_rect = Rect {
            left: item.item_rect.left,
            top: item.item_rect.top,
            right: item.item_rect.right,
            bottom: item.item_rect.bottom,
        };
        let text_rect = Rect {
            left: item.text_rect.left,
            top: item.text_rect.top,
            right: item.text_rect.right,
            bottom: item.text_rect.bottom,
        };
        let window = Rect {
            left: plan.window.left,
            top: plan.window.top,
            right: plan.window.right,
            bottom: plan.window.bottom,
        };
        if !finite_rect(item_rect) || !inside(item_rect, window) || !inside(text_rect, item_rect) {
            return Err("Qingfeng item/text rect escapes window".to_owned());
        }
        if let Some(comment) = item.comment_rect {
            let comment_rect = Rect {
                left: comment.left,
                top: comment.top,
                right: comment.right,
                bottom: comment.bottom,
            };
            if !inside(comment_rect, item_rect) {
                return Err("Qingfeng comment rect escapes item".to_owned());
            }
        }
    }
    for (index, left) in plan.items.iter().enumerate() {
        for right in plan.items.iter().skip(index + 1) {
            if overlaps(
                Rect {
                    left: left.item_rect.left,
                    top: left.item_rect.top,
                    right: left.item_rect.right,
                    bottom: left.item_rect.bottom,
                },
                Rect {
                    left: right.item_rect.left,
                    top: right.item_rect.top,
                    right: right.item_rect.right,
                    bottom: right.item_rect.bottom,
                },
            ) {
                return Err("Qingfeng items overlap".to_owned());
            }
        }
    }
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
        let items: Vec<Size> = (0..count)
            .map(|_| Size {
                width: rng.between(22.0, 92.0) * dpi,
                height: rng.between(22.0, 46.0) * dpi,
            })
            .collect();
        let work_area = Rect {
            left: 0.0,
            top: 0.0,
            right: rng.between(280.0, 980.0),
            bottom: rng.between(220.0, 720.0),
        };
        let layout_case = iteration % 10;
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
            6 => "vertical-writing-lr",
            7 => "qingfeng-horizontal",
            8 => "qingfeng-vertical",
            _ => "qingfeng-grid",
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
            7..=9 => {
                let inputs: Vec<_> = (0..count.min(18))
                    .map(|index| QingfengCandidateVisualInput {
                        label: format!("{}.", index + 1),
                        text: corpus_value(&mut rng, index % 7 == 0),
                        comment: if index % 3 == 0 {
                            corpus_value(&mut rng, false)
                        } else {
                            String::new()
                        },
                        selected: index == selected.min(count.min(18) - 1),
                        show_label: true,
                        reserve_label: true,
                    })
                    .collect();
                for input in &inputs {
                    check_text_clusters(&input.text).unwrap_or_else(|error| {
                        panic!(
                            "seed=0x{SEED:016x} iteration={iteration} layout={layout_name} \
                             page_size={page_size} selected={selected}: {error}"
                        )
                    });
                    check_text_clusters(&input.comment).unwrap_or_else(|error| {
                        panic!(
                            "seed=0x{SEED:016x} iteration={iteration} layout={layout_name} \
                             page_size={page_size} selected={selected}: {error}"
                        )
                    });
                }
                check_qingfeng_case(
                    match layout_case {
                        7 => QingfengOrientation::Horizontal,
                        8 => QingfengOrientation::Vertical,
                        _ => QingfengOrientation::Grid,
                    },
                    &inputs,
                    dpi,
                )
            }
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
