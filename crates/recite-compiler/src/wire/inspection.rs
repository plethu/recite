//! Borrowed compact JSON views of compiled tables. Field declarations stay in
//! lexicographic order independently of serde_json's preserve_order feature.
use recite_core::{
    SourceSpan,
    compiled::{CompiledAssetHeader, CompiledDialogue, ContentFingerprint, TableRange},
};
use serde::{Serialize, Serializer, ser::SerializeStruct};

use super::shared::{hex_lower, range_to_u32};
use crate::compile::CompileError;

mod control;
mod rows;
mod text;
mod values;

#[cfg(test)]
mod tests;

pub(crate) fn serialize_inspection_json(
    dialogue: &CompiledDialogue,
) -> Result<String, CompileError> {
    serde_json::to_string(&Inspection(dialogue)).map_err(|error| {
        CompileError::Serialization(format!("failed to encode inspection JSON: {error}"))
    })
}

/// A format-specific view; compiled model types retain their existing codecs.
struct Inspection<'a, T: ?Sized>(&'a T);

/// Project rows as they are serialized, without allocating an intermediate table.
struct Rows<'a, T>(&'a [T]);

impl<T> Serialize for Rows<'_, T>
where
    for<'a> Inspection<'a, T>: Serialize,
{
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.0.iter().map(Inspection))
    }
}

impl Serialize for Inspection<'_, CompiledDialogue> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let dialogue = self.0;
        let mut object = serializer.serialize_struct("CompiledDialogue", 17)?;
        object.serialize_field(
            "availability_reasons",
            &Rows(&dialogue.availability_reasons),
        )?;
        object.serialize_field("block_lookup", &Rows(dialogue.block_lookup.as_slice()))?;
        object.serialize_field("blocks", &Rows(&dialogue.blocks))?;
        object.serialize_field("choice_lookup", &Rows(dialogue.choice_lookup.as_slice()))?;
        object.serialize_field("choices", &Rows(&dialogue.choices))?;
        object.serialize_field(
            "condition_availability_reasons",
            &Rows(&dialogue.condition_availability_reasons),
        )?;
        object.serialize_field("default_block", &dialogue.default_block.as_u32())?;
        object.serialize_field("effects", &Rows(&dialogue.effects))?;
        object.serialize_field("header", &Inspection(&dialogue.header))?;
        object.serialize_field("line_lookup", &Rows(dialogue.line_lookup.as_slice()))?;
        object.serialize_field("lines", &Rows(&dialogue.lines))?;
        object.serialize_field("match_arms", &Rows(&dialogue.match_arms))?;
        object.serialize_field("metadata", &Rows(&dialogue.metadata))?;
        object.serialize_field("source_maps", &Rows(&dialogue.source_maps))?;
        object.serialize_field("sources", &Rows(&dialogue.sources))?;
        object.serialize_field("speakers", &Rows(&dialogue.speakers))?;
        object.serialize_field("statements", &Rows(&dialogue.statements))?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledAssetHeader> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let header = self.0;
        let mut object = serializer.serialize_struct("CompiledAssetHeader", 8)?;
        object.serialize_field("asset_id", header.asset_id.as_str())?;
        object.serialize_field(
            "compiler_compatibility_version",
            &header.compiler_compatibility_version,
        )?;
        object.serialize_field("compiler_version", header.compiler_version.as_str())?;
        object.serialize_field("format_version", &header.format_version)?;
        object.serialize_field(
            "inspection_encoding",
            &Inspection(&header.inspection_encoding),
        )?;
        object.serialize_field("primary_encoding", &Inspection(&header.primary_encoding))?;
        object.serialize_field(
            "schema_fingerprint",
            &Inspection(&header.schema_fingerprint),
        )?;
        object.serialize_field("source_map_id", header.source_map_id.as_str())?;
        object.end()
    }
}

impl Serialize for Inspection<'_, ContentFingerprint> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let fingerprint = self.0;
        let mut object = serializer.serialize_struct("ContentFingerprint", 2)?;
        object.serialize_field("algorithm", fingerprint.algorithm().as_str())?;
        object.serialize_field("digest", &hex_lower(fingerprint.digest().as_bytes()))?;
        object.end()
    }
}

impl Serialize for Inspection<'_, SourceSpan> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let span = self.0;
        let mut object = serializer.serialize_struct("SourceSpan", 5)?;
        object.serialize_field("end_column", &span.end.map(|position| position.column()))?;
        object.serialize_field("end_line", &span.end.map(|position| position.line()))?;
        object.serialize_field("file", span.file.as_str())?;
        object.serialize_field("start_column", &span.start.column())?;
        object.serialize_field("start_line", &span.start.line())?;
        object.end()
    }
}

#[derive(Serialize)]
struct Range {
    len: u32,
    start: u32,
}

fn range<I: Copy>(range: TableRange<I>, index: impl Fn(I) -> u32) -> Range {
    let (start, len) = range_to_u32(range, index);
    Range { len, start }
}

fn serialize_tagged<S: Serializer, T: Serialize + ?Sized>(
    serializer: S,
    tag: &'static str,
    payload: &T,
) -> Result<S::Ok, S::Error> {
    let mut object = serializer.serialize_struct("Tagged", 2)?;
    object.serialize_field("payload", payload)?;
    object.serialize_field("tag", tag)?;
    object.end()
}
