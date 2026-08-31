use recite_core::{
    AvailabilityReasonId, ChoiceId, EffectId, LineId, MetadataEntry, ScalarValue, SourcePosition,
    SourceSpan, Value,
};
use recite_runtime::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonOrigin,
    ChoiceAvailabilityReasonTree, ChoiceAvailabilityReasonValue, ChoiceEchoMode, DialogueChoice,
    DialogueEffectMode, DialogueEffectRequest, DialogueLine, DialoguePlural,
    DialoguePluralResolution, EffectAck, PreviewConditionRequestId, PreviewError, PreviewEvent,
};

use super::hash_event;

fn digest(event: &PreviewEvent) -> String {
    let mut hasher = blake3::Hasher::new();
    hash_event(event, &mut hasher);
    hasher.finalize().to_hex().to_string()
}

fn span(column: u32) -> SourceSpan {
    SourceSpan::point(
        "fixture.recite",
        SourcePosition::new(2, column).expect("test span positions are valid"),
    )
}

#[test]
fn metadata_values_are_part_of_line_evidence() {
    let mut line = DialogueLine {
        id: LineId::new("line").expect("test ID is valid"),
        source_text: "source".to_owned(),
        text: "text".to_owned(),
        speaker: None,
        metadata: vec![MetadataEntry::new(
            "tone",
            Value::Scalar(ScalarValue::String("quiet".to_owned())),
        )],
        plural: None,
    };
    let first = digest(&PreviewEvent::Line(line.clone()));
    line.metadata[0].value = Value::Scalar(ScalarValue::String("loud".to_owned()));
    assert_ne!(first, digest(&PreviewEvent::Line(line)));
}

#[test]
fn effect_source_spans_are_part_of_effect_evidence() {
    let mut effect = DialogueEffectRequest {
        id: EffectId::new("set_flag").expect("test ID is valid"),
        mode: DialogueEffectMode::Immediate,
        function: "set_flag".to_owned(),
        args: Vec::new(),
        source_span: span(1),
    };
    let first = digest(&PreviewEvent::EffectRequested(effect.clone()));
    effect.source_span = span(4);
    assert_ne!(first, digest(&PreviewEvent::EffectRequested(effect)));
}

#[test]
fn plural_provenance_is_part_of_line_evidence() {
    let mut line = DialogueLine {
        id: LineId::new("line").expect("test ID is valid"),
        source_text: "one".to_owned(),
        text: "one".to_owned(),
        speaker: None,
        metadata: Vec::new(),
        plural: Some(DialoguePlural {
            singular_source_text: "one".to_owned(),
            plural_source_text: "many".to_owned(),
            count: 2,
            selected_arm: 1,
            resolution: DialoguePluralResolution {
                attempts: Vec::new(),
                matched_locale: Some("en".to_owned()),
                matched_context: Some("dialogue".to_owned()),
                matched_key: Some("line".to_owned()),
                matched_arm: Some(1),
                source_fallback_arm: None,
                outcome: recite_runtime::DialoguePluralResolutionOutcome::Translated,
            },
        }),
    };
    let first = digest(&PreviewEvent::Line(line.clone()));
    line.plural
        .as_mut()
        .expect("test plural exists")
        .resolution
        .matched_key = Some("changed".to_owned());
    assert_ne!(first, digest(&PreviewEvent::Line(line)));
}

#[test]
fn error_payloads_are_part_of_error_evidence() {
    let first = PreviewEvent::Error(PreviewError::ConditionFailed {
        request_id: PreviewConditionRequestId::new(1),
        reason: "first".to_owned(),
    });
    let second = PreviewEvent::Error(PreviewError::ConditionFailed {
        request_id: PreviewConditionRequestId::new(1),
        reason: "second".to_owned(),
    });
    assert_ne!(digest(&first), digest(&second));
}

#[test]
fn acknowledgement_payloads_are_part_of_ack_evidence() {
    let effect_id = EffectId::new("set_flag").expect("test ID is valid");
    let first = PreviewEvent::EffectAcknowledged {
        effect_id: effect_id.clone(),
        ack: EffectAck::Completed,
    };
    let second = PreviewEvent::EffectAcknowledged {
        effect_id,
        ack: EffectAck::Failed {
            reason: "failed".to_owned(),
        },
    };
    assert_ne!(digest(&first), digest(&second));
}

#[test]
fn choice_reason_tree_and_echo_are_part_of_choice_evidence() {
    let mut choice = DialogueChoice {
        id: ChoiceId::new("choice").expect("test ID is valid"),
        source_text: "choose".to_owned(),
        text: "Choose".to_owned(),
        metadata: Vec::new(),
        availability: ChoiceAvailability {
            is_available: false,
            primary_reason: Some(ChoiceAvailabilityReason {
                id: AvailabilityReasonId::new("locked").expect("test ID is valid"),
                source_text: "locked".to_owned(),
                text: "Not ready".to_owned(),
                origin: Some(ChoiceAvailabilityReasonOrigin::ConditionCall {
                    function: "has_key".to_owned(),
                    args: vec![ChoiceAvailabilityReasonValue::Boolean(false)],
                }),
                args: Vec::new(),
            }),
            reason_tree: Some(ChoiceAvailabilityReasonTree::RequirementSourceText(
                "has_key".to_owned(),
            )),
        },
        echo: ChoiceEchoMode::None,
    };
    let mut first_hasher = blake3::Hasher::new();
    crate::preview_hash_dialogue::hash_choice(&mut first_hasher, &choice);
    let first = first_hasher.finalize();
    choice.echo = ChoiceEchoMode::SelectedText;
    choice.availability.reason_tree = Some(ChoiceAvailabilityReasonTree::RequirementSourceText(
        "changed".to_owned(),
    ));
    let mut second_hasher = blake3::Hasher::new();
    crate::preview_hash_dialogue::hash_choice(&mut second_hasher, &choice);
    assert_ne!(first, second_hasher.finalize());
}
