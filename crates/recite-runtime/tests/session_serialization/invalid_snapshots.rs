use super::*;

fn pending_prompt_snapshot_for_conversion_test()
-> (recite_core::CompiledDialogue, DialogueSessionSnapshot) {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@f58e7e11803840ed96f3\n",
            "  What next?\n",
            "  ? work@9e36f4ebaaeb53f27825\n",
            "    Work.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    (asset, snapshot_session(&session))
}

fn reason_snapshot(id: &str) -> DialogueChoiceAvailabilityReasonSnapshot {
    DialogueChoiceAvailabilityReasonSnapshot {
        id: id.to_owned(),
        source_text: "requires=(trust_gte(hazel, rhea, 3))".to_owned(),
        text: "hazel does not trust rhea enough (3).".to_owned(),
        origin: None,
        args: Vec::new(),
    }
}

#[test]
fn malformed_primary_reason_id_preserves_typed_conversion_source() {
    let (asset, mut snapshot) = pending_prompt_snapshot_for_conversion_test();
    snapshot
        .pending_prompt
        .as_mut()
        .expect("pending prompt")
        .choices[0]
        .availability
        .primary_reason = Some(reason_snapshot(""));

    let error = restore_session(&asset, snapshot).expect_err("empty reason ID is invalid");
    let DialogueError::InvalidSessionSnapshot {
        reason,
        source: Some(source),
    } = &error
    else {
        panic!("expected typed invalid-session-snapshot source, got {error:?}");
    };
    assert_eq!(reason, &source.to_string());
    assert!(matches!(
        source.as_ref(),
        DialogueSessionSnapshotConversionError::InvalidAvailabilityReasonId {
            id,
            source: recite_core::CoreValueError::EmptyId {
                kind: "AvailabilityReasonId"
            },
        } if id.is_empty()
    ));
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn malformed_nested_reason_id_preserves_typed_conversion_source() {
    let (asset, mut snapshot) = pending_prompt_snapshot_for_conversion_test();
    snapshot
        .pending_prompt
        .as_mut()
        .expect("pending prompt")
        .choices[0]
        .availability
        .reason_tree = Some(DialogueChoiceAvailabilityReasonTreeSnapshot::All(vec![
        DialogueChoiceAvailabilityReasonTreeSnapshot::Reason(reason_snapshot("  ")),
    ]));

    let error = restore_session(&asset, snapshot).expect_err("blank reason ID is invalid");
    let DialogueError::InvalidSessionSnapshot {
        reason,
        source: Some(source),
    } = &error
    else {
        panic!("expected typed invalid-session-snapshot source, got {error:?}");
    };
    assert_eq!(reason, &source.to_string());
    assert!(matches!(
        source.as_ref(),
        DialogueSessionSnapshotConversionError::InvalidAvailabilityReasonId {
            id,
            source: recite_core::CoreValueError::EmptyId {
                kind: "AvailabilityReasonId"
            },
        } if id == "  "
    ));
    assert!(std::error::Error::source(&error).is_some());
}

#[test]
fn malformed_pending_prompt_snapshot_is_rejected_before_choice_selection() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@d947b66f14fb18e70357\n",
            "  What next?\n",
            "  ? work@814ea8b77133bb90bfcc\n",
            "    Work.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    let mut snapshot = snapshot_session(&session);
    snapshot
        .pending_prompt
        .as_mut()
        .expect("pending prompt")
        .choices[0]
        .id = "forged_choice".to_owned();
    snapshot.previous_prompt_choices[0] = "forged_choice".to_owned();

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn out_of_range_current_block_snapshot_is_rejected_as_snapshot_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@6a4848ae75afc843e52f\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot = snapshot_session(&session);
    snapshot.current_block = 99;

    let error = restore_session(&asset, snapshot).expect_err("out-of-range block is invalid");
    let DialogueError::InvalidSessionSnapshot { reason, source } = &error else {
        panic!("expected invalid session snapshot, got {error:?}");
    };
    assert!(source.is_none());
    assert_eq!(
        error.to_string(),
        format!("invalid session snapshot: {reason}")
    );
    assert!(std::error::Error::source(&error).is_none());
}

