use crate::{ChoiceId, LineId, SourceSpan};

use super::{ConditionExpression, DivertTarget, SourceMetadata, SourceText, Statement};

/// A player-selectable choice. Missing IDs are represented for later
/// compiler/LSP validation.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub id: Option<ChoiceId>,
    pub source_text: SourceText,
    pub metadata: SourceMetadata,
    pub condition: Option<ConditionExpression>,
    pub target: Option<ChoiceTarget>,
    pub echo: ChoiceEcho,
    pub statements: Vec<Statement>,
    pub span: SourceSpan,
}

impl Choice {
    #[must_use]
    pub fn new(id: Option<ChoiceId>, source_text: SourceText, span: SourceSpan) -> Self {
        Self {
            id,
            source_text,
            metadata: SourceMetadata::new(),
            condition: None,
            target: None,
            echo: ChoiceEcho::None,
            statements: Vec::new(),
            span,
        }
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: SourceMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    #[must_use]
    pub fn with_condition(mut self, condition: ConditionExpression) -> Self {
        self.condition = Some(condition);
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
