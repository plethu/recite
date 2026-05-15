use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    BlockIndex, BlockLookupEntry, BlockLookupTable, ChoiceId, ChoiceRange, CompiledAssetId,
    CompiledConditionCall, CompiledConditionExpression, CompiledDialogue, CompiledDivertTarget,
    CompiledStatementKind, CompilerVersion, EffectIndex, LineIndex, MatchArmIndex, MatchArmRange,
    SchemaFingerprint, SourceMapId,
};
use recite_runtime::{
    ConditionArgument, ConditionEvaluationError, ConditionQuery, DialogueEffectArgument,
    DialogueEffectMode, DialogueEffectRequest, DialogueError, DialogueEvent, EmptyDialogueContext,
    UnsupportedStatementKind, choose as runtime_choose, next as runtime_next, start_scene,
};
use std::cell::RefCell;
use std::collections::BTreeMap;

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
    assert_eq!(next(&asset, &mut session), Ok(empty_end()));
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
fn true_condition_enters_then_branch_and_resumes_parent_range() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> before\n",
            "  Before.\n",
            ":if trusts(player, \"hazel rhea\", 3, 0.75, true)\n",
            "  > secret\n",
            "    Secret.\n",
            ":else\n",
            "  > fallback\n",
            "    Fallback.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "before",
        "Before.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "secret",
        "Secret.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "after",
        "After.",
    );
    assert_eq!(
        next_with_context(&asset, &mut session, &context),
        Ok(empty_end())
    );
    assert_eq!(
        context.calls(),
        [RecordedCall {
            function: "trusts".to_owned(),
            arguments: vec![
                RecordedArgument::Identifier("player".to_owned()),
                RecordedArgument::String("hazel rhea".to_owned()),
                RecordedArgument::Integer(3),
                RecordedArgument::Float(0.75),
                RecordedArgument::Boolean(true),
            ],
        }]
    );
}

#[test]
fn false_condition_enters_else_branch() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            ":else\n",
            "  > fallback\n",
            "    Fallback.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "fallback",
        "Fallback.",
    );
    assert_line(
        next_with_context(&asset, &mut session, &context),
        "after",
        "After.",
    );
}

#[test]
fn false_condition_without_else_skips_gated_statements() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "after",
        "After.",
    );
}

#[test]
fn not_condition_inverts_context_result() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if not trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(
        next_with_context(&asset, &mut session, &context),
        "secret",
        "Secret.",
    );
}

#[test]
fn condition_failure_is_structured_and_keeps_session_position() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            "-> END\n",
        ),
    );
    let failing = RecordingContext::default().failing("trusts", "condition is unavailable");
    let passing = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next_with_context(&asset, &mut session, &failing),
        Err(DialogueError::ConditionEvaluationFailed {
            function: "trusts".to_owned(),
            reason: "condition is unavailable".to_owned(),
        })
    );
    assert_line(
        next_with_context(&asset, &mut session, &passing),
        "secret",
        "Secret.",
    );
}

#[test]
fn deeply_nested_condition_returns_structured_depth_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            ":if trusts(player)\n",
            "  > secret\n",
            "    Secret.\n",
            "-> END\n",
        ),
    );
    let CompiledStatementKind::If { condition, .. } = &mut asset.statements[0].kind else {
        panic!("expected if statement");
    };
    *condition = deeply_nested_condition(150);
    let context = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next_with_context(&asset, &mut session, &context),
        Err(DialogueError::ConditionDepthLimitExceeded { limit: 128 })
    );
    assert!(
        context.calls().is_empty(),
        "runtime should stop before reaching the deeply nested call"
    );
}

