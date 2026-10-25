#![deny(unsafe_op_in_unsafe_fn)]

use super::*;

#[test]
fn immutable_ui_plan_keeps_cjk_annotation_and_selected_uia_semantics() {
    let mut state = CandidateUiState::default();
    assert_eq!(
        state.apply(CandidateUiInput {
            snapshot: CandidateSemanticSnapshot {
                identity: CandidateSnapshotIdentity {
                    engine_epoch: 1,
                    context_id: 7,
                    composition_id: 9,
                    revision: 1,
                },
                preedit: "ni".to_owned(),
                auxiliary_up: String::new(),
                auxiliary_down: String::new(),
                candidates: vec![
                    CandidateSemanticItem {
                        id: 1,
                        label: "1".to_owned(),
                        text: "你".to_owned(),
                        comment: "nǐ".to_owned(),
                    },
                    CandidateSemanticItem {
                        id: 2,
                        label: "2".to_owned(),
                        text: "呢".to_owned(),
                        comment: "ne".to_owned(),
                    },
                ],
                selected: Some(0),
                page: 0,
                total: 2,
                visibility: 1,
                popup_allowed: true,
            },
            locale: "zh-CN".to_owned(),
            caret: Point { x: 40.0, y: 60.0 },
            caret_height: 24.0,
            work_area: Rect {
                left: 0.0,
                top: 0.0,
                right: 800.0,
                bottom: 600.0,
            },
            candidate_bulk: false,
            config: CandidateUiConfig::default(),
        }),
        CandidateUiApplyResult::Applied
    );

    let plan = state.render_plan(&[
        CandidateUiMeasurement::new(18.0, 20.0, 24.0, 26.0),
        CandidateUiMeasurement::new(18.0, 20.0, 16.0, 26.0),
    ]);

    assert!(plan.popup_visible);
    assert_eq!(plan.orientation, Orientation::Horizontal);
    assert_eq!(plan.items.len(), 2);
    assert!(plan.items[0].text_rect.right - plan.items[0].text_rect.left >= 22.0);
    assert_eq!(plan.uia.items[0].name, "1. 你 nǐ");
    assert!(plan.uia.items[0].selected);
    assert!(!plan.uia.items[1].selected);
}

#[test]
fn immutable_ui_plan_keeps_selected_candidate_in_both_six_cell_scroll_axes() {
    let candidates = (0..42)
        .map(|index| CandidateSemanticItem {
            id: (index + 1) as u64,
            label: ((index % 6) + 1).to_string(),
            text: format!("候选{}", index + 1),
            comment: String::new(),
        })
        .collect::<Vec<_>>();
    let input = |orientation| CandidateUiInput {
        snapshot: CandidateSemanticSnapshot {
            identity: CandidateSnapshotIdentity {
                engine_epoch: 1,
                context_id: 8,
                composition_id: 10,
                revision: 1,
            },
            preedit: String::new(),
            auxiliary_up: String::new(),
            auxiliary_down: String::new(),
            candidates: candidates.clone(),
            selected: Some(30),
            page: 5,
            total: 42,
            visibility: 1,
            popup_allowed: true,
        },
        locale: "zh-CN".to_owned(),
        caret: Point { x: 40.0, y: 60.0 },
        caret_height: 24.0,
        work_area: Rect {
            left: 0.0,
            top: 0.0,
            right: 1600.0,
            bottom: 1200.0,
        },
        candidate_bulk: true,
        config: CandidateUiConfig {
            orientation,
            scroll_mode: true,
            page_size: 6,
            ..CandidateUiConfig::default()
        },
    };
    let measurements = vec![CandidateUiMeasurement::new(18.0, 40.0, 0.0, 26.0); 42];

    for orientation in [
        PresentationOrientation::Vertical,
        PresentationOrientation::Horizontal,
    ] {
        let mut state = CandidateUiState::default();
        assert_eq!(
            state.apply(input(orientation)),
            CandidateUiApplyResult::Applied
        );
        let plan = state.render_plan(&measurements);
        assert_eq!(plan.items.len(), 36);
        assert!(plan
            .items
            .iter()
            .any(|item| item.candidate_index == 30 && item.selected));
        assert!(plan
            .items
            .iter()
            .all(|item| item.text_rect.right > item.text_rect.left));
    }
}

fn snapshot(revision: u64) -> CandidateSnapshot {
    snapshot_with_identity(10, 20, 30, revision)
}

fn snapshot_with_identity(
    engine_epoch: u64,
    context_id: u64,
    composition_id: u64,
    revision: u64,
) -> CandidateSnapshot {
    CandidateSnapshot {
        engine_epoch,
        context_id,
        composition_id,
        revision,
        preedit: b"ni".to_vec(),
        auxiliary_up: Vec::new(),
        auxiliary_down: Vec::new(),
        candidates: vec![
            CandidateItem {
                id: 1,
                label: b"1".to_vec(),
                text: "你".as_bytes().to_vec(),
                comment: "nǐ".as_bytes().to_vec(),
            },
            CandidateItem {
                id: 2,
                label: b"2".to_vec(),
                text: "呢".as_bytes().to_vec(),
                comment: Vec::new(),
            },
        ],
        selected: Some(0),
        page: 0,
        total: 2,
        visibility: Visibility::Composition,
        popup_allowed: true,
    }
}

#[test]
fn semantic_snapshot_is_the_model_projection_for_all_consumers() {
    let mut model = CandidateModel::default();
    assert_eq!(model.apply(snapshot(1)), 0);
    let semantic = model.semantic_snapshot().expect("semantic snapshot");
    assert_eq!(semantic.identity.revision, 1);
    assert_eq!(semantic.selected, Some(0));
    assert_eq!(semantic.candidates[0].text, "你");
}

#[test]
fn semantic_snapshot_rejects_invalid_utf8_instead_of_replacing_text() {
    let mut model = CandidateModel::default();
    let mut invalid = snapshot(1);
    invalid.candidates[0].text = vec![0xff];
    assert_eq!(model.apply(invalid), 3);
    assert!(model.semantic_snapshot().is_none());
}

#[test]
fn ffi_current_snapshot_is_the_model_projection_for_native_adapters() {
    let mut model = CandidateModel::default();
    assert_eq!(model.apply(snapshot(1)), 0);
    let mut output = Fcitx5CandidateModelSnapshot {
        engine_epoch: 0,
        context_id: 0,
        composition_id: 0,
        revision: 0,
        preedit: Fcitx5CandidateUtf8 {
            ptr: std::ptr::null(),
            len: 0,
        },
        auxiliary_up: Fcitx5CandidateUtf8 {
            ptr: std::ptr::null(),
            len: 0,
        },
        auxiliary_down: Fcitx5CandidateUtf8 {
            ptr: std::ptr::null(),
            len: 0,
        },
        candidates: std::ptr::null(),
        candidate_count: 0,
        selected: 0,
        has_selected: 0,
        page: 0,
        total: 0,
        visibility: 0,
        popup_allowed: 0,
    };
    assert_eq!(
        // SAFETY: model and output are valid, uniquely borrowed test storage.
        unsafe {
            fcitx5_candidate_model_current((&mut model as *mut CandidateModel).cast(), &mut output)
        },
        1
    );
    assert_eq!(output.revision, 1);
    assert_eq!(output.selected, 0);
    assert_eq!(output.candidate_count, 2);
    // SAFETY: successful current call returned a non-null candidate pointer valid for model lifetime.
    let first = unsafe { &*output.candidates };
    assert_eq!(
        // SAFETY: candidate text pointer and length come from the valid model snapshot above.
        unsafe { std::slice::from_raw_parts(first.text.ptr, first.text.len) },
        "你".as_bytes()
    );
}

