use super::rows::{MsgChoice, MsgLine};
use super::tables::*;
use super::tags::{
    MsgAssetEncoding, MsgFingerprint, MsgInspectionEncoding, MsgMatchPattern, MsgSchemaFingerprint,
    MsgStatementKind,
};
use crate::compiled::CompiledDialogue;
use crate::compiled::SpeakerIndex;
use serde::Serialize;

pub(super) struct MsgDialogue<'a>(&'a CompiledDialogue);

impl<'a> From<&'a CompiledDialogue> for MsgDialogue<'a> {
    fn from(dialogue: &'a CompiledDialogue) -> Self {
        Self(dialogue)
    }
}

impl Serialize for MsgDialogue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeTuple;
        let dialogue = self.0;
        let mut tuple =
            serializer.serialize_tuple(crate::compiled::V0_COMPILED_DIALOGUE_FIELDS as usize)?;
        tuple.serialize_element(&MsgHeader::from(dialogue))?;
        tuple.serialize_element(&dialogue.default_block.as_u32())?;
        tuple.serialize_element(&TableRows(dialogue.sources.iter().map(MsgSourceFile::from)))?;
        tuple.serialize_element(&TableRows(dialogue.blocks.iter().map(MsgBlock::from)))?;
        tuple.serialize_element(&TableRows(
            dialogue.statements.iter().map(MsgStatement::from),
        ))?;
        tuple.serialize_element(&TableRows(
            dialogue.match_arms.iter().map(MsgMatchArm::from),
        ))?;
        tuple.serialize_element(&TableRows(dialogue.lines.iter().map(MsgLine::from)))?;
        tuple.serialize_element(&TableRows(dialogue.choices.iter().map(MsgChoice::from)))?;
        tuple.serialize_element(&TableRows(
            dialogue
                .availability_reasons
                .iter()
                .map(MsgAvailabilityReason::from),
        ))?;
        tuple.serialize_element(&TableRows(
            dialogue
                .condition_availability_reasons
                .iter()
                .map(MsgConditionAvailabilityReason::from),
        ))?;
        tuple.serialize_element(&TableRows(dialogue.speakers.iter().map(MsgSpeaker::from)))?;
        tuple.serialize_element(&TableRows(
            dialogue.metadata.iter().map(MsgMetadataEntry::from),
        ))?;
        tuple.serialize_element(&TableRows(dialogue.effects.iter().map(MsgEffect::from)))?;
        tuple.serialize_element(&TableRows(
            dialogue.source_maps.iter().map(MsgSourceMapEntry::from),
        ))?;
        tuple.serialize_element(&TableRows(
            dialogue
                .block_lookup
                .as_slice()
                .iter()
                .map(|entry| MsgLookupEntry(entry.id.as_str(), entry.index.as_u32())),
        ))?;
        tuple.serialize_element(&TableRows(
            dialogue
                .line_lookup
                .as_slice()
                .iter()
                .map(|entry| MsgLookupEntry(entry.id.as_str(), entry.index.as_u32())),
        ))?;
        tuple.serialize_element(&TableRows(
            dialogue
                .choice_lookup
                .as_slice()
                .iter()
                .map(|entry| MsgLookupEntry(entry.id.as_str(), entry.index.as_u32())),
        ))?;
        tuple.end()
    }
}

// Project each borrowed row as it is written instead of retaining whole wire tables.
struct TableRows<I>(I);

impl<I> Serialize for TableRows<I>
where
    I: ExactSizeIterator + Clone,
    I::Item: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_seq(self.0.clone())
    }
}

#[derive(Serialize)]
struct MsgHeader<'a>(
    u16,
    u16,
    MsgAssetEncoding,
    MsgInspectionEncoding,
    &'a str,
    &'a str,
    &'a str,
    MsgSchemaFingerprint<'a>,
);

impl<'a> From<&'a CompiledDialogue> for MsgHeader<'a> {
    fn from(dialogue: &'a CompiledDialogue) -> Self {
        let header = &dialogue.header;
        Self(
            header.format_version,
            header.compiler_compatibility_version,
            MsgAssetEncoding(header.primary_encoding),
            MsgInspectionEncoding(header.inspection_encoding),
            header.compiler_version.as_str(),
            header.asset_id.as_str(),
            header.source_map_id.as_str(),
            MsgSchemaFingerprint(&header.schema_fingerprint),
        )
    }
}

#[derive(Serialize)]
struct MsgSourceFile<'a>(&'a str, MsgFingerprint<'a>);

impl<'a> From<&'a crate::compiled::CompiledSourceFile> for MsgSourceFile<'a> {
    fn from(source: &'a crate::compiled::CompiledSourceFile) -> Self {
        Self(source.path.as_str(), MsgFingerprint(&source.fingerprint))
    }
}

#[derive(Serialize)]
struct MsgBlock<'a>(&'a str, u32, MsgRange, MsgRange, Option<u32>, u32);

impl<'a> From<&'a crate::compiled::CompiledBlock> for MsgBlock<'a> {
    fn from(block: &'a crate::compiled::CompiledBlock) -> Self {
        Self(
            block.id.as_str(),
            block.source_file.as_u32(),
            statement_range(block.statements),
            metadata_range(block.metadata),
            block.default_speaker.map(SpeakerIndex::as_u32),
            block.source_map.as_u32(),
        )
    }
}

#[derive(Serialize)]
struct MsgStatement<'a>(MsgStatementKind<'a>, u32);

impl<'a> From<&'a crate::compiled::CompiledStatement> for MsgStatement<'a> {
    fn from(statement: &'a crate::compiled::CompiledStatement) -> Self {
        Self(
            MsgStatementKind(&statement.kind),
            statement.source_map.as_u32(),
        )
    }
}

#[derive(Serialize)]
struct MsgMatchArm<'a>(MsgMatchPattern<'a>, MsgRange, u32);

impl<'a> From<&'a crate::compiled::CompiledMatchArm> for MsgMatchArm<'a> {
    fn from(arm: &'a crate::compiled::CompiledMatchArm) -> Self {
        Self(
            MsgMatchPattern(&arm.pattern),
            statement_range(arm.statements),
            arm.source_map.as_u32(),
        )
    }
}
