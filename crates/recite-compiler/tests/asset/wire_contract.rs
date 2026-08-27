//! Shape-level conformance checks for the v0 compiled MessagePack document.
//!
//! The typed round-trip test exercises the encoder and decoder together, while
//! the golden snapshot protects the selected fixture bytes. This check reads
//! the compiler output as an untyped MessagePack value and checks every fixed
//! array against the arity registry exported by `recite-core`. A simultaneous
//! change to both codec mirrors therefore still has to account for the shared
//! v0 shape before the typed round-trip can pass.

use recite_core::{
    V0_ASSET_HEADER_FIELDS, V0_AVAILABILITY_REASON_ARG_BINDING_FIELDS,
    V0_AVAILABILITY_REASON_FIELDS, V0_BLOCK_FIELDS, V0_CHOICE_FIELDS, V0_COMPILED_DIALOGUE_FIELDS,
    V0_CONDITION_AVAILABILITY_REASON_FIELDS, V0_CONDITION_CALL_FIELDS, V0_EFFECT_FIELDS,
    V0_FINGERPRINT_FIELDS, V0_IF_STATEMENT_PAYLOAD_FIELDS, V0_LINE_FIELDS, V0_LOOKUP_ENTRY_FIELDS,
    V0_MATCH_ARM_FIELDS, V0_MATCH_STATEMENT_PAYLOAD_FIELDS, V0_METADATA_ENTRY_FIELDS,
    V0_PROMPT_STATEMENT_PAYLOAD_FIELDS, V0_RANGE_FIELDS, V0_SOURCE_FILE_FIELDS,
    V0_SOURCE_MAP_ENTRY_FIELDS, V0_SOURCE_SPAN_FIELDS, V0_SPEAKER_FIELDS, V0_STATEMENT_FIELDS,
    V0_TAGGED_VALUE_FIELDS,
};
use serde_value::Value as WireValue;

use super::tag_surface::compile_schema_tag_surface_asset;

#[test]
fn compiler_output_matches_the_v0_fixed_array_shape() {
    let asset = compile_schema_tag_surface_asset();
    let wire: WireValue = rmp_serde::from_slice(&asset.messagepack)
        .expect("compiler output decodes as an untyped MessagePack value");
    assert_dialogue_shape(&wire);

    // The schema fixture covers the row and enum families. The separate
    // value fixture adds the array-valued metadata shape and all scalar tags.
    let value_asset = super::tag_surface::compile_value_tag_surface_asset();
    let value_wire: WireValue = rmp_serde::from_slice(&value_asset.messagepack)
        .expect("value-tag output decodes as an untyped MessagePack value");
    assert_dialogue_shape(&value_wire);

    let literal_reason_asset = super::tag_surface::compile_literal_reason_tag_surface_asset();
    let literal_reason_wire: WireValue = rmp_serde::from_slice(&literal_reason_asset.messagepack)
        .expect("literal-reason output decodes as an untyped MessagePack value");
    assert_dialogue_shape(&literal_reason_wire);
    assert_literal_reason_tags(&literal_reason_wire);
}

