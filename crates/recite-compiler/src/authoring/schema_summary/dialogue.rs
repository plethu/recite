use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    EnumTypeDefinition, ProducerOrigin, RegistryDefinition, SchemaTypeDefinition, SpeakerDefinition,
};

use super::identity::{SchemaCapability, SchemaDeclarationProvenance};

/// One named schema type, retaining its canonical definition.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaTypeSummary {
    pub(super) name: String,
    pub(super) definition: SchemaTypeDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl SchemaTypeSummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn definition(&self) -> &SchemaTypeDefinition {
        &self.definition
    }

    #[must_use]
    pub fn enum_definition(&self) -> Option<&EnumTypeDefinition> {
        match &self.definition {
            SchemaTypeDefinition::Enum(definition) => Some(definition),
        }
    }

    #[must_use]
    pub fn enum_values(&self) -> Option<&BTreeSet<String>> {
        self.enum_definition().map(|definition| &definition.values)
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

/// One named registry of stable project content IDs.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct RegistrySummary {
    pub(super) name: String,
    pub(super) definition: RegistryDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl RegistrySummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn definition(&self) -> &RegistryDefinition {
        &self.definition
    }

    #[must_use]
    pub fn values(&self) -> &BTreeSet<String> {
        &self.definition.values
    }

    #[must_use]
    pub fn origin(&self) -> Option<&ProducerOrigin> {
        self.definition.origin.as_ref()
    }

    #[must_use]
    pub fn value_origins(&self) -> &BTreeMap<String, ProducerOrigin> {
        &self.definition.value_origins
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

/// One declared speaker ID and its optional source display name.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SpeakerSummary {
    pub(super) name: String,
    pub(super) definition: SpeakerDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl SpeakerSummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn definition(&self) -> &SpeakerDefinition {
        &self.definition
    }

    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.definition.display_name.as_deref()
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