#[test]
fn notification_queue_drops_stale_and_cancelled_revisions() {
    let mut model = CandidateModel::default();
    assert_eq!(model.apply(snapshot(1)), 0);
    let first = model.semantic_snapshot().expect("first snapshot");
    let mut changed = snapshot(2);
    changed.selected = Some(1);
    assert_eq!(model.apply(changed), 0);
    let second = model.semantic_snapshot().expect("second snapshot");
    let mut queue = CandidateNotificationQueue::default();
    queue.enqueue(
        &first,
        CandidateCapabilities {
            narrator_nvda: true,
            ..Default::default()
        },
        CandidatePrivacyContext::Normal,
    );
    queue.enqueue(
        &second,
        CandidateCapabilities {
            narrator_nvda: true,
            ..Default::default()
        },
        CandidatePrivacyContext::Normal,
    );
    assert!(queue.drain_for(first.identity).is_empty());
    let second_notifications = queue.drain_for(second.identity);
    assert_eq!(second_notifications.len(), 1);
    assert_eq!(
        second_notifications[0].kind,
        CandidateNotificationKind::Selection
    );
    queue.cancel(second.identity);
    assert!(queue.drain_for(second.identity).is_empty());
}

#[test]
fn notification_queue_keeps_new_revision_when_context_returns_after_switch() {
    let mut model = CandidateModel::default();
    assert_eq!(model.apply(snapshot_with_identity(10, 20, 30, 1)), 0);
    let a1 = model.semantic_snapshot().expect("A1");
    assert_eq!(model.apply(snapshot_with_identity(10, 21, 40, 1)), 0);
    let b1 = model.semantic_snapshot().expect("B1");
    let mut returned = snapshot_with_identity(10, 20, 30, 2);
    returned.selected = Some(1);
    assert_eq!(model.apply(returned), 0);
    let a2 = model.semantic_snapshot().expect("A2");
    let mut queue = CandidateNotificationQueue::default();
    let caps = CandidateCapabilities::default();
    queue.enqueue(&a1, caps, CandidatePrivacyContext::Normal);
    queue.enqueue(&b1, caps, CandidatePrivacyContext::Normal);
    queue.enqueue(&a2, caps, CandidatePrivacyContext::Normal);
    assert_eq!(queue.drain_for(a1.identity).len(), 0);
    assert_eq!(queue.drain_for(b1.identity).len(), 1);
    assert_eq!(queue.drain_for(a2.identity).len(), 1);
}

#[test]
fn notification_text_is_suppressed_for_sensitive_contexts() {
    let mut model = CandidateModel::default();
    assert_eq!(model.apply(snapshot(1)), 0);
    let semantic = model.semantic_snapshot().expect("semantic snapshot");
    let mut queue = CandidateNotificationQueue::default();
    queue.enqueue(
        &semantic,
        CandidateCapabilities {
            narrator_nvda: true,
            ..Default::default()
        },
        CandidatePrivacyContext::Password,
    );
    let notification = queue
        .drain_for(semantic.identity)
        .pop()
        .expect("notification");
    assert!(notification.text.is_none());
    assert!(CandidatePrivacyContext::Pin.suppress_text());
    assert!(CandidatePrivacyContext::Sensitive.suppress_text());
    assert_eq!(
        CandidatePrivacyContext::Password.policy(),
        CandidatePrivacyPolicy {
            allow_speech: false,
            allow_text_logging: false,
            allow_learning: false,
            allow_network: false,
        }
    );
    let policy = CandidatePrivacyContext::Sensitive.policy();
    assert!(!policy.allows_speech());
    assert!(!policy.allows_text_logging());
    assert!(!policy.allows_learning());
    assert!(!policy.allows_network());
}

#[test]
fn notification_queue_emits_only_changed_selection_count_and_state() {
    let mut model = CandidateModel::default();
    assert_eq!(model.apply(snapshot(1)), 0);
    let first = model.semantic_snapshot().expect("first snapshot");
    let mut queue = CandidateNotificationQueue::default();
    let capabilities = CandidateCapabilities {
        narrator_nvda: true,
        ..Default::default()
    };
    queue.enqueue(&first, capabilities, CandidatePrivacyContext::Normal);
    assert_eq!(
        queue.drain_for(first.identity)[0].kind,
        CandidateNotificationKind::Snapshot
    );

    let mut changed = first.clone();
    changed.identity.revision = 2;
    changed.selected = Some(1);
    changed.total += 1;
    changed.visibility = 2;
    queue.enqueue(&changed, capabilities, CandidatePrivacyContext::Normal);
    let notifications = queue.drain_for(changed.identity);
    assert_eq!(notifications.len(), 3);
    assert_eq!(notifications[0].kind, CandidateNotificationKind::Selection);
    assert_eq!(notifications[1].kind, CandidateNotificationKind::Count);
    assert_eq!(notifications[2].kind, CandidateNotificationKind::State);
    assert_eq!(notifications[0].text.as_deref(), Some("呢"));
    assert!(notifications[1].text.is_none());
    assert!(notifications[2].text.is_none());

    let mut unchanged = changed.clone();
    unchanged.identity.revision = 3;
    queue.enqueue(&unchanged, capabilities, CandidatePrivacyContext::Normal);
    assert!(queue.drain_for(unchanged.identity).is_empty());
}

#[test]
fn capability_flags_compose_without_a_disability_mode() {
    let capabilities = CandidateCapabilities {
        keyboard: true,
        uia: true,
        narrator_nvda: true,
        high_contrast: true,
        large_text: true,
        reduced_motion: true,
        reduced_candidates: true,
        stable_layout: true,
    };
    assert!(capabilities.keyboard && capabilities.uia && capabilities.narrator_nvda);
    assert!(capabilities.high_contrast && capabilities.large_text && capabilities.reduced_motion);
    assert!(capabilities.reduced_candidates && capabilities.stable_layout);
}

fn width(result: &LayoutResult) -> f32 {
    result.window.right - result.window.left
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().collect()
}

