//! Parses the candidate host command line from its raw UTF-16 form.

#![deny(unsafe_op_in_unsafe_fn)]

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Fcitx5CandidateUtf16 {
    pub ptr: *const u16,
    pub len: usize,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Fcitx5CandidateCommandLine {
    pub status: u8,
    pub candidate_select_mode: u8,
    pub self_test: u8,
    pub interaction_self_test: u8,
    pub uiless_presentation_self_test: u8,
    pub scroll_expansion_self_test: u8,
    pub locale_self_test: u8,
    pub candidate_ux_self_test: u8,
    pub reload_test: u8,
    pub simulate_device_loss: u8,
    pub scroll_demo: u8,
    pub demo: u8,
    pub test_once: u8,
    pub safe_mode: u8,
    pub has_parent_id: u8,
    pub reserved: u8,
    pub generation_len: usize,
    pub candidate_peer_len: usize,
    pub parent_id: u32,
    pub target_process_id: u32,
    pub engine_epoch: u64,
    pub context_id: u64,
    pub composition_id: u64,
    pub revision: u64,
    pub candidate_id: u64,
}

pub(crate) fn split_windows_argument_string(input: &[u16]) -> Vec<Vec<u16>> {
    let mut arguments = Vec::new();
    let mut index = 0usize;
    while index < input.len() {
        while index < input.len() && matches!(input[index], 0x20 | 0x09) {
            index += 1;
        }
        if index >= input.len() {
            break;
        }
        let mut argument = Vec::new();
        let mut quoted = false;
        while index < input.len() {
            if !quoted && matches!(input[index], 0x20 | 0x09) {
                break;
            }
            let mut backslashes = 0usize;
            while index < input.len() && input[index] == b'\\' as u16 {
                backslashes += 1;
                index += 1;
            }
            if index < input.len() && input[index] == b'"' as u16 {
                argument.extend(std::iter::repeat_n(b'\\' as u16, backslashes / 2));
                if backslashes.is_multiple_of(2) {
                    quoted = !quoted;
                } else {
                    argument.push(b'"' as u16);
                }
                index += 1;
                continue;
            }
            argument.extend(std::iter::repeat_n(b'\\' as u16, backslashes));
            if index >= input.len() {
                break;
            }
            argument.push(input[index]);
            index += 1;
        }
        arguments.push(argument);
    }
    arguments
}

pub(crate) fn wide_ascii(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

pub(crate) fn utf16_eq_ascii(value: &[u16], ascii: &str) -> bool {
    value.len() == ascii.len()
        && value
            .iter()
            .zip(ascii.as_bytes())
            .all(|(left, right)| *left == *right as u16)
}

pub(crate) fn contains_utf16_ascii(haystack: &[u16], needle: &str) -> bool {
    let needle = wide_ascii(needle);
    !needle.is_empty()
        && haystack
            .windows(needle.len())
            .any(|window| window == needle)
}

pub(crate) fn find_utf16_ascii(haystack: &[u16], needle: &str) -> Option<usize> {
    let needle = wide_ascii(needle);
    (!needle.is_empty())
        .then(|| {
            haystack
                .windows(needle.len())
                .position(|window| window == needle)
        })
        .flatten()
}

pub(crate) fn parse_u64_utf16(value: &[u16]) -> Option<u64> {
    if value.is_empty() {
        return None;
    }
    let mut parsed = 0_u64;
    for character in value {
        let digit = character.checked_sub(b'0' as u16)?;
        if digit > 9 {
            return None;
        }
        parsed = parsed.checked_mul(10)?.checked_add(u64::from(digit))?;
    }
    Some(parsed)
}

pub(crate) fn parse_parent_id(arguments: &[u16]) -> Option<u32> {
    let marker = wide_ascii("--parent-pid ");
    let begin = find_utf16_ascii(arguments, "--parent-pid ")? + marker.len();
    let mut parsed = 0_u64;
    let mut saw_digit = false;
    for character in &arguments[begin..] {
        let Some(digit) = character.checked_sub(b'0' as u16) else {
            break;
        };
        if digit > 9 {
            break;
        }
        saw_digit = true;
        parsed = parsed.saturating_mul(10).saturating_add(u64::from(digit));
    }
    (saw_digit && parsed != 0).then_some(parsed.min(u64::from(u32::MAX)) as u32)
}

pub(crate) fn write_wide_units(value: &[u16], out: *mut u16, capacity: usize) -> usize {
    if !out.is_null() && capacity != 0 {
        let count = value.len().min(capacity);
        if count != 0 {
            // SAFETY: `out` is non-null and the caller provides `capacity` writable u16 units.
            unsafe {
                std::ptr::copy_nonoverlapping(value.as_ptr(), out, count);
            }
        }
    }
    value.len()
}

pub fn parse_candidate_command_line(
    arguments: &[u16],
) -> (Fcitx5CandidateCommandLine, Vec<u16>, Vec<u16>) {
    let tokens = split_windows_argument_string(arguments);
    let mut generation = Vec::new();
    for pair in tokens.windows(2) {
        if utf16_eq_ascii(&pair[0], "--generation") {
            generation = pair[1].clone();
        }
    }

    let interaction_self_test = contains_utf16_ascii(arguments, "--interaction-self-test");
    let scroll_expansion_self_test =
        contains_utf16_ascii(arguments, "--scroll-expansion-self-test");
    let locale_self_test = contains_utf16_ascii(arguments, "--locale-self-test");
    let candidate_ux_self_test = contains_utf16_ascii(arguments, "--candidate-ux-self-test");
    let scroll_demo = contains_utf16_ascii(arguments, "--scroll-demo");
    let demo = interaction_self_test
        || scroll_expansion_self_test
        || locale_self_test
        || candidate_ux_self_test
        || scroll_demo
        || contains_utf16_ascii(arguments, "--demo");
    let parent_id = parse_parent_id(arguments);

    let mut parsed = Fcitx5CandidateCommandLine {
        status: 1,
        self_test: contains_utf16_ascii(arguments, "--self-test") as u8,
        interaction_self_test: interaction_self_test as u8,
        uiless_presentation_self_test: contains_utf16_ascii(
            arguments,
            "--uiless-presentation-self-test",
        ) as u8,
        scroll_expansion_self_test: scroll_expansion_self_test as u8,
        locale_self_test: locale_self_test as u8,
        candidate_ux_self_test: candidate_ux_self_test as u8,
        reload_test: contains_utf16_ascii(arguments, "--reload-test") as u8,
        simulate_device_loss: contains_utf16_ascii(arguments, "--simulate-device-loss") as u8,
        scroll_demo: scroll_demo as u8,
        demo: demo as u8,
        test_once: contains_utf16_ascii(arguments, "--test-once") as u8,
        safe_mode: contains_utf16_ascii(arguments, "--safe-mode") as u8,
        has_parent_id: parent_id.is_some() as u8,
        parent_id: parent_id.unwrap_or(0),
        generation_len: generation.len(),
        ..Default::default()
    };

    let mut candidate_peer = Vec::new();
    if tokens
        .first()
        .is_some_and(|token| utf16_eq_ascii(token, "--candidate-select-test"))
    {
        if tokens.len() != 8 {
            parsed.candidate_select_mode = 64;
        } else if let (
            Some(target_process_id),
            Some(engine_epoch),
            Some(context_id),
            Some(composition_id),
            Some(revision),
            Some(candidate_id),
        ) = (
            parse_u64_utf16(&tokens[2]),
            parse_u64_utf16(&tokens[3]),
            parse_u64_utf16(&tokens[4]),
            parse_u64_utf16(&tokens[5]),
            parse_u64_utf16(&tokens[6]),
            parse_u64_utf16(&tokens[7]),
        ) {
            if target_process_id <= u64::from(u32::MAX) {
                parsed.candidate_select_mode = 1;
                parsed.target_process_id = target_process_id as u32;
                parsed.engine_epoch = engine_epoch;
                parsed.context_id = context_id;
                parsed.composition_id = composition_id;
                parsed.revision = revision;
                parsed.candidate_id = candidate_id;
                candidate_peer = tokens[1].clone();
                parsed.candidate_peer_len = candidate_peer.len();
            } else {
                parsed.candidate_select_mode = 65;
            }
        } else {
            parsed.candidate_select_mode = 65;
        }
    }

    (parsed, generation, candidate_peer)
}

#[no_mangle]
/// # Safety
///
/// `arguments` must be null only when `arguments_len` is zero, or point to a
/// readable UTF-16 buffer with exactly the provided length. Output buffers may
/// be null for size queries or point to writable UTF-16 storage for their
/// capacities. No pointer is retained.
pub unsafe extern "C" fn fcitx5_candidate_parse_command_line_utf16(
    arguments: *const u16,
    arguments_len: usize,
    generation_out: *mut u16,
    generation_capacity: usize,
    candidate_peer_out: *mut u16,
    candidate_peer_capacity: usize,
) -> Fcitx5CandidateCommandLine {
    if arguments.is_null() && arguments_len != 0 {
        return Fcitx5CandidateCommandLine::default();
    }
    let arguments = if arguments.is_null() {
        &[]
    } else {
        // SAFETY: the ABI contract keeps `arguments_len` UTF-16 units readable for this call.
        unsafe { std::slice::from_raw_parts(arguments, arguments_len) }
    };
    let (mut parsed, generation, candidate_peer) = parse_candidate_command_line(arguments);
    parsed.generation_len = write_wide_units(&generation, generation_out, generation_capacity);
    parsed.candidate_peer_len =
        write_wide_units(&candidate_peer, candidate_peer_out, candidate_peer_capacity);
    parsed
}
