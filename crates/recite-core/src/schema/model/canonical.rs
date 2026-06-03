use std::collections::{BTreeMap, BTreeSet};

use crate::compiled::canonical_blake3_fingerprint;
use crate::{AvailabilityReasonId, ContentFingerprint, EffectMode};

use super::{
    AvailabilityReasonArgBinding, AvailabilityReasonDefinition, ConditionAvailabilityReasonMapping,
    ConditionDefinition, ConditionReturnType, EffectDefinition, FlatMetadataDomain,
    MarkupDefinition, MetadataContextSelector, MetadataDomainDefinition, MetadataDefinition,
    MetadataTarget, MissingMetadataContextPolicy, ParameterDefinition, ProjectSchema,
    RegistryDefinition, SchemaLiteralValue, SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition,
};

pub(super) fn compute_canonical_fingerprint(schema: &ProjectSchema) -> ContentFingerprint {
    let mut bytes = CanonicalSchemaBytes::new();
    bytes.field_u32("schema_version", schema.schema_version);
    bytes.types(&schema.types);
    bytes.registries(&schema.registries);
    bytes.speakers(&schema.speakers);
    bytes.conditions(&schema.conditions);
    bytes.availability_reasons(&schema.availability_reasons);
    bytes.effects(&schema.effects);
    bytes.metadata_domains(&schema.metadata_domains);
    bytes.metadata(&schema.metadata);
    bytes.markup(&schema.markup);
    canonical_blake3_fingerprint(bytes.as_bytes())
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