fn assert_literal_reason_tags(value: &WireValue) {
    let dialogue = tuple(value, V0_COMPILED_DIALOGUE_FIELDS, "CompiledDialogue");
    let mapping = table(&dialogue[9], "condition availability reasons")
        .first()
        .expect("literal-reason fixture emits one condition mapping");
    let mapping = tuple(
        mapping,
        V0_CONDITION_AVAILABILITY_REASON_FIELDS,
        "CompiledConditionAvailabilityReason",
    );
    let bindings = table(&mapping[2], "literal-reason arguments");
    let tags = bindings
        .iter()
        .map(|binding| {
            let binding = tuple(
                binding,
                V0_AVAILABILITY_REASON_ARG_BINDING_FIELDS,
                "CompiledAvailabilityReasonArgBinding",
            );
            let tagged = tuple(&binding[1], V0_TAGGED_VALUE_FIELDS, "reason argument value");
            match &tagged[0] {
                WireValue::String(tag) => tag.as_str(),
                other => panic!("reason argument tag must be a string, got {other:?}"),
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        tags,
        ["LiteralBool", "LiteralFloat", "LiteralInt", "LiteralString"]
    );
}

fn assert_dialogue_shape(value: &WireValue) {
    let dialogue = tuple(value, V0_COMPILED_DIALOGUE_FIELDS, "CompiledDialogue");
    let header = tuple(&dialogue[0], V0_ASSET_HEADER_FIELDS, "CompiledAssetHeader");
    let (_, schema_payload) = tagged(&header[7], "schema fingerprint");
    if !is_nil(schema_payload) {
        fingerprint(schema_payload);
    }

    for source in table(&dialogue[2], "sources") {
        let source = tuple(source, V0_SOURCE_FILE_FIELDS, "CompiledSourceFile");
        fingerprint(&source[1]);
    }
    for block in table(&dialogue[3], "blocks") {
        let block = tuple(block, V0_BLOCK_FIELDS, "CompiledBlock");
        range(&block[2], "block statements");
        range(&block[3], "block metadata");
    }
    for statement in table(&dialogue[4], "statements") {
        let statement = tuple(statement, V0_STATEMENT_FIELDS, "CompiledStatement");
        statement_kind(&statement[0]);
    }
    for arm in table(&dialogue[5], "match_arms") {
        let arm = tuple(arm, V0_MATCH_ARM_FIELDS, "CompiledMatchArm");
        tagged(&arm[0], "match pattern");
        range(&arm[1], "match arm statements");
    }
    for line in table(&dialogue[6], "lines") {
        let line = tuple(line, V0_LINE_FIELDS, "CompiledLine");
        range(&line[3], "line metadata");
    }
    for choice in table(&dialogue[7], "choices") {
        let choice = tuple(choice, V0_CHOICE_FIELDS, "CompiledChoice");
        range(&choice[2], "choice metadata");
        if !is_nil(&choice[3]) {
            condition(&choice[3]);
        }
        tagged(&choice[6], "choice target");
        tagged(&choice[7], "choice echo");
    }
    for reason in table(&dialogue[8], "availability_reasons") {
        tuple(
            reason,
            V0_AVAILABILITY_REASON_FIELDS,
            "CompiledAvailabilityReason",
        );
    }
    for mapping in table(&dialogue[9], "condition_availability_reasons") {
        let mapping = tuple(
            mapping,
            V0_CONDITION_AVAILABILITY_REASON_FIELDS,
            "CompiledConditionAvailabilityReason",
        );
        for binding in table(&mapping[2], "availability reason args") {
            let binding = tuple(
                binding,
                V0_AVAILABILITY_REASON_ARG_BINDING_FIELDS,
                "CompiledAvailabilityReasonArgBinding",
            );
            tagged(&binding[1], "availability reason argument value");
        }
    }
    for speaker in table(&dialogue[10], "speakers") {
        tuple(speaker, V0_SPEAKER_FIELDS, "CompiledSpeaker");
    }
    for metadata in table(&dialogue[11], "metadata") {
        let metadata = tuple(metadata, V0_METADATA_ENTRY_FIELDS, "CompiledMetadataEntry");
        value_wire(&metadata[1]);
    }
    for effect in table(&dialogue[12], "effects") {
        let effect = tuple(effect, V0_EFFECT_FIELDS, "CompiledEffect");
        tagged(&effect[1], "effect mode");
        for argument_value in table(&effect[3], "effect args") {
            argument(argument_value);
        }
    }
    for source_map in table(&dialogue[13], "source_maps") {
        let source_map = tuple(
            source_map,
            V0_SOURCE_MAP_ENTRY_FIELDS,
            "CompiledSourceMapEntry",
        );
        tuple(&source_map[1], V0_SOURCE_SPAN_FIELDS, "SourceSpan");
    }
    for (name, lookup) in [
        ("block_lookup", &dialogue[14]),
        ("line_lookup", &dialogue[15]),
        ("choice_lookup", &dialogue[16]),
    ] {
        for entry in table(lookup, name) {
            tuple(entry, V0_LOOKUP_ENTRY_FIELDS, "lookup entry");
        }
    }
}

fn statement_kind(value: &WireValue) {
    let (tag, payload) = tagged(value, "statement kind");
    match integer(tag, "statement kind tag") {
        0 | 5 | 6 => {}
        1 => {
            let payload = tuple(
                payload,
                V0_PROMPT_STATEMENT_PAYLOAD_FIELDS,
                "prompt statement payload",
            );
            range(&payload[1], "prompt choices");
        }
        2 => {
            tagged(payload, "divert target");
        }
        3 => {
            let payload = tuple(
                payload,
                V0_IF_STATEMENT_PAYLOAD_FIELDS,
                "if statement payload",
            );
            condition(&payload[0]);
            range(&payload[1], "if then statements");
            range(&payload[2], "if else statements");
        }
        4 => {
            let payload = tuple(
                payload,
                V0_MATCH_STATEMENT_PAYLOAD_FIELDS,
                "match statement payload",
            );
            condition_call(&payload[0]);
            range(&payload[1], "match arms");
        }
        other => panic!("unexpected statement tag in tag-surface fixture: {other}"),
    }
}

fn condition(value: &WireValue) {
    let (tag, payload) = tagged(value, "condition expression");
    match integer(tag, "condition expression tag") {
        0 => condition_call(payload),
        1 | 2 => {
            for expression in table(payload, "condition expression group") {
                condition(expression);
            }
        }
        3 => condition(payload),
        other => panic!("unexpected condition tag in tag-surface fixture: {other}"),
    }
}

fn condition_call(value: &WireValue) {
    let call = tuple(value, V0_CONDITION_CALL_FIELDS, "condition call");
    for argument_value in table(&call[1], "condition args") {
        argument(argument_value);
    }
}

fn argument(value: &WireValue) {
    let (tag, payload) = tagged(value, "argument");
    match integer(tag, "argument tag") {
        0 => {}
        1 => scalar(payload),
        other => panic!("unexpected argument tag in tag-surface fixture: {other}"),
    }
}

fn value_wire(value: &WireValue) {
    let (tag, payload) = tagged(value, "value");
    match integer(tag, "value tag") {
        0 => scalar(payload),
        1 => {
            for scalar_value in table(payload, "array value") {
                scalar(scalar_value);
            }
        }
        other => panic!("unexpected value tag in value fixture: {other}"),
    }
}

fn scalar(value: &WireValue) {
    let (tag, _) = tagged(value, "scalar value");
    match integer(tag, "scalar value tag") {
        0..=3 => {}
        other => panic!("unexpected scalar tag in value fixture: {other}"),
    }
}

fn fingerprint(value: &WireValue) {
    tuple(value, V0_FINGERPRINT_FIELDS, "ContentFingerprint");
}

fn range(value: &WireValue, label: &str) {
    tuple(value, V0_RANGE_FIELDS, label);
}

fn tagged<'a>(value: &'a WireValue, label: &str) -> (&'a WireValue, &'a WireValue) {
    let fields = tuple(value, V0_TAGGED_VALUE_FIELDS, label);
    (&fields[0], &fields[1])
}

fn table<'a>(value: &'a WireValue, label: &str) -> &'a [WireValue] {
    match value {
        WireValue::Seq(values) => values,
        other => panic!("{label} must be a MessagePack array, got {other:?}"),
    }
}

fn tuple<'a>(value: &'a WireValue, expected: u8, label: &str) -> &'a [WireValue] {
    let values = table(value, label);
    assert_eq!(
        values.len(),
        expected as usize,
        "{label} has the wrong fixed MessagePack array arity"
    );
    values
}

fn integer(value: &WireValue, label: &str) -> u64 {
    match value {
        WireValue::U8(value) => u64::from(*value),
        WireValue::U16(value) => u64::from(*value),
        WireValue::U32(value) => u64::from(*value),
        WireValue::U64(value) => *value,
        WireValue::I8(value) => (*value).try_into().expect(label),
        WireValue::I16(value) => (*value).try_into().expect(label),
        WireValue::I32(value) => (*value).try_into().expect(label),
        WireValue::I64(value) => (*value).try_into().expect(label),
        other => panic!("{label} must be an integer, got {other:?}"),
    }
}

fn is_nil(value: &WireValue) -> bool {
    matches!(value, WireValue::Unit | WireValue::Option(None))
}
