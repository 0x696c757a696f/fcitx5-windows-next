//! Builds per-candidate visual text (label, text, comment) into a reusable arena.

#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

/// Rust-owned visual construction for one candidate (081D model slice).
///
/// Mirrors the frozen C++ `update()` loop body: the label is formatted when
/// labels are visible and non-empty, the reserved label starts as the label,
/// and a non-empty comment gains the fixed two-space prefix. Scroll-label
/// reservations (081D slice 3) apply only to generated labels: action 1
/// reserves the configured slot label, action 2 also shows it.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateVisualBuildInput {
    pub label: Fcitx5CandidateUtf8,
    pub text: Fcitx5CandidateUtf8,
    pub comment: Fcitx5CandidateUtf8,
    pub label_style: u32,
    pub labels_visible: u8,
    pub reservation_action: u8,
    pub reservation_slot: u32,
}

/// Build-wide configuration: the configured label sequence (1-based slots).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateVisualBuildConfig {
    pub configured_labels: *const Fcitx5CandidateUtf8,
    pub configured_label_count: usize,
}

/// Resolved visual fields; string storage is owned by the Rust arena.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateVisualBuildOutput {
    pub label: Fcitx5CandidateUtf8,
    pub reserved_label: Fcitx5CandidateUtf8,
    pub text: Fcitx5CandidateUtf8,
    pub comment: Fcitx5CandidateUtf8,
    /// 1 when the candidate carries its own formatted label.
    pub source_label: u8,
}

impl Fcitx5CandidateVisualBuildOutput {
    #[must_use]
    pub fn null_output() -> Self {
        Self {
            label: Fcitx5CandidateUtf8::default(),
            reserved_label: Fcitx5CandidateUtf8::default(),
            text: Fcitx5CandidateUtf8::default(),
            comment: Fcitx5CandidateUtf8::default(),
            source_label: 0,
        }
    }
}

/// Arena that owns built visual strings for the current composition.
#[derive(Default)]
pub struct CandidateVisualArena {
    strings: Vec<Vec<u8>>,
    outputs: Vec<Fcitx5CandidateVisualBuildOutput>,
}

impl CandidateVisualArena {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Built outputs from the last [`Self::build`] call; string pointers stay
    /// arena-owned and valid until the next build.
    pub fn built_outputs(&self) -> &[Fcitx5CandidateVisualBuildOutput] {
        &self.outputs
    }

    pub(crate) fn build(
        &mut self,
        inputs: &[Fcitx5CandidateVisualBuildInput],
        config: &Fcitx5CandidateVisualBuildConfig,
    ) -> usize {
        self.strings.clear();
        self.outputs.clear();
        // Each visual record owns four byte buffers. Reserve the outer vector
        // before exposing any pointers; a later Vec reallocation would leave
        // every previously returned `Fcitx5CandidateUtf8::ptr` dangling.
        self.strings.reserve(inputs.len().saturating_mul(4));
        let mut owned = |text: &[u8]| -> Fcitx5CandidateUtf8 {
            self.strings.push(text.to_vec());
            let stored = self.strings.last().expect("pushed above");
            Fcitx5CandidateUtf8 {
                ptr: stored.as_ptr(),
                len: stored.len(),
            }
        };
        // SAFETY: the C ABI contract requires configured_labels to cover
        // configured_label_count elements when non-zero.
        let configured_labels: &[Fcitx5CandidateUtf8] = if config.configured_label_count == 0 {
            &[]
        } else {
            // SAFETY: the C ABI contract requires configured_labels to cover this non-zero count.
            unsafe {
                std::slice::from_raw_parts(config.configured_labels, config.configured_label_count)
            }
        };
        for input in inputs {
            let label_bytes = utf8_slice(input.label);
            let text_bytes = utf8_slice(input.text);
            let comment_bytes = utf8_slice(input.comment);
            let label_text = String::from_utf8_lossy(label_bytes).into_owned();
            let comment_text = String::from_utf8_lossy(comment_bytes).into_owned();
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
                    .map(|value| String::from_utf8_lossy(utf8_slice(*value)).into_owned())
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
                label: owned(label.as_bytes()),
                reserved_label: owned(reserved.as_bytes()),
                text: owned(text_bytes),
                comment: owned(comment.as_bytes()),
                source_label,
            };
            self.outputs.push(output);
        }
        self.outputs.len()
    }
}

