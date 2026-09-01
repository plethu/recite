use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    ContextualMetadataDomain, ContextualMetadataProvenance, FlatMetadataDomain,
    FlatMetadataProvenance, MetadataContextSelector, MetadataDefinition, MetadataDomainDefinition,
    MetadataTarget, MissingMetadataContextPolicy, SchemaTypeRef,
};

use super::identity::{SchemaCapability, SchemaDeclarationProvenance};

/// One named metadata value domain.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct MetadataDomainSummary {
    pub(super) name: String,
    pub(super) definition: MetadataDomainDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl MetadataDomainSummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn definition(&self) -> &MetadataDomainDefinition {
        &self.definition
    }

    #[must_use]
    pub fn flat(&self) -> Option<&FlatMetadataDomain> {
        match &self.definition {
            MetadataDomainDefinition::Flat(domain) => Some(domain),
            MetadataDomainDefinition::Contextual(_) => None,
        }
    }

    #[must_use]
    pub fn contextual(&self) -> Option<&ContextualMetadataDomain> {
        match &self.definition {
            MetadataDomainDefinition::Flat(_) => None,
            MetadataDomainDefinition::Contextual(domain) => Some(domain),
        }
    }

    #[must_use]
    pub fn flat_values(&self) -> Option<&BTreeSet<String>> {
        self.flat().map(|domain| &domain.values)
    }

    #[must_use]
    pub fn selector(&self) -> Option<&MetadataContextSelector> {
        self.contextual().map(|domain| &domain.selector)
    }

    #[must_use]
    pub fn values_by_context(&self) -> Option<&BTreeMap<String, BTreeSet<String>>> {
        self.contextual().map(|domain| &domain.values_by_context)
    }

    #[must_use]
    pub fn missing_context(&self) -> Option<&MissingMetadataContextPolicy> {
        self.contextual().map(|domain| &domain.missing_context)
    }

    #[must_use]
    pub fn flat_provenance(&self) -> Option<&FlatMetadataProvenance> {
        self.flat().map(|domain| &domain.provenance)
    }

    #[must_use]
    pub fn contextual_provenance(&self) -> Option<&ContextualMetadataProvenance> {
        self.contextual().map(|domain| &domain.provenance)
    }

    #[must_use]
    pub const fn provenance(&self) -> &SchemaDeclarationProvenance {
        &self.provenance
    }

    #[must_use]
    pub const fn capability(&self) -> &SchemaCapability {
        &self.capability
    }
}

/// One metadata key declaration and its typed value contract.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct MetadataKeySummary {
    pub(super) name: String,
    pub(super) definition: MetadataDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl MetadataKeySummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn definition(&self) -> &MetadataDefinition {
        &self.definition
    }

    #[must_use]
    pub fn targets(&self) -> &BTreeSet<MetadataTarget> {
        &self.definition.targets
    }

    #[must_use]
    pub const fn type_ref(&self) -> &SchemaTypeRef {
        &self.definition.type_ref
    }

    #[must_use]
    pub const fn repeatable(&self) -> bool {
        self.definition.repeatable
    }

    #[must_use]
    pub fn domain(&self) -> Option<&str> {
        self.definition.domain.as_deref()
    }

    #[must_use]
    pub const fn provenance(&self) -> &SchemaDeclarationProvenance {
        &self.provenance
    }

    #[must_use]
    pub const fn capability(&self) -> &SchemaCapability {
        &self.capability
    }
}
