#![cfg(test)]

use recite_core::{
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
fn prepared_identity_changes_after_nested_payload_mutation() {
    let mut asset = CompiledDialogue::prepare(prepared_payload()).expect("asset prepares");
    let before = canonical_compiled_dialogue_fingerprint(&asset).expect("valid fingerprint");

    asset.sources[0].fingerprint = canonical_source_fingerprint("changed source");

    let after = canonical_compiled_dialogue_fingerprint(&asset).expect("mutated fingerprint");
    assert_ne!(before, after);
}

#[test]
fn cached_invalid_identity_is_replaced_after_repairing_nested_payload() {
    let mut asset = decoded_asset();
    asset.blocks[0].statements.len = 2;

    let invalid = canonical_compiled_dialogue_fingerprint(&asset)
        .expect_err("invalid payload fingerprint is cached as an error");
    assert!(invalid.to_string().contains("statements"));

    asset.blocks[0].statements.len = 1;

    assert_eq!(
        canonical_compiled_dialogue_fingerprint(&asset).expect("repaired payload fingerprints"),
        expected_fingerprint()
    );
}

#[test]
fn cloned_identity_cache_is_independent_after_mutation() {
    let asset = decoded_asset();
    let original = canonical_compiled_dialogue_fingerprint(&asset).expect("valid fingerprint");
    let mut clone = asset.clone();
    assert_eq!(
        canonical_compiled_dialogue_fingerprint(&clone).expect("cloned fingerprint"),
        original
    );

    clone.sources[0].fingerprint = canonical_source_fingerprint("clone-only source");

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
