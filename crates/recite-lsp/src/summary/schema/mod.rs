mod items;
mod metadata;
mod render;

use recite_core::ProjectSchema;

#[allow(unused_imports)]
pub(crate) use items::{
    ConditionSummary, EffectSummary, MarkupSummary, ParameterSummary, ProvenanceSummary,
    RegistrySummary, SchemaMetadataSummary, SpeakerSummary, TypeKindSummary, TypeSummary,
};
#[allow(unused_imports)]
pub(crate) use metadata::{
    ContextualDomainValuesSummary, MetadataContextSelectorSummary, MetadataDomainKindSummary,
    MetadataDomainSummary, MissingMetadataContextPolicySummary,
};
use render::{effect_mode_name, metadata_target_name, type_ref_summary};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SchemaSummary {
    pub(crate) schema_version: u32,
    pub(crate) types: Vec<TypeSummary>,
    pub(crate) registries: Vec<RegistrySummary>,
    pub(crate) speakers: Vec<SpeakerSummary>,
    pub(crate) conditions: Vec<ConditionSummary>,
    pub(crate) effects: Vec<EffectSummary>,
    pub(crate) metadata_domains: Vec<MetadataDomainSummary>,
    pub(crate) metadata: Vec<SchemaMetadataSummary>,
    pub(crate) markup: Vec<MarkupSummary>,
}

impl SchemaSummary {
    pub(crate) fn from_schema(schema: &ProjectSchema) -> Self {
        Self {
            schema_version: schema.schema_version,
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
                    provenance: ProvenanceSummary::from_optional_origin(
                        definition.origin.as_deref(),
                    ),
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
