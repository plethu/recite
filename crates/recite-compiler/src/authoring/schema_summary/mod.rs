mod builder;
mod dialogue;
mod errors;
mod evidence;
mod freshness;
mod functions;
mod helpers;
mod identity;
mod metadata;
mod projections;

pub use dialogue::{RegistrySummary, SchemaTypeSummary, SpeakerSummary};
pub use errors::{FreshnessSnapshotSide, SchemaSummaryBuildError, SchemaSummaryEvidenceError};
pub use evidence::{
    ProducerCapabilityStatus, ProducerFailureEvidence, SchemaSummaryEvidence,
    SchemaSummaryEvidenceBuilder,
};
pub use freshness::{SchemaFreshnessEvidence, SchemaFreshnessSnapshotIdentity};
pub use functions::{AvailabilityReasonSummary, ConditionSummary, EffectSummary, MarkupSummary};
pub use identity::{
    ProducerMetadataSummary, SchemaAction, SchemaCapability, SchemaCapabilityUnavailableReason,
    SchemaDeclarationProvenance, SchemaFingerprintSummary, SchemaFreshness,
    SchemaFreshnessUnavailableReason, SchemaOwnership,
};
pub use metadata::{MetadataDomainSummary, MetadataKeySummary};
pub use projections::{PresentationProjectorSummary, ProjectionQueryFunctionSummary};

use recite_core::SchemaFingerprint;

/// A deterministic, host-neutral view of one canonical project schema.
///
/// This is a read-only projection of [`recite_core::ProjectSchema`]. It does
/// not parse manifests, validate schema semantics, invoke producer processes,
/// access a filesystem, or edit standalone source. Those responsibilities stay
/// with `recite-core` and the host client boundaries.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaSummary {
    schema_version: u32,
    source: SchemaSourceSummary,
    capability: SchemaCapability,
    fingerprints: SchemaFingerprintSummary,
    producer_metadata: Option<ProducerMetadataSummary>,
    freshness: SchemaFreshness,
    types: Vec<SchemaTypeSummary>,
    registries: Vec<RegistrySummary>,
    speakers: Vec<SpeakerSummary>,
    conditions: Vec<ConditionSummary>,
    availability_reasons: Vec<AvailabilityReasonSummary>,
    effects: Vec<EffectSummary>,
    metadata_domains: Vec<MetadataDomainSummary>,
    metadata: Vec<MetadataKeySummary>,
    projection_queries: Vec<ProjectionQueryFunctionSummary>,
    presentation_projectors: Vec<PresentationProjectorSummary>,
    markup: Vec<MarkupSummary>,
}

/// Source ownership evidence for a schema summary.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaSourceSummary {
    ownership: SchemaOwnership,
}

impl SchemaSourceSummary {
    #[must_use]
    pub const fn ownership(&self) -> &SchemaOwnership {
        &self.ownership
    }

    #[must_use]
    pub const fn generated_output_is_read_only(&self) -> bool {
        self.ownership.is_generated()
    }
}

impl SchemaSummary {
    /// The semantic schema fingerprint, retained as its canonical type.
    #[must_use]
    pub const fn semantic_fingerprint(&self) -> &SchemaFingerprint {
        self.fingerprints.semantic()
    }

    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    #[must_use]
    pub const fn source(&self) -> &SchemaSourceSummary {
        &self.source
    }

    #[must_use]
    pub const fn ownership(&self) -> &SchemaOwnership {
        self.source.ownership()
    }

    #[must_use]
    pub const fn capability(&self) -> &SchemaCapability {
        &self.capability
    }

    #[must_use]
    pub const fn fingerprints(&self) -> &SchemaFingerprintSummary {
        &self.fingerprints
    }

    #[must_use]
    pub const fn producer_metadata(&self) -> Option<&ProducerMetadataSummary> {
        self.producer_metadata.as_ref()
    }

    #[must_use]
    pub const fn freshness(&self) -> &SchemaFreshness {
        &self.freshness
    }

    #[must_use]
    pub fn types(&self) -> &[SchemaTypeSummary] {
        &self.types
    }

    #[must_use]
    pub fn registries(&self) -> &[RegistrySummary] {
        &self.registries
    }

    #[must_use]
    pub fn speakers(&self) -> &[SpeakerSummary] {
        &self.speakers
    }

    #[must_use]
    pub fn conditions(&self) -> &[ConditionSummary] {
        &self.conditions
    }

    #[must_use]
    pub fn availability_reasons(&self) -> &[AvailabilityReasonSummary] {
        &self.availability_reasons
    }

    #[must_use]
    pub fn effects(&self) -> &[EffectSummary] {
        &self.effects
    }

    #[must_use]
    pub fn metadata_domains(&self) -> &[MetadataDomainSummary] {
        &self.metadata_domains
    }

    #[must_use]
    pub fn metadata(&self) -> &[MetadataKeySummary] {
        &self.metadata
    }

    #[must_use]
    pub fn projection_queries(&self) -> &[ProjectionQueryFunctionSummary] {
        &self.projection_queries
    }

    #[must_use]
    pub fn presentation_projectors(&self) -> &[PresentationProjectorSummary] {
        &self.presentation_projectors
    }

    #[must_use]
    pub fn markup(&self) -> &[MarkupSummary] {
        &self.markup
    }
}

/// A name emphasizing that this is the shared authoring-facing schema view.
pub type AuthoringSchemaSummary = SchemaSummary;

/// Compatibility-friendly name for callers that refer to declarations first.
pub type SchemaDeclarationSummary = SchemaSummary;
