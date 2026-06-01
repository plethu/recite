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
    pub(super) fn from_optional_origin(origin: Option<&str>) -> Self {
        match origin {
            Some(origin) => Self::Present {
                origin: origin.to_owned(),
            },
            None => Self::Absent,
        }
    }
}
use recite_core::SchemaTypeDefinition;
