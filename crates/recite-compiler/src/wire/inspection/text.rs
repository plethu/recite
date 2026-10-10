//! Text rows retain authored/plural forms and interpolation bindings.
use recite_core::{
    AvailabilityReasonId,
    ast::InterpolationType,
    compiled::{
        CompiledChoice, CompiledInterpolationBinding, CompiledLine, MetadataIndex, SpeakerIndex,
    },
};
use serde::{Serialize, Serializer, ser::SerializeStruct};

use super::{Inspection, Rows, range};

impl Serialize for Inspection<'_, CompiledLine> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let line = self.0;
        let mut object = serializer.serialize_struct("CompiledLine", 9)?;
        object.serialize_field(
            "authored_plural_source_text",
            &line.authored_plural_source_text,
        )?;
        object.serialize_field("authored_source_text", line.authored_source_text.as_str())?;
        object.serialize_field("id", line.id.as_str())?;
        object.serialize_field(
            "interpolation_bindings",
            &Rows(&line.interpolation_bindings),
        )?;
        object.serialize_field("metadata", &range(line.metadata, MetadataIndex::as_u32))?;
        object.serialize_field("plural_source_text", &line.plural_source_text)?;
        object.serialize_field("source_map", &line.source_map.as_u32())?;
        object.serialize_field("source_text", line.source_text.as_str())?;
        object.serialize_field("speaker", &line.speaker.map(SpeakerIndex::as_u32))?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledChoice> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let choice = self.0;
        let mut object = serializer.serialize_struct("CompiledChoice", 11)?;
        object.serialize_field("authored_source_text", choice.authored_source_text.as_str())?;
        object.serialize_field(
            "availability_reason_override",
            &choice
                .availability_reason_override
                .as_ref()
                .map(AvailabilityReasonId::as_str),
        )?;
        object.serialize_field(
            "availability_requirement",
            &choice.availability_requirement.as_ref().map(Inspection),
        )?;
        object.serialize_field(
            "availability_requirement_source_text",
            &choice.availability_requirement_source_text,
        )?;
        object.serialize_field("echo", &Inspection(&choice.echo))?;
        object.serialize_field("id", choice.id.as_str())?;
        object.serialize_field(
            "interpolation_bindings",
            &Rows(&choice.interpolation_bindings),
        )?;
        object.serialize_field("metadata", &range(choice.metadata, MetadataIndex::as_u32))?;
        object.serialize_field("source_map", &choice.source_map.as_u32())?;
        object.serialize_field("source_text", choice.source_text.as_str())?;
        object.serialize_field("target", &Inspection(&choice.target))?;
        object.end()
    }
}

impl Serialize for Inspection<'_, CompiledInterpolationBinding> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let binding = self.0;
        let value_type = match binding.value_type {
            InterpolationType::String => "string",
            InterpolationType::Integer => "int",
            InterpolationType::Float => "float",
            InterpolationType::Boolean => "bool",
        };
        let mut object = serializer.serialize_struct("CompiledInterpolationBinding", 3)?;
        object.serialize_field("name", binding.name.as_str())?;
        object.serialize_field("type", value_type)?;
        object.serialize_field("value", binding.value.as_str())?;
        object.end()
    }
}
