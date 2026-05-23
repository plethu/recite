use super::*;

#[test]
fn collects_deferred_effects_in_source_order_and_returns_them_at_end() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> before\n",
            "  Before.\n",
            "! deferred first(alpha, \"beta\", 3, 0.5, true)\n",
            "> middle\n",
            "  Middle.\n",
            "! deferred second()\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(next(&asset, &mut session), "before", "Before.");
    assert!(session.deferred_effects().is_empty());
    let middle = next(&asset, &mut session);
    assert!(
        !matches!(middle, Ok(DialogueEvent::Effect(_))),
        "deferred effects must not emit effect events"
    );
    assert_line(middle, "middle", "Middle.");
    assert_eq!(
        session
            .deferred_effects()
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        ["first"]
    );

    let effects = assert_end_effects(next(&asset, &mut session), ["first", "second"]);
    assert_eq!(effects[0].id.as_str(), "effect:dialogue/start.recite:4:1");
    assert_eq!(effects[0].mode, DialogueEffectMode::Deferred);
    assert_eq!(
        effects[0].args,
        vec![
            DialogueEffectArgument::Identifier("alpha".to_owned()),
            DialogueEffectArgument::String("beta".to_owned()),
            DialogueEffectArgument::Integer(3),
            DialogueEffectArgument::Float(0.5),
            DialogueEffectArgument::Boolean(true),
        ]
    );
    assert_eq!(effects[0].source_span.start.line(), 4);
}

#[test]
fn deferred_effects_are_collected_without_calling_game_context() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred advance_thread(start, asked)\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().failing("advance_thread", "should not be called");
    let mut session = start_scene(&asset, None).expect("starts");

    assert_end_effects(
        next_with_context(&asset, &mut session, &context),
        ["advance_thread"],
    );
    assert!(context.calls().is_empty());
}

#[test]
fn deferred_effects_follow_selected_choice_and_divert_paths() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred entered_start()\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? work\n",
            "    Work.\n",
            "    -> work\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "! deferred entered_work()\n",
            "-> finish\n",
            ":: finish\n",
            "! deferred entered_finish()\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { .. } = next(&asset, &mut session).expect("emits prompt") else {
        panic!("expected prompt");
    };

    assert_end_effects(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("work").expect("valid choice ID"),
        ),
        ["entered_start", "entered_work", "entered_finish"],
    );
}

#[test]
fn deferred_effects_only_collect_reached_conditional_branch() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred before_branch()\n",
            ":if trusts(player)\n",
            "  ! deferred then_branch()\n",
            ":else\n",
            "  ! deferred else_branch()\n",
            "! deferred after_branch()\n",
            "-> END\n",
        ),
    );

    let then_context = RecordingContext::default().with("trusts", true);
    let mut then_session = start_scene(&asset, None).expect("starts");
    assert_end_effects(
        next_with_context(&asset, &mut then_session, &then_context),
        ["before_branch", "then_branch", "after_branch"],
    );

    let else_context = RecordingContext::default().with("trusts", false);
    let mut else_session = start_scene(&asset, None).expect("starts");
    assert_end_effects(
        next_with_context(&asset, &mut else_session, &else_context),
        ["before_branch", "else_branch", "after_branch"],
    );
}

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
            "> prompt\n",
            "  Choose.\n",
            "  ? work\n",
            "    Work.\n",
            "    -> work\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> leave\n",
            ":: work\n",
            "! immediate work_sfx()\n",
            "> done\n",
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
            ChoiceId::new("work").expect("valid choice ID"),
            &context,
        ),
        "work_sfx",
        DialogueEffectMode::Immediate,
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "done",
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