#[test]
fn candidate_ui_command_line_matches_cpp_contract() {
    let arguments =
        wide("--generation nightly --self-test --safe-mode --parent-pid 42 --reload-test");
    let (parsed, generation, peer) = parse_candidate_command_line(&arguments);
    assert_eq!(parsed.status, 1);
    assert_eq!(generation, wide("nightly"));
    assert!(peer.is_empty());
    assert_eq!(parsed.self_test, 1);
    assert_eq!(parsed.safe_mode, 1);
    assert_eq!(parsed.reload_test, 1);
    assert_eq!(parsed.has_parent_id, 1);
    assert_eq!(parsed.parent_id, 42);

    let demo = parse_candidate_command_line(&wide("--locale-self-test --scroll-demo")).0;
    assert_eq!(demo.locale_self_test, 1);
    assert_eq!(demo.scroll_demo, 1);
    assert_eq!(demo.demo, 1);

    let (select, _, peer) = parse_candidate_command_line(&wide(
        r#"--candidate-select-test "C:\Program Files\fcitx5-engine.exe" 12 20 30 40 50 60"#,
    ));
    assert_eq!(select.candidate_select_mode, 1);
    assert_eq!(peer, wide(r"C:\Program Files\fcitx5-engine.exe"));
    assert_eq!(select.target_process_id, 12);
    assert_eq!(select.engine_epoch, 20);
    assert_eq!(select.context_id, 30);
    assert_eq!(select.composition_id, 40);
    assert_eq!(select.revision, 50);
    assert_eq!(select.candidate_id, 60);

    assert_eq!(
        parse_candidate_command_line(&wide("--candidate-select-test only-peer"))
            .0
            .candidate_select_mode,
        64
    );
    assert_eq!(
        parse_candidate_command_line(&wide("--candidate-select-test peer nope 2 3 4 5 6"))
            .0
            .candidate_select_mode,
        65
    );
}

#[test]
fn default_dwrite_locale_matches_cpp_fallback_contract() {
    let locale = default_dwrite_locale();
    assert!(!locale.is_empty());
    assert!(locale.len() < LOCALE_NAME_MAX_LENGTH);
    assert!(!locale.contains(&0));
}

#[test]
fn default_dwrite_locale_abi_uses_two_phase_utf16_output() {
    // SAFETY: null output with zero capacity is the documented size-query form.
    let required = unsafe { fcitx5_candidate_default_dwrite_locale_utf16(std::ptr::null_mut(), 0) };
    assert!(required > 0);
    let mut output = vec![0_u16; required];
    // SAFETY: output owns writable storage for exactly its supplied u16 capacity.
    let written =
        unsafe { fcitx5_candidate_default_dwrite_locale_utf16(output.as_mut_ptr(), output.len()) };
    assert_eq!(written, required);
    assert!(!output.contains(&0));
}

#[test]
fn content_locale_policy_matches_cpp_contract() {
    assert!(content_locale_valid(b"zh-CN"));
    assert!(content_locale_valid(b"en-US"));
    assert!(content_locale_valid(b"ja-JP-u-ca-japanese"));
    assert!(!content_locale_valid(b""));
    assert!(!content_locale_valid(b"----"));
    assert!(!content_locale_valid(b"zh_CN"));
    assert!(!content_locale_valid(&[b'a'; MAX_CONTENT_LOCALE_UTF8 + 1]));
    assert_eq!(content_locale_or_default(b"zh-CN"), wide("zh-CN"));
    assert_eq!(
        // SAFETY: UTF-8 view points to the static byte literal for its exact length.
        unsafe {
            fcitx5_candidate_content_locale_valid_utf8(Fcitx5CandidateUtf8 {
                ptr: b"ko-KR".as_ptr(),
                len: b"ko-KR".len(),
            })
        },
        1
    );
    assert_eq!(
        // SAFETY: UTF-8 view points to the static byte literal for its exact length.
        unsafe {
            fcitx5_candidate_locale_prefers_compact_horizontal_utf8(Fcitx5CandidateUtf8 {
                ptr: b"JA-jp".as_ptr(),
                len: b"JA-jp".len(),
            })
        },
        1
    );
}

#[test]
fn content_locale_or_default_abi_uses_two_phase_utf16_output() {
    let locale = Fcitx5CandidateUtf8 {
        ptr: b"en-US".as_ptr(),
        len: b"en-US".len(),
    };
    // SAFETY: locale view references the static byte literal and null output is a size query.
    let required = unsafe {
        fcitx5_candidate_content_locale_or_default_utf16(locale, std::ptr::null_mut(), 0)
    };
    assert_eq!(required, wide("en-US").len());
    let mut output = vec![0_u16; required];
    // SAFETY: locale view is valid and output owns writable storage for its supplied capacity.
    let written = unsafe {
        fcitx5_candidate_content_locale_or_default_utf16(locale, output.as_mut_ptr(), output.len())
    };
    assert_eq!(written, required);
    assert_eq!(output, wide("en-US"));

    let invalid = Fcitx5CandidateUtf8 {
        ptr: b"zh_CN".as_ptr(),
        len: b"zh_CN".len(),
    };
    // SAFETY: invalid locale bytes remain valid readable storage for the supplied length.
    let fallback_required = unsafe {
        fcitx5_candidate_content_locale_or_default_utf16(invalid, std::ptr::null_mut(), 0)
    };
    assert!(fallback_required > 0);
}

#[test]
fn scroll_label_policy_reserves_and_shows_current_row_or_column() {
    let inactive = scroll_label_policy(2, 8, 6, 20);
    assert_eq!(inactive.reserve, 1);
    assert_eq!(inactive.show, 0);
    assert_eq!(inactive.slot, 3);

    let active = scroll_label_policy(10, 8, 6, 20);
    assert_eq!(active.reserve, 1);
    assert_eq!(active.show, 1);
    assert_eq!(active.slot, 5);

    assert_eq!(scroll_label_policy(0, 0, 6, 6).reserve, 0);
    assert_eq!(scroll_label_policy(0, 0, 0, 20).reserve, 0);
}

#[test]
fn candidate_label_formatting_supports_custom_ordinals() {
    assert_eq!(
        format_candidate_label(1, "", CandidateLabelStyle::Dot, "", ""),
        "1."
    );
    assert_eq!(
        format_candidate_label(10, "", CandidateLabelStyle::Dot, "", ""),
        "10."
    );
    assert_eq!(
        format_candidate_label(10, "", CandidateLabelStyle::Dot, "#", ":"),
        "#10:"
    );
    assert_eq!(
        format_candidate_label(3, "abc", CandidateLabelStyle::Bracket, "", ""),
        "[abc]"
    );
    assert_eq!(
        format_candidate_label(1, "1", CandidateLabelStyle::Circled, "", ""),
        "①"
    );
}

#[test]
fn candidate_label_slot_plan_keeps_hidden_labels_aligned() {
    let sources = [
        CandidateLabelSlotSource {
            candidate_index: 0,
            row: 0,
            column: 0,
            label_width: 12.0,
        },
        CandidateLabelSlotSource {
            candidate_index: 1,
            row: 1,
            column: 0,
            label_width: 28.0,
        },
        CandidateLabelSlotSource {
            candidate_index: 2,
            row: 2,
            column: 0,
            label_width: 20.0,
        },
    ];
    let plan = candidate_label_slot_plan(
        CandidateLabelSlotConfig {
            display: CandidateLabelDisplay::SelectedScope,
            scope: CandidateLabelScope::Item,
            reserve_when_hidden: true,
            align: CandidateLabelAlign::Right,
            width_strategy: CandidateLabelWidthStrategy::PageMax,
            min_width: 0.0,
            gap: 4.0,
        },
        &sources,
        1,
    );
    assert_eq!(plan.label_slot_width, 28.0);
    assert!(plan.stable_text_origin);
    assert_eq!(
        plan.items
            .iter()
            .map(|item| item.show_label)
            .collect::<Vec<_>>(),
        vec![false, true, false]
    );
    assert!(plan.items.iter().all(|item| item.reserve_label));
    assert!(plan
        .items
        .iter()
        .all(|item| (item.text_origin_offset - 32.0).abs() <= f32::EPSILON));
}

