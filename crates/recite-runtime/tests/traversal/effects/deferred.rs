use super::*;

#[test]
fn collects_deferred_effects_in_source_order_and_returns_them_at_end() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> before@f4e165af87225d22bbeb\n",
            "  Before.\n",
            "! deferred first(alpha, \"beta\", 3, 0.5, true)\n",
            "> middle@a179e0df75e958fa949b\n",
            "  Middle.\n",
            "! deferred second()\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next(&asset, &mut session),
        "f4e165af87225d22bbeb",
        "Before.",
    );
    assert!(session.deferred_effects().is_empty());
    let middle = next(&asset, &mut session);
    assert!(
        !matches!(middle, Ok(DialogueEvent::Effect(_))),
        "deferred effects must not emit effect events"
    );
    assert_line(middle, "a179e0df75e958fa949b", "Middle.");
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
            "> prompt_line@3482ad56276d981ccd11\n",
            "  What next?\n",
            "  ? work@c2d6f363bce8384c6d3b\n",
            "    Work.\n",
            "    -> work\n",
            "  ? leave@6adaf25eb5d10a953c6d\n",
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
            ChoiceId::new("c2d6f363bce8384c6d3b").expect("valid choice ID"),
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
