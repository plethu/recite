use std::collections::BTreeSet;

use recite_core::{
    AvailabilityReasonDefinition, AvailabilityReasonId, ConditionDefinition, ConditionReturnType,
    EffectDefinition, EffectMode, MarkupDefinition, ParameterDefinition, ProducerOrigin,
};

use super::identity::{SchemaCapability, SchemaDeclarationProvenance};

/// One typed condition declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ConditionSummary {
    pub(super) name: String,
    pub(super) definition: ConditionDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl ConditionSummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn definition(&self) -> &ConditionDefinition {
        &self.definition
    }

    #[must_use]
    pub fn params(&self) -> &[ParameterDefinition] {
        &self.definition.params
    }

    #[must_use]
    pub const fn returns(&self) -> &ConditionReturnType {
        &self.definition.returns
    }

    #[must_use]
    pub const fn availability_reason(
        &self,
    ) -> Option<&recite_core::ConditionAvailabilityReasonMapping> {
        self.definition.availability_reason.as_ref()
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

/// One localisable unavailable-choice reason template.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AvailabilityReasonSummary {
    pub(super) id: AvailabilityReasonId,
    pub(super) definition: AvailabilityReasonDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl AvailabilityReasonSummary {
    #[must_use]
    pub const fn id(&self) -> &AvailabilityReasonId {
        &self.id
    }

    #[must_use]
    pub const fn definition(&self) -> &AvailabilityReasonDefinition {
        &self.definition
    }

    #[must_use]
    pub fn template(&self) -> &str {
        &self.definition.template
    }

    #[must_use]
    pub fn params(&self) -> &[ParameterDefinition] {
        &self.definition.params
    }

    #[must_use]
    pub fn origin(&self) -> Option<&ProducerOrigin> {
        self.definition.origin.as_ref()
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

/// One typed effect request declaration and its supported modes.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct EffectSummary {
    pub(super) name: String,
    pub(super) definition: EffectDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl EffectSummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn definition(&self) -> &EffectDefinition {
        &self.definition
    }

    #[must_use]
    pub fn modes(&self) -> &BTreeSet<EffectMode> {
        &self.definition.modes
    }

    #[must_use]
    pub fn params(&self) -> &[ParameterDefinition] {
        &self.definition.params
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

/// One inline markup declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct MarkupSummary {
    pub(super) name: String,
    pub(super) definition: MarkupDefinition,
    pub(super) provenance: SchemaDeclarationProvenance,
    pub(super) capability: SchemaCapability,
}

impl MarkupSummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn definition(&self) -> &MarkupDefinition {
        &self.definition
    }

    #[must_use]
    pub const fn requires_closing(&self) -> bool {
        self.definition.requires_closing
    }

    #[must_use]
    pub const fn translatable(&self) -> bool {
        self.definition.translatable
    }

    #[must_use]
    pub const fn allows_nesting(&self) -> bool {
        self.definition.allows_nesting
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
