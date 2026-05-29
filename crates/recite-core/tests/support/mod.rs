#![allow(dead_code)]
#![cfg(test)]

use recite_core::{
    BLAKE3_DIGEST_LEN, COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0,
    CompiledAssetDecodeError, decode_compiled_dialogue_messagepack,
};
use serde::Serialize;
use serde::ser::SerializeTuple;
use serde_bytes::Bytes;

pub(crate) fn assert_malformed_asset_contains(asset: WireAsset<'_>, expected: &str) {
    let bytes = rmp_serde::to_vec(&asset).expect("test wire encodes");
    let error = decode_compiled_dialogue_messagepack(&bytes).expect_err("asset is rejected");

    assert!(matches!(
        error,
        CompiledAssetDecodeError::MalformedAsset(message) if message.contains(expected)
    ));
}

pub(crate) fn valid_wire_asset() -> WireAsset<'static> {
    WireAsset {
        header: valid_header(),
        default_block: 0,
        sources: vec![WireSourceFile {
            path: "dialogue/main.recite",
            fingerprint: valid_fingerprint(),
        }],
        blocks: vec![WireBlock {
            id: "start",
            source_file: 0,
            statements: WireRange(0, 1),
            metadata: WireRange(0, 0),
            default_speaker: None,
            source_map: 0,
        }],
        statements: vec![WireStatement {
            kind: WireStatementKind::End,
            source_map: 0,
        }],
        match_arms: Vec::new(),
        lines: Vec::new(),
        choices: Vec::new(),
        speakers: Vec::new(),
        metadata: Vec::new(),
        effects: Vec::new(),
        source_maps: vec![WireSourceMapEntry {
            source_file: 0,
            span: WireSourceSpan {
                file: "dialogue/main.recite",
                start_line: 1,
                start_column: 1,
                end_line: None,
                end_column: None,
            },
        }],
        block_lookup: vec![WireLookupEntry {
            id: "start",
            index: 0,
        }],
        line_lookup: Vec::new(),
        choice_lookup: Vec::new(),
    }
}

pub(crate) fn valid_header() -> WireHeader<'static> {
    WireHeader {
        format_version: COMPILED_ASSET_FORMAT_VERSION_V0,
        compiler_compatibility_version: COMPILER_COMPATIBILITY_VERSION_V0,
        primary_encoding: Tagged::<u8>::nil(recite_core::V0_ASSET_ENCODING_MESSAGEPACK),
        inspection_encoding: Tagged::<u8>::nil(recite_core::V0_INSPECTION_ENCODING_COMPACT_JSON),
        compiler_version: "0.0.1",
        asset_id: "dialogue/main.recitec",
        source_map_id: "dialogue/main.recitec.map",
        schema_fingerprint: Tagged::nil(recite_core::V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA),
    }
}

pub(crate) fn valid_fingerprint() -> WireFingerprint<'static> {
    WireFingerprint {
        algorithm: "blake3",
        digest: Bytes::new(&VALID_DIGEST),
    }
}

pub(crate) const VALID_DIGEST: [u8; BLAKE3_DIGEST_LEN] = [7; BLAKE3_DIGEST_LEN];
pub(crate) const SHORT_DIGEST: [u8; 3] = [1, 2, 3];

pub(crate) struct WireAsset<'a> {
    pub(crate) header: WireHeader<'a>,
    pub(crate) default_block: u32,
    pub(crate) sources: Vec<WireSourceFile<'a>>,
    pub(crate) blocks: Vec<WireBlock<'a>>,
    pub(crate) statements: Vec<WireStatement>,
    pub(crate) match_arms: Vec<()>,
    pub(crate) lines: Vec<WireLine<'a>>,
    pub(crate) choices: Vec<WireChoice<'a>>,
    pub(crate) speakers: Vec<WireSpeaker<'a>>,
    pub(crate) metadata: Vec<WireMetadataEntry<'a>>,
    pub(crate) effects: Vec<WireEffect<'a>>,
    pub(crate) source_maps: Vec<WireSourceMapEntry<'a>>,
    pub(crate) block_lookup: Vec<WireLookupEntry<'a>>,
    pub(crate) line_lookup: Vec<WireLookupEntry<'a>>,
    pub(crate) choice_lookup: Vec<WireLookupEntry<'a>>,
}

