use recite_compiler::compile::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ScalarValue, Value,
    compiled::{
        CompiledAssetId, CompiledDialogue, CompilerVersion, SchemaFingerprint, SourceMapId,
    },
};
use recite_runtime::{
    ChoiceEchoMode, DialogueEvent, DialogueSession, EmptyDialogueContext, LocaleResolution,
    localisation::InterpolationValues, next_with, start_scene,
};

const OPERATIONS: usize = 64;
const SOURCE: &str = include_str!("../../../../fixtures/recite/valid/language_pressure.recite");

pub(super) struct BoundText {
    pub(super) asset: CompiledDialogue,
    pub(super) sessions: Vec<DialogueSession>,
    pub(super) values: InterpolationValues,
    pub(super) events: Vec<DialogueEvent>,
    expected_event: DialogueEvent,
    expected_session: DialogueSession,
}

pub(super) fn authored() -> BoundText {
    let report = compile_inputs(
        [CompileInput::new(
            "fixtures/recite/valid/language_pressure.recite",
            SOURCE,
        )],
        CompileOptions::new(
            CompilerVersion::new("0.0.1")
                .unwrap_or_else(|error| panic!("valid compiler version: {error}")),
            CompiledAssetId::new("bound-text")
                .unwrap_or_else(|error| panic!("valid asset ID: {error}")),
            SourceMapId::new("bound-text-map")
                .unwrap_or_else(|error| panic!("valid source map ID: {error}")),
            SchemaFingerprint::NoSchema,
        ),
    )
    .unwrap_or_else(|error| panic!("bound-text fixture compiles: {error}"));
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let asset = report
        .asset
        .unwrap_or_else(|| panic!("bound-text fixture emits an asset"))
        .dialogue;
    let session = start_scene(&asset, None)
        .unwrap_or_else(|error| panic!("bound-text fixture starts: {error}"));
    let mut values = InterpolationValues::new();
    values.insert("traveller_name".to_owned(), ScalarValue::from("Élise 🦊"));
    values.insert("topic".to_owned(), ScalarValue::from("la marée"));
    let mut expected_session = session.clone();
    let expected_event = next_with(
        &asset,
        &mut expected_session,
        &EmptyDialogueContext,
        LocaleResolution::new().with_values(&values),
    )
    .unwrap_or_else(|error| panic!("bound-text fixture emits a prompt: {error}"));
    validate_prompt(&expected_event);
    BoundText {
        asset,
        sessions: vec![session; OPERATIONS],
        values,
        events: Vec::with_capacity(OPERATIONS),
        expected_event,
        expected_session,
    }
}

fn validate_prompt(event: &DialogueEvent) {
    let DialogueEvent::Prompt { line, choices } = event else {
        panic!("bound-text fixture must emit a prompt");
    };
    let line = line
        .as_ref()
        .unwrap_or_else(|| panic!("bound-text prompt retains its line"));
    assert_eq!(line.id.as_str(), "55667788990011223344");
    assert_eq!(
        line.source_text,
        "[slow]The tide is turning, {traveller_name}.[/slow]"
    );
    assert_eq!(line.text, "[slow]The tide is turning, Élise 🦊.[/slow]");
    assert_eq!(line.speaker.as_ref().map(|id| id.as_str()), Some("élise"));
    assert!(line.plural.is_none());
    assert_eq!(line.metadata.len(), 2);
    for (entry, expected) in line.metadata.iter().zip(["calm", "urgent"]) {
        assert_eq!(entry.key, "mood");
        assert_eq!(entry.value, Value::Scalar(ScalarValue::from(expected)));
        assert!(entry.source_span.is_some());
        assert!(entry.key_span.is_none());
        assert!(entry.value_span.is_none());
    }
    assert_eq!(choices.len(), 1);
    let choice = &choices[0];
    assert_eq!(choice.id.as_str(), "66778899001122334455");
    assert_eq!(choice.source_text, "Tell me about {topic}.");
    assert_eq!(choice.text, "Tell me about la marée.");
    assert_eq!(choice.echo, ChoiceEchoMode::None);
    assert!(choice.availability.is_available);
    assert!(choice.availability.primary_reason.is_none());
    assert!(choice.availability.reason_tree.is_none());
    assert_eq!(choice.metadata.len(), 1);
    assert_eq!(choice.metadata[0].key, "tone");
    assert_eq!(
        choice.metadata[0].value,
        Value::Scalar(ScalarValue::from("plain"))
    );
    assert!(choice.metadata[0].source_span.is_some());
    assert!(choice.metadata[0].key_span.is_none());
    assert!(choice.metadata[0].value_span.is_none());
}

pub(super) fn validate(workload: BoundText) {
    assert_eq!(workload.sessions.len(), OPERATIONS);
    assert_eq!(workload.events.len(), OPERATIONS);
    for (session, event) in workload.sessions.iter().zip(&workload.events) {
        assert_eq!(event, &workload.expected_event);
        assert_eq!(session, &workload.expected_session);
    }
}
