use super::support::*;
use recite_core::{
    AvailabilityReasonId, ChoiceId, ChoiceIndex, ChoiceLookupEntry, ChoiceLookupTable,
    CompiledArgument, CompiledAssetEncodeError, CompiledAvailabilityReason,
    CompiledAvailabilityReasonArgBinding, CompiledAvailabilityReasonArgValue, CompiledChoice,
    CompiledChoiceEcho, CompiledConditionCall, CompiledConditionExpression, CompiledDivertTarget,
    CompiledEffect, CompiledEffectMode, CompiledInterpolationMode, CompiledLine,
    CompiledMetadataEntry, CompiledStatement, CompiledStatementKind, LineId, LineIndex,
    LineLookupEntry, LineLookupTable, MetadataIndex, ScalarValue, SourceMapIndex, SourcePosition,
    TableRange, Value, canonical_compiled_dialogue_fingerprint,
    decode_compiled_dialogue_messagepack, encode_compiled_dialogue_messagepack,
};

#[test]
fn mutable_models_are_checked_by_encode_and_fingerprint() {
    let valid = decode_valid();

    let mut metadata = valid.clone();
    metadata.metadata.push(CompiledMetadataEntry {
        key: "score".to_owned(),
        value: Value::Scalar(ScalarValue::Float(f64::NAN)),
        source_map: None,
    });
    assert_rejected(metadata, "float scalar");

    let mut span = valid.clone();
    span.source_maps[0].span.start = SourcePosition::new(2, 1).expect("valid position");
    span.source_maps[0].span.end = Some(SourcePosition::new(1, 1).expect("valid position"));
    assert_rejected(span, "span end precedes span start");

    let mut condition = valid.clone();
    condition.blocks[0].statements.len = 2;
    condition.statements.push(CompiledStatement {
        kind: CompiledStatementKind::If {
            condition: CompiledConditionExpression::Call(CompiledConditionCall {
                function: "bad function".to_owned(),
                args: Vec::new(),
            }),
            then_statements: TableRange::new(recite_core::StatementIndex::new(0), 0),
            else_statements: TableRange::new(recite_core::StatementIndex::new(0), 0),
        },
        source_map: SourceMapIndex::new(0),
    });
    assert_rejected(condition, "condition function");

    let mut effect = valid.clone();
    effect.effects.push(CompiledEffect {
        id: recite_core::EffectId::new("effect").expect("valid effect id"),
        mode: CompiledEffectMode::Deferred,
        function: "bad function".to_owned(),
        args: Vec::new(),
        source_map: SourceMapIndex::new(0),
    });
    assert_rejected(effect, "effect function");

    let mut argument = valid.clone();
    argument.effects.push(CompiledEffect {
        id: recite_core::EffectId::new("effect").expect("valid effect id"),
        mode: CompiledEffectMode::Deferred,
        function: "advance_thread".to_owned(),
        args: vec![CompiledArgument::Identifier("bad argument".to_owned())],
        source_map: SourceMapIndex::new(0),
    });
    assert_rejected(argument, "argument identifier");

    let mut reason = valid.clone();
    reason
        .availability_reasons
        .push(CompiledAvailabilityReason {
            id: AvailabilityReasonId::new("weight_reason").expect("valid reason id"),
            template: "Weight is {weight}.".to_owned(),
        });
    reason
        .condition_availability_reasons
        .push(recite_core::CompiledConditionAvailabilityReason {
            function: "can_answer".to_owned(),
            reason: AvailabilityReasonId::new("weight_reason").expect("valid reason id"),
            args: vec![CompiledAvailabilityReasonArgBinding {
                name: "weight".to_owned(),
                value: CompiledAvailabilityReasonArgValue::Literal(ScalarValue::Float(f64::NAN)),
            }],
        });
    assert_rejected(reason, "availability reason float literal");

    let mut mapping = decode_valid();
    mapping
        .availability_reasons
        .push(CompiledAvailabilityReason {
            id: AvailabilityReasonId::new("reason").expect("valid reason id"),
            template: "Reason.".to_owned(),
        });
    mapping
        .condition_availability_reasons
        .push(recite_core::CompiledConditionAvailabilityReason {
            function: String::new(),
            reason: AvailabilityReasonId::new("reason").expect("valid reason id"),
            args: Vec::new(),
        });
    assert_rejected(mapping, "condition availability reason function");

    let mut binding = decode_valid();
    binding
        .availability_reasons
        .push(CompiledAvailabilityReason {
            id: AvailabilityReasonId::new("reason").expect("valid reason id"),
            template: "Reason.".to_owned(),
        });
    binding
        .condition_availability_reasons
        .push(recite_core::CompiledConditionAvailabilityReason {
            function: "can_answer".to_owned(),
            reason: AvailabilityReasonId::new("reason").expect("valid reason id"),
            args: vec![CompiledAvailabilityReasonArgBinding {
                name: String::new(),
                value: CompiledAvailabilityReasonArgValue::ConditionArg(0),
            }],
        });
    assert_rejected(binding, "availability reason argument name");

    let mut empty_metadata = decode_valid();
    empty_metadata.metadata.push(CompiledMetadataEntry {
        key: String::new(),
        value: Value::Scalar(ScalarValue::Boolean(true)),
        source_map: None,
    });
    assert_rejected(empty_metadata, "metadata key");
}

