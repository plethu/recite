use super::*;

#[test]
fn chooses_pending_prompt_option_by_stable_choice_id() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? ask_work\n",
            "    Ask about work.\n",
            "    -> work\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> work_line\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } = next(&asset, &mut session).expect("emits prompt")
    else {
        panic!("expected prompt");
    };
    assert_eq!(choices[0].id.as_str(), "ask_work");
    assert_eq!(choices[1].id.as_str(), "leave");

    assert_eq!(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("leave").expect("valid choice ID")
        ),
        Ok(empty_end())
    );
    assert_eq!(
        session
            .selected_choice_history()
            .iter()
            .map(ChoiceId::as_str)
            .collect::<Vec<_>>(),
        ["leave"]
    );
}

#[test]
fn choosing_choice_target_continues_from_target_block() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? ask_work\n",
            "    Ask about work.\n",
            "    -> work\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> work_line\n",
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
            ChoiceId::new("ask_work").expect("valid choice ID"),
        ),
        "work_line",
        "Work waits.",
    );
}

#[test]
fn invalid_choice_for_pending_prompt_is_structured_error_and_keeps_prompt_pending() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? ask_work\n",
            "    Ask about work.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    let missing = ChoiceId::new("missing").expect("valid choice ID");
    let ask_work = ChoiceId::new("ask_work").expect("valid choice ID");

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
            "> prompt_line\n",
            "  What next?\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    let leave = ChoiceId::new("leave").expect("valid choice ID");

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
            "> first_prompt\n",
            "  First?\n",
            "  ? first_choice\n",
            "    Continue.\n",
            "    -> second\n",
            ":: second\n",
            "> second_prompt\n",
            "  Second?\n",
            "  ? second_choice\n",
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
            "> first_prompt\n",
            "  First?\n",
            "  ? choose_work\n",
            "    Work.\n",
            "    -> work\n",
            ":: work\n",
            "> second_prompt\n",
            "  Second?\n",
            "  ? choose_end\n",
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
            "> prompt_line\n",
            "  What next?\n",
            "  ? ask_work\n",
            "    Ask about work.\n",
            "    -> work\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> work_line\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );

    let first = run_trace(&asset, ["ask_work"]);
    let second = run_trace(&asset, ["ask_work"]);

    assert_eq!(first, second);
}

#[test]
fn malformed_choice_target_is_structured_error_and_keeps_prompt_pending() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? ask_work\n",
            "    Ask about work.\n",
            "    -> END\n",
        ),
    );
    asset.choices[0].target = CompiledDivertTarget::Block(BlockIndex::new(99));
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    let ask_work = ChoiceId::new("ask_work").expect("valid choice ID");

    assert!(matches!(
        choose(&asset, &mut session, ask_work.clone()),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
    assert_eq!(
        next(&asset, &mut session),
        Err(DialogueError::PromptPending {
            choices: vec![ask_work]
        })
    );
}