#[test]
fn candidate_label_slot_plan_reveals_selected_rows_and_columns() {
    let sources = [
        CandidateLabelSlotSource {
            candidate_index: 0,
            row: 0,
            column: 0,
            label_width: 10.0,
        },
        CandidateLabelSlotSource {
            candidate_index: 1,
            row: 0,
            column: 1,
            label_width: 10.0,
        },
        CandidateLabelSlotSource {
            candidate_index: 2,
            row: 1,
            column: 0,
            label_width: 24.0,
        },
        CandidateLabelSlotSource {
            candidate_index: 3,
            row: 1,
            column: 1,
            label_width: 24.0,
        },
    ];
    let row_plan = candidate_label_slot_plan(
        CandidateLabelSlotConfig {
            display: CandidateLabelDisplay::SelectedScope,
            scope: CandidateLabelScope::Row,
            ..CandidateLabelSlotConfig::default()
        },
        &sources,
        2,
    );
    assert_eq!(
        row_plan
            .items
            .iter()
            .map(|item| item.show_label)
            .collect::<Vec<_>>(),
        vec![false, false, true, true]
    );
    let column_plan = candidate_label_slot_plan(
        CandidateLabelSlotConfig {
            display: CandidateLabelDisplay::SelectedScope,
            scope: CandidateLabelScope::Column,
            ..CandidateLabelSlotConfig::default()
        },
        &sources,
        1,
    );
    assert_eq!(
        column_plan
            .items
            .iter()
            .map(|item| item.show_label)
            .collect::<Vec<_>>(),
        vec![false, true, false, true]
    );
    assert!(row_plan.stable_text_origin);
    assert!(column_plan.stable_text_origin);
}

#[test]
fn interaction_helpers_match_cpp_contract() {
    let rectangles = [
        Fcitx5CandidateLayoutRect {
            left: 8.0,
            top: 8.0,
            right: 120.0,
            bottom: 36.0,
        },
        Fcitx5CandidateLayoutRect {
            left: 8.0,
            top: 38.0,
            right: 120.0,
            bottom: 66.0,
        },
    ];
    let mut index = usize::MAX;
    assert_eq!(
        // SAFETY: rectangles is a live contiguous slice and index is writable test storage.
        unsafe {
            fcitx5_candidate_hit_test(
                rectangles.as_ptr(),
                rectangles.len(),
                20.0,
                50.0,
                &mut index,
            )
        },
        1
    );
    assert_eq!(index, 1);
    assert_eq!(
        // SAFETY: rectangles is a live contiguous slice and index is writable test storage.
        unsafe {
            fcitx5_candidate_hit_test(rectangles.as_ptr(), rectangles.len(), 4.0, 50.0, &mut index)
        },
        0
    );
    assert_eq!(
        // SAFETY: rectangles is a live contiguous slice and index is writable test storage.
        unsafe {
            fcitx5_candidate_hit_test(
                rectangles.as_ptr(),
                rectangles.len(),
                20.0,
                37.0,
                &mut index,
            )
        },
        0
    );

    let intent = fcitx5_candidate_selection_intent(41, 9, 10, 11, 12, 13);
    assert!(selection_intent_valid(intent));
    assert_eq!(intent.target_process_id, 41);
    assert_eq!(intent.engine_epoch, 9);
    assert_eq!(intent.context_id, 10);
    assert_eq!(intent.composition_id, 11);
    assert_eq!(intent.revision, 12);
    assert_eq!(intent.candidate_id, 13);
    assert!(!selection_intent_valid(fcitx5_candidate_selection_intent(
        0, 9, 10, 11, 12, 13
    )));
    assert!(!selection_intent_valid(fcitx5_candidate_selection_intent(
        41, 0, 10, 11, 12, 13
    )));
    assert!(!selection_intent_valid(fcitx5_candidate_selection_intent(
        41, 9, 10, 0, 12, 13
    )));
    assert!(!selection_intent_valid(fcitx5_candidate_selection_intent(
        41, 9, 10, 11, 12, 0
    )));
}

#[test]
fn render_segments_match_label_column_gap_and_comment_contract() {
    let input = [
        Fcitx5CandidateRenderItemInput {
            bounds: Fcitx5CandidateLayoutRect {
                left: 10.0,
                top: 20.0,
                right: 210.0,
                bottom: 48.0,
            },
            label_width: 18.0,
            label_gap: 4.0,
            text_width: 80.0,
            comment_width: 40.0,
            has_label: 1,
            reserve_label: 1,
        },
        Fcitx5CandidateRenderItemInput {
            bounds: Fcitx5CandidateLayoutRect {
                left: 10.0,
                top: 52.0,
                right: 110.0,
                bottom: 80.0,
            },
            label_width: 10.0,
            label_gap: 4.0,
            text_width: 96.0,
            comment_width: 20.0,
            has_label: 1,
            reserve_label: 1,
        },
    ];
    let mut output = [Fcitx5CandidateRenderItemOutput::default(); 2];
    let mut label_column = 0.0_f32;
    assert_eq!(
        // SAFETY: input and output arrays are live contiguous test storage for their supplied lengths.
        unsafe {
            fcitx5_candidate_render_segments(
                input.as_ptr(),
                input.len(),
                1,
                0,
                output.as_mut_ptr(),
                &mut label_column,
            )
        },
        0
    );
    assert_eq!(label_column, 20.0);
    assert_eq!(output[0].label.right, 30.0);
    assert_eq!(output[0].text.left, 34.0);
    assert_eq!(output[0].comment.left, 118.0);
    assert_eq!(output[0].draw_comment, 1);
    assert_eq!(output[1].label.right, 30.0);
    assert_eq!(output[1].text.left, 34.0);
    assert_eq!(output[1].comment.left, 114.0);
    assert_eq!(output[1].draw_comment, 0);

    let no_label = [Fcitx5CandidateRenderItemInput {
        bounds: input[0].bounds,
        label_width: 18.0,
        label_gap: 4.0,
        text_width: 80.0,
        comment_width: 0.0,
        has_label: 0,
        reserve_label: 0,
    }];
    let mut no_label_output = [Fcitx5CandidateRenderItemOutput::default(); 1];
    assert_eq!(
        // SAFETY: input and output arrays are live contiguous test storage for their supplied lengths.
        unsafe {
            fcitx5_candidate_render_segments(
                no_label.as_ptr(),
                no_label.len(),
                0,
                1,
                no_label_output.as_mut_ptr(),
                &mut label_column,
            )
        },
        0
    );
    assert_eq!(label_column, 0.0);
    assert_eq!(no_label_output[0].text.left, 10.0);

    let hidden_label = [Fcitx5CandidateRenderItemInput {
        bounds: input[0].bounds,
        label_width: 18.0,
        label_gap: 4.0,
        text_width: 80.0,
        comment_width: 0.0,
        has_label: 0,
        reserve_label: 1,
    }];
    let mut hidden_output = [Fcitx5CandidateRenderItemOutput::default(); 1];
    assert_eq!(
        // SAFETY: input and output arrays are live contiguous test storage for their supplied lengths.
        unsafe {
            fcitx5_candidate_render_segments(
                hidden_label.as_ptr(),
                hidden_label.len(),
                1,
                0,
                hidden_output.as_mut_ptr(),
                &mut label_column,
            )
        },
        0
    );
    assert_eq!(label_column, 20.0);
    assert_eq!(hidden_output[0].label.right, 30.0);
    assert_eq!(hidden_output[0].text.left, 34.0);
}

