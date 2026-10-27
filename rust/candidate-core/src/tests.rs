#![forbid(unsafe_code)]

use super::*;

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
fn content_locale_policy_matches_cpp_contract() {
    assert!(content_locale_valid(b"zh-CN"));
    assert!(content_locale_valid(b"en-US"));
    assert!(content_locale_valid(b"ja-JP-u-ca-japanese"));
    assert!(!content_locale_valid(b""));
    assert!(!content_locale_valid(b"----"));
    assert!(!content_locale_valid(b"zh_CN"));
    assert!(!content_locale_valid(&[b'a'; MAX_CONTENT_LOCALE_UTF8 + 1]));
    assert_eq!(content_locale_or_default(b"zh-CN"), wide("zh-CN"));
    assert!(locale_prefers_compact_horizontal("JA-jp"));
}
