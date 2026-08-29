use recite_core::{CompiledAssetDecodeError, decode_compiled_dialogue_messagepack};
use serde::Serialize;
use serde::ser::SerializeTuple;

use super::support::*;

struct WireAssetRows<'a> {
    asset: WireAsset<'a>,
    line: Option<WireCurrentLine<'a>>,
    choice: Option<WireCurrentChoice<'a>>,
}

impl Serialize for WireAssetRows<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let asset = &self.asset;
        let mut tuple = serializer.serialize_tuple(17)?;
        tuple.serialize_element(&asset.header)?;
        tuple.serialize_element(&asset.default_block)?;
        tuple.serialize_element(&asset.sources)?;
        tuple.serialize_element(&asset.blocks)?;
        tuple.serialize_element(&asset.statements)?;
        tuple.serialize_element(&asset.match_arms)?;
        if let Some(line) = &self.line {
            tuple.serialize_element(&vec![line])?;
        } else {
            tuple.serialize_element(&asset.lines)?;
        }
        if let Some(choice) = &self.choice {
            tuple.serialize_element(&vec![choice])?;
        } else {
            tuple.serialize_element(&asset.choices)?;
        }
        tuple.serialize_element(&asset.availability_reasons)?;
        tuple.serialize_element(&asset.condition_availability_reasons)?;
        tuple.serialize_element(&asset.speakers)?;
        tuple.serialize_element(&asset.metadata)?;
        tuple.serialize_element(&asset.effects)?;
        tuple.serialize_element(&asset.source_maps)?;
        tuple.serialize_element(&asset.block_lookup)?;
        tuple.serialize_element(&asset.line_lookup)?;
        tuple.serialize_element(&asset.choice_lookup)?;
        tuple.end()
    }
}

fn assert_rejected(asset: WireAssetRows<'_>, expected: &str) {
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");
    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("asset is rejected");
    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains(expected)
    ));
}

#[test]
fn decode_rejects_malformed_current_interpolation_rows() {
    let mut asset = valid_wire_asset();
    asset.line_lookup.push(WireLookupEntry {
        id: "line",
        index: 0,
    });
    assert_rejected(
        WireAssetRows {
            asset,
            line: Some(WireCurrentLine {
                id: "line",
                source_text: "Hello {name}.",
                speaker: None,
                metadata: WireRange(0, 0),
                source_map: 0,
                authored_source_text: "Hello {name}.",
                interpolation_bindings: Vec::new(),
            }),
            choice: None,
        },
        "placeholder `name` has no interpolation binding",
    );

    let mut asset = valid_wire_asset();
    asset.line_lookup.push(WireLookupEntry {
        id: "line",
        index: 0,
    });
    assert_rejected(
        WireAssetRows {
            asset,
            line: Some(WireCurrentLine {
                id: "line",
                source_text: "Hello.",
                speaker: None,
                metadata: WireRange(0, 0),
                source_map: 0,
                authored_source_text: "Hello.",
                interpolation_bindings: vec![WireInterpolationBinding("name", "display", "string")],
            }),
            choice: None,
        },
        "binding `name` is not used",
    );
}

#[test]
fn decode_rejects_incomplete_plural_source_rows() {
    let mut asset = valid_wire_asset();
    asset.line_lookup.push(WireLookupEntry {
        id: "line",
        index: 0,
    });
    assert_rejected_plural(
        WirePluralAssetRows {
            asset,
            line: WirePluralLine {
                id: "line",
                source_text: "One {count} item.",
                speaker: None,
                metadata: WireRange(0, 0),
                source_map: 0,
                authored_source_text: "One {count} item.",
                interpolation_bindings: vec![WireInterpolationBinding("count", "items", "int")],
                plural_source_text: Some("Many {count} items."),
                authored_plural_source_text: None,
            },
        },
        "compiled plural source text must include both decoded and authored forms",
    );
}

