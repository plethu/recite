use super::*;

#[test]
fn restores_deferred_effects_collected_before_save_and_continues_in_order() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred entered_start(alpha, \"beta\", 3, 0.5, true)\n",
            "> prompt_line@fdc4caf6396ac130f27b\n",
            "  What next?\n",
            "  ? work@d32d0f33dfd989109d84\n",
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

    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores deferred state");
    let effects = assert_end_effects(
        choose(
            &asset,
            &mut restored,
            ChoiceId::new("d32d0f33dfd989109d84").expect("valid choice ID"),
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
fn forged_deferred_effect_snapshot_must_reference_a_compiled_deferred_effect() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! immediate play_sfx(snap)\n",
            "> start_line@4ea3ba655c6213a87719\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot = snapshot_session(&session);
    snapshot.deferred_effects = vec![DialogueDeferredEffectSnapshot {
        id: "effect:dialogue/start.recite:2:1".to_owned(),
    }];

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}
