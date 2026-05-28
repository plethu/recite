use super::*;

#[test]
fn match_enters_first_matching_variant_and_skips_later_arms() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match thread_stage(thread)\n",
            "  :case tired\n",
            "    > tired_line\n",
            "      Tired.\n",
            "  :case _\n",
            "    > fallback_line\n",
            "      Fallback.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with_enum("thread_stage", "tired");
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "tired_line",
        "Tired.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "after",
        "After.",
    );
    assert_eq!(
        context.calls(),
        [RecordedCall {
            function: "thread_stage".to_owned(),
            arguments: vec![RecordedArgument::Identifier("thread".to_owned())],
        }]
    );
}

#[test]
fn match_wildcard_runs_only_when_no_variant_matches() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match thread_stage(thread)\n",
            "  :case tired\n",
            "    > tired_line\n",
            "      Tired.\n",
            "  :case _\n",
            "    > fallback_line\n",
            "      Fallback.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with_enum("thread_stage", "rested");
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "fallback_line",
        "Fallback.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "after",
        "After.",
    );
}

#[test]
fn match_arm_bodies_traverse_runtime_constructs() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match stage()\n",
            "  :case tired\n",
            "    :match mood()\n",
            "      :case tired\n",
            "        > low_line\n",
            "          Low.\n",
            "      :case _\n",
            "        > other_line\n",
            "          Other.\n",
            "  :case _\n",
            "    > fallback_line\n",
            "      Fallback.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default()
        .with_enum("stage", "tired")
        .with_enum("mood", "tired");
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "low_line",
        "Low.",
    );
}

#[test]
fn match_arm_can_present_choice_and_divert() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match stage()\n",
            "  :case tired\n",
            "    > prompt_line\n",
            "      Pick.\n",
            "      ? continue\n",
            "        Continue.\n",
            "        -> done\n",
            "  :case _\n",
            "    > fallback_line\n",
            "      Fallback.\n",
            ":: done\n",
            "> done_line\n",
            "  Done.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with_enum("stage", "tired");
    let mut session = start_scene(&asset, None).expect("starts");

    assert!(matches!(
        next_with_context(&asset, &mut session, &context),
        Ok(DialogueEvent::Prompt { .. })
    ));
    assert_line(
        choose_with_context(
            &asset,
            &mut session,
            ChoiceId::new("continue").expect("valid choice id"),
            &context,
        ),
        "done_line",
        "Done.",
    );
}

#[test]
fn match_arm_can_emit_immediate_effect() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match stage()\n",
            "  :case tired\n",
            "    ! immediate notify(tired)\n",
            "  :case _\n",
            "    > fallback_line\n",
            "      Fallback.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with_enum("stage", "tired");
    let mut session = start_scene(&asset, None).expect("starts");

    let event = next_with_context(&asset, &mut session, &context).expect("effect succeeds");
    let DialogueEvent::Effect(effect) = event else {
        panic!("expected effect event");
    };
    assert_eq!(effect.function, "notify");
    assert_eq!(effect.mode, DialogueEffectMode::Immediate);
}

#[test]
fn match_arm_can_enter_nested_if() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match stage()\n",
            "  :case tired\n",
            "    :if trusts(player)\n",
            "      > trusted_line\n",
            "        Trusted.\n",
            "  :case _\n",
            "    > fallback_line\n",
            "      Fallback.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default()
        .with_enum("stage", "tired")
        .with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "trusted_line",
        "Trusted.",
    );
}

#[test]
fn match_condition_failure_keeps_session_position() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match mood()\n",
            "  :case tired\n",
            "    > tired_line\n",
            "      Tired.\n",
            "  :case _\n",
            "    > fallback_line\n",
            "      Fallback.\n",
            "-> END\n",
        ),
    );
    let failing = RecordingContext::default().failing("mood", "mood service unavailable");
    let passing = RecordingContext::default().with_enum("mood", "tired");
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next_with_context(&asset, &mut session, &failing),
        Err(DialogueError::ConditionEvaluationFailed {
            function: "mood".to_owned(),
            reason: "mood service unavailable".to_owned(),
        })
    );
    assert_line(
        next_with_context(&asset, &mut session, &passing),
        "tired_line",
        "Tired.",
    );
}

#[test]
fn match_reports_wrong_condition_kind() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match mood()\n",
            "  :case tired\n",
            "    > tired_line\n",
            "      Tired.\n",
            "  :case _\n",
            "    > fallback_line\n",
            "      Fallback.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("mood", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next_with_context(&asset, &mut session, &context),
        Err(DialogueError::ConditionResultTypeMismatch {
            function: "mood".to_owned(),
            expected: ConditionExpectedType::Enum,
            actual: ConditionExpectedType::Bool,
        })
    );
}

#[test]
fn boolean_condition_reports_wrong_condition_kind() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if mood()\n",
            "  > secret\n",
            "    Secret.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with_enum("mood", "tired");
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next_with_context(&asset, &mut session, &context),
        Err(DialogueError::ConditionResultTypeMismatch {
            function: "mood".to_owned(),
            expected: ConditionExpectedType::Bool,
            actual: ConditionExpectedType::Enum,
        })
    );
}
