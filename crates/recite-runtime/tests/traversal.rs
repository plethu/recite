use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    BlockIndex, BlockLookupEntry, BlockLookupTable, ChoiceId, ChoiceRange, CompiledAssetId,
    CompiledConditionCall, CompiledDialogue, CompiledDivertTarget, CompiledStatementKind,
    CompilerVersion, LineIndex, MatchArmIndex, MatchArmRange, SchemaFingerprint, SourceMapId,
};
use recite_runtime::{
    DialogueError, DialogueEvent, UnsupportedStatementKind, choose, next, start_scene,
};

#[test]
fn starts_at_compiled_default_block_even_when_it_is_not_first() {
    let asset = compile_asset(
        "dialogue/alpha.recite",
        concat!(
            ":: alpha\n",
            "> alpha_line\n",
            "  Alpha.\n",
            "-> END\n",
            ":: zed default\n",
            "> zed_line\n",
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
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
            ":: work\n",
            "> work_line\n",
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
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(next(&asset, &mut session), "start_line", "Start.");
    assert_eq!(next(&asset, &mut session), Ok(DialogueEvent::End));
    assert_eq!(next(&asset, &mut session), Err(DialogueError::SessionEnded));
}

#[test]
fn line_output_uses_block_default_speaker_when_line_has_none() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=narrator\n",
            "> start_line\n",
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
            "> start_line speaker=hazel\n",
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
            "> prompt_line mood=calm\n",
            "  What next?\n",
            "  ? ask_work\n",
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
    assert_eq!(line.id.as_str(), "prompt_line");
    assert_eq!(line.source_text, "What next?");
    assert_eq!(line.text, "What next?");
    assert_eq!(
        line.speaker.as_ref().map(|speaker| speaker.as_str()),
        Some("narrator")
    );
    assert_eq!(line.metadata[0].key, "mood");
    assert_eq!(choices.len(), 1);
    assert_eq!(choices[0].id.as_str(), "ask_work");
    assert_eq!(choices[0].source_text, "Ask about work.");
    assert!(choices[0].is_available);

    assert_eq!(
        next(&asset, &mut session),
        Err(DialogueError::PromptPending {
            choices: vec![choices[0].id.clone()]
        })
    );
}

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
        Ok(DialogueEvent::End)
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
            available_choices: vec![ask_work.clone()]
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
    assert_eq!(
        choose(&asset, &mut session, leave.clone()),
        Ok(DialogueEvent::End)
    );
    assert_eq!(
        choose(&asset, &mut session, leave.clone()),
        Err(DialogueError::NoPromptPending { choice: leave })
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
        Ok(DialogueEvent::End)
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

#[test]
fn follows_diverts_to_the_target_block_before_emitting() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "-> work\n",
            ":: work\n",
            "> work_line\n",
            "  Work waits.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(next(&asset, &mut session), "work_line", "Work waits.");
}

#[test]
fn traversal_is_deterministic_for_repeated_sessions() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> first\n",
            "  First.\n",
            "-> work\n",
            ":: work\n",
            "> second\n",
            "  Second.\n",
            "-> END\n",
        ),
    );

    let first = run_to_end(&asset);
    let second = run_to_end(&asset);

    assert_eq!(first, second);
}

#[test]
fn unknown_explicit_block_is_structured_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );

    assert_eq!(
        start_scene(&asset, Some("missing")),
        Err(DialogueError::UnknownBlock {
            block: "missing".to_owned()
        })
    );
}

#[test]
fn asset_mismatch_is_structured_error() {
    let first = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
        "dialogue/first.recitec",
    );
    let second = compile_asset_with_id(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
        "dialogue/second.recitec",
    );
    let mut session = start_scene(&first, None).expect("starts");

    assert!(matches!(
        next(&second, &mut session),
        Err(DialogueError::AssetMismatch { .. })
    ));
}

