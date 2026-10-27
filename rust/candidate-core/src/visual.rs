//! Builds per-candidate visual text (label, text, comment) into a reusable arena.

#![forbid(unsafe_code)]

use super::*;

/// Rust-owned visual construction for one candidate (081D model slice).
///
/// Mirrors the frozen C++ `update()` loop body: the label is formatted when
/// labels are visible and non-empty, the reserved label starts as the label,
/// and a non-empty comment gains the fixed two-space prefix. Scroll-label
/// reservations (081D slice 3) apply only to generated labels: action 1
/// reserves the configured slot label, action 2 also shows it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fcitx5CandidateVisualBuildInput<'a> {
    pub label: &'a [u8],
    pub text: &'a [u8],
    pub comment: &'a [u8],
    pub label_style: u32,
    pub labels_visible: u8,
    pub reservation_action: u8,
    pub reservation_slot: u32,
}

/// Build-wide configuration: the configured label sequence (1-based slots).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fcitx5CandidateVisualBuildConfig<'a> {
    pub configured_labels: &'a [String],
}

/// Resolved visual fields.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Fcitx5CandidateVisualBuildOutput {
    pub label: String,
    pub reserved_label: String,
    pub text: String,
    pub comment: String,
    /// 1 when the candidate carries its own formatted label.
    pub source_label: u8,
}

/// Visual records built for the current composition.
#[derive(Default)]
pub struct CandidateVisualArena {
    outputs: Vec<Fcitx5CandidateVisualBuildOutput>,
}

impl CandidateVisualArena {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Built outputs from the last [`Self::build`] call.
    pub fn built_outputs(&self) -> &[Fcitx5CandidateVisualBuildOutput] {
        &self.outputs
    }

    pub(crate) fn build(
        &mut self,
        inputs: &[Fcitx5CandidateVisualBuildInput<'_>],
        config: &Fcitx5CandidateVisualBuildConfig<'_>,
    ) -> usize {
        self.outputs.clear();
        let configured_labels = config.configured_labels;
        for input in inputs {
            let label_text = String::from_utf8_lossy(input.label).into_owned();
            let comment_text = String::from_utf8_lossy(input.comment).into_owned();
            let Some(style) = label_style_from_ffi(input.label_style) else {
                return self.outputs.len();
            };
            let formatted = if input.labels_visible != 0 && !label_text.is_empty() {
                format_candidate_label(0, &label_text, style, "", "")
            } else {
                String::new()
            };
            let source_label = u8::from(!formatted.is_empty());
            // Generated scroll-label reservations (081D slice 3): mirror the
            // frozen applyScrollLabelReservations semantics.
            let (reserved, show) = if source_label != 0 {
                (formatted.clone(), false)
            } else if matches!(input.reservation_action, 1 | 2) {
                let slot_index = input.reservation_slot.max(1) as usize - 1;
                let slot_source = configured_labels
                    .get(slot_index)
                    .cloned()
                    .unwrap_or_default();
                (
                    format_candidate_label(input.reservation_slot, &slot_source, style, "", ""),
                    input.reservation_action == 2,
                )
            } else {
                (String::new(), false)
            };
            let label = if source_label != 0 {
                formatted.clone()
            } else if show {
                reserved.clone()
            } else {
                String::new()
            };
            let comment = if comment_text.is_empty() {
                String::new()
            } else {
                format!("  {comment_text}")
            };
            let output = Fcitx5CandidateVisualBuildOutput {
                label,
                reserved_label: reserved,
                text: String::from_utf8_lossy(input.text).into_owned(),
                comment,
                source_label,
            };
            self.outputs.push(output);
        }
        self.outputs.len()
    }
}

#[cfg(test)]
mod candidate_visual_arena_tests {
    use super::*;

