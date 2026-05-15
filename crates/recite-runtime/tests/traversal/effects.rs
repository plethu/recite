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
    assert_line(next(&asset, &mut session), "middle", "Middle.");
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
fn immediate_and_blocking_effects_are_structured_unsupported_mode_errors() {
    let immediate_asset = compile_asset(
        "dialogue/immediate.recite",
        concat!(
            ":: start default\n",
            "! immediate play_sfx(snap)\n",
            "-> END\n",
        ),
    );
    let mut immediate_session = start_scene(&immediate_asset, None).expect("starts");
    assert_eq!(
        next(&immediate_asset, &mut immediate_session),
        Err(DialogueError::UnsupportedEffectMode {
            mode: DialogueEffectMode::Immediate,
        })
    );

    let blocking_asset = compile_asset(
        "dialogue/blocking.recite",
        concat!(
            ":: start default\n",
            "! blocking grant_item(map)\n",
            "-> END\n",
        ),
    );
    let mut blocking_session = start_scene(&blocking_asset, None).expect("starts");
    assert_eq!(
        next(&blocking_asset, &mut blocking_session),
        Err(DialogueError::UnsupportedEffectMode {
            mode: DialogueEffectMode::Blocking,
        })
    );
}
