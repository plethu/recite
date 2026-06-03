//! Canonical project schema model and generated manifest loading.

mod manifest;
mod model;

pub use manifest::{SchemaLoadReport, load_schema_manifest_str};
pub use model::{
    AvailabilityReasonArgBinding, AvailabilityReasonDefinition, ConditionAvailabilityReasonMapping,
    ConditionDefinition, ConditionReturnType, ContextualMetadataDomain, EffectDefinition,
    EnumTypeDefinition, FlatMetadataDomain, MarkupDefinition, MetadataContextSelector,
    MetadataDefinition, MetadataDomainDefinition, MetadataTarget, MissingMetadataContextPolicy,
    ParameterDefinition, ProjectSchema, RegistryDefinition, SchemaLiteralValue,
    SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition, canonical_schema_fingerprint,
};
