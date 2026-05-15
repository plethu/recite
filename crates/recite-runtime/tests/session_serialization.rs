use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId, CompiledAssetId, CompiledDialogue, CompilerVersion, LocaleId, SchemaFingerprint,
    SourceMapId,
};
use recite_runtime::{
    DialogueEffectArgument, DialogueEffectRequest, DialogueError, DialogueEvent,
    DialogueSessionOptions, EmptyDialogueContext, choose as runtime_choose,
    decode_session_messagepack, encode_session_messagepack, next as runtime_next, restore_session,
    snapshot_session, start_scene, start_scene_with_options,
};

#[test]
fn messagepack_round_trip_resumes_line_progress_without_asset_payload() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> first\n",
            "  First.\n",
            "> second\n",
            "  Second.\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");

    assert_line(next(&asset, &mut session), "first", "First.");
    let bytes = encode_session_messagepack(&session).expect("encodes session");
    let mut restored =
        decode_session_messagepack(&asset, &bytes).expect("restores from messagepack");

    assert_line(next(&asset, &mut restored), "second", "Second.");
    assert_eq!(next(&asset, &mut restored), Ok(empty_end()));
}

#[test]
fn structured_snapshot_records_locale_and_compact_runtime_location() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let locale = LocaleId::new("en-GB").expect("valid locale");
    let mut session = start_scene_with_options(
        &asset,
        None,
        DialogueSessionOptions::new().with_locale(locale.clone()),
    )
    .expect("starts with options");
    assert_line(next(&asset, &mut session), "start_line", "Start.");

    let snapshot = snapshot_session(&session);
    assert_eq!(snapshot.asset_id, "dialogue/main.recitec");
    assert_eq!(snapshot.current_block, 0);
    assert_eq!(snapshot.current_range.start, 0);
    assert_eq!(snapshot.locale.as_deref(), Some("en-GB"));
    assert!(snapshot.pending_prompt.is_none());
    assert!(snapshot.deferred_effects.is_empty());

    let restored = restore_session(&asset, snapshot).expect("restores snapshot");
    assert_eq!(restored.locale(), Some(&locale));
}

#[test]
fn restores_pending_prompt_and_selects_choice_using_matching_asset() {
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
    assert_eq!(
        choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["ask_work", "leave"]
    );

    let snapshot = snapshot_session(&session);
    assert_eq!(
        snapshot
            .pending_prompt
            .as_ref()
            .expect("pending prompt is serialized")
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["ask_work", "leave"]
    );

    let mut restored = restore_session(&asset, snapshot).expect("restores pending prompt");
    assert_line(
        choose(
            &asset,
            &mut restored,
            ChoiceId::new("ask_work").expect("valid choice ID"),
        ),
        "work_line",
        "Work waits.",
    );
}

#[test]
fn restores_selected_choice_history_after_choice_continuation() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? work\n",
            "    Work.\n",
            "    -> work\n",
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
            ChoiceId::new("work").expect("valid choice ID"),
        ),
        "work_line",
        "Work waits.",
    );

    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores after choice");
    assert_eq!(
        restored
            .selected_choice_history()
            .iter()
            .map(ChoiceId::as_str)
            .collect::<Vec<_>>(),
        ["work"]
    );
    assert_eq!(next(&asset, &mut restored), Ok(empty_end()));
}

#[test]
fn restores_deferred_effects_collected_before_save_and_continues_in_order() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "! deferred entered_start(alpha, \"beta\", 3, 0.5, true)\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? work\n",
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
fn restores_end_state_without_replaying_scene() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "! deferred finished()\n",
            "-> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    assert_line(next(&asset, &mut session), "start_line", "Start.");
    assert_end_effects(next(&asset, &mut session), ["finished"]);

    let mut restored =
        restore_session(&asset, snapshot_session(&session)).expect("restores ended state");
    assert_eq!(
        next(&asset, &mut restored),
        Err(DialogueError::SessionEnded)
    );
    assert_effect_functions(restored.deferred_effects(), ["finished"]);
}

#[test]
fn mismatched_asset_identity_returns_structured_error() {
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
    let session = start_scene(&first, None).expect("starts");

    assert!(matches!(
        restore_session(&second, snapshot_session(&session)),
        Err(DialogueError::AssetMismatch { .. })
    ));
}

#[test]
fn mismatched_asset_version_returns_structured_error() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> start_line\n",
            "  Start.\n",
            "-> END\n",
        ),
    );
    let session = start_scene(&asset, None).expect("starts");
    let mut snapshot = snapshot_session(&session);
    snapshot.asset_format_version = 99;

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::AssetMismatch {
            expected_format_version: 99,
            actual_format_version: 0,
            ..
        })
    ));
}

#[test]
fn malformed_pending_prompt_snapshot_is_rejected_before_choice_selection() {
    let asset = compile_asset(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> prompt_line\n",
            "  What next?\n",
            "  ? work\n",
            "    Work.\n",
            "    -> END\n",
        ),
    );
    let mut session = start_scene(&asset, None).expect("starts");
    next(&asset, &mut session).expect("emits prompt");
    let mut snapshot = snapshot_session(&session);
    snapshot
        .pending_prompt
        .as_mut()
        .expect("pending prompt")
        .choices[0]
        .id = "forged_choice".to_owned();
    snapshot.previous_prompt_choices[0] = "forged_choice".to_owned();

    assert!(matches!(
        restore_session(&asset, snapshot),
        Err(DialogueError::InvalidSessionSnapshot { .. })
    ));
}

fn next(
    asset: &CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
) -> Result<DialogueEvent, DialogueError> {
    runtime_next(asset, session, &EmptyDialogueContext)
}

fn choose(
    asset: &CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
    choice_id: ChoiceId,
) -> Result<DialogueEvent, DialogueError> {
    runtime_choose(asset, session, choice_id, &EmptyDialogueContext)
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

    assert_effect_functions(&deferred_effects, expected_functions);
    deferred_effects
}

fn assert_effect_functions<const N: usize>(
    effects: &[DialogueEffectRequest],
    expected_functions: [&str; N],
) {
    assert_eq!(
        effects
            .iter()
            .map(|effect| effect.function.as_str())
            .collect::<Vec<_>>(),
        expected_functions
    );
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
