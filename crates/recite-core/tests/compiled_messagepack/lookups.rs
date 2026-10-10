use recite_core::compiled::{
    CompiledAssetDecodeError, decode_compiled_dialogue_messagepack,
    encode_compiled_dialogue_messagepack,
};

use super::support::*;

#[test]
fn sorted_lookups_preserve_rows_in_authored_order() {
    let asset = asset_with_ids(&["zulu", "bravo", "mike"], &["uniform", "alpha", "yankee"]);
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");
    let decoded = decode_compiled_dialogue_messagepack(&bytes).expect("disjoint IDs decode");

    assert_eq!(
        decoded
            .lines
            .iter()
            .map(|line| line.id.as_str())
            .collect::<Vec<_>>(),
        ["zulu", "bravo", "mike"]
    );
    assert_eq!(
        decoded
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect::<Vec<_>>(),
        ["uniform", "alpha", "yankee"]
    );
    let canonical = encode_compiled_dialogue_messagepack(&decoded).expect("valid rows encode");
    let decoded_again =
        decode_compiled_dialogue_messagepack(&canonical).expect("canonical rows decode");
    assert_eq!(decoded_again, decoded);
    assert_eq!(
        encode_compiled_dialogue_messagepack(&decoded_again).expect("valid rows encode"),
        canonical
    );
}

#[test]
fn duplicate_line_and_choice_ids_report_the_first_id_in_lookup_order() {
    let asset = asset_with_ids(&["zulu", "alpha"], &["zulu", "alpha"]);
    assert_error(
        asset,
        "line and choice ids must be unique, got duplicate `alpha`",
    );
}

#[test]
fn missing_lookup_entries_are_rejected_before_shared_ids() {
    let mut asset = asset_with_ids(&["zulu", "alpha"], &["zulu"]);
    asset.line_lookup.pop();
    assert_error(asset, "line lookup has 1 entries for 2 table rows");
}

#[test]
fn out_of_range_lookup_entries_are_rejected_before_shared_ids() {
    let mut asset = asset_with_ids(&["zulu"], &["zulu"]);
    asset.choice_lookup[0].index = 1;
    assert_error(
        asset,
        "choice lookup index 1 is out of range for table length 1",
    );
}

#[test]
fn lookup_aliases_cannot_cover_a_duplicate_row() {
    let mut asset = asset_with_ids(&["same", "same"], &[]);
    asset.line_lookup[0].id = "alias";
    assert_error(
        asset,
        "line lookup entry `alias` points to row `same` at index 0",
    );
}

#[test]
fn block_lookup_errors_take_precedence_over_line_and_choice_collisions() {
    let mut asset = asset_with_ids(&["same"], &["same"]);
    asset.block_lookup[0].index = 1;
    assert_error(
        asset,
        "block lookup index 1 is out of range for table length 1",
    );
}

fn asset_with_ids(line_ids: &[&'static str], choice_ids: &[&'static str]) -> WireAsset<'static> {
    let mut asset = valid_wire_asset();
    asset.lines = line_ids
        .iter()
        .map(|id| WireLine {
            id,
            source_text: "Line.",
            speaker: None,
            metadata: WireRange(0, 0),
            source_map: 0,
        })
        .collect();
    asset.choices = choice_ids
        .iter()
        .map(|id| WireChoice {
            id,
            source_text: "Choice?",
            metadata: WireRange(0, 0),
            availability_requirement: None,
            availability_requirement_source_text: None,
            availability_reason_override: None,
            target: Tagged::nil(recite_core::compiled::V0_DIVERT_TARGET_TAG_END),
            echo: Tagged::nil(recite_core::compiled::V0_CHOICE_ECHO_TAG_NONE),
            source_map: 0,
        })
        .collect();
    asset.line_lookup = lookup_entries(line_ids);
    asset.choice_lookup = lookup_entries(choice_ids);
    asset
}

fn lookup_entries(ids: &[&'static str]) -> Vec<WireLookupEntry<'static>> {
    let mut entries = ids
        .iter()
        .enumerate()
        .map(|(index, id)| WireLookupEntry {
            id,
            index: index.try_into().expect("test row index fits u32"),
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.id);
    entries
}

fn assert_error(asset: WireAsset<'_>, expected: &str) {
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");
    assert_eq!(
        decode_compiled_dialogue_messagepack(&bytes),
        Err(CompiledAssetDecodeError::MalformedAsset(
            expected.to_owned()
        ))
    );
}