#[test]
fn candidate_model_matches_frozen_cpp_contract() {
    let mut model = CandidateModel::default();
    assert_eq!(model.apply(snapshot(1)), 0);
    assert_eq!(model.apply(snapshot(1)), 1);
    assert_eq!(model.apply(snapshot(0)), 3);
    assert_eq!(model.apply(snapshot(2)), 0);

    let mut uiless = snapshot(3);
    uiless.popup_allowed = false;
    uiless.selected = Some(1);
    assert_eq!(model.apply(uiless), 0);
    assert!(!model.current.as_ref().expect("current").popup_allowed);
    assert_eq!(model.current.as_ref().expect("current").candidates.len(), 2);
    assert_eq!(model.current.as_ref().expect("current").selected, Some(1));

    let mut stale = snapshot(4);
    stale.engine_epoch = 9;
    assert_eq!(model.apply(stale), 2);

    let mut prediction = snapshot(4);
    prediction.preedit.clear();
    prediction.visibility = Visibility::Prediction;
    assert_eq!(model.apply(prediction), 0);

    let mut invalid = snapshot(5);
    invalid.selected = Some(5);
    assert_eq!(model.apply(invalid), 3);

    let mut switched = snapshot(1);
    switched.context_id = 21;
    switched.composition_id = 40;
    assert_eq!(model.apply(switched), 0);
    assert_eq!(model.current.as_ref().expect("current").context_id, 21);
    assert!(model.current.as_ref().expect("current").popup_allowed);

    let mut returned = snapshot(5);
    returned.context_id = 20;
    returned.composition_id = 30;
    returned.popup_allowed = false;
    assert_eq!(model.apply(returned), 0);
    assert_eq!(model.current.as_ref().expect("current").context_id, 20);
    assert_eq!(model.current.as_ref().expect("current").revision, 5);
    assert!(!model.current.as_ref().expect("current").popup_allowed);

    let mut duplicate_inactive_a = snapshot_with_identity(10, 20, 30, 5);
    duplicate_inactive_a.popup_allowed = false;
    assert_eq!(model.apply(duplicate_inactive_a), 1);

    let mut newer_a = snapshot_with_identity(10, 20, 30, 6);
    newer_a.selected = Some(1);
    newer_a.popup_allowed = false;
    assert_eq!(model.apply(newer_a), 0);
    assert_eq!(model.current.as_ref().expect("current").context_id, 20);
    assert_eq!(model.current.as_ref().expect("current").revision, 6);
    assert_eq!(model.current.as_ref().expect("current").selected, Some(1));
    assert!(!model.current.as_ref().expect("current").popup_allowed);

    assert_eq!(model.apply(snapshot_with_identity(10, 20, 30, 5)), 2);
    assert_eq!(model.current.as_ref().expect("current").revision, 6);

    let mut smaller_b = snapshot_with_identity(10, 22, 2, 1);
    smaller_b.preedit = b"bo".to_vec();
    smaller_b.candidates = vec![CandidateItem {
        id: 3,
        label: b"1".to_vec(),
        text: b"bo".to_vec(),
        comment: Vec::new(),
    }];
    smaller_b.total = 1;
    assert_eq!(model.apply(smaller_b), 0);
    assert_eq!(model.current.as_ref().expect("current").context_id, 22);
    assert_eq!(model.current.as_ref().expect("current").composition_id, 2);
    assert_eq!(model.current.as_ref().expect("current").candidates.len(), 1);

    let mut final_a = snapshot_with_identity(10, 20, 30, 7);
    final_a.auxiliary_down = b"visible".to_vec();
    assert_eq!(model.apply(final_a), 0);
    assert_eq!(
        model.current.as_ref().expect("current").auxiliary_down,
        b"visible"
    );

    assert_eq!(model.apply(snapshot_with_identity(9, 20, 30, 6)), 2);
    assert_eq!(model.current.as_ref().expect("current").engine_epoch, 10);

    let mut new_composition = snapshot_with_identity(10, 20, 31, 1);
    new_composition.preedit = b"xin".to_vec();
    assert_eq!(model.apply(new_composition), 0);
    assert_eq!(model.current.as_ref().expect("current").composition_id, 31);
    assert_eq!(model.current.as_ref().expect("current").revision, 1);
    assert_eq!(model.apply(snapshot_with_identity(10, 20, 30, 6)), 2);
    assert_eq!(model.current.as_ref().expect("current").composition_id, 31);

    let mut preedit_only = snapshot(2);
    preedit_only.context_id = 23;
    preedit_only.candidates.clear();
    preedit_only.selected = None;
    preedit_only.total = 0;
    preedit_only.visibility = Visibility::Hidden;
    assert_eq!(model.apply(preedit_only), 0);

    model.reset();
    assert!(model.current.is_none());

    let mut reconnected = snapshot(1);
    reconnected.popup_allowed = false;
    assert_eq!(model.apply(reconnected), 0);
    assert!(!model.current.as_ref().expect("current").popup_allowed);
    assert_eq!(model.current.as_ref().expect("current").candidates.len(), 2);
}

#[test]
fn layout_matches_frozen_cpp_contract() {
    let mut input = LayoutInput {
        orientation: Orientation::Vertical,
        items: vec![
            Size {
                width: 100.0,
                height: 24.0,
            },
            Size {
                width: 140.0,
                height: 24.0,
            },
        ],
        caret: Point {
            x: 1900.0,
            y: 1060.0,
        },
        caret_height: 20.0,
        work_area: Rect {
            left: 0.0,
            top: 0.0,
            right: 1920.0,
            bottom: 1080.0,
        },
        max_width: 720.0,
        padding_x: 8.0,
        padding_y: 6.0,
        row_gap: 2.0,
        column_gap: 8.0,
        placement: Placement::Unlocked,
        ..LayoutInput::default()
    };
    let first = layout(&input);
    assert_eq!(first.placement, Placement::Above);
    assert!(first.window.right <= 1920.0);
    assert!(first.window.bottom <= 1080.0);
    assert_eq!(first.items.len(), 2);

    input.placement = first.placement;
    input.items[1].width = 300.0;
    let stable = layout(&input);
    assert_eq!(stable.placement, Placement::Above);

    input.orientation = Orientation::Horizontal;
    input.placement = Placement::Below;
    input.caret = Point { x: 10.0, y: 10.0 };
    let horizontal = layout(&input);
    assert!(horizontal.items[1].left > horizontal.items[0].right);
}

#[test]
fn flow_paged_bounds_wraps_into_rows_without_overlap() {
    let sizes = vec![
        Size {
            width: 100.0,
            height: 24.0
        };
        5
    ];
    let (rects, width, height) = flow_paged_bounds(&sizes, 250.0, 10.0, 2.0);
    // Row capacity: 100+10+100=210<=250, +10+100=320>250 => 2 per row => 2,2,1 rows.
    assert_eq!(rects.len(), 5);
    assert_eq!(width, 210.0);
    assert_eq!(height, 24.0 * 3.0 + 2.0 * 2.0);
    assert_eq!(rects[0].top, rects[1].top);
    assert!(rects[1].left > rects[0].right);
    assert!(rects[2].top >= rects[1].bottom + 2.0);
    assert!(rects[3].top == rects[2].top);
}

