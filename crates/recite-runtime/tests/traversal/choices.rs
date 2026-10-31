use super::*;

#[test]
fn chooses_pending_prompt_option_by_stable_choice_id() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @hk0gnng\n",
            "  What next?\n",
            "  ? @hvn5j9a\n",
            "    Ask about work.\n",
            "    -> work\n",
            "  ? @xax2k09\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> @kgk4w24\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } = next(&asset, &mut session).expect("emits prompt")
    else {
        panic!("expected prompt");
    };
    assert_eq!(choices[0].id.as_str(), "hvn5j9a");
    assert_eq!(choices[1].id.as_str(), "xax2k09");

    assert_eq!(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("xax2k09").expect("valid choice ID")
        ),
        Ok(empty_end())
    );
    assert_eq!(
        session
            .selected_choice_history()
            .iter()
            .map(ChoiceId::as_str)
            .collect::<Vec<_>>(),
        ["xax2k09"]
    );
}

#[test]
fn choosing_choice_target_continues_from_target_block() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @nbcw5zp\n",
            "  What next?\n",
            "  ? @ff3qmh4\n",
            "    Ask about work.\n",
            "    -> work\n",
            "  ? @wx5c3yz\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> @v0sbp9e\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");

    assert_line(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("ff3qmh4").expect("valid choice ID"),
        ),
        "v0sbp9e",
        "Work waits.",
    );
}

#[test]
fn invalid_choice_for_pending_prompt_is_structured_error_and_keeps_prompt_pending() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @d2hsqtd\n",
            "  What next?\n",
            "  ? @faj0q2n\n",
            "    Ask about work.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    let missing = ChoiceId::new("missing").expect("valid choice ID");
    let ask_work = ChoiceId::new("faj0q2n").expect("valid choice ID");

    assert_eq!(
        choose(&asset, &mut session, missing.clone()),
        Err(DialogueError::InvalidChoice {
            choice: missing,
            prompt_choices: vec![ask_work.clone()]
        })
    );
    assert_eq!(
        next(&asset, &mut session),
        Err(DialogueError::PromptPending {
            choices: vec![ask_work]
        })
    );
}

#[test]
fn stale_or_non_pending_choice_selection_is_structured_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @wac3p8g\n",
            "  What next?\n",
            "  ? @cykgd25\n",
            "    Leave.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    let leave = ChoiceId::new("cykgd25").expect("valid choice ID");

    assert_eq!(
        choose(&asset, &mut session, leave.clone()),
        Err(DialogueError::NoPromptPending {
            choice: leave.clone()
        })
    );

    next(&asset, &mut session).expect("emits prompt");
    assert_eq!(choose(&asset, &mut session, leave.clone()), Ok(empty_end()));
    assert_eq!(
        choose(&asset, &mut session, leave.clone()),
        Err(DialogueError::NoPromptPending { choice: leave })
    );
}

#[test]
fn stale_choice_id_is_invalid_when_a_later_prompt_is_pending() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @c7w6vwk\n",
            "  First?\n",
            "  ? @first_choice\n",
            "    Continue.\n",
            "    -> second\n",
            ":: second\n",
            "> @na994pb\n",
            "  Second?\n",
            "  ? @second_choice\n",
            "    End.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("first prompt");
    assert!(matches!(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("first_choice").expect("valid choice ID"),
        ),
        Ok(DialogueEvent::Prompt { .. })
    ));

    let stale = ChoiceId::new("first_choice").expect("valid choice ID");
    let current = ChoiceId::new("second_choice").expect("valid choice ID");
    assert_eq!(
        choose(&asset, &mut session, stale.clone()),
        Err(DialogueError::InvalidChoice {
            choice: stale,
            prompt_choices: vec![current]
        })
    );
}

#[test]
fn selected_choice_history_records_choices_in_selection_order() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @s4v2m9a\n",
            "  First?\n",
            "  ? @choose_work\n",
            "    Work.\n",
            "    -> work\n",
            ":: work\n",
            "> @kkt5zh6\n",
            "  Second?\n",
            "  ? @choose_end\n",
            "    End.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    next(&asset, &mut session).expect("first prompt");
    assert!(matches!(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("choose_work").expect("valid choice ID"),
        ),
        Ok(DialogueEvent::Prompt { .. })
    ));
    assert_eq!(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("choose_end").expect("valid choice ID"),
        ),
        Ok(empty_end())
    );

    assert_eq!(
        session
            .selected_choice_history()
            .iter()
            .map(ChoiceId::as_str)
            .collect::<Vec<_>>(),
        ["choose_work", "choose_end"]
    );
}

#[test]
fn choice_selection_continuation_is_deterministic() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @wm58255\n",
            "  What next?\n",
            "  ? @rxhxjbh\n",
            "    Ask about work.\n",
            "    -> work\n",
            "  ? @naxnpdk\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> @n40bcyh\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );

    let first = run_trace(&asset, ["rxhxjbh"]);
    let second = run_trace(&asset, ["rxhxjbh"]);

    assert_eq!(first, second);
}

#[test]
fn malformed_choice_target_is_rejected_at_start_as_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @wmmgeyg\n",
            "  What next?\n",
            "  ? @nz74aj1\n",
            "    Ask about work.\n",
            "    -> END\n",
        ),
    )
    .into_payload();
    asset.choices[0].target = CompiledDivertTarget::Block(BlockIndex::new(99));
    let asset = CompiledDialogue::new(asset);
    assert!(matches!(
        start_scene(&asset, None),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn malformed_choice_target_after_start_is_structured_error_and_keeps_prompt_pending() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @wmmgeyg\n",
            "  What next?\n",
            "  ? @nz74aj1\n",
            "    Ask about work.\n",
            "    -> work\n",
            ":: work\n",
            "> @n40bcyh\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("valid asset starts");
    next(&asset, &mut session).expect("emits prompt");
    let original = asset.clone();
    let mut payload = asset.into_payload();
    payload.blocks.truncate(1);
    let asset = CompiledDialogue::new(payload);
    let ask_work = ChoiceId::new("nz74aj1").expect("valid choice ID");

    assert!(matches!(
        choose(&asset, &mut session, ask_work.clone()),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
    assert_eq!(
        next(&original, &mut session),
        Err(DialogueError::PromptPending {
            choices: vec![ask_work]
        })
    );
}
