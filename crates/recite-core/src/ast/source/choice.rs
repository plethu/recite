use crate::{AvailabilityReasonId, ChoiceId, LineId, SourceAnchor, SourceId, SourceSpan};

use super::{
    ConditionExpression, DivertTarget, InterpolationBinding, SourceMetadata, SourceText, Statement,
};

/// A player-selectable choice. Missing IDs are represented for later
/// compiler/LSP validation.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub source_id: SourceId,
    pub source_id_span: Option<SourceSpan>,
    pub source_id_insertion_span: SourceSpan,
    pub id: Option<ChoiceId>,
    pub source_text: SourceText,
    pub interpolation_bindings: Vec<InterpolationBinding>,
    pub metadata: SourceMetadata,
    pub availability_requirement: Option<ChoiceAvailabilityRequirement>,
    pub availability_reason_override: Option<ChoiceAvailabilityReasonOverride>,
    pub target: Option<ChoiceTarget>,
    pub echo: ChoiceEcho,
    pub statements: Vec<Statement>,
    pub span: SourceSpan,
}

impl Choice {
    #[must_use]
    pub fn new(id: Option<ChoiceId>, source_text: SourceText, span: SourceSpan) -> Self {
        Self {
            source_id: id
                .as_ref()
                .and_then(|id| SourceAnchor::new(id.as_str()).ok())
                .and_then(|anchor| SourceId::frozen("choice", anchor))
                .unwrap_or(SourceId::Missing),
            source_id_span: None,
            source_id_insertion_span: span.clone(),
            id,
            source_text,
            interpolation_bindings: Vec::new(),
            metadata: SourceMetadata::new(),
            availability_requirement: None,
            availability_reason_override: None,
            target: None,
            echo: ChoiceEcho::None,
            statements: Vec::new(),
            span,
        }
    }

    #[must_use]
    pub fn with_source_id(mut self, source_id: SourceId) -> Self {
        self.id = source_id.canonical_choice_id();
        self.source_id = source_id;
        self
    }

    #[must_use]
    pub fn with_source_id_spans(
        mut self,
        source_id_span: Option<SourceSpan>,
        source_id_insertion_span: SourceSpan,
    ) -> Self {
        self.source_id_span = source_id_span;
        self.source_id_insertion_span = source_id_insertion_span;
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: SourceMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    #[must_use]
    pub fn with_interpolation_bindings(mut self, bindings: Vec<InterpolationBinding>) -> Self {
        self.interpolation_bindings = bindings;
        self
    }

    #[must_use]
    pub fn with_availability_requirement(
        mut self,
        requirement: ChoiceAvailabilityRequirement,
    ) -> Self {
        self.availability_requirement = Some(requirement);
        self
    }

    #[must_use]
    pub fn with_availability_reason_override(
        mut self,
        reason: ChoiceAvailabilityReasonOverride,
    ) -> Self {
        self.availability_reason_override = Some(reason);
        self
    }

    #[must_use]
    pub fn with_target(mut self, target: ChoiceTarget) -> Self {
        self.target = Some(target);
        self
    }

    #[must_use]
    pub fn with_echo(mut self, echo: ChoiceEcho) -> Self {
        self.echo = echo;
        self
    }

    #[must_use]
    pub fn with_statements(mut self, statements: Vec<Statement>) -> Self {
        self.statements = statements;
        self
    }
}

/// A visible choice availability requirement authored with `requires=(...)`.
#[derive(Clone, Debug, PartialEq)]
pub struct ChoiceAvailabilityRequirement {
    pub condition: ConditionExpression,
    pub span: SourceSpan,
}

impl ChoiceAvailabilityRequirement {
    #[must_use]
    pub fn new(condition: ConditionExpression, span: SourceSpan) -> Self {
        Self { condition, span }
    }
}

/// An explicit primary unavailable reason override authored with `reason=...`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChoiceAvailabilityReasonOverride {
    pub reason_id: AvailabilityReasonId,
    pub span: SourceSpan,
    pub id_span: SourceSpan,
    pub argument_span: Option<SourceSpan>,
}

impl ChoiceAvailabilityReasonOverride {
    #[must_use]
    pub fn new(reason_id: AvailabilityReasonId, span: SourceSpan, id_span: SourceSpan) -> Self {
        Self {
            reason_id,
            span,
            id_span,
            argument_span: None,
        }
    }

    #[must_use]
    pub fn with_argument_span(mut self, argument_span: SourceSpan) -> Self {
        self.argument_span = Some(argument_span);
        self
    }
}

/// The block or end target selected by a choice, with the source span of the
/// authored target statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChoiceTarget {
    pub target: DivertTarget,
    pub span: SourceSpan,
}

impl ChoiceTarget {
    #[must_use]
    pub fn new(target: DivertTarget, span: SourceSpan) -> Self {
        Self { target, span }
    }
}

/// Explicit choice echo policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChoiceEcho {
    None,
    SelectedText,
    Line(LineId),
}
