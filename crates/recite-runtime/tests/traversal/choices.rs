use super::*;

#[test]
fn chooses_pending_prompt_option_by_stable_choice_id() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@e916af2ad24c6d76430d\n",
            "  What next?\n",
            "  ? ask_work@87b6a20a2d7c0dd4762b\n",
            "    Ask about work.\n",
            "    -> work\n",
            "  ? leave@66081ca34fe3be53894b\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> work_line@7bc4d66869d29685546f\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } = next(&asset, &mut session).expect("emits prompt")
    else {
        panic!("expected prompt");
    };
    assert_eq!(choices[0].id.as_str(), "87b6a20a2d7c0dd4762b");
    assert_eq!(choices[1].id.as_str(), "66081ca34fe3be53894b");

    assert_eq!(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("66081ca34fe3be53894b").expect("valid choice ID")
        ),
        Ok(empty_end())
    );
    assert_eq!(
        session
            .selected_choice_history()
            .iter()
            .map(ChoiceId::as_str)
            .collect::<Vec<_>>(),
        ["66081ca34fe3be53894b"]
    );
}

#[test]
fn choosing_choice_target_continues_from_target_block() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@76f6d971c2a7cc48b495\n",
            "  What next?\n",
            "  ? ask_work@f64263406018e104aafe\n",
            "    Ask about work.\n",
            "    -> work\n",
            "  ? leave@b154453e8c982be17492\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> work_line@b590b05ff40eaa24a03f\n",
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
            ChoiceId::new("f64263406018e104aafe").expect("valid choice ID"),
        ),
        "b590b05ff40eaa24a03f",
        "Work waits.",
    );
}

#[test]
fn invalid_choice_for_pending_prompt_is_structured_error_and_keeps_prompt_pending() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@cb7b860e8dfc1ff333c6\n",
            "  What next?\n",
            "  ? ask_work@045f1b5dedec2e254a39\n",
            "    Ask about work.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    let missing = ChoiceId::new("missing").expect("valid choice ID");
    let ask_work = ChoiceId::new("045f1b5dedec2e254a39").expect("valid choice ID");

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
            "> prompt_line@4c0b7daed3e779e7a839\n",
            "  What next?\n",
            "  ? leave@a1fddf2211ed214305f7\n",
            "    Leave.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    let leave = ChoiceId::new("a1fddf2211ed214305f7").expect("valid choice ID");

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
            "> first_prompt@c6f9f7c0f46b248cd8df\n",
            "  First?\n",
            "  ? first_choice@38e0570be871d66e4cbf\n",
            "    Continue.\n",
            "    -> second\n",
            ":: second\n",
            "> second_prompt@1d7ad5c3882327431fc9\n",
            "  Second?\n",
            "  ? second_choice@3a9ef657b925d44f1d3c\n",
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
            ChoiceId::new("38e0570be871d66e4cbf").expect("valid choice ID"),
        ),
        Ok(DialogueEvent::Prompt { .. })
    ));

    let stale = ChoiceId::new("38e0570be871d66e4cbf").expect("valid choice ID");
    let current = ChoiceId::new("3a9ef657b925d44f1d3c").expect("valid choice ID");
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
            "> first_prompt@2d8dd9cc74938709ff9d\n",
            "  First?\n",
            "  ? choose_work@81dbc24ae994accbe505\n",
            "    Work.\n",
            "    -> work\n",
            ":: work\n",
            "> second_prompt@53c658a2e0f994882c82\n",
            "  Second?\n",
            "  ? choose_end@7443039202cdec1772bf\n",
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
            ChoiceId::new("81dbc24ae994accbe505").expect("valid choice ID"),
        ),
        Ok(DialogueEvent::Prompt { .. })
    ));
    assert_eq!(
        choose(
            &asset,
            &mut session,
            ChoiceId::new("7443039202cdec1772bf").expect("valid choice ID"),
        ),
        Ok(empty_end())
    );

    assert_eq!(
        session
            .selected_choice_history()
            .iter()
            .map(ChoiceId::as_str)
            .collect::<Vec<_>>(),
        ["81dbc24ae994accbe505", "7443039202cdec1772bf"]
    );
}

#[test]
fn choice_selection_continuation_is_deterministic() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@c4437d1cda42714fa62e\n",
            "  What next?\n",
            "  ? ask_work@1f03a4a76838d337fa1d\n",
            "    Ask about work.\n",
            "    -> work\n",
            "  ? leave@4450c0c3f2c7f49e53f2\n",
            "    Leave.\n",
            "    -> END\n",
            ":: work\n",
            "> work_line@c6fec4a314d3e1c36a53\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );

    let first = run_trace(&asset, ["1f03a4a76838d337fa1d"]);
    let second = run_trace(&asset, ["1f03a4a76838d337fa1d"]);

    assert_eq!(first, second);
}

#[test]
fn malformed_choice_target_is_structured_error_and_keeps_prompt_pending() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@b310fd8aa3baf5cdecce\n",
            "  What next?\n",
            "  ? ask_work@f0d4d54acca265cffc88\n",
            "    Ask about work.\n",
            "    -> END\n",
        ),
    );
    asset.choices[0].target = CompiledDivertTarget::Block(BlockIndex::new(99));
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    let ask_work = ChoiceId::new("f0d4d54acca265cffc88").expect("valid choice ID");

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