    fn input<'a>(
        label: &'a str,
        text: &'a str,
        comment: &'a str,
        visible: u8,
    ) -> Fcitx5CandidateVisualBuildInput<'a> {
        Fcitx5CandidateVisualBuildInput {
            label: label.as_bytes(),
            text: text.as_bytes(),
            comment: comment.as_bytes(),
            label_style: 1,
            labels_visible: visible,
            reservation_action: 0,
            reservation_slot: 0,
        }
    }

    fn empty_config() -> Fcitx5CandidateVisualBuildConfig<'static> {
        Fcitx5CandidateVisualBuildConfig {
            configured_labels: &[],
        }
    }

    fn read(value: &str) -> String {
        value.to_owned()
    }

    #[test]
    fn build_formats_labels_and_prefixes_comments() {
        let mut arena = CandidateVisualArena::default();
        let inputs = [
            input("1", "你", "nǐ", 1),
            input("", "好", "", 1),
            input("2", "汉", "hàn", 0),
        ];
        let config = empty_config();
        let built = arena.build(&inputs[..3], &config);
        let outputs = arena.built_outputs().to_vec();
        assert_eq!(built, 3);
        assert_eq!(read(&outputs[0].label), "1.");
        assert_eq!(read(&outputs[0].reserved_label), "1.");
        assert_eq!(outputs[0].source_label, 1);
        assert_eq!(read(&outputs[0].comment), "  nǐ");
        assert_eq!(read(&outputs[1].label), "");
        assert_eq!(outputs[1].source_label, 0);
        assert_eq!(read(&outputs[2].label), "", "hidden labels stay empty");
        assert_eq!(outputs[2].source_label, 0);
        assert_eq!(read(&outputs[2].comment), "  hàn");
    }

    #[test]
    fn build_keeps_text_pointers_valid_across_the_full_candidate_batch() {
        let mut arena = CandidateVisualArena::default();
        let texts: Vec<String> = (0..32).map(|index| format!("候选{index}🙂")).collect();
        let labels: Vec<String> = (1..=32).map(|index| index.to_string()).collect();
        let inputs: Vec<_> = texts
            .iter()
            .enumerate()
            .map(|(index, text)| input(&labels[index], text, "注释", 1))
            .collect();
        let config = empty_config();
        let built = arena.build(&inputs[..inputs.len()], &config);
        let outputs = arena.built_outputs().to_vec();
        assert_eq!(built, inputs.len());
        for (output, expected) in outputs.iter().zip(texts) {
            assert_eq!(read(&output.text), expected);
            assert_eq!(read(&output.comment), "  注释");
        }
    }

    #[test]
    fn build_rejects_invalid_style() {
        let mut arena = CandidateVisualArena::default();
        let mut bad = input("1", "你", "", 1);
        bad.label_style = 99;
        let config = empty_config();
        let built = arena.build(std::slice::from_ref(&bad), &config);
        assert_eq!(built, 0, "invalid style stops the build at the offender");
    }

    #[test]
    fn build_applies_scroll_label_reservations_to_generated_labels() {
        let mut arena = CandidateVisualArena::default();
        let configured = ["甲".to_owned(), "乙".to_owned()];
        let config = Fcitx5CandidateVisualBuildConfig {
            configured_labels: &configured,
        };
        let mut inputs = [
            input("1", "你", "", 1),
            input("", "好", "", 1),
            input("", "汉", "", 1),
        ];
        inputs[1].reservation_action = 2;
        inputs[1].reservation_slot = 1; // configured 甲
        inputs[2].reservation_action = 1;
        inputs[2].reservation_slot = 5; // out of range → slot number 5
        let built = arena.build(&inputs[..3], &config);
        let outputs = arena.built_outputs().to_vec();
        assert_eq!(built, 3);
        // Source labels keep their own formatted label.
        assert_eq!(read(&outputs[0].label), "1.");
        assert_eq!(read(&outputs[0].reserved_label), "1.");
        // Reserved + shown uses the configured slot label.
        assert_eq!(read(&outputs[1].label), "甲.");
        assert_eq!(read(&outputs[1].reserved_label), "甲.");
        // Reserve-only leaves the shown label empty.
        assert_eq!(read(&outputs[2].label), "");
        assert_eq!(read(&outputs[2].reserved_label), "5.");
    }
}
