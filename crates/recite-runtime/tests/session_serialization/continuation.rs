use super::*;

#[test]
fn restores_continuation_stack_inside_conditional_branch() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > inside@eedc6e8ea24b3e6809f8\n",
            "    Inside.\n",
            "> after@26a5f3da3383cf221b7d\n",
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
        "eedc6e8ea24b3e6809f8",
        "Inside.",
    );
    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores branch state");

    assert_line(
        runtime_next(&asset, &mut restored, &context),
        "26a5f3da3383cf221b7d",
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
            "> after@e1c13dacc2bb0e82080d\n",
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
            "    > inside@2faca4e74f3382336ff7\n",
            "      Inside.\n",
            "  :case _\n",
            "    > fallback@66cec1a7117cd4905dd7\n",
            "      Fallback.\n",
            "> after@3cc87e383f3f09945030\n",
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
        "2faca4e74f3382336ff7",
        "Inside.",
    );
    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores match state");

    assert_line(
        runtime_next(&asset, &mut restored, &context),
        "3cc87e383f3f09945030",
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
            "          > deep@fb4f4bf0d1d3c7ee0a26\n",
            "            Deep.\n",
            "        :case _\n",
            "          > other@796fb82b1df076c6d2c9\n",
            "            Other.\n",
            "  :case _\n",
            "    > fallback@ce1625c7fb901d4f6833\n",
            "      Fallback.\n",
            "> after@0275abeda43f946b048a\n",
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
        "fb4f4bf0d1d3c7ee0a26",
        "Deep.",
    );
    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores nested state");

    assert_line(
        runtime_next(&asset, &mut restored, &context),
        "0275abeda43f946b048a",
        "After.",
    );
}
