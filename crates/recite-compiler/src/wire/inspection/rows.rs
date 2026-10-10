//! Object projections for compiled rows and sorted lookup entries.
use recite_core::compiled::{
    BlockLookupEntry, ChoiceLookupEntry, CompiledAvailabilityReason, CompiledBlock,
    CompiledConditionAvailabilityReason, CompiledEffect, CompiledMatchArm, CompiledMetadataEntry,
    CompiledSourceFile, CompiledSourceMapEntry, CompiledSpeaker, CompiledStatement,
    LineLookupEntry, MetadataIndex, SpeakerIndex, StatementIndex,
};
use serde::{Serialize, Serializer, ser::SerializeStruct};

use super::{Inspection, Rows, range};

impl Serialize for Inspection<'_, CompiledSourceFile> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let source = self.0;
        let mut object = serializer.serialize_struct("CompiledSourceFile", 2)?;
        object.serialize_field("fingerprint", &Inspection(&source.fingerprint))?;
        object.serialize_field("path", source.path.as_str())?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledBlock> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let block = self.0;
        let mut object = serializer.serialize_struct("CompiledBlock", 6)?;
        object.serialize_field(
            "default_speaker",
            &block.default_speaker.map(SpeakerIndex::as_u32),
        )?;
        object.serialize_field("id", block.id.as_str())?;
        object.serialize_field("metadata", &range(block.metadata, MetadataIndex::as_u32))?;
        object.serialize_field("source_file", &block.source_file.as_u32())?;
        object.serialize_field("source_map", &block.source_map.as_u32())?;
        object.serialize_field(
            "statements",
            &range(block.statements, StatementIndex::as_u32),
        )?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledStatement> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let statement = self.0;
        let mut object = serializer.serialize_struct("CompiledStatement", 2)?;
        object.serialize_field("kind", &Inspection(&statement.kind))?;
        object.serialize_field("source_map", &statement.source_map.as_u32())?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledMatchArm> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let arm = self.0;
        let mut object = serializer.serialize_struct("CompiledMatchArm", 3)?;
        object.serialize_field("pattern", &Inspection(&arm.pattern))?;
        object.serialize_field("source_map", &arm.source_map.as_u32())?;
        object.serialize_field("statements", &range(arm.statements, StatementIndex::as_u32))?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledAvailabilityReason> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let reason = self.0;
        let mut object = serializer.serialize_struct("CompiledAvailabilityReason", 2)?;
        object.serialize_field("id", reason.id.as_str())?;
        object.serialize_field("template", reason.template.as_str())?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledConditionAvailabilityReason> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mapping = self.0;
        let mut object = serializer.serialize_struct("CompiledConditionAvailabilityReason", 3)?;
        object.serialize_field("args", &Rows(&mapping.args))?;
        object.serialize_field("function", mapping.function.as_str())?;
        object.serialize_field("reason", mapping.reason.as_str())?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledSpeaker> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut object = serializer.serialize_struct("CompiledSpeaker", 1)?;
        object.serialize_field("id", self.0.id.as_str())?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledMetadataEntry> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let entry = self.0;
        let mut object = serializer.serialize_struct("CompiledMetadataEntry", 3)?;
        object.serialize_field("key", entry.key.as_str())?;
        object.serialize_field("source_map", &entry.source_map.map(|index| index.as_u32()))?;
        object.serialize_field("value", &Inspection(&entry.value))?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledEffect> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let effect = self.0;
        let mut object = serializer.serialize_struct("CompiledEffect", 5)?;
        object.serialize_field("args", &Rows(&effect.args))?;
        object.serialize_field("function", effect.function.as_str())?;
        object.serialize_field("id", effect.id.as_str())?;
        object.serialize_field("mode", &Inspection(&effect.mode))?;
        object.serialize_field("source_map", &effect.source_map.as_u32())?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledSourceMapEntry> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let entry = self.0;
        let mut object = serializer.serialize_struct("CompiledSourceMapEntry", 2)?;
        object.serialize_field("source_file", &entry.source_file.as_u32())?;
        object.serialize_field("span", &Inspection(&entry.span))?;
        object.end()
    }
}

impl Serialize for Inspection<'_, BlockLookupEntry> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut object = serializer.serialize_struct("BlockLookupEntry", 2)?;
        object.serialize_field("id", self.0.id.as_str())?;
        object.serialize_field("index", &self.0.index.as_u32())?;
        object.end()
    }
}

impl Serialize for Inspection<'_, LineLookupEntry> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut object = serializer.serialize_struct("LineLookupEntry", 2)?;
        object.serialize_field("id", self.0.id.as_str())?;
        object.serialize_field("index", &self.0.index.as_u32())?;
        object.end()
    }
}

impl Serialize for Inspection<'_, ChoiceLookupEntry> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut object = serializer.serialize_struct("ChoiceLookupEntry", 2)?;
        object.serialize_field("id", self.0.id.as_str())?;
        object.serialize_field("index", &self.0.index.as_u32())?;
        object.end()
    }
}