#[test]
fn boolean_conditions_short_circuit_left_to_right() {
    let and_asset = compile_asset(
        "dialogue/and.recite",
        concat!(
            ":: start default\n",
            ":if first() and missing()\n",
            "  > secret\n",
            "    Secret.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let and_context = RecordingContext::default().with("first", false);
    let mut and_session = start_scene(&and_asset, None).expect("starts");

    assert_line(
        next_with_context(&and_asset, &mut and_session, &and_context),
        "after",
        "After.",
    );
    assert_eq!(
        and_context.calls(),
        [RecordedCall {
            function: "first".to_owned(),
            arguments: Vec::new(),
        }]
    );

    let or_asset = compile_asset(
        "dialogue/or.recite",
        concat!(
            ":: start default\n",
            ":if first() or missing()\n",
            "  > secret\n",
            "    Secret.\n",
            "> after\n",
            "  After.\n",
            "-> END\n",
        ),
    );
    let or_context = RecordingContext::default().with("first", true);
    let mut or_session = start_scene(&or_asset, None).expect("starts");

    assert_line(
        next_with_context(&or_asset, &mut or_session, &or_context),
        "secret",
        "Secret.",
    );
    assert_eq!(
        or_context.calls(),
        [RecordedCall {
            function: "first".to_owned(),
            arguments: Vec::new(),
        }]
    );
}

#[test]
fn choice_condition_failure_keeps_session_position() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? locked if trusts(player)\n",
            "    Locked.\n",
            "    -> END\n",
        ),
    );
    let failing = RecordingContext::default().failing("trusts", "condition is unavailable");
    let passing = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    assert_eq!(
        next_with_context(&asset, &mut session, &failing),
        Err(DialogueError::ConditionEvaluationFailed {
            function: "trusts".to_owned(),
            reason: "condition is unavailable".to_owned(),
        })
    );

    let DialogueEvent::Prompt { choices, .. } =
        next_with_context(&asset, &mut session, &passing).expect("emits prompt")
    else {
        panic!("expected prompt event");
    };
    assert_eq!(choices[0].id.as_str(), "locked");
    assert!(choices[0].is_available);
}

#[test]
fn choice_conditions_mark_unavailable_choices_without_hiding_them() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? locked if trusts(player)\n",
            "    Locked.\n",
            "    -> locked\n",
            "  ? leave\n",
            "    Leave.\n",
            "    -> END\n",
            ":: locked\n",
            "> locked_line\n",
            "  Locked path.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", false);
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } =
        next_with_context(&asset, &mut session, &context).expect("emits prompt")
    else {
        panic!("expected prompt event");
    };
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].id.as_str(), "locked");
    assert!(!choices[0].is_available);
    assert_eq!(choices[0].unavailable_reason, None);
    assert_eq!(choices[1].id.as_str(), "leave");
    assert!(choices[1].is_available);

    let locked = ChoiceId::new("locked").expect("valid choice ID");
    assert_eq!(
        choose_with_context(&asset, &mut session, locked.clone(), &context),
        Err(DialogueError::UnavailableChoice {
            choice: locked,
            reason: None,
        })
    );
    assert_eq!(
        choose_with_context(
            &asset,
            &mut session,
            ChoiceId::new("leave").expect("valid choice ID"),
            &context,
        ),
        Ok(empty_end())
    );
}