#[test]
fn malformed_default_block_index_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    asset.default_block = BlockIndex::new(99);

    assert!(matches!(
        start_scene(&asset, None),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn malformed_line_index_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    asset.statements[0].kind = CompiledStatementKind::Line(LineIndex::new(99));
    let mut session = start_scene(&asset, None).expect("starts");

    assert!(matches!(
        next(&asset, &mut session),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn mismatched_explicit_block_lookup_entry_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
            ":: work\n",
            "> work_line\n",
            "  Work.\n",
            "-> END\n",
        ),
    );
    asset.block_lookup = BlockLookupTable::new(vec![
        BlockLookupEntry {
            id: asset.blocks[0].id.clone(),
            index: BlockIndex::new(0),
        },
        BlockLookupEntry {
            id: asset.blocks[1].id.clone(),
            index: BlockIndex::new(0),
        },
    ])
    .expect("lookup entries remain sorted");

    assert!(matches!(
        start_scene(&asset, Some("work")),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn prompt_with_empty_choice_range_is_structured_error() {
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
    let CompiledStatementKind::Prompt { choices, .. } = &mut asset.statements[0].kind else {
        panic!("expected prompt statement");
    };
    *choices = ChoiceRange::new(choices.start, 0);
    let mut session = start_scene(&asset, None).expect("starts");

    assert!(matches!(
        next(&asset, &mut session),
        Err(DialogueError::MalformedCompiledAsset { .. })
    ));
}

#[test]
fn unsupported_statements_are_structured_errors() {
    let if_asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            "-> END\n",
        ),
    );
    let mut if_session = start_scene(&if_asset, None).expect("starts");
    assert_eq!(
        next(&if_asset, &mut if_session),
        Err(DialogueError::UnsupportedStatement {
            kind: UnsupportedStatementKind::If
        })
    );

    let effect_asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred advance_thread(start, asked)\n",
            "-> END\n",
        ),
    );
    let mut effect_session = start_scene(&effect_asset, None).expect("starts");
    assert_eq!(
        next(&effect_asset, &mut effect_session),
        Err(DialogueError::UnsupportedStatement {
            kind: UnsupportedStatementKind::Effect
        })
    );
}

#[test]
fn unsupported_match_statement_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    asset.statements[0].kind = CompiledStatementKind::Match {
        scrutinee: CompiledConditionCall {
            function: "mood".to_owned(),
            args: Vec::new(),
        },
        arms: MatchArmRange::new(MatchArmIndex::new(0), 0),
    };
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next(&asset, &mut session),
        Err(DialogueError::UnsupportedStatement {
            kind: UnsupportedStatementKind::Match
        })
    );
}

#[test]
fn internal_divert_loop_returns_traversal_limit_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(":: start default\n", "-> start\n",),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next(&asset, &mut session),
        Err(DialogueError::TraversalLimitExceeded { limit: 10_000 })
    );
}

fn run_to_end(asset: &CompiledDialogue) -> Vec<DialogueEvent> {
    let mut session = start_scene(asset, None).expect("starts");
    let mut events = Vec::new();

    loop {
        let event = next(asset, &mut session).expect("next succeeds");
        let is_end = matches!(event, DialogueEvent::End);
        events.push(event);
        if is_end {
            break;
        }
    }

    events
}

fn run_trace<const N: usize>(
    asset: &CompiledDialogue,
    choice_ids: [&str; N],
) -> Vec<DialogueEvent> {
    let mut session = start_scene(asset, None).expect("starts");
    let mut choices = choice_ids.into_iter();
    let mut events = Vec::new();

    loop {
        let event = next(asset, &mut session).expect("next succeeds");
        let is_prompt = matches!(event, DialogueEvent::Prompt { .. });
        let is_end = matches!(event, DialogueEvent::End);
        events.push(event);

        if is_prompt {
            let choice_id = choices.next().expect("choice provided for prompt");
            let event = choose(
                asset,
                &mut session,
                ChoiceId::new(choice_id).expect("valid choice ID"),
            )
            .expect("choice succeeds");
            let is_end = matches!(event, DialogueEvent::End);
            events.push(event);
            if is_end {
                break;
            }
        } else if is_end {
            break;
        }
    }

    events
}

fn assert_line(event: Result<DialogueEvent, DialogueError>, id: &str, text: &str) {
    let DialogueEvent::Line(line) = event.expect("next succeeds") else {
        panic!("expected line event");
    };

    assert_eq!(line.id.as_str(), id);
    assert_eq!(line.source_text, text);
    assert_eq!(line.text, text);
}

fn compile_asset(path: &str, source: &str) -> CompiledDialogue {
    compile_asset_with_id(path, source, "dialogue/main.recitec")
}

fn compile_asset_with_id(path: &str, source: &str, asset_id: &str) -> CompiledDialogue {
    let report = compile_inputs(
        [CompileInput::new(path, source)],
        CompileOptions::new(
            CompilerVersion::new("0.0.1").expect("valid compiler version"),
            CompiledAssetId::new(asset_id).expect("valid asset id"),
            SourceMapId::new("dialogue/main.recitec.map").expect("valid source map id"),
            SchemaFingerprint::NoSchema,
        ),
    )
    .expect("compile does not hard fail");

    assert!(
        report.diagnostics.is_empty(),
        "test source should compile without diagnostics: {:?}",
        report.diagnostics
    );

    report.asset.expect("asset emitted").dialogue
}
