use super::*;

#[test]
fn malformed_pending_prompt_snapshot_is_rejected_before_choice_selection() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? work\n",
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
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot = snapshot_session(&session);
    snapshot.current_block = 99;

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

#[test]
fn out_of_range_active_statement_range_snapshot_is_rejected_as_snapshot_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
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
            "  > inside\n",
            "    Inside.\n",
            "> after\n",
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
            "> prompt_line\n",
            "  What next?\n",
            "  ? work\n",
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
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
            ":: other\n",
            "> other_line\n",
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
            "> first_prompt\n",
            "  First?\n",
            "  ? first_choice\n",
            "    First.\n",
            "    -> END\n",
            "> second_prompt\n",
            "  Second?\n",
            "  ? second_choice\n",
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
            "> start_line\n",
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
            "> start_line\n",
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
            "> prompt_line\n",
            "  What next?\n",
            "  ? work\n",
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
            "> after_grant\n",
            "  Granted.\n",
            "-> END\n",
        ),
    )
}
