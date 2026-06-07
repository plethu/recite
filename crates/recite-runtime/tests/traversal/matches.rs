use super::*;

#[test]
fn match_enters_first_matching_variant_and_skips_later_arms() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":match thread_stage(thread)\n",
            "  :case tired\n",
            "    > tired_line@f9c433be77d9486d158f\n",
            "      Tired.\n",
            "  :case _\n",
            "    > fallback_line@e0e4e075e77925ffc767\n",
            "      Fallback.\n",
            "> after@1d9b481352f484253839\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with_enum("thread_stage", "tired");
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "f9c433be77d9486d158f",
        "Tired.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "1d9b481352f484253839",
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
            "    > tired_line@8346d134e58c12c254c3\n",
            "      Tired.\n",
            "  :case _\n",
            "    > fallback_line@078ef090046530a363e0\n",
            "      Fallback.\n",
            "> after@776956c8b42042ae32a5\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with_enum("thread_stage", "rested");
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "078ef090046530a363e0",
        "Fallback.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "776956c8b42042ae32a5",
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
            "        > low_line@39bdae88ca9ffd1503ee\n",
            "          Low.\n",
            "      :case _\n",
            "        > other_line@e46e71801736f8af303b\n",
            "          Other.\n",
            "  :case _\n",
            "    > fallback_line@daaf0ddfb563b4db5813\n",
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
        "39bdae88ca9ffd1503ee",
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
            "    > prompt_line@ccdc2ad1752b7e7faeb4\n",
            "      Pick.\n",
            "      ? continue@292c3bdaedddbe25c300\n",
            "        Continue.\n",
            "        -> done\n",
            "  :case _\n",
            "    > fallback_line@a763bf9b1a92598199c0\n",
            "      Fallback.\n",
            ":: done\n",
            "> done_line@da9debbfa2ea916aa980\n",
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
            ChoiceId::new("292c3bdaedddbe25c300").expect("valid choice id"),
            &context,
        ),
        "da9debbfa2ea916aa980",
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
            "    > fallback_line@703e738521faa003319c\n",
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
            "      > trusted_line@7d0ab384f5f7ff552de0\n",
            "        Trusted.\n",
            "  :case _\n",
            "    > fallback_line@32e96a0baca8d5b7ed3a\n",
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
        "7d0ab384f5f7ff552de0",
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
            "    > tired_line@1a2610e2e792ccb3df70\n",
            "      Tired.\n",
            "  :case _\n",
            "    > fallback_line@76c9977d372b39f703f8\n",
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
        "1a2610e2e792ccb3df70",
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
            "    > tired_line@4541f1d39acb6460faae\n",
            "      Tired.\n",
            "  :case _\n",
            "    > fallback_line@ffa51f61deec39ca5910\n",
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
            "  > secret@fdd09cd4f3f31e064584\n",
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
