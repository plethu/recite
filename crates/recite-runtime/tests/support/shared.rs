use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId, CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
};
use recite_runtime::{
    DialogueEffectRequest, DialogueError, DialogueEvent, EmptyDialogueContext,
    choose as runtime_choose, next as runtime_next,
};

pub(super) fn next(
    asset: &CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
) -> Result<DialogueEvent, DialogueError> {
    runtime_next(asset, session, &EmptyDialogueContext)
}

pub(super) fn choose(
    asset: &CompiledDialogue,
    session: &mut recite_runtime::DialogueSession,
    choice_id: ChoiceId,
) -> Result<DialogueEvent, DialogueError> {
    runtime_choose(asset, session, choice_id, &EmptyDialogueContext)
}

pub(super) fn assert_line(event: Result<DialogueEvent, DialogueError>, id: &str, text: &str) {
    let DialogueEvent::Line(line) = event.expect("next succeeds") else {
        panic!("expected line event");
    };

    assert_eq!(line.id.as_str(), id);
    assert_eq!(line.source_text, text);
    assert_eq!(line.text, text);
}

pub(super) fn empty_end() -> DialogueEvent {
    DialogueEvent::End {
        deferred_effects: Vec::new(),
    }
}

pub(super) fn assert_end_effects<const N: usize>(
    event: Result<DialogueEvent, DialogueError>,
    expected_functions: [&str; N],
) -> Vec<DialogueEffectRequest> {
    let DialogueEvent::End { deferred_effects } = event.expect("next succeeds") else {
        panic!("expected end event");
    };

    assert_effect_functions(&deferred_effects, expected_functions);
    deferred_effects
}

pub(super) fn assert_effect_functions<const N: usize>(
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

pub(super) fn compile_asset(path: &str, source: &str) -> CompiledDialogue {
    compile_asset_with_id(path, source, "dialogue/main.recitec")
}

pub(super) fn compile_asset_with_id(path: &str, source: &str, asset_id: &str) -> CompiledDialogue {
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
