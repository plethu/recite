use std::collections::{BTreeMap, BTreeSet};

use crate::EffectMode;

/// Canonical, deterministic schema model consumed by compiler and tooling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectSchema {
    pub schema_version: u32,
    pub types: BTreeMap<String, SchemaTypeDefinition>,
    pub registries: BTreeMap<String, RegistryDefinition>,
    pub speakers: BTreeMap<String, SpeakerDefinition>,
    pub conditions: BTreeMap<String, ConditionDefinition>,
    pub effects: BTreeMap<String, EffectDefinition>,
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
            effects: BTreeMap::new(),
            metadata: BTreeMap::new(),
            markup: BTreeMap::new(),
        }
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
}

/// Supported condition return domains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConditionReturnType {
    Bool,
    Enum(String),
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
