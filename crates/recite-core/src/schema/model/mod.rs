use std::collections::{BTreeMap, BTreeSet};

use crate::compiled::SchemaFingerprint;
use crate::{AvailabilityReasonId, ContentFingerprint, EffectMode};

mod canonical;
mod freshness;
mod producer;
mod producer_validation;

pub use freshness::{
    ContentFingerprintFreshness, SchemaProducerFreshness, compare_schema_producer_freshness,
    compare_schema_producer_freshness_detailed,
};
pub use producer::{
    ContextualMetadataProvenance, FlatMetadataProvenance, ProducerFingerprint,
    ProducerFingerprintMismatch, ProducerFreshness, ProducerIdentity, ProducerIdentityError,
    ProducerIdentityPart, ProducerMetadata, ProducerMetadataValue, ProducerOrigin,
    compare_producer_fingerprints, producer_content_fingerprint,
};
pub(crate) use producer::{ProducerContentFingerprintError, producer_content_fingerprint_detailed};
pub(crate) use producer_validation::{is_json_number_lexeme, is_namespaced_extension_key};
#[must_use]
pub fn canonical_schema_fingerprint(schema: &ProjectSchema) -> SchemaFingerprint {
    SchemaFingerprint::Fingerprint(schema.canonical_content_fingerprint())
}

/// Canonical, deterministic schema model consumed by compiler and tooling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSchema {
    pub schema_version: u32,
    pub types: BTreeMap<String, SchemaTypeDefinition>,
    pub registries: BTreeMap<String, RegistryDefinition>,
    pub speakers: BTreeMap<String, SpeakerDefinition>,
    pub conditions: BTreeMap<String, ConditionDefinition>,
    pub availability_reasons: BTreeMap<AvailabilityReasonId, AvailabilityReasonDefinition>,
    pub effects: BTreeMap<String, EffectDefinition>,
    pub metadata_domains: BTreeMap<String, MetadataDomainDefinition>,
    pub metadata: BTreeMap<String, MetadataDefinition>,
    pub projection_queries: BTreeMap<String, ProjectionQueryFunctionDefinition>,
    pub presentation_projectors: BTreeMap<String, SchemaPresentationProjectorDefinition>,
    pub markup: BTreeMap<String, MarkupDefinition>,
    pub producer_metadata: Option<ProducerMetadata>,
}

impl ProjectSchema {
    #[must_use]
    pub fn empty_v1() -> Self {
        Self {
            schema_version: 1,
            types: BTreeMap::new(),
            registries: BTreeMap::new(),
            speakers: BTreeMap::new(),
            conditions: BTreeMap::new(),
            availability_reasons: BTreeMap::new(),
            effects: BTreeMap::new(),
            metadata_domains: BTreeMap::new(),
            metadata: BTreeMap::new(),
            projection_queries: BTreeMap::new(),
            presentation_projectors: BTreeMap::new(),
            markup: BTreeMap::new(),
            producer_metadata: None,
        }
    }

    #[must_use]
    pub fn canonical_fingerprint(&self) -> SchemaFingerprint {
        canonical_schema_fingerprint(self)
    }

    #[must_use]
    pub fn canonical_content_fingerprint(&self) -> ContentFingerprint {
        canonical::compute_canonical_fingerprint(self)
    }
}

/// Schema-level type definitions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaTypeDefinition {
    Enum(EnumTypeDefinition),
}

/// A closed set of stable string variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumTypeDefinition {
    pub values: BTreeSet<String>,
}

/// A type reference used by parameters, metadata, and constraints.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SchemaTypeRef {
    String,
    Symbol,
    Int,
    Float,
    Bool,
    Speaker,
    Enum(String),
    Registry(String),
    Array(Box<SchemaTypeRef>),
}

/// A generated snapshot of stable project content IDs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RegistryDefinition {
    pub values: BTreeSet<String>,
    pub origin: Option<ProducerOrigin>,
    pub value_origins: BTreeMap<String, ProducerOrigin>,
    pub producer_fingerprints: Vec<ProducerFingerprint>,
}

/// A declared speaker ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpeakerDefinition {
    pub display_name: Option<String>,
}

/// A named typed parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParameterDefinition {
    pub name: String,
    pub type_ref: SchemaTypeRef,
}

/// A declared condition function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionDefinition {
    pub params: Vec<ParameterDefinition>,
    pub returns: ConditionReturnType,
    pub availability_reason: Option<ConditionAvailabilityReasonMapping>,
}

/// Supported condition return domains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConditionReturnType {
    Bool,
    Enum(String),
}

/// A schema-owned localisable template for unavailable-choice explanations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AvailabilityReasonDefinition {
    pub template: String,
    pub params: Vec<ParameterDefinition>,
    pub origin: Option<ProducerOrigin>,
}

/// Default availability reason mapping declared on a boolean condition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionAvailabilityReasonMapping {
    pub reason: AvailabilityReasonId,
    pub args: BTreeMap<String, AvailabilityReasonArgBinding>,
}

