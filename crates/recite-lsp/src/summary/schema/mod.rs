mod items;
mod metadata;
mod render;

use recite_core::ProjectSchema;

#[allow(unused_imports)]
pub(crate) use items::{
    AvailabilityReasonSummary, ConditionSummary, EffectSummary, MarkupSummary, ParameterSummary,
    PresentationOutputSummary, PresentationProjectorSummary, ProjectionInputSummary,
    ProjectionQuerySummary, ProjectorQuerySummary, ProvenanceSummary, RegistrySummary,
    SchemaMetadataSummary, SpeakerSummary, TypeKindSummary, TypeSummary,
};
#[allow(unused_imports)]
pub(crate) use metadata::{
    ContextualDomainValuesSummary, MetadataContextSelectorSummary, MetadataDomainKindSummary,
    MetadataDomainSummary, MissingMetadataContextPolicySummary,
};
use render::{
    effect_mode_name, metadata_target_name, projection_output_target_name,
    projection_selector_summary, type_ref_summary,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SchemaSummary {
    pub(crate) schema_version: u32,
    pub(crate) producer_metadata: Option<ProducerMetadataSummary>,
    pub(crate) types: Vec<TypeSummary>,
    pub(crate) registries: Vec<RegistrySummary>,
    pub(crate) speakers: Vec<SpeakerSummary>,
    pub(crate) conditions: Vec<ConditionSummary>,
    pub(crate) availability_reasons: Vec<AvailabilityReasonSummary>,
    pub(crate) effects: Vec<EffectSummary>,
    pub(crate) projection_queries: Vec<ProjectionQuerySummary>,
    pub(crate) presentation_projectors: Vec<PresentationProjectorSummary>,
    pub(crate) metadata_domains: Vec<MetadataDomainSummary>,
    pub(crate) metadata: Vec<SchemaMetadataSummary>,
    pub(crate) markup: Vec<MarkupSummary>,
}

impl SchemaSummary {
    pub(crate) fn from_schema(schema: &ProjectSchema) -> Self {
        Self {
            schema_version: schema.schema_version,
            producer_metadata: schema
                .producer_metadata
                .as_ref()
                .map(ProducerMetadataSummary::from),
            types: schema
                .types
                .iter()
                .map(|(name, definition)| TypeSummary::from_definition(name, definition))
                .collect(),
            registries: schema
                .registries
                .iter()
                .map(|(name, definition)| RegistrySummary {
                    name: name.clone(),
                    values: definition.values.iter().cloned().collect(),
                    provenance: ProvenanceSummary::from_optional_origin(definition.origin.as_ref()),
                    value_provenance: definition
                        .value_origins
                        .iter()
                        .map(|(key, origin)| {
                            (
                                key.clone(),
                                ProvenanceSummary::from_optional_origin(Some(origin)),
                            )
                        })
                        .collect(),
                    producer_fingerprints: definition.producer_fingerprints.clone(),
                })
                .collect(),
            speakers: schema
                .speakers
                .iter()
                .map(|(name, definition)| SpeakerSummary {
                    name: name.clone(),
                    display_name: definition.display_name.clone(),
                })
                .collect(),
            conditions: schema
                .conditions
                .iter()
                .map(|(name, definition)| ConditionSummary {
                    name: name.clone(),
                    params: definition
                        .params
                        .iter()
                        .map(|param| ParameterSummary {
                            name: param.name.clone(),
                            type_ref: type_ref_summary(&param.type_ref),
                        })
                        .collect(),
                    returns: format!("{:?}", definition.returns),
                })
                .collect(),
            availability_reasons: schema
                .availability_reasons
                .iter()
                .map(|(name, definition)| AvailabilityReasonSummary {
                    name: name.to_string(),
                    provenance: ProvenanceSummary::from_optional_origin(definition.origin.as_ref()),
                })
                .collect(),
            effects: schema
                .effects
                .iter()
                .map(|(name, definition)| EffectSummary {
                    name: name.clone(),
                    modes: definition
                        .modes
                        .iter()
                        .map(|mode| effect_mode_name(*mode).to_owned())
                        .collect(),
                    params: definition
                        .params
                        .iter()
                        .map(|param| ParameterSummary {
                            name: param.name.clone(),
                            type_ref: type_ref_summary(&param.type_ref),
                        })
                        .collect(),
                })
                .collect(),
            projection_queries: schema
                .projection_queries
                .iter()
                .map(|(name, definition)| ProjectionQuerySummary {
                    name: name.clone(),
                    params: definition
                        .params
                        .iter()
                        .map(|param| ParameterSummary {
                            name: param.name.clone(),
                            type_ref: type_ref_summary(&param.type_ref),
                        })
                        .collect(),
                    returns: type_ref_summary(&definition.returns),
                    max_calls_per_event: definition.max_calls_per_event,
                })
                .collect(),
            presentation_projectors: schema
                .presentation_projectors
                .iter()
                .map(|(name, definition)| PresentationProjectorSummary {
                    name: name.clone(),
                    candidates: projection_selector_summary(&definition.candidates),
                    inputs: definition
                        .inputs
                        .iter()
                        .map(|input| ProjectionInputSummary {
                            name: input.name.clone(),
                            type_ref: type_ref_summary(&input.type_ref),
                            required: input.required,
                        })
                        .collect(),
                    queries: definition
                        .queries
                        .iter()
                        .map(|(name, query)| ProjectorQuerySummary {
                            name: name.clone(),
                            function: query.function.clone(),
                        })
                        .collect(),
                    outputs: definition
                        .outputs
                        .iter()
                        .map(|(name, output)| PresentationOutputSummary {
                            name: name.clone(),
                            target: projection_output_target_name(&output.target).to_owned(),
                            kind: output.kind.clone(),
                            slot: output.slot.clone(),
                            label_template: output
                                .label
                                .as_ref()
                                .map(|label| label.template_id.clone()),
                        })
                        .collect(),
                })
                .collect(),
            metadata_domains: schema
                .metadata_domains
                .iter()
                .map(|(name, definition)| MetadataDomainSummary::from_definition(name, definition))
                .collect(),
            metadata: schema
                .metadata
                .iter()
                .map(|(name, definition)| SchemaMetadataSummary {
                    name: name.clone(),
                    targets: definition
                        .targets
                        .iter()
                        .map(|target| metadata_target_name(*target).to_owned())
                        .collect(),
                    type_ref: type_ref_summary(&definition.type_ref),
                    repeatable: definition.repeatable,
                    domain: definition.domain.clone(),
                })
                .collect(),
            markup: schema
                .markup
                .iter()
                .map(|(name, definition)| MarkupSummary {
                    name: name.clone(),
                    requires_closing: definition.requires_closing,
                    translatable: definition.translatable,
                    allows_nesting: definition.allows_nesting,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProducerMetadataSummary {
    pub(crate) producer: Option<recite_core::ProducerIdentity>,
    pub(crate) content_fingerprint: Option<recite_core::ContentFingerprint>,
    pub(crate) schema_export_version: Option<u32>,
    pub(crate) inclusion_policy: Option<String>,
    pub(crate) producer_fingerprints: Vec<recite_core::ProducerFingerprint>,
}

impl From<&recite_core::ProducerMetadata> for ProducerMetadataSummary {
    fn from(metadata: &recite_core::ProducerMetadata) -> Self {
        Self {
            producer: metadata.producer.clone(),
            content_fingerprint: metadata.content_fingerprint.clone(),
            schema_export_version: metadata.schema_export_version,
            inclusion_policy: metadata.inclusion_policy.clone(),
            producer_fingerprints: metadata.producer_fingerprints.clone(),
        }
    }
}
