//! Canonical project schema model and generated manifest loading.

mod diagnostics;
mod manifest;
mod model;
mod source;

pub(super) use diagnostics::schema_diagnostic;
pub(crate) use model::{ProducerContentFingerprintError, producer_content_fingerprint_detailed};

pub use manifest::{
    SchemaLoadReport, load_schema_manifest_for_freshness_str, load_schema_manifest_str,
};
pub use model::{
    AvailabilityReasonArgBinding, AvailabilityReasonDefinition, ConditionAvailabilityReasonMapping,
    ConditionDefinition, ConditionReturnType, ContentFingerprintFreshness,
    ContextualMetadataDomain, ContextualMetadataProvenance, EffectDefinition, EnumTypeDefinition,
    FlatMetadataDomain, FlatMetadataProvenance, MarkupDefinition, MetadataContextSelector,
    MetadataDefinition, MetadataDomainDefinition, MetadataOccurrence, MetadataTarget,
    MissingMetadataContextPolicy, ParameterDefinition, PresentationAffordanceFieldDefinition,
    PresentationAffordanceFieldSource, PresentationAffordanceOutputDefinition,
    PresentationLabelArgDefinition, PresentationLabelDefinition, ProducerFingerprint,
    ProducerFingerprintMismatch, ProducerFreshness, ProducerIdentity, ProducerIdentityError,
    ProducerIdentityPart, ProducerMetadata, ProducerMetadataValue, ProducerOrigin, ProjectSchema,
    ProjectionInput, ProjectionInputRef, ProjectionOutputTarget, ProjectionQueryDefinition,
    ProjectionQueryFunctionDefinition, RegistryDefinition, SchemaLiteralValue,
    SchemaPresentationProjectorDefinition, SchemaProducerFreshness, SchemaProjectionInputSource,
    SchemaProjectionSelector, SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition,
    canonical_schema_fingerprint, compare_producer_fingerprints, compare_schema_producer_freshness,
    compare_schema_producer_freshness_detailed, producer_content_fingerprint,
};
pub use source::{
    SchemaDeclarationKind, SchemaSource, SchemaSourceEdit, SchemaSourceEditError,
    SchemaSourceLoadReport, load_schema_source_str,
};
