#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TypeSummary {
    pub(crate) name: String,
    pub(crate) kind: TypeKindSummary,
}

impl TypeSummary {
    pub(super) fn from_definition(name: &str, definition: &SchemaTypeDefinition) -> Self {
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
    pub(crate) value_provenance: std::collections::BTreeMap<String, ProvenanceSummary>,
    pub(crate) producer_fingerprints: Vec<recite_core::ProducerFingerprint>,
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
pub(crate) struct AvailabilityReasonSummary {
    pub(crate) name: String,
    pub(crate) provenance: ProvenanceSummary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EffectSummary {
    pub(crate) name: String,
    pub(crate) modes: Vec<String>,
    pub(crate) params: Vec<ParameterSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectionQuerySummary {
    pub(crate) name: String,
    pub(crate) params: Vec<ParameterSummary>,
    pub(crate) returns: String,
    pub(crate) max_calls_per_event: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PresentationProjectorSummary {
    pub(crate) name: String,
    pub(crate) candidates: String,
    pub(crate) inputs: Vec<ProjectionInputSummary>,
    pub(crate) queries: Vec<ProjectorQuerySummary>,
    pub(crate) outputs: Vec<PresentationOutputSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectionInputSummary {
    pub(crate) name: String,
    pub(crate) type_ref: String,
    pub(crate) required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProjectorQuerySummary {
    pub(crate) name: String,
    pub(crate) function: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PresentationOutputSummary {
    pub(crate) name: String,
    pub(crate) target: String,
    pub(crate) kind: String,
    pub(crate) slot: String,
    pub(crate) label_template: Option<String>,
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
    Present { origin: recite_core::ProducerOrigin },
    Absent,
}

impl ProvenanceSummary {
    pub(super) fn from_optional_origin(origin: Option<&recite_core::ProducerOrigin>) -> Self {
        match origin {
            Some(origin) => Self::Present {
                origin: origin.clone(),
            },
            None => Self::Absent,
        }
    }
}
use recite_core::SchemaTypeDefinition;