#[test]
fn decode_accepts_legacy_line_and_choice_rows() {
    let mut asset = valid_wire_asset();
    asset.line_lookup.push(WireLookupEntry {
        id: "line",
        index: 0,
    });
    asset.choice_lookup.push(WireLookupEntry {
        id: "choice",
        index: 0,
    });
    let rows = LegacyRows {
        asset,
        line: WireLegacyLine("line", "Hello {unbound}.", None, WireRange(0, 0), 0),
        choice: WireLegacyChoice(
            "choice",
            "Choose {unbound}.",
            WireRange(0, 0),
            None,
            None,
            None,
            Tagged::nil(recite_core::V0_DIVERT_TARGET_TAG_END),
            Tagged::nil(recite_core::V0_CHOICE_ECHO_TAG_NONE),
            0,
        ),
    };
    let decoded = decode_compiled_dialogue_messagepack(
        &rmp_serde::to_vec(&rows).expect("legacy test wire encodes"),
    )
    .expect("legacy row shapes remain decodable");
    assert_eq!(decoded.lines[0].source_text, "Hello {unbound}.");
    assert!(decoded.lines[0].interpolation_bindings.is_empty());
    assert_eq!(decoded.choices[0].source_text, "Choose {unbound}.");
    assert!(decoded.choices[0].interpolation_bindings.is_empty());
}

struct LegacyRows<'a> {
    asset: WireAsset<'a>,
    line: WireLegacyLine<'a>,
    choice: WireLegacyChoice<'a>,
}

struct WirePluralLine<'a> {
    id: &'a str,
    source_text: &'a str,
    speaker: Option<u32>,
    metadata: WireRange,
    source_map: u32,
    authored_source_text: &'a str,
    interpolation_bindings: Vec<WireInterpolationBinding<'a>>,
    plural_source_text: Option<&'a str>,
    authored_plural_source_text: Option<&'a str>,
}

impl Serialize for WirePluralLine<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(9)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&self.speaker)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.serialize_element(&self.authored_source_text)?;
        tuple.serialize_element(&self.interpolation_bindings)?;
        tuple.serialize_element(&self.plural_source_text)?;
        tuple.serialize_element(&self.authored_plural_source_text)?;
        tuple.end()
    }
}

struct WirePluralAssetRows<'a> {
    asset: WireAsset<'a>,
    line: WirePluralLine<'a>,
}

impl Serialize for WirePluralAssetRows<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let asset = &self.asset;
        let mut tuple = serializer.serialize_tuple(17)?;
        tuple.serialize_element(&asset.header)?;
        tuple.serialize_element(&asset.default_block)?;
        tuple.serialize_element(&asset.sources)?;
        tuple.serialize_element(&asset.blocks)?;
        tuple.serialize_element(&asset.statements)?;
        tuple.serialize_element(&asset.match_arms)?;
        tuple.serialize_element(&vec![&self.line])?;
        tuple.serialize_element(&asset.choices)?;
        tuple.serialize_element(&asset.availability_reasons)?;
        tuple.serialize_element(&asset.condition_availability_reasons)?;
        tuple.serialize_element(&asset.speakers)?;
        tuple.serialize_element(&asset.metadata)?;
        tuple.serialize_element(&asset.effects)?;
        tuple.serialize_element(&asset.source_maps)?;
        tuple.serialize_element(&asset.block_lookup)?;
        tuple.serialize_element(&asset.line_lookup)?;
        tuple.serialize_element(&asset.choice_lookup)?;
        tuple.end()
    }
}

fn assert_rejected_plural(asset: WirePluralAssetRows<'_>, expected: &str) {
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");
    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("asset is rejected");
    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains(expected)
    ));
}

impl Serialize for LegacyRows<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let asset = &self.asset;
        let mut tuple = serializer.serialize_tuple(17)?;
        tuple.serialize_element(&asset.header)?;
        tuple.serialize_element(&asset.default_block)?;
        tuple.serialize_element(&asset.sources)?;
        tuple.serialize_element(&asset.blocks)?;
        tuple.serialize_element(&asset.statements)?;
        tuple.serialize_element(&asset.match_arms)?;
        tuple.serialize_element(&vec![&self.line])?;
        tuple.serialize_element(&vec![&self.choice])?;
        tuple.serialize_element(&asset.availability_reasons)?;
        tuple.serialize_element(&asset.condition_availability_reasons)?;
        tuple.serialize_element(&asset.speakers)?;
        tuple.serialize_element(&asset.metadata)?;
        tuple.serialize_element(&asset.effects)?;
        tuple.serialize_element(&asset.source_maps)?;
        tuple.serialize_element(&asset.block_lookup)?;
        tuple.serialize_element(&asset.line_lookup)?;
        tuple.serialize_element(&asset.choice_lookup)?;
        tuple.end()
    }
}
