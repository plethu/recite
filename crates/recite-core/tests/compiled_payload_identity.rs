#![cfg(test)]

use recite_core::compiled::{
    CompiledDialogue, CompiledDialoguePayload, ContentFingerprint,
    canonical_compiled_dialogue_fingerprint, canonical_source_fingerprint,
    decode_compiled_dialogue_messagepack,
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