pub(crate) fn utf8_slice(value: Fcitx5CandidateUtf8) -> &'static [u8] {
    if value.len == 0 {
        return &[];
    }
    // SAFETY: the C ABI contract requires valid `len` readable bytes.
    unsafe { std::slice::from_raw_parts(value.ptr, value.len) }
}

#[no_mangle]
pub extern "C" fn fcitx5_candidate_visual_arena_create() -> *mut c_void {
    Box::into_raw(Box::<CandidateVisualArena>::default()).cast()
}

#[no_mangle]
/// # Safety
///
/// `arena` must be null or a pointer returned by
/// `fcitx5_candidate_visual_arena_create` that has not already been destroyed.
pub unsafe extern "C" fn fcitx5_candidate_visual_arena_destroy(arena: *mut c_void) {
    if !arena.is_null() {
        // SAFETY: caller guarantees this is the unique allocation from create.
        unsafe { drop(Box::from_raw(arena.cast::<CandidateVisualArena>())) };
    }
}

#[no_mangle]
/// # Safety
///
/// `arena` must be a valid arena pointer; `inputs` must be valid for
/// `input_count` elements when non-zero; `config.configured_labels` must be
/// valid for `config.configured_label_count` elements when non-zero;
/// `out_outputs` must point to writable storage for `input_count` outputs when
/// non-zero. Returned string pointers remain valid until the next build call
/// or arena destruction.
pub unsafe extern "C" fn fcitx5_candidate_visual_build(
    arena: *mut c_void,
    inputs: *const Fcitx5CandidateVisualBuildInput,
    input_count: usize,
    config: *const Fcitx5CandidateVisualBuildConfig,
    out_outputs: *mut Fcitx5CandidateVisualBuildOutput,
) -> usize {
    if arena.is_null()
        || config.is_null()
        || (input_count > 0 && (inputs.is_null() || out_outputs.is_null()))
    {
        return 0;
    }
    // SAFETY: caller guarantees the arena allocation is valid for this call.
    let arena = unsafe { &mut *arena.cast::<CandidateVisualArena>() };
    // SAFETY: the C ABI contract requires this slice to be valid.
    let inputs = if input_count == 0 {
        &[]
    } else {
        // SAFETY: non-null was checked above and the ABI provides `input_count` readable records.
        unsafe { std::slice::from_raw_parts(inputs, input_count) }
    };
    // SAFETY: non-null checked above; callers provide an initialized config.
    let config = unsafe { *config };
    let built = arena.build(inputs, &config);
    if built > 0 {
        // SAFETY: out_outputs is non-null for non-zero counts and covers all `built` outputs.
        unsafe { std::ptr::copy_nonoverlapping(arena.outputs.as_ptr(), out_outputs, built) };
    }
    built
}

#[cfg(test)]
mod candidate_visual_arena_tests {
    use super::*;

    fn input(
        label: &str,
        text: &str,
        comment: &str,
        visible: u8,
    ) -> Fcitx5CandidateVisualBuildInput {
        Fcitx5CandidateVisualBuildInput {
            label: Fcitx5CandidateUtf8 {
                ptr: label.as_ptr(),
                len: label.len(),
            },
            text: Fcitx5CandidateUtf8 {
                ptr: text.as_ptr(),
                len: text.len(),
            },
            comment: Fcitx5CandidateUtf8 {
                ptr: comment.as_ptr(),
                len: comment.len(),
            },
            label_style: 1,
            labels_visible: visible,
            reservation_action: 0,
            reservation_slot: 0,
        }
    }

    fn empty_config() -> Fcitx5CandidateVisualBuildConfig {
        Fcitx5CandidateVisualBuildConfig {
            configured_labels: core::ptr::null(),
            configured_label_count: 0,
        }
    }

