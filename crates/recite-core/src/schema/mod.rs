//! Canonical project schema model and generated manifest loading.

mod diagnostics;
mod manifest;
mod model;
mod source;

pub(super) use diagnostics::schema_diagnostic;
pub(crate) use model::{
    ProducerContentFingerprintError, is_json_number_lexeme, is_namespaced_extension_key,
    producer_content_fingerprint_detailed,
};

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
    SchemaSourceEditPlan, SchemaSourceLoadReport, SchemaSourceStaleDetails, load_schema_source_str,
};

/// Export a host-built schema through Recite's canonical generated-manifest
/// serializer. A mutable `ProjectSchema` may be constructed by an engine
/// producer, so the generated document is checked by the ordinary manifest
/// loader before it can be published.
pub fn export_schema_manifest_json(
    schema: &ProjectSchema,
) -> Result<String, Vec<crate::Diagnostic>> {
    let json = source::export_json(schema);
    let report = load_schema_manifest_str("<generated schema>", &json);
    if report.diagnostics.is_empty() && report.schema.is_some() {
        Ok(json)
    } else {
        Err(report.diagnostics)
    }
}

/// Export a validated schema under an engine-owned stable producer identity.
/// This replaces only the manifest transport owner and its input fingerprint;
/// authored declaration provenance and unrelated producer evidence remain.
pub fn export_schema_manifest_json_with_producer(
    schema: &ProjectSchema,
    producer: ProducerIdentity,
) -> Result<String, Vec<crate::Diagnostic>> {
    // Mutable host builders are untrusted; validate before stamping metadata.
    export_schema_manifest_json(schema)?;
    let mut stamped = schema.clone();
    {
        let metadata = stamped
            .producer_metadata
            .get_or_insert_with(|| ProducerMetadata {
                producer: None,
                content_fingerprint: None,
                schema_export_version: None,
                inclusion_policy: None,
                producer_fingerprints: Vec::new(),
            });
        if let Some(previous) = metadata.producer.as_ref() {
            metadata.producer_fingerprints.retain(|fingerprint| {
                fingerprint.kind != previous.kind() || fingerprint.id != previous.id()
            });
        }
        metadata.producer = Some(producer);
    }
    let fingerprint = source::source_fingerprint(&stamped);
    if let Some(fingerprint) = source::source_producer_fingerprint(&stamped, &fingerprint)
        && let Some(metadata) = stamped.producer_metadata.as_mut()
    {
        metadata.producer_fingerprints.push(fingerprint);
        metadata.producer_fingerprints.sort();
    }
    export_schema_manifest_json(&stamped)
}
