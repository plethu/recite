use super::rows::{MsgChoice, MsgLine};
use super::tables::*;
use super::tags::{
    MsgAssetEncoding, MsgFingerprint, MsgInspectionEncoding, MsgMatchPattern, MsgSchemaFingerprint,
    MsgStatementKind,
};
use crate::CompiledDialogue;
use crate::SpeakerIndex;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct MsgDialogue<'a>(
    MsgHeader<'a>,
    u32,
    Vec<MsgSourceFile<'a>>,
    Vec<MsgBlock<'a>>,
    Vec<MsgStatement<'a>>,
    Vec<MsgMatchArm<'a>>,
    Vec<MsgLine<'a>>,
    Vec<MsgChoice<'a>>,
    Vec<MsgAvailabilityReason<'a>>,
    Vec<MsgConditionAvailabilityReason<'a>>,
    Vec<MsgSpeaker<'a>>,
    Vec<MsgMetadataEntry<'a>>,
    Vec<MsgEffect<'a>>,
    Vec<MsgSourceMapEntry<'a>>,
    Vec<MsgLookupEntry<'a>>,
    Vec<MsgLookupEntry<'a>>,
    Vec<MsgLookupEntry<'a>>,
);

impl<'a> From<&'a CompiledDialogue> for MsgDialogue<'a> {
    fn from(dialogue: &'a CompiledDialogue) -> Self {
        Self(
            MsgHeader::from(dialogue),
            dialogue.default_block.as_u32(),
            dialogue.sources.iter().map(MsgSourceFile::from).collect(),
            dialogue.blocks.iter().map(MsgBlock::from).collect(),
            dialogue.statements.iter().map(MsgStatement::from).collect(),
            dialogue.match_arms.iter().map(MsgMatchArm::from).collect(),
            dialogue.lines.iter().map(MsgLine::from).collect(),
            dialogue.choices.iter().map(MsgChoice::from).collect(),
            dialogue
                .availability_reasons
                .iter()
                .map(MsgAvailabilityReason::from)
                .collect(),
            dialogue
                .condition_availability_reasons
                .iter()
                .map(MsgConditionAvailabilityReason::from)
                .collect(),
            dialogue.speakers.iter().map(MsgSpeaker::from).collect(),
            dialogue
                .metadata
                .iter()
                .map(MsgMetadataEntry::from)
                .collect(),
            dialogue.effects.iter().map(MsgEffect::from).collect(),
            dialogue
                .source_maps
                .iter()
                .map(MsgSourceMapEntry::from)
                .collect(),
            dialogue
                .block_lookup
                .iter()
                .map(|entry| MsgLookupEntry(entry.id.as_str(), entry.index.as_u32()))
                .collect(),
            dialogue
                .line_lookup
                .iter()
                .map(|entry| MsgLookupEntry(entry.id.as_str(), entry.index.as_u32()))
                .collect(),
            dialogue
                .choice_lookup
                .iter()
                .map(|entry| MsgLookupEntry(entry.id.as_str(), entry.index.as_u32()))
                .collect(),
        )
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

impl<'a> From<&'a crate::CompiledSourceFile> for MsgSourceFile<'a> {
    fn from(source: &'a crate::CompiledSourceFile) -> Self {
        Self(source.path.as_str(), MsgFingerprint(&source.fingerprint))
    }
}

#[derive(Serialize)]
struct MsgBlock<'a>(&'a str, u32, MsgRange, MsgRange, Option<u32>, u32);

impl<'a> From<&'a crate::CompiledBlock> for MsgBlock<'a> {
    fn from(block: &'a crate::CompiledBlock) -> Self {
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

impl<'a> From<&'a crate::CompiledStatement> for MsgStatement<'a> {
    fn from(statement: &'a crate::CompiledStatement) -> Self {
        Self(
            MsgStatementKind(&statement.kind),
            statement.source_map.as_u32(),
        )
    }
}

#[derive(Serialize)]
struct MsgMatchArm<'a>(MsgMatchPattern<'a>, MsgRange, u32);

impl<'a> From<&'a crate::CompiledMatchArm> for MsgMatchArm<'a> {
    fn from(arm: &'a crate::CompiledMatchArm) -> Self {
        Self(
            MsgMatchPattern(&arm.pattern),
            statement_range(arm.statements),
            arm.source_map.as_u32(),
        )
    }
}