impl Serialize for WireAsset<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(15)?;
        tuple.serialize_element(&self.header)?;
        tuple.serialize_element(&self.default_block)?;
        tuple.serialize_element(&self.sources)?;
        tuple.serialize_element(&self.blocks)?;
        tuple.serialize_element(&self.statements)?;
        tuple.serialize_element(&self.match_arms)?;
        tuple.serialize_element(&self.lines)?;
        tuple.serialize_element(&self.choices)?;
        tuple.serialize_element(&self.speakers)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.effects)?;
        tuple.serialize_element(&self.source_maps)?;
        tuple.serialize_element(&self.block_lookup)?;
        tuple.serialize_element(&self.line_lookup)?;
        tuple.serialize_element(&self.choice_lookup)?;
        tuple.end()
    }
}

pub(crate) struct WireHeader<'a> {
    pub(crate) format_version: u16,
    pub(crate) compiler_compatibility_version: u16,
    pub(crate) primary_encoding: Tagged<u8>,
    pub(crate) inspection_encoding: Tagged<u8>,
    pub(crate) compiler_version: &'a str,
    pub(crate) asset_id: &'a str,
    pub(crate) source_map_id: &'a str,
    pub(crate) schema_fingerprint: Tagged<WireFingerprint<'a>>,
}

impl Serialize for WireHeader<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(8)?;
        tuple.serialize_element(&self.format_version)?;
        tuple.serialize_element(&self.compiler_compatibility_version)?;
        tuple.serialize_element(&self.primary_encoding)?;
        tuple.serialize_element(&self.inspection_encoding)?;
        tuple.serialize_element(&self.compiler_version)?;
        tuple.serialize_element(&self.asset_id)?;
        tuple.serialize_element(&self.source_map_id)?;
        tuple.serialize_element(&self.schema_fingerprint)?;
        tuple.end()
    }
}

pub(crate) struct Tagged<T> {
    pub(crate) tag: u8,
    pub(crate) payload: Option<T>,
}

impl<T> Tagged<T> {
    pub(crate) fn nil(tag: u8) -> Self {
        Self { tag, payload: None }
    }

    pub(crate) fn payload(tag: u8, payload: T) -> Self {
        Self {
            tag,
            payload: Some(payload),
        }
    }
}

impl<T: Serialize> Serialize for Tagged<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.tag)?;
        tuple.serialize_element(&self.payload)?;
        tuple.end()
    }
}

pub(crate) struct WireFingerprint<'a> {
    pub(crate) algorithm: &'a str,
    pub(crate) digest: &'a Bytes,
}

impl Serialize for WireFingerprint<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.algorithm)?;
        tuple.serialize_element(&self.digest)?;
        tuple.end()
    }
}

pub(crate) struct WireSourceFile<'a> {
    pub(crate) path: &'a str,
    pub(crate) fingerprint: WireFingerprint<'a>,
}

impl Serialize for WireSourceFile<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.path)?;
        tuple.serialize_element(&self.fingerprint)?;
        tuple.end()
    }
}

#[derive(Clone, Copy, Serialize)]
pub(crate) struct WireRange(pub(crate) u32, pub(crate) u32);

pub(crate) struct WireBlock<'a> {
    pub(crate) id: &'a str,
    pub(crate) source_file: u32,
    pub(crate) statements: WireRange,
    pub(crate) metadata: WireRange,
    pub(crate) default_speaker: Option<u32>,
    pub(crate) source_map: u32,
}

impl Serialize for WireBlock<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(6)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_file)?;
        tuple.serialize_element(&self.statements)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.default_speaker)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

pub(crate) struct WireStatement {
    pub(crate) kind: WireStatementKind,
    pub(crate) source_map: u32,
}

impl Serialize for WireStatement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.kind)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

pub(crate) enum WireStatementKind {
    End,
    Prompt {
        line: Option<u32>,
        choices: WireRange,
    },
    Unknown(u8),
}

impl Serialize for WireStatementKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::End => Tagged::<u8>::nil(recite_core::V0_STATEMENT_TAG_END).serialize(serializer),
            Self::Prompt { line, choices } => {
                Tagged::payload(recite_core::V0_STATEMENT_TAG_PROMPT, (*line, *choices))
                    .serialize(serializer)
            }
            Self::Unknown(tag) => Tagged::<u8>::nil(*tag).serialize(serializer),
        }
    }
}

