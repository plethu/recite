use super::*;

#[test]
fn starts_at_compiled_default_block_even_when_it_is_not_first() {
    let asset = compile_asset(
        "dialogue/alpha.recite",
        concat!(
            ":: alpha\n",
            "> alpha_line@c8f2347f7bb1df8fe28e\n",
            "  Alpha.\n",
            "-> END\n",
            ":: zed default\n",
            "> zed_line@23c0aa68401ee705c0d4\n",
            "  Zed.\n",
            "-> END\n",
        ),
    );

    let mut session = start_scene(&asset, None).expect("starts at default block");

    assert_line(next(&asset, &mut session), "23c0aa68401ee705c0d4", "Zed.");
}

#[test]
fn starts_at_explicit_block_when_requested() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@52d569297c0a800d68da\n",
            "  Start.\n",
            "-> END\n",
            ":: work\n",
            "> work_line@fc3d2c9deb6cb8546183\n",
            "  Work.\n",
            "-> END\n",
        ),
    );

    let mut session = start_scene(&asset, Some("work")).expect("starts at explicit block");

    assert_line(next(&asset, &mut session), "fc3d2c9deb6cb8546183", "Work.");
}

#[test]
fn emits_line_then_end_from_compiled_tables() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line@ec205a5da70ffbc8dddc\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(next(&asset, &mut session), "ec205a5da70ffbc8dddc", "Start.");
    assert_eq!(next(&asset, &mut session), Ok(empty_end()));
    assert_eq!(next(&asset, &mut session), Err(DialogueError::SessionEnded));
}

#[test]
fn line_output_uses_block_default_speaker_when_line_has_none() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=narrator\n",
            "> start_line@fe66123fff4dd0143b77\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Line(line) = next(&asset, &mut session).expect("emits line") else {
        panic!("expected line event");
    };

    assert_eq!(
        line.speaker.as_ref().map(|speaker| speaker.as_str()),
        Some("narrator")
    );
}

#[test]
fn explicit_line_speaker_overrides_block_default_speaker() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=narrator\n",
            "> start_line@d153a6453d140f44f071 speaker=hazel\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Line(line) = next(&asset, &mut session).expect("emits line") else {
        panic!("expected line event");
    };

    assert_eq!(
        line.speaker.as_ref().map(|speaker| speaker.as_str()),
        Some("hazel")
    );
}

#[test]
fn emits_prompt_with_stable_choice_ids_and_waits_for_selection() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=narrator\n",
            "> prompt_line@0a52fc2a3c597b3685bf mood=calm\n",
            "  What next?\n",
            "  ? ask_work@4e4d44c2ac2daf8545b4\n",
            "    Ask about work.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let event = next(&asset, &mut session).expect("emits prompt");
    let DialogueEvent::Prompt { line, choices } = event else {
        panic!("expected prompt event");
    };
    let line = line.expect("prompt line is present");
    assert_eq!(line.id.as_str(), "0a52fc2a3c597b3685bf");
    assert_eq!(line.source_text, "What next?");
    assert_eq!(line.text, "What next?");
    assert_eq!(
        line.speaker.as_ref().map(|speaker| speaker.as_str()),
        Some("narrator")
    );
    assert_eq!(line.metadata[0].key, "mood");
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].id.as_str(), "4e4d44c2ac2daf8545b4");
    assert_eq!(choices[0].source_text, "Ask about work.");
    assert!(choices[0].availability.is_available);

    assert_eq!(
        next(&asset, &mut session),
        Err(DialogueError::PromptPending {
            choices: vec![choices[0].id.clone()]
        })
    );
}

#[test]
fn runtime_preserves_inline_markup_without_interpreting_it() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line@de3bb2d6b361a684704c\n",
            "  [slow]What next?[/slow]\n",
            "  ? ask_work@63120a11616ddd9296fd\n",
            "    [shake]Ask about work.[/shake]\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    let event = next(&asset, &mut session).expect("emits prompt");
    let DialogueEvent::Prompt { line, choices } = event else {
        panic!("expected prompt event");
    };
    let line = line.expect("prompt line is present");
    assert_eq!(line.source_text, "[slow]What next?[/slow]");
    assert_eq!(line.text, "[slow]What next?[/slow]");
    assert_eq!(choices[0].source_text, "[shake]Ask about work.[/shake]");
    assert_eq!(choices[0].text, "[shake]Ask about work.[/shake]");
}
