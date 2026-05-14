use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    BlockIndex, CompiledAssetId, CompiledConditionCall, CompiledDialogue, CompiledStatementKind,
    CompilerVersion, LineIndex, MatchArmIndex, MatchArmRange, SchemaFingerprint, SourceMapId,
};
use recite_runtime::{DialogueError, DialogueEvent, UnsupportedStatementKind, next, start_scene};

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
