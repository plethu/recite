use std::collections::{BTreeMap, BTreeSet};

use crate::compiled::SchemaFingerprint;
use crate::{AvailabilityReasonId, ContentFingerprint, EffectMode};

mod canonical;

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
    pub markup: BTreeMap<String, MarkupDefinition>,
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
            markup: BTreeMap::new(),
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
}

/// A generated snapshot of stable project content IDs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryDefinition {
    pub values: BTreeSet<String>,
    pub origin: Option<String>,
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
    pub origin: Option<String>,
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FlatMetadataDomain {
    pub values: BTreeSet<String>,
}

/// A deterministic context-indexed set of valid metadata symbol values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextualMetadataDomain {
    pub selector: MetadataContextSelector,
    pub values_by_context: BTreeMap<String, BTreeSet<String>>,
    pub missing_context: MissingMetadataContextPolicy,
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
