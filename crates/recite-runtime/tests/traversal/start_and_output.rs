use super::*;

#[test]
fn starts_at_compiled_default_block_even_when_it_is_not_first() {
    let asset = compile_asset(
        "dialogue/alpha.recite",
        concat!(
            ":: alpha\n",
            "> @alpha_line\n",
            "  Alpha.\n",
            "-> END\n",
            ":: zed default\n",
            "> @zed_line\n",
            "  Zed.\n",
            "-> END\n",
        ),
    );

    let mut session = start_scene(&asset, None).expect("starts at default block");

    assert_line(next(&asset, &mut session), "zed_line", "Zed.");
}

#[test]
fn starts_at_explicit_block_when_requested() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @zgd8ka6\n",
            "  Start.\n",
            "-> END\n",
            ":: work\n",
            "> @work_line\n",
            "  Work.\n",
            "-> END\n",
        ),
    );

    let mut session = start_scene(&asset, Some("work")).expect("starts at explicit block");

    assert_line(next(&asset, &mut session), "work_line", "Work.");
}

#[test]
fn emits_line_then_end_from_compiled_tables() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> @sf06n1h\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(next(&asset, &mut session), "sf06n1h", "Start.");
    assert_eq!(next(&asset, &mut session), Ok(empty_end()));
    assert_eq!(next(&asset, &mut session), Err(DialogueError::SessionEnded));
}

#[test]
fn line_output_uses_block_default_speaker_when_line_has_none() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=narrator\n",
            "> @wg6z5rt\n",
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
            "> speaker=hazel @vtt6dtm\n",
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
            "> mood=calm @n28f2js\n",
            "  What next?\n",
            "  ? @maf07ar\n",
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
    assert_eq!(line.id.as_str(), "n28f2js");
    assert_eq!(line.source_text, "What next?");
    assert_eq!(line.text, "What next?");
    assert_eq!(
        line.speaker.as_ref().map(|speaker| speaker.as_str()),
        Some("narrator")
    );
    assert_eq!(line.metadata[0].key, "mood");
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].id.as_str(), "maf07ar");
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
            "> @xayggh9\n",
            "  [slow]What next?[/slow]\n",
            "  ? @g6ssfsb\n",
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