#[test]
fn flow_paged_bounds_scales_with_dpi_item_sizes() {
    // 200% DPI doubles every item; the wrap geometry stays non-overlapping
    // and keeps full-item bounds.
    let sizes = vec![
        Size {
            width: 200.0,
            height: 48.0
        };
        6
    ];
    let (rects, width, height) = flow_paged_bounds(&sizes, 500.0, 20.0, 4.0);
    assert_eq!(rects.len(), 6);
    assert!(width <= 500.0);
    assert!(height > 48.0);
    for window in rects.windows(2) {
        if (window[1].top - window[0].top).abs() < f32::EPSILON {
            assert!(window[1].left >= window[0].right);
        } else {
            assert!(window[1].top >= window[0].bottom);
        }
    }
}

#[test]
fn vertical_text_columns_left_to_right_keep_full_glyph_budget() {
    // 150% DPI: full CJK advance 36.0 * 1.5 = 54.0 column budget.
    let sizes = vec![
        Size {
            width: 54.0,
            height: 96.0
        };
        3
    ];
    let (rects, width, height) = vertical_text_columns(&sizes, 12.0, true);
    assert_eq!(rects.len(), 3);
    assert_eq!(width, 54.0 * 3.0 + 12.0 * 2.0);
    assert_eq!(height, 96.0);
    assert!(rects[1].left >= rects[0].right);
    assert!(rects[2].left >= rects[1].right);
    assert_eq!(rects[0].top, 0.0);
    // Each column keeps its full-glyph advance budget (no truncation).
    for rect in &rects {
        assert_eq!(rect.right - rect.left, 54.0);
    }
}

#[test]
fn vertical_text_columns_right_to_left_reverse_order() {
    let sizes = vec![
        Size {
            width: 40.0,
            height: 80.0,
        },
        Size {
            width: 40.0,
            height: 80.0,
        },
    ];
    let (rects, width, _) = vertical_text_columns(&sizes, 10.0, false);
    assert_eq!(rects.len(), 2);
    assert_eq!(width, 90.0);
    assert!(
        rects[0].left > rects[1].left,
        "right-to-left puts first candidate on the right"
    );
    assert_eq!(rects[1].left, 0.0);
    assert!(rects[0].left >= rects[1].right);
}

#[test]
fn scaled_negative_monitor_is_clamped() {
    for scale in [1.25_f32, 1.5, 2.0] {
        let input = LayoutInput {
            orientation: Orientation::Vertical,
            caret: Point {
                x: -1900.0,
                y: 900.0,
            },
            caret_height: 20.0 * scale,
            work_area: Rect {
                left: -1920.0,
                top: 0.0,
                right: 0.0,
                bottom: 1080.0,
            },
            max_width: 720.0 * scale,
            padding_x: 8.0 * scale,
            padding_y: 6.0 * scale,
            items: vec![
                Size {
                    width: 500.0 * scale,
                    height: 24.0 * scale,
                },
                Size {
                    width: 900.0 * scale,
                    height: 24.0 * scale,
                },
            ],
            ..LayoutInput::default()
        };
        let scaled = layout(&input);
        assert!(scaled.window.left >= input.work_area.left);
        assert!(scaled.window.right <= input.work_area.right);
        assert!(scaled.window.top >= input.work_area.top);
        assert!(scaled.window.bottom <= input.work_area.bottom);
    }
}

#[test]
fn horizontal_scroll_viewport_and_width_match_cpp_contract() {
    let mut scroll = LayoutInput {
        scroll_mode: true,
        scroll_columns: 6,
        scroll_visible_rows: 6,
        caret: Point { x: 100.0, y: 100.0 },
        caret_height: 24.0,
        work_area: Rect {
            left: 0.0,
            top: 0.0,
            right: 1920.0,
            bottom: 1080.0,
        },
        max_width: 860.0,
        items: vec![
            Size {
                width: 120.0,
                height: 34.0,
            };
            60
        ],
        orientation: Orientation::Horizontal,
        selected: 6,
        ..LayoutInput::default()
    };
    let same_viewport = layout(&scroll);
    assert_eq!(same_viewport.first_visible, 0);
    assert_eq!(same_viewport.item_indices.first(), Some(&0));
    assert_eq!(same_viewport.item_indices.last(), Some(&35));

    let mut horizontal_baseline = scroll.clone();
    horizontal_baseline.scroll_mode = false;
    horizontal_baseline.items.truncate(6);
    let horizontal_baseline_layout = layout(&horizontal_baseline);
    assert!((width(&same_viewport) - width(&horizontal_baseline_layout)).abs() <= 0.01);

    scroll.selected = 42;
    let next_viewport = layout(&scroll);
    assert_eq!(next_viewport.items.len(), 36);
    assert_eq!(next_viewport.item_indices.first(), Some(&24));
    assert_eq!(next_viewport.item_indices.last(), Some(&59));
    assert!(next_viewport.has_scrollbar);
    assert!(next_viewport.scrollbar_thumb.bottom <= next_viewport.scrollbar_track.bottom);
}

#[test]
fn horizontal_scroll_grid_gives_each_column_its_widest_candidate() {
    let widths = [200.0, 40.0, 40.0, 60.0, 40.0, 40.0, 50.0, 120.0, 40.0];
    let input = LayoutInput {
        scroll_mode: true,
        scroll_columns: 3,
        scroll_visible_rows: 6,
        caret: Point { x: 100.0, y: 100.0 },
        caret_height: 24.0,
        work_area: Rect {
            left: 0.0,
            top: 0.0,
            right: 1920.0,
            bottom: 1080.0,
        },
        max_width: 860.0,
        items: widths
            .iter()
            .map(|width| Size {
                width: *width,
                height: 34.0,
            })
            .collect(),
        orientation: Orientation::Horizontal,
        ..LayoutInput::default()
    };
    let grid = layout(&input);
    assert_eq!(grid.items.len(), 9);
    for (rect, natural) in grid.items.iter().zip(widths) {
        assert!(rect.right - rect.left >= natural - 0.01);
    }
    for row in grid.items.chunks(3) {
        for pair in row.windows(2) {
            assert!(pair[0].right + input.column_gap <= pair[1].left + 0.01);
        }
    }
    for column in 0..3 {
        let left = grid.items[column].left;
        assert!(grid
            .items
            .iter()
            .skip(column)
            .step_by(3)
            .all(|r| (r.left - left).abs() < 0.01));
    }
    let expected = 200.0 + 120.0 + 40.0 + input.column_gap * 2.0 + input.padding_x * 2.0;
    assert!((width(&grid) - expected).abs() <= 0.01);
}

