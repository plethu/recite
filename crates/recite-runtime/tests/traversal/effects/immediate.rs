use super::*;

#[test]
fn immediate_effects_emit_in_reached_source_order_and_traversal_continues() {
    let asset = compile_asset(
        "dialogue/effects.recite",
        concat!(
            ":: start default\n",
            "! immediate intro_sfx(snap)\n",
            ":if trusts(player)\n",
            "  ! immediate trusted_sfx()\n",
            ":else\n",
            "  ! immediate wary_sfx()\n",
            "> prompt@8a32954b25c59aba6d2b\n",
            "  Choose.\n",
            "  ? work@0d22b65183c6720f5f76\n",
            "    Work.\n",
            "    -> work\n",
            "  ? leave@eed3dece07494ef59413\n",
            "    Leave.\n",
            "    -> leave\n",
            ":: work\n",
            "! immediate work_sfx()\n",
            "> done@47ec1949e7254b1015dd\n",
            "  Done.\n",
            "-> END\n",
            ":: leave\n",
            "! immediate leave_sfx()\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_effect(
        next_with_context(&asset, &mut session, &context),
        "intro_sfx",
        DialogueEffectMode::Immediate,
    );
    assert_effect(
        next_with_context(&asset, &mut session, &context),
        "trusted_sfx",
        DialogueEffectMode::Immediate,
    );
    let DialogueEvent::Prompt { .. } =
        next_with_context(&asset, &mut session, &context).expect("emits prompt")
    else {
        panic!("expected prompt");
    };
    assert_effect(
        choose_with_context(
            &asset,
            &mut session,
            ChoiceId::new("0d22b65183c6720f5f76").expect("valid choice ID"),
            &context,
        ),
        "work_sfx",
        DialogueEffectMode::Immediate,
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "47ec1949e7254b1015dd",
        "Done.",
    );
    assert_end_effects(next_with_context(&asset, &mut session, &context), []);
    assert!(session.deferred_effects().is_empty());
}

#[test]
fn mixed_effect_modes_preserve_reached_source_order_without_emitting_deferred_effects() {
    let asset = compile_asset(
        "dialogue/effects.recite",
        concat!(
            ":: start default\n",
            "! deferred first_deferred()\n",
            "! immediate first_immediate()\n",
            "! deferred second_deferred()\n",
            "! blocking wait_for_overlay()\n",
            "! deferred third_deferred()\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let immediate = assert_effect(
        next(&asset, &mut session),
        "first_immediate",
        DialogueEffectMode::Immediate,
    );
    assert!(immediate.id.as_str().ends_with("#1"));
    assert_eq!(
        session
            .deferred_effects()
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        ["first_deferred"]
    );

    let blocking = assert_effect(
        next(&asset, &mut session),
        "wait_for_overlay",
        DialogueEffectMode::Blocking,
    );
    assert!(blocking.id.as_str().ends_with("#2"));
    assert_eq!(
        session
            .deferred_effects()
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        ["first_deferred", "second_deferred"]
    );

    acknowledge_effect(&mut session, blocking.id, EffectAck::Completed)
        .expect("acknowledgement succeeds");
    assert_end_effects(
        next(&asset, &mut session),
        ["first_deferred", "second_deferred", "third_deferred"],
    );
}

#[test]
fn immediate_effects_only_emit_reached_conditional_branch() {
    let asset = compile_asset(
        "dialogue/effects.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  ! immediate trusted_sfx()\n",
            ":else\n",
            "  ! immediate wary_sfx()\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_effect(
        next_with_context(&asset, &mut session, &context),
        "wary_sfx",
        DialogueEffectMode::Immediate,
    );
    assert_end_effects(next_with_context(&asset, &mut session, &context), []);
}
