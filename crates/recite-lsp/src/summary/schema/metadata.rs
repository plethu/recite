#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MetadataDomainSummary {
    pub(crate) name: String,
    pub(crate) kind: MetadataDomainKindSummary,
    pub(crate) provenance: ProvenanceSummary,
    pub(crate) context_provenance: std::collections::BTreeMap<String, ProvenanceSummary>,
    pub(crate) flat_value_provenance: std::collections::BTreeMap<String, ProvenanceSummary>,
    pub(crate) contextual_value_provenance:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, ProvenanceSummary>>,
    pub(crate) producer_fingerprints: Vec<recite_core::ProducerFingerprint>,
}

impl MetadataDomainSummary {
    pub(super) fn from_definition(name: &str, definition: &MetadataDomainDefinition) -> Self {
        let provenance = match definition {
            MetadataDomainDefinition::Flat(domain) => {
                ProvenanceSummary::from_optional_origin(domain.provenance.origin.as_ref())
            }
            MetadataDomainDefinition::Contextual(domain) => {
                ProvenanceSummary::from_optional_origin(domain.provenance.origin.as_ref())
            }
        };
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
            provenance,
            context_provenance: match definition {
                MetadataDomainDefinition::Flat(_) => std::collections::BTreeMap::new(),
                MetadataDomainDefinition::Contextual(domain) => domain
                    .provenance
                    .context_origins
                    .iter()
                    .map(|(key, origin)| {
                        (
                            key.clone(),
                            ProvenanceSummary::from_optional_origin(Some(origin)),
                        )
                    })
                    .collect(),
            },
            flat_value_provenance: match definition {
                MetadataDomainDefinition::Flat(domain) => domain
                    .provenance
                    .value_origins
                    .iter()
                    .map(|(key, origin)| {
                        (
                            key.clone(),
                            ProvenanceSummary::from_optional_origin(Some(origin)),
                        )
                    })
                    .collect(),
                MetadataDomainDefinition::Contextual(_) => std::collections::BTreeMap::new(),
            },
            contextual_value_provenance: match definition {
                MetadataDomainDefinition::Flat(_) => std::collections::BTreeMap::new(),
                MetadataDomainDefinition::Contextual(domain) => domain
                    .provenance
                    .value_origins
                    .iter()
                    .map(|(context, origins)| {
                        (
                            context.clone(),
                            origins
                                .iter()
                                .map(|(key, origin)| {
                                    (
                                        key.clone(),
                                        ProvenanceSummary::from_optional_origin(Some(origin)),
                                    )
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            },
            producer_fingerprints: match definition {
                MetadataDomainDefinition::Flat(domain) => {
                    domain.provenance.producer_fingerprints.clone()
                }
                MetadataDomainDefinition::Contextual(domain) => {
                    domain.provenance.producer_fingerprints.clone()
                }
            },
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
use recite_core::{
    MetadataContextSelector, MetadataDomainDefinition, MissingMetadataContextPolicy,
};

use super::ProvenanceSummary;
