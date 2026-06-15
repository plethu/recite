//! Canonical project schema model and generated manifest loading.

mod manifest;
mod model;

pub use manifest::{SchemaLoadReport, load_schema_manifest_str};
pub use model::{
    AvailabilityReasonArgBinding, AvailabilityReasonDefinition, ConditionAvailabilityReasonMapping,
    ConditionDefinition, ConditionReturnType, ContextualMetadataDomain, EffectDefinition,
    EnumTypeDefinition, FlatMetadataDomain, MarkupDefinition, MetadataContextSelector,
    MetadataDefinition, MetadataDomainDefinition, MetadataOccurrence, MetadataTarget,
    MissingMetadataContextPolicy, ParameterDefinition, PresentationAffordanceFieldDefinition,
    PresentationAffordanceFieldSource, PresentationAffordanceOutputDefinition,
    PresentationLabelArgDefinition, PresentationLabelDefinition, ProjectSchema, ProjectionInput,
    ProjectionInputRef, ProjectionOutputTarget, ProjectionQueryDefinition,
    ProjectionQueryFunctionDefinition, RegistryDefinition, SchemaLiteralValue,
    SchemaPresentationProjectorDefinition, SchemaProjectionInputSource, SchemaProjectionSelector,
    SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition, canonical_schema_fingerprint,
};