#[test]
fn horizontal_scroll_grid_shrinks_widest_columns_first() {
    let input = LayoutInput {
        scroll_mode: true,
        scroll_columns: 3,
        scroll_visible_rows: 6,
        work_area: Rect {
            left: 0.0,
            top: 0.0,
            right: 1920.0,
            bottom: 1080.0,
        },
        max_width: 300.0,
        items: [500.0, 40.0, 60.0]
            .iter()
            .map(|width| Size {
                width: *width,
                height: 34.0,
            })
            .collect(),
        orientation: Orientation::Horizontal,
        ..LayoutInput::default()
    };
    let grid = layout(&input);
    assert!(width(&grid) <= 300.0 + 0.01);
    assert!((grid.items[1].right - grid.items[1].left - 40.0).abs() < 0.01);
    assert!((grid.items[2].right - grid.items[2].left - 60.0).abs() < 0.01);
    assert!(grid.items[2].right <= grid.window.right - input.padding_x + 0.01);
}

#[test]
fn vertical_scroll_uses_bounded_natural_columns() {
    let mut vertical_scroll = LayoutInput {
        scroll_mode: true,
        scroll_columns: 6,
        scroll_visible_rows: 6,
        caret: Point { x: 100.0, y: 100.0 },
        caret_height: 24.0,
        work_area: Rect {
            left: 0.0,
            top: 0.0,
            right: 1920.0,
            bottom: 1080.0,
        },
        max_width: 720.0,
        scroll_cell_width: 96.0,
        orientation: Orientation::Vertical,
        selected: 6,
        items: vec![
            Size {
                width: 48.0,
                height: 34.0,
            };
            60
        ],
        ..LayoutInput::default()
    };
    let first_columns = layout(&vertical_scroll);
    let fixed_cell_width = 96.0 * 6.0 + 8.0 * 2.0 + 8.0 * 5.0;
    assert_eq!(first_columns.items.len(), 36);
    assert_eq!(first_columns.item_indices.first(), Some(&0));
    assert_eq!(first_columns.item_indices.last(), Some(&35));
    assert!(width(&first_columns) < fixed_cell_width - 0.01);
    for item in &first_columns.items {
        assert!(((item.right - item.left) - 48.0).abs() <= 0.01);
    }

    vertical_scroll.items = vec![
        Size {
            width: 420.0,
            height: 34.0,
        };
        60
    ];
    let long_candidate_columns = layout(&vertical_scroll);
    assert_eq!(long_candidate_columns.items.len(), 36);
    assert!(width(&long_candidate_columns) <= vertical_scroll.max_width + 0.01);

    vertical_scroll.selected = 58;
    let final_column = layout(&vertical_scroll);
    assert_eq!(final_column.items.len(), 36);
    assert_eq!(final_column.item_indices.first(), Some(&24));
    assert_eq!(final_column.item_indices.last(), Some(&59));
}

#[test]
fn automatic_orientation_and_width_stability_are_composition_scoped() {
    let mut state = CompositionLayoutState::default();
    let identity = CompositionIdentity {
        engine_epoch: 1,
        context_id: 2,
        composition_id: 3,
    };
    state.begin(identity);
    let work_area = Rect {
        left: 0.0,
        top: 0.0,
        right: 1920.0,
        bottom: 1080.0,
    };
    let compact = vec![
        CandidateText {
            text: "你".into(),
            comment: String::new(),
        },
        CandidateText {
            text: "好".into(),
            comment: String::new(),
        },
    ];
    assert_eq!(
        state.resolve_orientation(
            PresentationOrientation::Automatic,
            AutomaticOrientationInput {
                candidates: &compact,
                locale: "zh-CN",
                work_area,
                caret_x: 100.0,
                scale: 1.0,
                page_size: 9,
            },
        ),
        Orientation::Horizontal
    );
    let annotated = vec![CandidateText {
        text: "你".into(),
        comment: "annotation-longer-than-limit".into(),
    }];
    assert_eq!(
        state.resolve_orientation(
            PresentationOrientation::Automatic,
            AutomaticOrientationInput {
                candidates: &annotated,
                locale: "zh-CN",
                work_area,
                caret_x: 100.0,
                scale: 1.0,
                page_size: 9,
            },
        ),
        Orientation::Horizontal,
        "orientation remains stable inside one composition"
    );
    assert_eq!(
        state.resolve_orientation(
            PresentationOrientation::Horizontal,
            AutomaticOrientationInput {
                candidates: &annotated,
                locale: "en-US",
                work_area,
                caret_x: 1910.0,
                scale: 1.0,
                page_size: 9,
            },
        ),
        Orientation::Horizontal
    );
    assert_eq!(
        state.resolve_orientation(
            PresentationOrientation::Vertical,
            AutomaticOrientationInput {
                candidates: &compact,
                locale: "zh-CN",
                work_area,
                caret_x: 100.0,
                scale: 1.0,
                page_size: 9,
            },
        ),
        Orientation::Vertical
    );

    state.begin(CompositionIdentity {
        engine_epoch: 1,
        context_id: 2,
        composition_id: 4,
    });
    assert_eq!(
        state.resolve_orientation(
            PresentationOrientation::Automatic,
            AutomaticOrientationInput {
                candidates: &annotated,
                locale: "zh-CN",
                work_area,
                caret_x: 100.0,
                scale: 1.0,
                page_size: 9,
            },
        ),
        Orientation::Vertical
    );
}

#[test]
fn automatic_orientation_prefers_vertical_near_right_edge_or_non_cjk() {
    let mut state = CompositionLayoutState::default();
    let work_area = Rect {
        left: 0.0,
        top: 0.0,
        right: 1920.0,
        bottom: 1080.0,
    };
    let compact = vec![CandidateText {
        text: "你".into(),
        comment: String::new(),
    }];
    state.begin(CompositionIdentity {
        engine_epoch: 1,
        context_id: 1,
        composition_id: 1,
    });
    assert_eq!(
        state.resolve_orientation(
            PresentationOrientation::Automatic,
            AutomaticOrientationInput {
                candidates: &compact,
                locale: "zh-CN",
                work_area,
                caret_x: 1900.0,
                scale: 1.0,
                page_size: 9,
            },
        ),
        Orientation::Vertical
    );
    state.begin(CompositionIdentity {
        engine_epoch: 1,
        context_id: 1,
        composition_id: 2,
    });
    assert_eq!(
        state.resolve_orientation(
            PresentationOrientation::Automatic,
            AutomaticOrientationInput {
                candidates: &compact,
                locale: "en-US",
                work_area,
                caret_x: 100.0,
                scale: 1.0,
                page_size: 9,
            },
        ),
        Orientation::Vertical
    );
}

