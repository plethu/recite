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
        Ok::<_, recite_runtime::ConditionEvaluationError>(true)
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
