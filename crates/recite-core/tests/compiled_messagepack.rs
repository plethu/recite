#![cfg(test)]

use recite_core::{
    BlockIndex, COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0,
    CompiledAssetDecodeError, CompiledAssetEncodeError, canonical_compiled_dialogue_fingerprint,
    decode_compiled_dialogue_messagepack, encode_compiled_dialogue_messagepack,
};

mod support;
use support::*;
#[path = "compiled_messagepack/encode_validation.rs"]
mod encode_validation;
#[path = "compiled_messagepack/interpolation.rs"]
mod interpolation;
#[path = "compiled_messagepack/shape.rs"]
mod shape;
#[path = "compiled_messagepack/validation.rs"]
mod validation;

#[test]
fn decode_rejects_unexpected_top_level_array_length() {
    let bytes = rmp_serde::to_vec(&(valid_header(), 0_u32)).expect("test wire encodes");

    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("arity is rejected");

    assert!(matches!(error, CompiledAssetDecodeError::MalformedAsset(_)));
}

#[test]
fn encode_rejects_unsupported_headers_and_invalid_dialogues() {
    let bytes = rmp_serde::to_vec(&valid_wire_asset()).expect("test wire encodes");
    let mut unsupported =
        decode_compiled_dialogue_messagepack(&bytes).expect("valid asset decodes");
    unsupported.header.format_version = COMPILED_ASSET_FORMAT_VERSION_V0 + 1;
    assert!(matches!(
        encode_compiled_dialogue_messagepack(&unsupported),
        Err(CompiledAssetEncodeError::UnsupportedFormat {
            format_version,
            compiler_compatibility_version
        }) if format_version == COMPILED_ASSET_FORMAT_VERSION_V0 + 1
            && compiler_compatibility_version == COMPILER_COMPATIBILITY_VERSION_V0
    ));

    unsupported.header.format_version = COMPILED_ASSET_FORMAT_VERSION_V0;
    unsupported.header.compiler_compatibility_version = COMPILER_COMPATIBILITY_VERSION_V0 + 1;
    assert!(matches!(
        encode_compiled_dialogue_messagepack(&unsupported),
        Err(CompiledAssetEncodeError::UnsupportedFormat {
            format_version,
            compiler_compatibility_version
        }) if format_version == COMPILED_ASSET_FORMAT_VERSION_V0
            && compiler_compatibility_version == COMPILER_COMPATIBILITY_VERSION_V0 + 1
    ));

    let mut invalid = decode_compiled_dialogue_messagepack(&bytes).expect("valid asset decodes");
    invalid.default_block = BlockIndex::new(1);
    assert!(matches!(
        encode_compiled_dialogue_messagepack(&invalid),
        Err(CompiledAssetEncodeError::InvalidDialogue(reason))
            if reason.contains("default block")
    ));
    assert!(matches!(
        canonical_compiled_dialogue_fingerprint(&invalid),
        Err(CompiledAssetEncodeError::InvalidDialogue(reason))
            if reason.contains("default block")
    ));
}

