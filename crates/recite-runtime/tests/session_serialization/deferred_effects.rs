use super::*;

#[test]
fn restores_deferred_effects_collected_before_save_and_continues_in_order() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred entered_start(alpha, \"beta\", 3, 0.5, true)\n",
            "> @prompt_line\n",
            "  What next?\n",
            "  ? @work\n",
            "    Work.\n",
            "    -> work\n",
            ":: work\n",
            "! deferred entered_work()\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    let DialogueEvent::Prompt { .. } = next(&asset, &mut session).expect("emits prompt") else {
        panic!("expected prompt");
    };
    assert_effect_functions(session.deferred_effects(), ["entered_start"]);

    assert_snapshot_encoding(&asset, &session);
    let mut restored = restore_session(
        &asset,
        snapshot_session(&asset, &session).expect("session snapshots"),
    )
    .expect("restores deferred state");
    let effects = assert_end_effects(
        choose(
            &asset,
            &mut restored,
            ChoiceId::new("work").expect("valid choice ID"),
        ),
        ["entered_start", "entered_work"],
    );
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
}

#[test]
fn saved_deferred_effects_are_self_contained_but_must_be_deferred() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! immediate play_sfx(snap)\n",
            "> @start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let snapshot = snapshot_session(&asset, &session).expect("session snapshots");

    // A request collected before a patch keeps its saved ID and span, even
    // though no compiled effect has that positional ID any more.
    let mut carried = snapshot.clone();
    carried.deferred_effects = vec![forged_effect_request(
        "effect:dialogue/start.recite:9:1",
        DialogueEffectModeSnapshot::Deferred,
    )];
    let restored = restore_session(&asset, carried).expect("restores carried request");
    assert_eq!(
        restored.deferred_effects()[0].id.as_str(),
        "effect:dialogue/start.recite:9:1"
    );

    let mut immediate = snapshot;
    immediate.deferred_effects = vec![forged_effect_request(
        "effect:dialogue/start.recite:2:1",
        DialogueEffectModeSnapshot::Immediate,
    )];
    assert!(matches!(
        restore_session(&asset, immediate),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}