pub(crate) struct WireLine<'a> {
    pub(crate) id: &'a str,
    pub(crate) source_text: &'a str,
    pub(crate) speaker: Option<u32>,
    pub(crate) metadata: WireRange,
    pub(crate) source_map: u32,
}

impl Serialize for WireLine<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(5)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&self.speaker)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

pub(crate) struct WireChoice<'a> {
    pub(crate) id: &'a str,
    pub(crate) source_text: &'a str,
    pub(crate) metadata: WireRange,
    pub(crate) condition: Option<WireConditionExpression<'a>>,
    pub(crate) target: Tagged<u32>,
    pub(crate) echo: Tagged<&'a str>,
    pub(crate) source_map: u32,
}

impl Serialize for WireChoice<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(7)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.source_text)?;
        tuple.serialize_element(&self.metadata)?;
        tuple.serialize_element(&self.condition)?;
        tuple.serialize_element(&self.target)?;
        tuple.serialize_element(&self.echo)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

pub(crate) enum WireConditionExpression<'a> {
    Call(WireConditionCall<'a>),
    EmptyAnd,
    EmptyOr,
    Unknown(u8),
}

impl Serialize for WireConditionExpression<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Call(call) => {
                Tagged::payload(recite_core::V0_CONDITION_TAG_CALL, call).serialize(serializer)
            }
            Self::EmptyAnd => {
                Tagged::payload(recite_core::V0_CONDITION_TAG_AND, Vec::<Self>::new())
                    .serialize(serializer)
            }
            Self::EmptyOr => Tagged::payload(recite_core::V0_CONDITION_TAG_OR, Vec::<Self>::new())
                .serialize(serializer),
            Self::Unknown(tag) => Tagged::<u8>::nil(*tag).serialize(serializer),
        }
    }
}

pub(crate) struct WireConditionCall<'a> {
    pub(crate) function: &'a str,
    pub(crate) args: Vec<Tagged<&'a str>>,
}

impl Serialize for WireConditionCall<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.function)?;
        tuple.serialize_element(&self.args)?;
        tuple.end()
    }
}

pub(crate) struct WireSpeaker<'a> {
    pub(crate) id: &'a str,
}

impl Serialize for WireSpeaker<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(1)?;
        tuple.serialize_element(&self.id)?;
        tuple.end()
    }
}

pub(crate) struct WireMetadataEntry<'a> {
    pub(crate) key: &'a str,
    pub(crate) value: Tagged<Tagged<f64>>,
    pub(crate) source_map: Option<u32>,
}

impl Serialize for WireMetadataEntry<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(3)?;
        tuple.serialize_element(&self.key)?;
        tuple.serialize_element(&self.value)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

pub(crate) struct WireEffect<'a> {
    pub(crate) id: &'a str,
    pub(crate) mode: Tagged<u8>,
    pub(crate) function: &'a str,
    pub(crate) args: Vec<Tagged<&'a str>>,
    pub(crate) source_map: u32,
}

impl Serialize for WireEffect<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(5)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.mode)?;
        tuple.serialize_element(&self.function)?;
        tuple.serialize_element(&self.args)?;
        tuple.serialize_element(&self.source_map)?;
        tuple.end()
    }
}

pub(crate) struct WireSourceMapEntry<'a> {
    pub(crate) source_file: u32,
    pub(crate) span: WireSourceSpan<'a>,
}

impl Serialize for WireSourceMapEntry<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.source_file)?;
        tuple.serialize_element(&self.span)?;
        tuple.end()
    }
}

pub(crate) struct WireSourceSpan<'a> {
    pub(crate) file: &'a str,
    pub(crate) start_line: u32,
    pub(crate) start_column: u32,
    pub(crate) end_line: Option<u32>,
    pub(crate) end_column: Option<u32>,
}

impl Serialize for WireSourceSpan<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(5)?;
        tuple.serialize_element(&self.file)?;
        tuple.serialize_element(&self.start_line)?;
        tuple.serialize_element(&self.start_column)?;
        tuple.serialize_element(&self.end_line)?;
        tuple.serialize_element(&self.end_column)?;
        tuple.end()
    }
}

pub(crate) struct WireLookupEntry<'a> {
    pub(crate) id: &'a str,
    pub(crate) index: u32,
}

impl Serialize for WireLookupEntry<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(&self.id)?;
        tuple.serialize_element(&self.index)?;
        tuple.end()
    }
}