#[test]
fn decode_rejects_unknown_wire_tags() {
    let mut asset = valid_wire_asset();
    asset.header.primary_encoding = Tagged::nil(99);
    assert_malformed_asset_contains(asset, "unknown asset encoding tag 99");

    let mut asset = valid_wire_asset();
    asset.header.inspection_encoding = Tagged::nil(99);
    assert_malformed_asset_contains(asset, "unknown inspection encoding tag 99");

    let mut asset = valid_wire_asset();
    asset.header.schema_fingerprint = Tagged::nil(99);
    assert_malformed_asset_contains(asset, "unknown schema fingerprint tag 99");

    let mut asset = valid_wire_asset();
    asset.statements[0].kind = WireStatementKind::Unknown(99);
    assert_malformed_asset_contains(asset, "unknown statement kind tag 99");

    let mut asset = valid_wire_asset();
    asset.statements[0].kind = WireStatementKind::Prompt {
        line: None,
        choices: WireRange(0, 1),
    };
    asset.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        availability_requirement: None,
        availability_requirement_source_text: None,
        availability_reason_override: None,
        target: Tagged::nil(99),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    asset.availability_reasons.push(WireAvailabilityReason {
        id: "innkeeper_trust_hint",
        template: "Need more trust.",
    });
    asset.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    assert_malformed_asset_contains(asset, "unknown divert target tag 99");

    let mut asset = valid_wire_asset();
    asset.statements[0].kind = WireStatementKind::Prompt {
        line: None,
        choices: WireRange(0, 1),
    };
    asset.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        availability_requirement: None,
        availability_requirement_source_text: None,
        availability_reason_override: None,
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(99),
        source_map: 0,
    });
    asset.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    assert_malformed_asset_contains(asset, "unknown choice echo tag 99");

    let mut asset = valid_wire_asset();
    asset.effects.push(WireEffect {
        id: "effect:dialogue/main.recite:1:1",
        mode: Tagged::nil(99),
        function: "advance_thread",
        args: Vec::new(),
        source_map: 0,
    });
    assert_malformed_asset_contains(asset, "unknown effect mode tag 99");

    let mut asset = valid_wire_asset();
    asset.statements[0].kind = WireStatementKind::Prompt {
        line: None,
        choices: WireRange(0, 1),
    };
    asset.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        availability_requirement: Some(WireConditionExpression::Unknown(99)),
        availability_requirement_source_text: None,
        availability_reason_override: None,
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    asset.availability_reasons.push(WireAvailabilityReason {
        id: "innkeeper_trust_hint",
        template: "Need more trust.",
    });
    asset.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    assert_malformed_asset_contains(asset, "unknown condition expression tag 99");

    let mut asset = valid_wire_asset();
    asset.effects.push(WireEffect {
        id: "effect:dialogue/main.recite:1:1",
        mode: Tagged::nil(recite_core::V0_EFFECT_MODE_TAG_DEFERRED),
        function: "advance_thread",
        args: vec![Tagged::nil(99)],
        source_map: 0,
    });
    assert_malformed_asset_contains(asset, "unknown argument tag 99");

    let mut asset = valid_wire_asset();
    asset.metadata.push(WireMetadataEntry {
        key: "score",
        value: Tagged::nil(99),
        source_map: None,
    });
    assert_malformed_asset_contains(asset, "unknown value tag 99");

    let mut asset = valid_wire_asset();
    asset.metadata.push(WireMetadataEntry {
        key: "score",
        value: Tagged::payload(recite_core::V0_VALUE_TAG_SCALAR, Tagged::payload(99, 1.0)),
        source_map: None,
    });
    assert_malformed_asset_contains(asset, "unknown scalar value tag 99");
}

#[test]
fn decode_rejects_invalid_availability_reason_references_and_duplicates() {
    let mut missing_override = valid_wire_asset();
    missing_override.statements[0].kind = WireStatementKind::Prompt {
        line: None,
        choices: WireRange(0, 1),
    };
    missing_override.choices.push(WireChoice {
        id: "ask",
        source_text: "Ask?",
        metadata: WireRange(0, 0),
        availability_requirement: None,
        availability_requirement_source_text: None,
        availability_reason_override: Some("missing_reason"),
        target: Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
        echo: Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
        source_map: 0,
    });
    missing_override.choice_lookup.push(WireLookupEntry {
        id: "ask",
        index: 0,
    });
    assert_malformed_asset_contains(
        missing_override,
        "choice availability reason override references unknown availability reason `missing_reason`",
    );

    let mut missing_mapping_reason = valid_wire_asset();
    missing_mapping_reason
        .condition_availability_reasons
        .push(WireConditionAvailabilityReason {
            function: "trust_gte",
            reason: "missing_reason",
            args: Vec::new(),
        });
    assert_malformed_asset_contains(
        missing_mapping_reason,
        "condition availability reason mapping references unknown availability reason `missing_reason`",
    );

    let mut duplicate_reason = valid_wire_asset();
    duplicate_reason
        .availability_reasons
        .push(WireAvailabilityReason {
            id: "trust_hint",
            template: "Need trust.",
        });
    duplicate_reason
        .availability_reasons
        .push(WireAvailabilityReason {
            id: "trust_hint",
            template: "Still need trust.",
        });
    assert_malformed_asset_contains(
        duplicate_reason,
        "availability reason id `trust_hint` appears more than once",
    );

    let mut duplicate_mapping = valid_wire_asset();
    duplicate_mapping
        .availability_reasons
        .push(WireAvailabilityReason {
            id: "trust_hint",
            template: "Need trust.",
        });
    duplicate_mapping
        .condition_availability_reasons
        .push(WireConditionAvailabilityReason {
            function: "trust_gte",
            reason: "trust_hint",
            args: Vec::new(),
        });
    duplicate_mapping
        .condition_availability_reasons
        .push(WireConditionAvailabilityReason {
            function: "trust_gte",
            reason: "trust_hint",
            args: Vec::new(),
        });
    assert_malformed_asset_contains(
        duplicate_mapping,
        "condition availability reason function `trust_gte` appears more than once",
    );
}
