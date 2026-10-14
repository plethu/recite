#![cfg(test)]

use recite_core::compiled::{
    CompiledDialogue, CompiledDialoguePayload, ContentFingerprint,
    canonical_compiled_dialogue_fingerprint, canonical_source_fingerprint,
    decode_compiled_dialogue_messagepack, encode_compiled_dialogue_messagepack,
};

mod support;

fn decoded_asset() -> CompiledDialogue {
    let bytes = rmp_serde::to_vec(&support::valid_wire_asset()).expect("test wire encodes");
    decode_compiled_dialogue_messagepack(&bytes).expect("valid asset decodes")
}

fn prepared_payload() -> CompiledDialoguePayload {
    let asset = decoded_asset();
    (*asset).clone()
}

fn expected_fingerprint() -> ContentFingerprint {
    canonical_compiled_dialogue_fingerprint(&decoded_asset()).expect("valid asset fingerprints")
}

#[test]
fn canonical_validation_matches_encoder_header_and_payload_rejection() {
    let asset = decoded_asset();
    assert_eq!(
        recite_core::compiled::validate_compiled_dialogue(&asset),
        Ok(())
    );
    for (format, compatibility) in [(0, 1), (1, 0), (1, 1)] {
        let mut payload = asset.clone().into_payload();
        payload.header.format_version = format;
        payload.header.compiler_compatibility_version = compatibility;
        let changed = CompiledDialogue::new(payload);
        assert_eq!(
            recite_core::compiled::validate_compiled_dialogue(&changed).err(),
            encode_compiled_dialogue_messagepack(&changed).err()
        );
    }
}

#[test]
fn uncached_identity_matches_canonical_bytes_for_large_unicode_prose() {
    for repetitions in [1, 10_000] {
        let text = "Café 🦉\n".repeat(repetitions);
        let mut wire = support::valid_wire_asset();
        wire.lines.push(support::WireLine {
            id: "11111111111111111111",
            source_text: &text,
            speaker: None,
            metadata: support::WireRange(0, 0),
            source_map: 0,
        });
        wire.line_lookup.push(support::WireLookupEntry {
            id: "11111111111111111111",
            index: 0,
        });
        let bytes = rmp_serde::to_vec(&wire).expect("wire fixture encodes");
        let decoded = decode_compiled_dialogue_messagepack(&bytes).expect("fixture decodes");
        let asset = CompiledDialogue::new(decoded.into_payload());

        let identity = canonical_compiled_dialogue_fingerprint(&asset).expect("asset fingerprints");
        let canonical = encode_compiled_dialogue_messagepack(&asset).expect("asset encodes");
        assert_eq!(
            identity.digest().as_bytes(),
            blake3::hash(&canonical).as_bytes()
        );
    }
}

#[test]
fn rebuilding_a_prepared_asset_changes_its_identity() {
    let asset = CompiledDialogue::prepare(prepared_payload()).expect("asset prepares");
    let before = canonical_compiled_dialogue_fingerprint(&asset).expect("valid fingerprint");

    let mut payload = asset.into_payload();
    payload.sources[0].fingerprint = canonical_source_fingerprint("changed source");
    let asset = CompiledDialogue::prepare(payload).expect("changed asset prepares");

    let after = canonical_compiled_dialogue_fingerprint(&asset).expect("mutated fingerprint");
    assert_ne!(before, after);
}

#[test]
fn invalid_raw_payload_can_be_repaired_and_rebuilt() {
    let mut payload = decoded_asset().into_payload();
    payload.blocks[0].statements.len = 2;
    let asset = CompiledDialogue::new(payload);

    let invalid = canonical_compiled_dialogue_fingerprint(&asset)
        .expect_err("invalid payload fingerprint is cached as an error");
    assert!(invalid.to_string().contains("statements"));

    let mut payload = asset.into_payload();
    payload.blocks[0].statements.len = 1;
    let asset = CompiledDialogue::prepare(payload).expect("repaired asset prepares");

    assert_eq!(
        canonical_compiled_dialogue_fingerprint(&asset).expect("repaired payload fingerprints"),
        expected_fingerprint()
    );
}

#[test]
fn cloned_identity_cache_is_independent_after_rebuilding() {
    let asset = decoded_asset();
    let original = canonical_compiled_dialogue_fingerprint(&asset).expect("valid fingerprint");
    let clone = asset.clone();
    assert_eq!(
        canonical_compiled_dialogue_fingerprint(&clone).expect("cloned fingerprint"),
        original
    );

    let mut payload = clone.into_payload();
    payload.sources[0].fingerprint = canonical_source_fingerprint("clone-only source");
    let clone = CompiledDialogue::prepare(payload).expect("changed clone prepares");

    assert_ne!(
        canonical_compiled_dialogue_fingerprint(&clone).expect("changed clone fingerprints"),
        original
    );
    assert_eq!(
        canonical_compiled_dialogue_fingerprint(&asset).expect("original remains unchanged"),
        original
    );
}

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn compiled_dialogue_cache_is_send_and_sync() {
    assert_send_sync::<CompiledDialogue>();
}
