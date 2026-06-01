//! Canonical project schema model and generated manifest loading.

mod manifest;
mod model;

pub use manifest::{SchemaLoadReport, load_schema_manifest_str};
pub use model::{
    ConditionDefinition, ConditionReturnType, ContextualMetadataDomain, EffectDefinition,
    EnumTypeDefinition, FlatMetadataDomain, MarkupDefinition, MetadataContextSelector,
    MetadataDefinition, MetadataDomainDefinition, MetadataTarget, MissingMetadataContextPolicy,
    ParameterDefinition, ProjectSchema, RegistryDefinition, SchemaTypeDefinition, SchemaTypeRef,
    SpeakerDefinition, canonical_schema_fingerprint,
};
