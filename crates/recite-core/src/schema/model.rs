use std::collections::{BTreeMap, BTreeSet};

use crate::compiled::{SchemaFingerprint, canonical_blake3_fingerprint};
use crate::{AvailabilityReasonId, ContentFingerprint, EffectMode};

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
        let mut canonical = CanonicalSchemaBytes::new();
        canonical.field_u32("schema_version", self.schema_version);
        canonical.types(&self.types);
        canonical.registries(&self.registries);
        canonical.speakers(&self.speakers);
        canonical.conditions(&self.conditions);
        canonical.availability_reasons(&self.availability_reasons);
        canonical.effects(&self.effects);
        canonical.metadata_domains(&self.metadata_domains);
        canonical.metadata(&self.metadata);
        canonical.markup(&self.markup);
        canonical_blake3_fingerprint(canonical.as_bytes())
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

struct CanonicalSchemaBytes {
    bytes: Vec<u8>,
}

impl CanonicalSchemaBytes {
    fn new() -> Self {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"recite-schema-fingerprint-v1\0");
        Self { bytes }
    }

    fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn section(&mut self, name: &str, len: usize) {
        self.token("section");
        self.string(name);
        self.usize(len);
    }

    fn entry(&mut self, key: &str) {
        self.token("entry");
        self.string(key);
    }

    fn field(&mut self, name: &str) {
        self.token("field");
        self.string(name);
    }

    fn field_bool(&mut self, name: &str, value: bool) {
        self.field(name);
        self.bool(value);
    }

    fn field_u32(&mut self, name: &str, value: u32) {
        self.field(name);
        self.u32(value);
    }

    fn field_string(&mut self, name: &str, value: &str) {
        self.field(name);
        self.string(value);
    }

    fn field_option_string(&mut self, name: &str, value: Option<&str>) {
        self.field(name);
        self.option_string(value);
    }

    fn types(&mut self, types: &BTreeMap<String, SchemaTypeDefinition>) {
        self.section("types", types.len());
        for (name, definition) in types {
            self.entry(name);
            match definition {
                SchemaTypeDefinition::Enum(definition) => {
                    self.field_string("kind", "enum");
                    self.string_set("values", &definition.values);
                }
            }
        }
    }

    fn registries(&mut self, registries: &BTreeMap<String, RegistryDefinition>) {
        self.section("registries", registries.len());
        for (name, definition) in registries {
            self.entry(name);
            self.string_set("values", &definition.values);
            self.field_option_string("origin", definition.origin.as_deref());
        }
    }

    fn speakers(&mut self, speakers: &BTreeMap<String, SpeakerDefinition>) {
        self.section("speakers", speakers.len());
        for (name, definition) in speakers {
            self.entry(name);
            self.field_option_string("display_name", definition.display_name.as_deref());
        }
    }

    fn conditions(&mut self, conditions: &BTreeMap<String, ConditionDefinition>) {
        self.section("conditions", conditions.len());
        for (name, definition) in conditions {
            self.entry(name);
            self.params("params", &definition.params);
            self.field("returns");
            match &definition.returns {
                ConditionReturnType::Bool => self.string("bool"),
                ConditionReturnType::Enum(name) => {
                    self.token("enum");
                    self.string(name);
                }
            }
            self.field("availability_reason");
            self.availability_reason_mapping(definition.availability_reason.as_ref());
        }
    }

    fn availability_reasons(
        &mut self,
        reasons: &BTreeMap<AvailabilityReasonId, AvailabilityReasonDefinition>,
    ) {
        self.section("availability_reasons", reasons.len());
        for (id, definition) in reasons {
            self.entry(id.as_str());
            self.field_string("template", &definition.template);
            self.params("params", &definition.params);
            self.field_option_string("origin", definition.origin.as_deref());
        }
    }

    fn effects(&mut self, effects: &BTreeMap<String, EffectDefinition>) {
        self.section("effects", effects.len());
        for (name, definition) in effects {
            self.entry(name);
            self.effect_modes("modes", &definition.modes);
            self.params("params", &definition.params);
        }
    }

    fn metadata(&mut self, metadata: &BTreeMap<String, MetadataDefinition>) {
        self.section("metadata", metadata.len());
        for (name, definition) in metadata {
            self.entry(name);
            self.metadata_targets("targets", &definition.targets);
            self.field("type");
            self.type_ref(&definition.type_ref);
            self.field_bool("repeatable", definition.repeatable);
            self.field_option_string("domain", definition.domain.as_deref());
        }
    }

    fn metadata_domains(&mut self, domains: &BTreeMap<String, MetadataDomainDefinition>) {
        self.section("metadata_domains", domains.len());
        for (name, definition) in domains {
            self.entry(name);
            match definition {
                MetadataDomainDefinition::Flat(domain) => {
                    self.field_string("kind", "flat");
                    self.string_set("values", &domain.values);
                }
                MetadataDomainDefinition::Contextual(domain) => {
                    self.field_string("kind", "contextual");
                    self.field("selector");
                    self.metadata_context_selector(&domain.selector);
                    self.field("values_by_context");
                    self.usize(domain.values_by_context.len());
                    for (context, values) in &domain.values_by_context {
                        self.entry(context);
                        self.string_set("values", values);
                    }
                    self.field("missing_context");
                    self.missing_metadata_context_policy(&domain.missing_context);
                }
            }
        }
    }

    fn markup(&mut self, markup: &BTreeMap<String, MarkupDefinition>) {
        self.section("markup", markup.len());
        for (name, definition) in markup {
            self.entry(name);
            self.field_bool("requires_closing", definition.requires_closing);
            self.field_bool("translatable", definition.translatable);
            self.field_bool("allows_nesting", definition.allows_nesting);
        }
    }

    fn params(&mut self, name: &str, params: &[ParameterDefinition]) {
        self.field(name);
        self.usize(params.len());
        for param in params {
            self.entry(&param.name);
            self.field("type");
            self.type_ref(&param.type_ref);
        }
    }

    fn availability_reason_mapping(
        &mut self,
        mapping: Option<&ConditionAvailabilityReasonMapping>,
    ) {
        match mapping {
            Some(mapping) => {
                self.token("some");
                self.field_string("reason", mapping.reason.as_str());
                self.field("args");
                self.usize(mapping.args.len());
                for (name, binding) in &mapping.args {
                    self.entry(name);
                    self.availability_reason_arg_binding(binding);
                }
            }
            None => self.token("none"),
        }
    }

    fn availability_reason_arg_binding(&mut self, binding: &AvailabilityReasonArgBinding) {
        match binding {
            AvailabilityReasonArgBinding::ConditionParam(name) => {
                self.token("condition_param");
                self.string(name);
            }
            AvailabilityReasonArgBinding::Literal(value) => {
                self.token("literal");
                self.schema_literal_value(value);
            }
        }
    }

    fn schema_literal_value(&mut self, value: &SchemaLiteralValue) {
        match value {
            SchemaLiteralValue::String(value) => {
                self.token("string");
                self.string(value);
            }
            SchemaLiteralValue::Int(value) => {
                self.token("int");
                self.bytes.extend_from_slice(&value.to_le_bytes());
            }
            SchemaLiteralValue::Float(value) => {
                self.token("float");
                self.string(value);
            }
            SchemaLiteralValue::Bool(value) => {
                self.token("bool");
                self.bool(*value);
            }
        }
    }

    fn string_set(&mut self, name: &str, values: &BTreeSet<String>) {
        self.field(name);
        self.usize(values.len());
        for value in values {
            self.string(value);
        }
    }

    fn effect_modes(&mut self, name: &str, modes: &BTreeSet<EffectMode>) {
        self.field(name);
        self.usize(modes.len());
        for mode in modes {
            self.string(effect_mode_name(*mode));
        }
    }

    fn metadata_targets(&mut self, name: &str, targets: &BTreeSet<MetadataTarget>) {
        self.field(name);
        self.usize(targets.len());
        for target in targets {
            self.string(metadata_target_name(*target));
        }
    }

    fn type_ref(&mut self, type_ref: &SchemaTypeRef) {
        match type_ref {
            SchemaTypeRef::String => self.string("string"),
            SchemaTypeRef::Symbol => self.string("symbol"),
            SchemaTypeRef::Int => self.string("int"),
            SchemaTypeRef::Float => self.string("float"),
            SchemaTypeRef::Bool => self.string("bool"),
            SchemaTypeRef::Speaker => self.string("speaker"),
            SchemaTypeRef::Enum(name) => {
                self.token("enum");
                self.string(name);
            }
            SchemaTypeRef::Registry(name) => {
                self.token("registry");
                self.string(name);
            }
        }
    }

    fn metadata_context_selector(&mut self, selector: &MetadataContextSelector) {
        match selector {
            MetadataContextSelector::FieldSpeaker => self.string("field:speaker"),
            MetadataContextSelector::MetadataKey(key) => {
                self.token("metadata");
                self.string(key);
            }
        }
    }

    fn missing_metadata_context_policy(&mut self, policy: &MissingMetadataContextPolicy) {
        match policy {
            MissingMetadataContextPolicy::Diagnostic => self.string("diagnostic"),
            MissingMetadataContextPolicy::Empty => self.string("empty"),
            MissingMetadataContextPolicy::Fallback { domain } => {
                self.token("fallback");
                self.string(domain);
            }
        }
    }

    fn option_string(&mut self, value: Option<&str>) {
        match value {
            Some(value) => {
                self.token("some");
                self.string(value);
            }
            None => self.token("none"),
        }
    }

    fn bool(&mut self, value: bool) {
        self.token(if value { "true" } else { "false" });
    }

    fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    // Invariant: supported Recite targets have usize widths no larger than u64.
    #[allow(clippy::expect_used)]
    fn usize(&mut self, value: usize) {
        let value = u64::try_from(value).expect("schema collection length fits into u64");
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn string(&mut self, value: &str) {
        self.usize(value.len());
        self.bytes.extend_from_slice(value.as_bytes());
    }

    fn token(&mut self, value: &str) {
        self.string(value);
    }
}

fn effect_mode_name(mode: EffectMode) -> &'static str {
    match mode {
        EffectMode::Deferred => "deferred",
        EffectMode::Immediate => "immediate",
        EffectMode::Blocking => "blocking",
    }
}

fn metadata_target_name(target: MetadataTarget) -> &'static str {
    match target {
        MetadataTarget::Block => "block",
        MetadataTarget::Choice => "choice",
        MetadataTarget::Line => "line",
        MetadataTarget::Project => "project",
    }
}
