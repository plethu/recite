use std::collections::BTreeMap;

use recite_core::{
    ParameterDefinition, PresentationAffordanceOutputDefinition, PresentationLabelDefinition,
    ProjectionInput, ProjectionQueryDefinition, ProjectionQueryFunctionDefinition,
    SchemaPresentationProjectorDefinition, SchemaProjectionSelector, SchemaTypeRef,
};

use super::identity::{SchemaCapability, SchemaDeclarationProvenance};

/// One schema-global typed query function used by presentation projectors.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProjectionQueryFunctionSummary {
    pub(super) name: String,
    pub(super) definition: ProjectionQueryFunctionDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl ProjectionQueryFunctionSummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn definition(&self) -> &ProjectionQueryFunctionDefinition {
        &self.definition
    }

    #[must_use]
    pub fn params(&self) -> &[ParameterDefinition] {
        &self.definition.params
    }

    #[must_use]
    pub const fn returns(&self) -> &SchemaTypeRef {
        &self.definition.returns
    }

    #[must_use]
    pub const fn max_calls_per_event(&self) -> Option<u32> {
        self.definition.max_calls_per_event
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

/// One schema-owned presentation projector and its typed inputs, queries, and
/// output affordances.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PresentationProjectorSummary {
    pub(super) name: String,
    pub(super) definition: SchemaPresentationProjectorDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl PresentationProjectorSummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn definition(&self) -> &SchemaPresentationProjectorDefinition {
        &self.definition
    }

    #[must_use]
    pub const fn candidates(&self) -> &SchemaProjectionSelector {
        &self.definition.candidates
    }

    #[must_use]
    pub fn inputs(&self) -> &[ProjectionInput] {
        &self.definition.inputs
    }

    #[must_use]
    pub fn queries(&self) -> &BTreeMap<String, ProjectionQueryDefinition> {
        &self.definition.queries
    }

    #[must_use]
    pub fn outputs(&self) -> &BTreeMap<String, PresentationAffordanceOutputDefinition> {
        &self.definition.outputs
    }

    pub fn labels(&self) -> impl Iterator<Item = (&str, &PresentationLabelDefinition)> {
        self.definition
            .outputs
            .iter()
            .filter_map(|(name, output)| output.label.as_ref().map(|label| (name.as_str(), label)))
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