#[test]
fn repeated_blocking_effect_emissions_reject_stale_acknowledgement_ids() {
    let asset = compile_asset(
        "dialogue/blocking.recite",
        concat!(
            ":: start default\n",
            "! blocking grant_item(map)\n",
            "-> start\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let first_effect = assert_effect(
        next(&asset, &mut session),
        "grant_item",
        DialogueEffectMode::Blocking,
    );
    acknowledge_effect(&mut session, first_effect.id.clone(), EffectAck::Completed)
        .expect("first acknowledgement succeeds");

    let second_effect = assert_effect(
        next(&asset, &mut session),
        "grant_item",
        DialogueEffectMode::Blocking,
    );
    assert_ne!(first_effect.id, second_effect.id);
    assert_eq!(session.pending_effect(), Some(&second_effect));
    assert_eq!(
        acknowledge_effect(&mut session, first_effect.id.clone(), EffectAck::Completed),
        Err(DialogueError::WrongEffectAcknowledgement {
            expected: second_effect.id.clone(),
            actual: first_effect.id,
        })
    );
    assert_eq!(session.pending_effect(), Some(&second_effect));

    acknowledge_effect(&mut session, second_effect.id, EffectAck::Completed)
        .expect("second acknowledgement succeeds");
    assert!(session.pending_effect().is_none());
}

#[test]
fn blocking_effects_pause_until_acknowledged_and_resume_after_completion_or_failure() {
    let asset = compile_asset(
        "dialogue/blocking.recite",
        concat!(
            ":: start default\n",
            "> prompt\n",
            "  Choose.\n",
            "  ? work\n",
            "    Work.\n",
            "    -> work\n",
            ":: work\n",
            "! blocking grant_item(map)\n",
            "> after_grant\n",
            "  Granted.\n",
            "! blocking open_overlay(inventory)\n",
            "> after_overlay\n",
            "  Closed.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    let DialogueEvent::Prompt { .. } = next(&asset, &mut session).expect("emits prompt") else {
        panic!("expected prompt");
    };

    let first_effect = assert_effect(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("work").expect("valid choice ID"),
        ),
        "grant_item",
        DialogueEffectMode::Blocking,
    );
    assert_eq!(session.pending_effect(), Some(&first_effect));
    assert_eq!(
        next(&asset, &mut session),
        Err(DialogueError::EffectPending {
            effect: first_effect.id.clone(),
        })
    );
    assert_eq!(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("work").expect("valid choice ID"),
        ),
        Err(DialogueError::EffectPending {
            effect: first_effect.id.clone(),
        })
    );

    let wrong_id = EffectId::new("effect:wrong").expect("valid effect ID");
    assert_eq!(
        acknowledge_effect(&mut session, wrong_id.clone(), EffectAck::Completed),
        Err(DialogueError::WrongEffectAcknowledgement {
            expected: first_effect.id.clone(),
            actual: wrong_id,
        })
    );
    assert_eq!(session.pending_effect(), Some(&first_effect));

    acknowledge_effect(&mut session, first_effect.id.clone(), EffectAck::Completed)
        .expect("completed acknowledgement succeeds");
    assert!(session.pending_effect().is_none());
    assert_line(next(&asset, &mut session), "after_grant", "Granted.");

    let second_effect = assert_effect(
        next(&asset, &mut session),
        "open_overlay",
        DialogueEffectMode::Blocking,
    );
    acknowledge_effect(
        &mut session,
        second_effect.id.clone(),
        EffectAck::Failed {
            reason: "overlay closed externally".to_owned(),
        },
    )
    .expect("failed acknowledgement succeeds");
    assert!(session.pending_effect().is_none());
    assert_eq!(
        acknowledge_effect(&mut session, second_effect.id.clone(), EffectAck::Completed),
        Err(DialogueError::NoEffectPending {
            effect: second_effect.id,
        })
    );
    assert_line(next(&asset, &mut session), "after_overlay", "Closed.");
    assert_end_effects(next(&asset, &mut session), []);
}

fn assert_effect(
    event: Result<DialogueEvent, DialogueError>,
    function: &str,
    mode: DialogueEffectMode,
) -> DialogueEffectRequest {
    let DialogueEvent::Effect(effect) = event.expect("effect event succeeds") else {
        panic!("expected effect event");
    };

    assert_eq!(effect.function, function);
    assert_eq!(effect.mode, mode);
    effect
}