#[test]
fn canonical_encoding_checks_choice_and_legacy_interpolation_rows() {
    let mut choice = decode_valid();
    choice.choices.push(CompiledChoice {
        id: ChoiceId::new("choice").expect("valid choice id"),
        source_text: "Choose {name}.".to_owned(),
        authored_source_text: "Choose.".to_owned(),
        interpolation_bindings: Vec::new(),
        interpolation_mode: CompiledInterpolationMode::Current,
        metadata: TableRange::new(MetadataIndex::new(0), 0),
        availability_requirement: None,
        availability_requirement_source_text: None,
        availability_reason_override: None,
        target: CompiledDivertTarget::End,
        echo: CompiledChoiceEcho::None,
        source_map: SourceMapIndex::new(0),
    });
    choice.choice_lookup = ChoiceLookupTable::new(vec![ChoiceLookupEntry {
        id: ChoiceId::new("choice").expect("valid choice id"),
        index: ChoiceIndex::new(0),
    }])
    .expect("sorted lookup");
    assert_rejected(choice, "compiled interpolation source text");

    let mut legacy = decode_valid();
    legacy.lines.push(CompiledLine {
        id: LineId::new("legacy").expect("valid line id"),
        source_text: "{missing}".to_owned(),
        plural_source_text: None,
        authored_source_text: "{missing}".to_owned(),
        authored_plural_source_text: None,
        interpolation_bindings: Vec::new(),
        interpolation_mode: CompiledInterpolationMode::Legacy,
        speaker: None,
        metadata: TableRange::new(MetadataIndex::new(0), 0),
        source_map: SourceMapIndex::new(0),
    });
    legacy.line_lookup = LineLookupTable::new(vec![LineLookupEntry {
        id: LineId::new("legacy").expect("valid line id"),
        index: LineIndex::new(0),
    }])
    .expect("sorted lookup");
    assert_rejected(legacy, "placeholder `missing` has no interpolation binding");
}

fn decode_valid() -> recite_core::CompiledDialogue {
    let bytes = rmp_serde::to_vec(&valid_wire_asset()).expect("test wire encodes");
    decode_compiled_dialogue_messagepack(&bytes).expect("valid asset decodes")
}

fn assert_rejected(dialogue: recite_core::CompiledDialogue, expected: &str) {
    assert!(matches!(
        encode_compiled_dialogue_messagepack(&dialogue),
        Err(CompiledAssetEncodeError::InvalidDialogue(reason)) if reason.contains(expected)
    ));
    assert!(matches!(
        canonical_compiled_dialogue_fingerprint(&dialogue),
        Err(CompiledAssetEncodeError::InvalidDialogue(reason)) if reason.contains(expected)
    ));
}