#[test]
fn out_of_range_active_statement_range_snapshot_is_rejected_as_snapshot_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@ca0eb19145f21226c70f\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot = snapshot_session(&session);
    snapshot.current_range.start = 99;
    snapshot.current_range.len = 1;
    snapshot.next_statement = 99;

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn out_of_range_continuation_frame_range_snapshot_is_rejected_as_snapshot_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > inside@a761d40e0c8df66b8de1\n",
            "    Inside.\n",
            "> after@e8224e6a79bfb239d513\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = |_: recite_runtime::ConditionQuery<'_>| {
        Ok::<_, recite_runtime::ConditionEvaluationError>(recite_runtime::ConditionValue::Bool(
            true,
        ))
    };
    let mut session = start_scene(&asset, None).expect("starts");
    runtime_next(&asset, &mut session, &context).expect("emits branch line");
    let mut snapshot = snapshot_session(&session);
    snapshot.continuation_stack[0].range.start = 99;
    snapshot.continuation_stack[0].range.len = 1;
    snapshot.continuation_stack[0].next_statement = 99;

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn out_of_range_pending_prompt_statement_snapshot_is_rejected_as_snapshot_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@f58e7e11803840ed96f3\n",
            "  What next?\n",
            "  ? work@9e36f4ebaaeb53f27825\n",
            "    Work.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    let mut snapshot = snapshot_session(&session);
    snapshot
        .pending_prompt
        .as_mut()
        .expect("pending prompt")
        .statement = 99;

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn forged_cross_block_active_range_is_rejected() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@2a8c0659114df64848df\n",
            "  Start.\n",
            "-> END\n",
            ":: other\n",
            "> other_line@c84286e246b8ef9de1a6\n",
            "  Other.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot = snapshot_session(&session);
    snapshot.current_range.start = asset.blocks[1].statements.start.as_u32();
    snapshot.current_range.len = asset.blocks[1].statements.len;
    snapshot.next_statement = asset.blocks[1].statements.start.as_u32();

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn unreachable_pending_prompt_statement_is_rejected() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> first_prompt@957672b6cabc784de8c1\n",
            "  First?\n",
            "  ? first_choice@59cbd94cd3df72b40945\n",
            "    First.\n",
            "    -> END\n",
            "> second_prompt@46a2fba25ea6e47d5ba9\n",
            "  Second?\n",
            "  ? second_choice@4b12cfcad884bd373854\n",
            "    Second.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits first prompt");
    let mut snapshot = snapshot_session(&session);
    let second_prompt_statement = snapshot.next_statement;
    let pending_prompt = snapshot
        .pending_prompt
        .as_mut()
        .expect("pending prompt is serialized");
    pending_prompt.statement = second_prompt_statement;
    pending_prompt.choices[0].id = "second_choice".to_owned();
    snapshot.previous_prompt_choices[0] = "second_choice".to_owned();

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn out_of_range_pending_effect_statement_snapshot_is_rejected_as_snapshot_error() {
    let asset = blocking_effect_asset();
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits blocking effect");
    let mut snapshot = snapshot_session(&session);
    snapshot
        .pending_effect
        .as_mut()
        .expect("pending effect")
        .statement = 99;

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn pending_effect_statement_must_be_immediately_before_next_statement() {
    let asset = blocking_effect_asset();
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits blocking effect");
    let mut snapshot = snapshot_session(&session);
    snapshot.next_statement = snapshot
        .pending_effect
        .as_ref()
        .expect("pending effect")
        .statement;

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn pending_effect_statement_must_reference_an_effect_statement() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@67f2537a1a0f5419fb49\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits line");
    let mut snapshot = snapshot_session(&session);
    snapshot.pending_effect = Some(DialogueSessionPendingEffectSnapshot {
        statement: 0,
        id: "effect:dialogue/start.recite:2:1#1".to_owned(),
    });

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn pending_effect_statement_must_reference_a_blocking_effect() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! immediate play_sfx(snap)\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits immediate effect");
    let mut snapshot = snapshot_session(&session);
    snapshot.pending_effect = Some(DialogueSessionPendingEffectSnapshot {
        statement: 0,
        id: "effect:dialogue/start.recite:2:1#1".to_owned(),
    });

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn pending_effect_rejects_forged_deferred_effect_statement() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred entered_start()\n",
            "> start_line@a0affbc838e2daaabdfc\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot = snapshot_session(&session);
    snapshot.next_statement = 1;
    snapshot.trace_counter = 1;
    snapshot.pending_effect = Some(DialogueSessionPendingEffectSnapshot {
        statement: 0,
        id: "effect:dialogue/start.recite:2:1#1".to_owned(),
    });

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn forged_pending_effect_id_is_rejected_as_snapshot_error() {
    let asset = blocking_effect_asset();
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits blocking effect");
    let mut snapshot = snapshot_session(&session);
    snapshot.pending_effect.as_mut().expect("pending effect").id = "effect:forged#1".to_owned();

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn ended_session_with_pending_effect_is_rejected_as_snapshot_error() {
    let asset = blocking_effect_asset();
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits blocking effect");
    let mut snapshot = snapshot_session(&session);
    snapshot.ended = true;

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn snapshot_cannot_have_both_pending_prompt_and_pending_effect() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@9cdefad04532b99377d3\n",
            "  What next?\n",
            "  ? work@3cd675af1fbaa0143bfe\n",
            "    Work.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    let mut snapshot = snapshot_session(&session);
    snapshot.pending_effect = Some(DialogueSessionPendingEffectSnapshot {
        statement: 0,
        id: "effect:dialogue/start.recite:2:1#1".to_owned(),
    });

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

fn blocking_effect_asset() -> CompiledDialogue {
    compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! blocking grant_item(map)\n",
            "> after_grant@6b640d6c242d099e3b2a\n",
            "  Granted.\n",
            "-> END\n",
        ),
    )
}