#[test]
fn available_choice_condition_can_be_selected() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? locked if trusts(player)\n",
            "    Locked.\n",
            "    -> locked\n",
            ":: locked\n",
            "> locked_line\n",
            "  Locked path.\n",
            "-> END\n",
        ),
    );
    let context = RecordingContext::default().with("trusts", true);
    let mut session = start_scene(&asset, None).expect("starts");

    let DialogueEvent::Prompt { choices, .. } =
        next_with_context(&asset, &mut session, &context).expect("emits prompt")
    else {
        panic!("expected prompt event");
    };
    assert!(choices[0].is_available);

    assert_line(
        choose_with_context(
            &asset,
            &mut session,
            ChoiceId::new("locked").expect("valid choice ID"),
            &context,
        ),
        "locked_line",
        "Locked path.",
    );
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
fn malformed_effect_index_is_structured_error() {
    let mut asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    asset.statements[0].kind = CompiledStatementKind::Effect(EffectIndex::new(99));
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

#[derive(Debug, Default)]
struct RecordingContext {
    results: BTreeMap<String, bool>,
    failures: BTreeMap<String, String>,
    calls: RefCell<Vec<RecordedCall>>,
}

impl RecordingContext {
    fn with(mut self, function: &str, result: bool) -> Self {
        self.results.insert(function.to_owned(), result);
        self
    }

    fn failing(mut self, function: &str, reason: &str) -> Self {
        self.failures.insert(function.to_owned(), reason.to_owned());
        self
    }

    fn calls(&self) -> Vec<RecordedCall> {
        self.calls.borrow().clone()
    }
}

impl recite_runtime::DialogueContext for RecordingContext {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<bool, ConditionEvaluationError> {
        let function = query.function().to_owned();
        let arguments = query
            .arguments()
            .into_iter()
            .map(RecordedArgument::from)
            .collect();
        self.calls.borrow_mut().push(RecordedCall {
            function: function.clone(),
            arguments,
        });

        if let Some(reason) = self.failures.get(&function) {
            return Err(ConditionEvaluationError::new(reason.clone()));
        }

        self.results
            .get(&function)
            .copied()
            .ok_or_else(|| ConditionEvaluationError::new(format!("missing condition `{function}`")))
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RecordedCall {
    function: String,
    arguments: Vec<RecordedArgument>,
}

#[derive(Clone, Debug, PartialEq)]
enum RecordedArgument {
    Identifier(String),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl From<ConditionArgument<'_>> for RecordedArgument {
    fn from(argument: ConditionArgument<'_>) -> Self {
        match argument {
            ConditionArgument::Identifier(value) => Self::Identifier(value.to_owned()),
            ConditionArgument::String(value) => Self::String(value.to_owned()),
            ConditionArgument::Integer(value) => Self::Integer(value),
            ConditionArgument::Float(value) => Self::Float(value),
            ConditionArgument::Boolean(value) => Self::Boolean(value),
        }
    }
}

fn deeply_nested_condition(depth: usize) -> CompiledConditionExpression {
    let mut expression = CompiledConditionExpression::Call(CompiledConditionCall {
        function: "trusts".to_owned(),
        args: Vec::new(),
    });

    for _ in 0..depth {
        expression = CompiledConditionExpression::Not(Box::new(expression));
    }

    expression
}

fn run_to_end(asset: &CompiledDialogue) -> Vec<DialogueEvent> {
    let mut session = start_scene(asset, None).expect("starts");
    let mut events = Vec::new();

    loop {
        let event = next(asset, &mut session).expect("next succeeds");
        let is_end = matches!(event, DialogueEvent::End { .. });
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
        let is_end = matches!(event, DialogueEvent::End { .. });
        events.push(event);

        if is_prompt {
            let choice_id = choices.next().expect("choice provided for prompt");
            let event = choose(
                asset,
                &mut session,
                ChoiceId::new(choice_id).expect("valid choice ID"),
            )
            .expect("choice succeeds");
            let is_end = matches!(event, DialogueEvent::End { .. });
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

fn next(
    asset: &CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
) -> Result<DialogueEvent, DialogueError> {
    runtime_next(asset, session, &EmptyDialogueContext)
}

fn next_with_context(
    asset: &CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
    context: &dyn recite_runtime::DialogueContext,
) -> Result<DialogueEvent, DialogueError> {
    runtime_next(asset, session, context)
}

fn choose(
    asset: &CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
    choice_id: ChoiceId,
) -> Result<DialogueEvent, DialogueError> {
    runtime_choose(asset, session, choice_id, &EmptyDialogueContext)
}

fn choose_with_context(
    asset: &CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
    choice_id: ChoiceId,
    context: &dyn recite_runtime::DialogueContext,
) -> Result<DialogueEvent, DialogueError> {
    runtime_choose(asset, session, choice_id, context)
}

fn assert_line(event: Result<DialogueEvent, DialogueError>, id: &str, text: &str) {
    let DialogueEvent::Line(line) = event.expect("next succeeds") else {
        panic!("expected line event");
    };

    assert_eq!(line.id.as_str(), id);
    assert_eq!(line.source_text, text);
    assert_eq!(line.text, text);
}

fn empty_end() -> DialogueEvent {
    DialogueEvent::End {
        deferred_effects: Vec::new(),
    }
}

fn assert_end_effects<const N: usize>(
    event: Result<DialogueEvent, DialogueError>,
    expected_functions: [&str; N],
) -> Vec<DialogueEffectRequest> {
    let DialogueEvent::End { deferred_effects } = event.expect("next succeeds") else {
        panic!("expected end event");
    };

    assert_eq!(
        deferred_effects
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        expected_functions
    );

    deferred_effects
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
