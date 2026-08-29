use std::collections::{BTreeMap, BTreeSet};

use crate::compiled::canonical_blake3_fingerprint;
use crate::{AvailabilityReasonId, ContentFingerprint, EffectMode};

use super::{
    AvailabilityReasonArgBinding, AvailabilityReasonDefinition, ConditionAvailabilityReasonMapping,
    ConditionDefinition, ConditionReturnType, EffectDefinition, MarkupDefinition,
    MetadataContextSelector, MetadataDefinition, MetadataDomainDefinition, MetadataOccurrence,
    MetadataTarget, MissingMetadataContextPolicy, ParameterDefinition,
    PresentationAffordanceFieldDefinition, PresentationAffordanceFieldSource,
    PresentationAffordanceOutputDefinition, PresentationLabelArgDefinition,
    PresentationLabelDefinition, ProjectSchema, ProjectionInput, ProjectionInputRef,
    ProjectionOutputTarget, ProjectionQueryDefinition, ProjectionQueryFunctionDefinition,
    RegistryDefinition, SchemaLiteralValue, SchemaPresentationProjectorDefinition,
    SchemaProjectionInputSource, SchemaProjectionSelector, SchemaTypeDefinition, SchemaTypeRef,
    SpeakerDefinition,
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
    bytes.projection_queries(&schema.projection_queries);
    bytes.presentation_projectors(&schema.presentation_projectors);
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

    fn projection_queries(
        &mut self,
        queries: &BTreeMap<String, ProjectionQueryFunctionDefinition>,
    ) {
        self.section("projection_queries", queries.len());
        for (name, definition) in queries {
            self.entry(name);
            self.params("params", &definition.params);
            self.field("returns");
            self.type_ref(&definition.returns);
            self.field("max_calls_per_event");
            match definition.max_calls_per_event {
                Some(value) => {
                    self.token("some");
                    self.u32(value);
                }
                None => self.token("none"),
            }
        }
    }

    fn presentation_projectors(
        &mut self,
        projectors: &BTreeMap<String, SchemaPresentationProjectorDefinition>,
    ) {
        self.section("presentation_projectors", projectors.len());
        for (name, definition) in projectors {
            self.entry(name);
            self.field("candidates");
            self.schema_projection_selector(&definition.candidates);
            self.projection_inputs(&definition.inputs);
            self.projection_query_definitions(&definition.queries);
            self.presentation_outputs(&definition.outputs);
        }
    }

    fn projection_inputs(&mut self, inputs: &[ProjectionInput]) {
        self.field("inputs");
        self.usize(inputs.len());
        for input in inputs {
            self.entry(&input.name);
            self.field("source");
            self.schema_projection_input_source(&input.source);
            self.field("type");
            self.type_ref(&input.type_ref);
            self.field_bool("required", input.required);
        }
    }

    fn projection_query_definitions(
        &mut self,
        queries: &BTreeMap<String, ProjectionQueryDefinition>,
    ) {
        self.field("queries");
        self.usize(queries.len());
        for (name, query) in queries {
            self.entry(name);
            self.field_string("function", &query.function);
            self.projection_input_refs("args", &query.args);
        }
    }

    fn presentation_outputs(
        &mut self,
        outputs: &BTreeMap<String, PresentationAffordanceOutputDefinition>,
    ) {
        self.field("outputs");
        self.usize(outputs.len());
        for (name, output) in outputs {
            self.entry(name);
            self.field("target");
            self.projection_output_target(&output.target);
            self.field_string("kind", &output.kind);
            self.field_string("slot", &output.slot);
            self.field("label");
            match &output.label {
                Some(label) => {
                    self.token("some");
                    self.presentation_label(label);
                }
                None => self.token("none"),
            }
            self.field("fields");
            self.usize(output.fields.len());
            for (name, field) in &output.fields {
                self.entry(name);
                self.presentation_field(field);
            }
        }
    }

    fn presentation_label(&mut self, label: &PresentationLabelDefinition) {
        self.field_string("template_id", &label.template_id);
        self.field_string("source_text", &label.source_text);
        self.field("args");
        self.usize(label.args.len());
        for (name, arg) in &label.args {
            self.entry(name);
            self.presentation_label_arg(arg);
        }
    }

    fn presentation_label_arg(&mut self, arg: &PresentationLabelArgDefinition) {
        self.field("source");
        self.projection_input_ref(&arg.source);
        self.field("type");
        self.type_ref(&arg.type_ref);
    }

    fn presentation_field(&mut self, field: &PresentationAffordanceFieldDefinition) {
        self.field("source");
        self.presentation_field_source(&field.source);
        self.field("type");
        self.type_ref(&field.type_ref);
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
            SchemaTypeRef::Array(inner) => {
                self.token("array");
                self.type_ref(inner);
            }
        }
    }

    fn schema_projection_selector(&mut self, selector: &SchemaProjectionSelector) {
        match selector {
            SchemaProjectionSelector::RuntimeEvent { kind } => {
                self.token("runtime_event");
                self.string(kind);
            }
            SchemaProjectionSelector::MetadataKey { target, key } => {
                self.token("metadata_key");
                self.metadata_target(*target);
                self.string(key);
            }
            SchemaProjectionSelector::MetadataSet {
                target,
                required_keys,
            } => {
                self.token("metadata_set");
                self.metadata_target(*target);
                self.usize(required_keys.len());
                for key in required_keys {
                    self.string(key);
                }
            }
            SchemaProjectionSelector::AvailabilityReason { reason_id } => {
                self.token("availability_reason");
                self.string(reason_id.as_str());
            }
        }
    }

    fn schema_projection_input_source(&mut self, source: &SchemaProjectionInputSource) {
        match source {
            SchemaProjectionInputSource::EventKind => self.string("event_kind"),
            SchemaProjectionInputSource::CandidateLineId => self.string("candidate_line_id"),
            SchemaProjectionInputSource::CandidateChoiceId => self.string("candidate_choice_id"),
            SchemaProjectionInputSource::CandidateEffectRequestId => {
                self.string("candidate_effect_request_id");
            }
            SchemaProjectionInputSource::CandidateBlockId => self.string("candidate_block_id"),
            SchemaProjectionInputSource::CandidateProject => self.string("candidate_project"),
            SchemaProjectionInputSource::CandidateMetadata { key, occurrence } => {
                self.token("candidate_metadata");
                self.string(key);
                self.metadata_occurrence(occurrence);
            }
            SchemaProjectionInputSource::AvailabilityReasonArg { name } => {
                self.token("availability_reason_arg");
                self.string(name);
            }
            SchemaProjectionInputSource::Literal(value) => {
                self.token("literal");
                self.schema_literal_value(value);
            }
        }
    }

    fn projection_input_refs(&mut self, name: &str, refs: &[ProjectionInputRef]) {
        self.field(name);
        self.usize(refs.len());
        for input_ref in refs {
            self.projection_input_ref(input_ref);
        }
    }

    fn projection_input_ref(&mut self, input_ref: &ProjectionInputRef) {
        match input_ref {
            ProjectionInputRef::Input { name } => {
                self.token("input");
                self.string(name);
            }
            ProjectionInputRef::QueryResult { name } => {
                self.token("query_result");
                self.string(name);
            }
        }
    }

    fn presentation_field_source(&mut self, source: &PresentationAffordanceFieldSource) {
        match source {
            PresentationAffordanceFieldSource::Input { name } => {
                self.token("input");
                self.string(name);
            }
            PresentationAffordanceFieldSource::QueryResult { name } => {
                self.token("query_result");
                self.string(name);
            }
            PresentationAffordanceFieldSource::Literal(value) => {
                self.token("literal");
                self.schema_literal_value(value);
            }
        }
    }

    fn projection_output_target(&mut self, target: &ProjectionOutputTarget) {
        match target {
            ProjectionOutputTarget::Candidate => self.string("candidate"),
            ProjectionOutputTarget::Event => self.string("event"),
            ProjectionOutputTarget::Prompt => self.string("prompt"),
        }
    }

    fn metadata_target(&mut self, target: MetadataTarget) {
        self.string(metadata_target_name(target));
    }

    fn metadata_occurrence(&mut self, occurrence: &MetadataOccurrence) {
        match occurrence {
            MetadataOccurrence::Only => self.string("only"),
            MetadataOccurrence::First => self.string("first"),
            MetadataOccurrence::Last => self.string("last"),
            MetadataOccurrence::Index(index) => {
                self.token("index");
                self.u32(*index);
            }
            MetadataOccurrence::All => self.string("all"),
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
