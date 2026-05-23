//! Canonical project schema model and generated manifest loading.

mod manifest;
mod model;

pub use manifest::{SchemaLoadReport, load_schema_manifest_str};
pub use model::{
    ConditionDefinition, ConditionReturnType, EffectDefinition, EnumTypeDefinition,
    MarkupDefinition, MetadataDefinition, MetadataTarget, ParameterDefinition, ProjectSchema,
    RegistryDefinition, SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition,
    canonical_schema_fingerprint,
};