    fn read(value: &Fcitx5CandidateUtf8) -> String {
        // SAFETY: arena-owned pointers from a live build are valid.
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(value.ptr, value.len) })
            .into_owned()
    }

    #[test]
    fn build_formats_labels_and_prefixes_comments() {
        let arena = CandidateVisualArena::default();
        let inputs = [
            input("1", "你", "nǐ", 1),
            input("", "好", "", 1),
            input("2", "汉", "hàn", 0),
        ];
        let mut outputs = [Fcitx5CandidateVisualBuildOutput::null_output(); 3];
        let config = empty_config();
        // SAFETY: this test supplies live arena/input/config/output storage for the declared count.
        let built = unsafe {
            fcitx5_candidate_visual_build(
                &arena as *const _ as *mut c_void,
                inputs.as_ptr(),
                3,
                &config,
                outputs.as_mut_ptr(),
            )
        };
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
        let arena = CandidateVisualArena::default();
        let texts: Vec<String> = (0..32).map(|index| format!("候选{index}🙂")).collect();
        let labels: Vec<String> = (1..=32).map(|index| index.to_string()).collect();
        let inputs: Vec<_> = texts
            .iter()
            .enumerate()
            .map(|(index, text)| input(&labels[index], text, "注释", 1))
            .collect();
        let mut outputs = vec![Fcitx5CandidateVisualBuildOutput::null_output(); inputs.len()];
        let config = empty_config();
        // SAFETY: this test supplies live arena/input/config/output storage for the declared count.
        let built = unsafe {
            fcitx5_candidate_visual_build(
                &arena as *const _ as *mut c_void,
                inputs.as_ptr(),
                inputs.len(),
                &config,
                outputs.as_mut_ptr(),
            )
        };
        assert_eq!(built, inputs.len());
        for (output, expected) in outputs.iter().zip(texts) {
            assert_eq!(read(&output.text), expected);
            assert_eq!(read(&output.comment), "  注释");
        }
    }

    #[test]
    fn build_rejects_invalid_style_and_null_arena() {
        let arena = CandidateVisualArena::default();
        let mut bad = input("1", "你", "", 1);
        bad.label_style = 99;
        let mut outputs = [Fcitx5CandidateVisualBuildOutput::null_output(); 1];
        let config = empty_config();
        // SAFETY: this test supplies live arena/input/config/output storage for the declared count.
        let built = unsafe {
            fcitx5_candidate_visual_build(
                &arena as *const _ as *mut c_void,
                &bad,
                1,
                &config,
                outputs.as_mut_ptr(),
            )
        };
        assert_eq!(built, 0, "invalid style stops the build at the offender");
        assert_eq!(
            // SAFETY: null arena is intentional while all other test buffers remain live.
            unsafe {
                fcitx5_candidate_visual_build(
                    core::ptr::null_mut(),
                    &bad,
                    1,
                    &config,
                    outputs.as_mut_ptr(),
                )
            },
            0
        );
    }

    #[test]
    fn build_applies_scroll_label_reservations_to_generated_labels() {
        let arena = CandidateVisualArena::default();
        let configured = ["甲", "乙"];
        let configured_refs = [
            Fcitx5CandidateUtf8 {
                ptr: configured[0].as_ptr(),
                len: configured[0].len(),
            },
            Fcitx5CandidateUtf8 {
                ptr: configured[1].as_ptr(),
                len: configured[1].len(),
            },
        ];
        let config = Fcitx5CandidateVisualBuildConfig {
            configured_labels: configured_refs.as_ptr(),
            configured_label_count: configured_refs.len(),
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
        let mut outputs = [Fcitx5CandidateVisualBuildOutput::null_output(); 3];
        // SAFETY: this test supplies live arena/input/config/output storage for the declared count.
        let built = unsafe {
            fcitx5_candidate_visual_build(
                &arena as *const _ as *mut c_void,
                inputs.as_ptr(),
                3,
                &config,
                outputs.as_mut_ptr(),
            )
        };
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