/// Binding from an availability reason parameter to a condition argument or literal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AvailabilityReasonArgBinding {
    ConditionParam(String),
    Literal(SchemaLiteralValue),
}

/// Eq-safe schema literal used in canonical schema-owned mappings.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaLiteralValue {
    String(String),
    Int(i64),
    Float(String),
    Bool(bool),
}

/// A declared effect request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectDefinition {
    pub modes: BTreeSet<EffectMode>,
    pub params: Vec<ParameterDefinition>,
}

/// A declared metadata key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetadataDefinition {
    pub targets: BTreeSet<MetadataTarget>,
    pub type_ref: SchemaTypeRef,
    pub repeatable: bool,
    pub domain: Option<String>,
}

/// A named metadata value domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetadataDomainDefinition {
    Flat(FlatMetadataDomain),
    Contextual(ContextualMetadataDomain),
}

/// A deterministic flat set of valid metadata symbol values.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FlatMetadataDomain {
    pub values: BTreeSet<String>,
    pub provenance: FlatMetadataProvenance,
}

/// A deterministic context-indexed set of valid metadata symbol values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextualMetadataDomain {
    pub selector: MetadataContextSelector,
    pub values_by_context: BTreeMap<String, BTreeSet<String>>,
    pub missing_context: MissingMetadataContextPolicy,
    pub provenance: ContextualMetadataProvenance,
}

/// The v1 metadata context selector slice.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum MetadataContextSelector {
    FieldSpeaker,
    MetadataKey(String),
}

/// Policy used when a contextual domain selector cannot resolve context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MissingMetadataContextPolicy {
    Diagnostic,
    Empty,
    Fallback { domain: String },
}

/// Authoring targets that metadata may attach to.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum MetadataTarget {
    Block,
    Choice,
    Line,
    Project,
}

/// A declared inline markup tag.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkupDefinition {
    pub requires_closing: bool,
    pub translatable: bool,
    pub allows_nesting: bool,
}

/// A schema-global pure host query available to presentation projectors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionQueryFunctionDefinition {
    pub params: Vec<ParameterDefinition>,
    pub returns: SchemaTypeRef,
    pub max_calls_per_event: Option<u32>,
}

/// A schema-owned presentation projector declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaPresentationProjectorDefinition {
    pub candidates: SchemaProjectionSelector,
    pub inputs: Vec<ProjectionInput>,
    pub queries: BTreeMap<String, ProjectionQueryDefinition>,
    pub outputs: BTreeMap<String, PresentationAffordanceOutputDefinition>,
}

/// Candidate set selected by a schema-owned projector.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaProjectionSelector {
    RuntimeEvent {
        kind: String,
    },
    MetadataKey {
        target: MetadataTarget,
        key: String,
    },
    MetadataSet {
        target: MetadataTarget,
        required_keys: Vec<String>,
    },
    AvailabilityReason {
        reason_id: AvailabilityReasonId,
    },
}

/// A typed projector input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionInput {
    pub name: String,
    pub source: SchemaProjectionInputSource,
    pub type_ref: SchemaTypeRef,
    pub required: bool,
}

/// Schema-manifest input sources for presentation projection.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum SchemaProjectionInputSource {
    EventKind,
    CandidateLineId,
    CandidateChoiceId,
    CandidateEffectRequestId,
    CandidateBlockId,
    CandidateProject,
    CandidateMetadata {
        key: String,
        occurrence: MetadataOccurrence,
    },
    AvailabilityReasonArg {
        name: String,
    },
    Literal(SchemaLiteralValue),
}

/// How a projector reads repeated candidate metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MetadataOccurrence {
    Only,
    First,
    Last,
    Index(u32),
    All,
}

/// A named projector query call.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionQueryDefinition {
    pub function: String,
    pub args: Vec<ProjectionInputRef>,
}

/// Reference to a projector input or query result.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProjectionInputRef {
    Input { name: String },
    QueryResult { name: String },
}

/// A schema-owned output affordance declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationAffordanceOutputDefinition {
    pub target: ProjectionOutputTarget,
    pub kind: String,
    pub slot: String,
    pub label: Option<PresentationLabelDefinition>,
    pub fields: BTreeMap<String, PresentationAffordanceFieldDefinition>,
}

/// Runtime presentation target for an output affordance.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProjectionOutputTarget {
    Candidate,
    Event,
    Prompt,
}

/// Schema-owned localisable template for presentation labels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationLabelDefinition {
    pub template_id: String,
    pub source_text: String,
    pub args: BTreeMap<String, PresentationLabelArgDefinition>,
}

/// A typed presentation label placeholder binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationLabelArgDefinition {
    pub source: ProjectionInputRef,
    pub type_ref: SchemaTypeRef,
}

/// A typed output field binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PresentationAffordanceFieldDefinition {
    pub source: PresentationAffordanceFieldSource,
    pub type_ref: SchemaTypeRef,
}

/// Source for a typed output field.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum PresentationAffordanceFieldSource {
    Input { name: String },
    QueryResult { name: String },
    Literal(SchemaLiteralValue),
}