#[test]
fn candidate_presentation_state_owns_scroll_and_composition_transitions() {
    let mut state = CandidatePresentationState::default();
    let first = CandidatePresentationUpdate {
        engine_epoch: 1,
        context_id: 2,
        composition_id: 3,
        revision: 1,
        selected: 0,
        has_selected: 1,
        candidate_count: 30,
        page: 0,
        page_size: 5,
        candidate_bulk: 1,
        configured_scroll_mode: 1,
    };

    assert_eq!(state.apply(first), 0);
    let output = state.output();
    assert_eq!(output.selected, 0);
    assert_eq!(output.has_selected, 1);
    assert_eq!(output.scroll_mode, 0);
    assert_eq!(output.scroll_columns, 5);
    assert_eq!(output.ordinary_start, 0);
    assert_eq!(output.ordinary_count, 5);

    assert_eq!(state.apply(first), 1);
    let mut second = first;
    second.revision = 2;
    second.page = 1;
    second.selected = 5;
    assert_eq!(state.apply(second), 0);
    let output = state.output();
    assert_eq!(output.scroll_mode, 1);
    assert_eq!(output.scroll_expanded, 1);
    assert_eq!(output.ordinary_start, 0);
    assert_eq!(output.ordinary_count, 5);
    assert_eq!(state.apply(first), 2);

    let ended = CandidatePresentationUpdate {
        composition_id: 0,
        revision: 3,
        selected: 0,
        has_selected: 0,
        candidate_count: 0,
        page: 0,
        page_size: 0,
        candidate_bulk: 0,
        configured_scroll_mode: 1,
        ..second
    };
    assert_eq!(state.apply(ended), 0);

    state.set_placement(Placement::Above);
    assert_eq!(state.output().placement, 2);
    let mut next_composition = second;
    next_composition.composition_id = 4;
    next_composition.revision = 1;
    next_composition.page = 0;
    next_composition.selected = 0;
    assert_eq!(state.apply(next_composition), 0);
    let output = state.output();
    assert_eq!(output.scroll_mode, 0);
    assert_eq!(output.scroll_expanded, 0);
    assert_eq!(output.placement, 0);
}

#[test]
fn candidate_presentation_state_tracks_bulk_page_window() {
    let mut state = CandidatePresentationState::default();
    assert_eq!(
        state.apply(CandidatePresentationUpdate {
            engine_epoch: 1,
            context_id: 2,
            composition_id: 3,
            revision: 1,
            selected: 2,
            has_selected: 1,
            candidate_count: 12,
            page: 2,
            page_size: 3,
            candidate_bulk: 1,
            configured_scroll_mode: 0,
        }),
        0
    );
    let output = state.output();
    assert_eq!(output.selected, 2);
    assert_eq!(output.ordinary_start, 6);
    assert_eq!(output.ordinary_count, 3);
    assert_eq!(output.scroll_mode, 0);
}

#[test]
fn candidate_presentation_render_plan_owns_selected_and_page_indices() {
    let state = fcitx5_candidate_presentation_create();
    let update = CandidatePresentationUpdate {
        engine_epoch: 1,
        context_id: 2,
        composition_id: 3,
        revision: 1,
        selected: 2,
        has_selected: 1,
        candidate_count: 12,
        page: 2,
        page_size: 3,
        candidate_bulk: 1,
        configured_scroll_mode: 0,
    };
    assert_eq!(
        // SAFETY: state was created by the paired constructor and update is a valid live reference.
        unsafe { fcitx5_candidate_presentation_apply(state, &update) },
        0
    );

    let mut indices = [usize::MAX; 12];
    let mut plan = Fcitx5CandidatePresentationRenderPlan::default();
    assert_eq!(
        // SAFETY: state is live; indices and plan provide writable storage for their supplied bounds.
        unsafe {
            fcitx5_candidate_presentation_render_plan(
                state,
                indices.as_mut_ptr(),
                indices.len(),
                &mut plan,
            )
        },
        1
    );
    assert_eq!(plan.selected, 2);
    assert_eq!(plan.has_selected, 1);
    assert_eq!(plan.render_count, 3);
    assert_eq!(&indices[..plan.render_count], &[6, 7, 8]);

    let scroll_update = CandidatePresentationUpdate {
        revision: 2,
        selected: 5,
        page: 1,
        configured_scroll_mode: 1,
        ..update
    };
    assert_eq!(
        // SAFETY: state is live and scroll_update is a valid live reference.
        unsafe { fcitx5_candidate_presentation_apply(state, &scroll_update) },
        0
    );
    assert_eq!(
        // SAFETY: state is live; indices and plan provide writable storage for their supplied bounds.
        unsafe {
            fcitx5_candidate_presentation_render_plan(
                state,
                indices.as_mut_ptr(),
                indices.len(),
                &mut plan,
            )
        },
        1
    );
    assert_eq!(plan.selected, 5);
    assert_eq!(plan.render_count, 12);
    assert!(indices[..plan.render_count].iter().copied().eq(0..12));

    // SAFETY: state is the unique live allocation returned by the paired constructor.
    unsafe { fcitx5_candidate_presentation_destroy(state) };
}

#[test]
fn candidate_presentation_orientation_fails_soft_on_invalid_utf8() {
    let state = fcitx5_candidate_presentation_create();
    let candidate = Fcitx5CandidatePresentationText {
        text: Fcitx5CandidateUtf8 {
            ptr: [0xff].as_ptr(),
            len: 1,
        },
        comment: Fcitx5CandidateUtf8::default(),
    };
    // SAFETY: state is live and candidate points to valid test storage for the supplied count.
    let orientation = unsafe {
        fcitx5_candidate_presentation_resolve_orientation(
            state,
            0,
            &candidate,
            1,
            Fcitx5CandidateUtf8::default(),
            Fcitx5CandidateLayoutRect {
                left: 0.0,
                top: 0.0,
                right: 1920.0,
                bottom: 1080.0,
            },
            100.0,
            1.0,
            9,
        )
    };
    assert_eq!(orientation, 0);
    // SAFETY: state is the unique live allocation returned by the paired constructor.
    unsafe { fcitx5_candidate_presentation_destroy(state) };
}

#[test]
fn candidate_poc_self_check_covers_required_evidence() {
    let report = run_candidate_poc_self_check().expect("candidate poc self-check");
    assert!(report.contains("\"cpp_ffi\":false"));
    assert!(report.contains("\"global_hooks\":false"));
    assert!(report.contains("\"process_injection\":false"));
    assert!(report.contains("\"name\":\"vscode-uiless-accessibility\""));
    assert!(report.contains("\"popup_allowed\":false"));
    assert!(report.contains("\"name\":\"emoji-color-font\""));
    assert!(report.contains("\"color_font_candidate_present\":true"));
    assert!(report.contains("\"dpi_scale\":2.00"));
    assert!(report.contains("\"result\":\"PASS\""));
}

#[test]
fn config_preview_paint_plan_preserves_labels_emoji_and_bounds() {
    let plan =
        candidate_preview_paint_plan(1.0, 596.0, 166.0).expect("candidate preview paint plan");
    assert_eq!(plan.selected_background_color, 0x0060_c107);
    assert_eq!(plan.selected_text_color, 0x00ff_ffff);
    assert_eq!(plan.items.len(), 3);
    assert!(plan.items[0].selected);
    assert!(plan.items.iter().any(|item| item.text.contains("1.")));
    assert!(plan.items.iter().any(|item| item.text.contains('你')));
    assert!(plan.items.iter().any(|item| item.text.contains('好')));
    assert!(plan.items.iter().any(|item| item.text.contains('😀')));
    let emoji_item = plan
        .items
        .iter()
        .find(|item| item.text == "3. 😀 emoji")
        .expect("emoji preview candidate");
    assert!(emoji_item.bounds.right - emoji_item.bounds.left >= 128.0);
    assert!(emoji_item.bounds.bottom - emoji_item.bounds.top >= 42.0);
    for item in &plan.items {
        assert!(rect_inside(
            item.bounds,
            Rect {
                left: 0.0,
                top: 0.0,
                right: 596.0,
                bottom: 166.0,
            }
        ));
    }
}
