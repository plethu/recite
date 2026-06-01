use recite_core::{
    EffectMode, MetadataContextSelector, MetadataDomainDefinition, MetadataTarget,
    MissingMetadataContextPolicy, ProjectSchema, SchemaTypeDefinition, SchemaTypeRef,
};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TypeSummary {
    pub(crate) name: String,
    pub(crate) kind: TypeKindSummary,
}

impl TypeSummary {
    fn from_definition(name: &str, definition: &SchemaTypeDefinition) -> Self {
        match definition {
            SchemaTypeDefinition::Enum(definition) => Self {
                name: name.to_owned(),
                kind: TypeKindSummary::Enum {
                    values: definition.values.iter().cloned().collect(),
                },
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TypeKindSummary {
    Enum { values: Vec<String> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RegistrySummary {
    pub(crate) name: String,
    pub(crate) values: Vec<String>,
    pub(crate) provenance: ProvenanceSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SpeakerSummary {
    pub(crate) name: String,
    pub(crate) display_name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ParameterSummary {
    pub(crate) name: String,
    pub(crate) type_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ConditionSummary {
    pub(crate) name: String,
    pub(crate) params: Vec<ParameterSummary>,
    pub(crate) returns: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EffectSummary {
    pub(crate) name: String,
    pub(crate) modes: Vec<String>,
    pub(crate) params: Vec<ParameterSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MetadataDomainSummary {
    pub(crate) name: String,
    pub(crate) kind: MetadataDomainKindSummary,
    pub(crate) provenance: ProvenanceSummary,
}

impl MetadataDomainSummary {
    fn from_definition(name: &str, definition: &MetadataDomainDefinition) -> Self {
        let kind = match definition {
            MetadataDomainDefinition::Flat(domain) => MetadataDomainKindSummary::Flat {
                values: domain.values.iter().cloned().collect(),
            },
            MetadataDomainDefinition::Contextual(domain) => MetadataDomainKindSummary::Contextual {
                selector: MetadataContextSelectorSummary::from_selector(&domain.selector),
                values_by_context: domain
                    .values_by_context
                    .iter()
                    .map(|(context, values)| ContextualDomainValuesSummary {
                        context: context.clone(),
                        values: values.iter().cloned().collect(),
                    })
                    .collect(),
                missing_context: MissingMetadataContextPolicySummary::from_policy(
                    &domain.missing_context,
                ),
            },
        };

        Self {
            name: name.to_owned(),
            kind,
            provenance: ProvenanceSummary::Absent,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MetadataDomainKindSummary {
    Flat {
        values: Vec<String>,
    },
    Contextual {
        selector: MetadataContextSelectorSummary,
        values_by_context: Vec<ContextualDomainValuesSummary>,
        missing_context: MissingMetadataContextPolicySummary,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContextualDomainValuesSummary {
    pub(crate) context: String,
    pub(crate) values: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MetadataContextSelectorSummary {
    FieldSpeaker,
    MetadataKey { key: String },
}

impl MetadataContextSelectorSummary {
    fn from_selector(selector: &MetadataContextSelector) -> Self {
        match selector {
            MetadataContextSelector::FieldSpeaker => Self::FieldSpeaker,
            MetadataContextSelector::MetadataKey(key) => Self::MetadataKey { key: key.clone() },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum MissingMetadataContextPolicySummary {
    Diagnostic,
    Empty,
    Fallback { domain: String },
}

impl MissingMetadataContextPolicySummary {
    fn from_policy(policy: &MissingMetadataContextPolicy) -> Self {
        match policy {
            MissingMetadataContextPolicy::Diagnostic => Self::Diagnostic,
            MissingMetadataContextPolicy::Empty => Self::Empty,
            MissingMetadataContextPolicy::Fallback { domain } => Self::Fallback {
                domain: domain.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SchemaMetadataSummary {
    pub(crate) name: String,
    pub(crate) targets: Vec<String>,
    pub(crate) type_ref: String,
    pub(crate) repeatable: bool,
    pub(crate) domain: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MarkupSummary {
    pub(crate) name: String,
    pub(crate) requires_closing: bool,
    pub(crate) translatable: bool,
    pub(crate) allows_nesting: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProvenanceSummary {
    Present { origin: String },
    Absent,
}

impl ProvenanceSummary {
    fn from_optional_origin(origin: Option<&str>) -> Self {
        match origin {
            Some(origin) => Self::Present {
                origin: origin.to_owned(),
            },
            None => Self::Absent,
        }
    }
}

fn type_ref_summary(type_ref: &SchemaTypeRef) -> String {
    match type_ref {
        SchemaTypeRef::String => "string".to_owned(),
        SchemaTypeRef::Symbol => "symbol".to_owned(),
        SchemaTypeRef::Int => "int".to_owned(),
        SchemaTypeRef::Float => "float".to_owned(),
        SchemaTypeRef::Bool => "bool".to_owned(),
        SchemaTypeRef::Speaker => "speaker".to_owned(),
        SchemaTypeRef::Enum(name) => format!("enum:{name}"),
        SchemaTypeRef::Registry(name) => format!("registry:{name}"),
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

fn effect_mode_name(mode: EffectMode) -> &'static str {
    match mode {
        EffectMode::Deferred => "deferred",
        EffectMode::Immediate => "immediate",
        EffectMode::Blocking => "blocking",
    }
}
