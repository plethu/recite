use recite_core::{
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, CompiledAssetDecodeError,
    decode_compiled_dialogue_messagepack,
};
use serde_bytes::Bytes;

use super::support::*;

#[test]
fn decode_reports_unsupported_format_before_v0_body_validation() {
    let mut asset = valid_wire_asset();
    asset.header.format_version = COMPILED_ASSET_FORMAT_VERSION_V0 + 1;
    asset.header.primary_encoding = Tagged::nil(99);
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("unsupported format rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::UnsupportedFormat {
            format_version,
            compiler_compatibility_version
        } if format_version == COMPILED_ASSET_FORMAT_VERSION_V0 + 1
            && compiler_compatibility_version == COMPILER_COMPATIBILITY_VERSION_V0
    ));
}

#[test]
fn decode_reports_unsupported_format_before_future_body_shape_validation() {
    let bytes = rmp_serde::to_vec(&((
        COMPILED_ASSET_FORMAT_VERSION_V0 + 1,
        COMPILER_COMPATIBILITY_VERSION_V0,
    ),))
    .expect("future-shaped test wire encodes");

    let error =
        decode_compiled_dialogue_messagepack(&bytes).expect_err("unsupported format rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::UnsupportedFormat {
            format_version,
            compiler_compatibility_version
        } if format_version == COMPILED_ASSET_FORMAT_VERSION_V0 + 1
            && compiler_compatibility_version == COMPILER_COMPATIBILITY_VERSION_V0
    ));
}

#[test]
fn decode_rejects_invalid_fingerprint_digest_length() {
    let mut asset = valid_wire_asset();
    asset.header.schema_fingerprint = Tagged::payload(
        recite_core::V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT,
        WireFingerprint {
            algorithm: "blake3",
            digest: Bytes::new(&SHORT_DIGEST),
        },
    );
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("short digest is rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("blake3 fingerprint digest must be 32 bytes")
    ));
}

#[test]
fn decode_rejects_trailing_messagepack_bytes() {
    let mut bytes = rmp_serde::to_vec(&valid_wire_asset()).expect("test wire encodes");
    bytes.extend_from_slice(&[0xc0]);

    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("trailing bytes rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("trailing bytes")
    ));
}

#[test]
fn decode_preserves_choice_availability_reason_override() {
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
        availability_reason_override: Some("innkeeper_trust_hint"),
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
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");

    let decoded = decode_compiled_dialogue_messagepack(&bytes).expect("valid asset decodes");

    assert_eq!(
        decoded.choices[0]
            .availability_reason_override
            .as_ref()
            .map(recite_core::AvailabilityReasonId::as_str),
        Some("innkeeper_trust_hint")
    );
}

#[test]
fn decode_rejects_unsorted_or_duplicate_lookup_entries() {
    let mut unsorted = valid_wire_asset();
    unsorted.blocks.push(WireBlock {
        id: "alpha",
        source_file: 0,
        statements: WireRange(0, 0),
        metadata: WireRange(0, 0),
        default_speaker: None,
        source_map: 0,
    });
    unsorted.block_lookup.push(WireLookupEntry {
        id: "alpha",
        index: 1,
    });
    let unsorted_bytes = rmp_serde::to_vec(&unsorted).expect("test wire encodes");

    let unsorted_error = decode_compiled_dialogue_messagepack(&unsorted_bytes)
        .expect_err("unsorted lookup rejected");

    assert!(matches!(
        unsorted_error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("strictly sorted and unique")
    ));

    let mut duplicate = valid_wire_asset();
    duplicate.block_lookup.push(WireLookupEntry {
        id: "start",
        index: 0,
    });
    let duplicate_bytes = rmp_serde::to_vec(&duplicate).expect("test wire encodes");

    let duplicate_error = decode_compiled_dialogue_messagepack(&duplicate_bytes)
        .expect_err("duplicate lookup rejected");

    assert!(matches!(
        duplicate_error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("strictly sorted and unique")
    ));

    let mut alias = valid_wire_asset();
    alias.block_lookup.push(WireLookupEntry {
        id: "zzz_alias",
        index: 0,
    });
    let alias_bytes = rmp_serde::to_vec(&alias).expect("test wire encodes");

    let alias_error =
        decode_compiled_dialogue_messagepack(&alias_bytes).expect_err("lookup alias rejected");

    assert!(matches!(
        alias_error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains("entries for 1 table rows")
    ));
}
