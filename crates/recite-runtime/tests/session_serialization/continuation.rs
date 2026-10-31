use super::*;

#[test]
fn restores_continuation_stack_inside_conditional_branch() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > @pg92x7f\n",
            "    Inside.\n",
            "> @tkmsp5e\n",
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

    assert_line(
        runtime_next(&asset, &mut session, &context),
        "pg92x7f",
        "Inside.",
    );
    assert_snapshot_encoding(&session);
    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores branch state");

    assert_line(
        runtime_next(&asset, &mut restored, &context),
        "tkmsp5e",
        "After.",
    );
    assert_eq!(
        runtime_next(&asset, &mut restored, &context),
        Ok(empty_end())
    );
}

#[test]
fn restores_end_state_reached_inside_conditional_branch() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  ! deferred branch_done()\n",
            "  -> END\n",
            "> @f750fzk\n",
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
    assert_end_effects(
        runtime_next(&asset, &mut session, &context),
        ["branch_done"],
    );

    assert_snapshot_encoding(&session);
    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores branch end state");
    assert_eq!(
        runtime_next(&asset, &mut restored, &context),
        Err(DialogueError::SessionEnded)
    );
    assert_effect_functions(restored.deferred_effects(), ["branch_done"]);
}

#[test]
fn restores_continuation_stack_inside_match_arm() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match mood()\n",
            "  :case tired\n",
            "    > @wwjwj2z\n",
            "      Inside.\n",
            "  :case _\n",
            "    > @frta933\n",
            "      Fallback.\n",
            "> @ckwk4wx\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = |_: recite_runtime::ConditionQuery<'_>| {
        Ok::<_, recite_runtime::ConditionEvaluationError>(
            recite_runtime::ConditionValue::EnumVariant("tired".to_owned()),
        )
    };
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        runtime_next(&asset, &mut session, &context),
        "wwjwj2z",
        "Inside.",
    );
    assert_snapshot_encoding(&session);
    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores match state");

    assert_line(
        runtime_next(&asset, &mut restored, &context),
        "ckwk4wx",
        "After.",
    );
}

#[test]
fn restores_nested_match_and_if_continuations() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match mood()\n",
            "  :case tired\n",
            "    :if trusts(player)\n",
            "      :match stage()\n",
            "        :case tired\n",
            "          > @deep\n",
            "            Deep.\n",
            "        :case _\n",
            "          > @other\n",
            "            Other.\n",
            "  :case _\n",
            "    > @b3yssg4\n",
            "      Fallback.\n",
            "> @trjba8e\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = |query: recite_runtime::ConditionQuery<'_>| {
        let value = match query.function() {
            "mood" => recite_runtime::ConditionValue::EnumVariant("tired".to_owned()),
            "stage" => recite_runtime::ConditionValue::EnumVariant("tired".to_owned()),
            "trusts" => recite_runtime::ConditionValue::Bool(true),
            function => panic!("unexpected condition {function}"),
        };
        Ok::<_, recite_runtime::ConditionEvaluationError>(value)
    };
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        runtime_next(&asset, &mut session, &context),
        "deep",
        "Deep.",
    );
    assert_snapshot_encoding(&session);
    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores nested state");

    assert_line(
        runtime_next(&asset, &mut restored, &context),
        "trjba8e",
        "After.",
    );
}
