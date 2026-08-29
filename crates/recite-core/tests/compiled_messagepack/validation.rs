use recite_core::{CompiledAssetDecodeError, decode_compiled_dialogue_messagepack};

use super::support::*;

#[test]
fn decode_rejects_choice_echo_referencing_unknown_line() {
    let mut asset = valid_wire_asset();
    asset.lines.push(WireLine {
        id: "known_line",
        source_text: "Known.",
        speaker: None,
        metadata: WireRange(0, 0),
        source_map: 0,
    });
    asset.line_lookup.push(WireLookupEntry {
        id: "known_line",
        index: 0,
    });
    asset.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        availability_requirement: None,
        availability_requirement_source_text: None,
        availability_reason_override: None,
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::payload(
            recite_core::V0_CHOICE_ECHO_TAG_EXPLICIT_LINE,
            "missing_line",
        ),
        source_map: 0,
    });
    asset.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("unknown echo line rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message)
            if message.contains("choice echo")
                && message.contains("missing_line")
    ));
}

#[test]
fn decode_rejects_source_map_file_mismatch() {
    let mut asset = valid_wire_asset();
    asset.source_maps[0].span.file = "dialogue/other.recite";
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("source map mismatch rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message)
            if message.contains("source map")
                && message.contains("dialogue/other.recite")
                && message.contains("dialogue/main.recite")
    ));
}

#[test]
fn decode_rejects_source_span_end_before_start() {
    let mut asset = valid_wire_asset();
    asset.source_maps[0].span.start_line = 2;
    asset.source_maps[0].span.start_column = 1;
    asset.source_maps[0].span.end_line = Some(1);
    asset.source_maps[0].span.end_column = Some(1);
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("reversed source span rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message)
            if message.contains("span end precedes span start")
    ));
}

#[test]
fn decode_rejects_non_finite_float_values() {
    let mut asset = valid_wire_asset();
    asset.metadata.push(WireMetadataEntry {
        key: "score",
        value: Tagged::payload(
            recite_core::V0_VALUE_TAG_SCALAR,
            Tagged::payload(recite_core::V0_SCALAR_TAG_FLOAT, f64::NAN),
        ),
        source_map: None,
    });
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("non-finite float rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message)
            if message.contains("float scalar")
                && message.contains("finite")
    ));
}

#[test]
fn decode_rejects_non_finite_availability_reason_float_literals() {
    let mut asset = valid_wire_asset();
    asset.availability_reasons.push(WireAvailabilityReason {
        id: "blocked",
        template: "{weight}",
    });
    asset
        .condition_availability_reasons
        .push(WireConditionAvailabilityReason {
            function: "can_answer",
            reason: "blocked",
            args: vec![WireAvailabilityReasonArgBinding {
                name: "weight",
                value: WireAvailabilityReasonArgValue::LiteralFloat(f64::NAN),
            }],
        });

    assert_malformed_asset_contains(asset, "availability reason float literal must be finite");
}

#[test]
fn decode_rejects_invalid_compiled_names() {
    let mut metadata = valid_wire_asset();
    metadata.metadata.push(WireMetadataEntry {
        key: "",
        value: Tagged::payload(
            recite_core::V0_VALUE_TAG_SCALAR,
            Tagged::payload(recite_core::V0_SCALAR_TAG_FLOAT, 1.0),
        ),
        source_map: None,
    });
    assert_malformed_asset_contains(metadata, "metadata key");

    let mut effect = valid_wire_asset();
    effect.effects.push(WireEffect {
        id: "fx",
        mode: Tagged::nil(recite_core::V0_EFFECT_MODE_TAG_DEFERRED),
        function: "bad function",
        args: Vec::new(),
        source_map: 0,
    });
    assert_malformed_asset_contains(effect, "effect function");

    let mut condition = valid_wire_asset();
    condition.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        availability_requirement: Some(WireConditionExpression::Call(WireConditionCall {
            function: "",
            args: Vec::new(),
        })),
        availability_requirement_source_text: None,
        availability_reason_override: None,
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    condition.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    assert_malformed_asset_contains(condition, "condition function");

    let mut argument = valid_wire_asset();
    argument.effects.push(WireEffect {
        id: "fx",
        mode: Tagged::nil(recite_core::V0_EFFECT_MODE_TAG_DEFERRED),
        function: "advance_thread",
        args: vec![Tagged::payload(
            recite_core::V0_ARGUMENT_TAG_IDENTIFIER,
            "bad argument",
        )],
        source_map: 0,
    });
    assert_malformed_asset_contains(argument, "argument identifier");
}

#[test]
fn decode_rejects_duplicate_source_paths_and_effect_ids() {
    let mut source = valid_wire_asset();
    source.sources.push(WireSourceFile {
        path: "dialogue/main.recite",
        fingerprint: valid_fingerprint(),
    });
    assert_malformed_asset_contains(source, "source file path");

    let mut effect = valid_wire_asset();
    effect.effects.push(WireEffect {
        id: "fx",
        mode: Tagged::nil(recite_core::V0_EFFECT_MODE_TAG_DEFERRED),
        function: "advance_thread",
        args: Vec::new(),
        source_map: 0,
    });
    effect.effects.push(WireEffect {
        id: "fx",
        mode: Tagged::nil(recite_core::V0_EFFECT_MODE_TAG_IMMEDIATE),
        function: "advance_thread",
        args: Vec::new(),
        source_map: 0,
    });
    assert_malformed_asset_contains(effect, "effect id");
}

#[test]
fn decode_rejects_duplicate_line_and_choice_ids() {
    let mut asset = valid_wire_asset();
    asset.lines.push(WireLine {
        id: "shared_id",
        source_text: "Line.",
        speaker: None,
        metadata: WireRange(0, 0),
        source_map: 0,
    });
    asset.line_lookup.push(WireLookupEntry {
        id: "shared_id",
        index: 0,
    });
    asset.choices.push(WireChoice {
        id: "shared_id",
        source_text: "Choice?",
        metadata: WireRange(0, 0),
        availability_requirement: None,
        availability_requirement_source_text: None,
        availability_reason_override: None,
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    asset.choice_lookup.push(WireLookupEntry {
        id: "shared_id",
        index: 0,
    });

    assert_malformed_asset_contains(asset, "line and choice ids");
}

#[test]
fn decode_rejects_empty_prompt_choices_and_condition_groups() {
    let mut prompt = valid_wire_asset();
    prompt.statements[0].kind = WireStatementKind::Prompt {
        line: None,
        choices: WireRange(0, 0),
    };
    assert_malformed_asset_contains(prompt, "prompt choices");

    let mut and_group = valid_wire_asset();
    and_group.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        availability_requirement: Some(WireConditionExpression::EmptyAnd),
        availability_requirement_source_text: None,
        availability_reason_override: None,
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    and_group.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    assert_malformed_asset_contains(and_group, "condition and group");

    let mut or_group = valid_wire_asset();
    or_group.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        availability_requirement: Some(WireConditionExpression::EmptyOr),
        availability_requirement_source_text: None,
        availability_reason_override: None,
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    or_group.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    assert_malformed_asset_contains(or_group, "condition or group");
}
