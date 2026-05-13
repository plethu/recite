use crate::{BlockId, ChoiceId, LineId, Metadata, SourceSpan, SpeakerId};

use super::{ConditionCall, ConditionExpression, Effect};

/// A parsed Recite source file.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub blocks: Vec<Block>,
}

impl SourceFile {
    #[must_use]
    pub fn new(path: impl Into<String>, blocks: Vec<Block>) -> Self {
        Self {
            path: path.into(),
            blocks,
        }
    }

    pub fn visit_statements_depth_first<'a>(&'a self, visitor: &mut impl FnMut(&'a Statement)) {
        for block in &self.blocks {
            block.visit_statements_depth_first(visitor);
        }
    }
}

/// A named dialogue block.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub id: BlockId,
    pub is_default: bool,
    pub default_speaker: Option<SpeakerId>,
    pub metadata: Metadata,
    pub statements: Vec<Statement>,
    pub span: SourceSpan,
}

impl Block {
    #[must_use]
    pub fn new(id: BlockId, statements: Vec<Statement>, span: SourceSpan) -> Self {
        Self {
            id,
            is_default: false,
            default_speaker: None,
            metadata: Metadata::new(),
            statements,
            span,
        }
    }

    #[must_use]
    pub fn with_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    #[must_use]
    pub fn with_default_speaker(mut self, speaker: SpeakerId) -> Self {
        self.default_speaker = Some(speaker);
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn visit_statements_depth_first<'a>(&'a self, visitor: &mut impl FnMut(&'a Statement)) {
        for statement in &self.statements {
            statement.visit_depth_first(visitor);
        }
    }
}

/// One source-level statement in the order it appears in a block or body.
#[derive(Clone, Debug, PartialEq)]
pub enum Statement {
    Line(Line),
    Choice(Choice),
    Divert(Divert),
    If(IfBranch),
    Match(MatchBranch),
    Effect(Effect),
    Comment(Comment),
}

impl Statement {
    pub fn visit_depth_first<'a>(&'a self, visitor: &mut impl FnMut(&'a Statement)) {
        visitor(self);

        match self {
            Self::Line(line) => {
                for statement in &line.statements {
                    statement.visit_depth_first(visitor);
                }
            }
            Self::Choice(choice) => {
                for statement in &choice.statements {
                    statement.visit_depth_first(visitor);
                }
            }
            Self::If(branch) => {
                for statement in &branch.then_statements {
                    statement.visit_depth_first(visitor);
                }
                for statement in &branch.else_statements {
                    statement.visit_depth_first(visitor);
                }
            }
            Self::Match(branch) => {
                for arm in &branch.arms {
                    for statement in &arm.statements {
                        statement.visit_depth_first(visitor);
                    }
                }
            }
            Self::Divert(_) | Self::Effect(_) | Self::Comment(_) => {}
        }
    }

    #[must_use]
    pub const fn kind(&self) -> StatementKind {
        match self {
            Self::Line(_) => StatementKind::Line,
            Self::Choice(_) => StatementKind::Choice,
            Self::Divert(_) => StatementKind::Divert,
            Self::If(_) => StatementKind::If,
            Self::Match(_) => StatementKind::Match,
            Self::Effect(_) => StatementKind::Effect,
            Self::Comment(_) => StatementKind::Comment,
        }
    }
}

/// Stable statement category for traversal tests and diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum StatementKind {
    Line,
    Choice,
    Divert,
    If,
    Match,
    Effect,
    Comment,
}

/// Localisable source text with its own span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceText {
    pub text: String,
    pub span: SourceSpan,
}

impl SourceText {
    #[must_use]
    pub fn new(text: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            text: text.into(),
            span,
        }
    }
}

/// A localisable dialogue line. Missing IDs are represented for later
/// compiler/LSP validation.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub id: Option<LineId>,
    pub speaker: Option<SpeakerId>,
    pub source_text: SourceText,
    pub metadata: Metadata,
    pub statements: Vec<Statement>,
    pub span: SourceSpan,
}

impl Line {
    #[must_use]
    pub fn new(id: Option<LineId>, source_text: SourceText, span: SourceSpan) -> Self {
        Self {
            id,
            speaker: None,
            source_text,
            metadata: Metadata::new(),
            statements: Vec::new(),
            span,
        }
    }

    #[must_use]
    pub fn with_speaker(mut self, speaker: SpeakerId) -> Self {
        self.speaker = Some(speaker);
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    #[must_use]
    pub fn with_statements(mut self, statements: Vec<Statement>) -> Self {
        self.statements = statements;
        self
    }
}

/// A player-selectable choice. Missing IDs are represented for later
/// compiler/LSP validation.
#[derive(Clone, Debug, PartialEq)]
pub struct Choice {
    pub id: Option<ChoiceId>,
    pub source_text: SourceText,
    pub metadata: Metadata,
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
            metadata: Metadata::new(),
            condition: None,
            target: None,
            echo: ChoiceEcho::None,
            statements: Vec::new(),
            span,
        }
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
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

/// A standalone divert statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Divert {
    pub target: DivertTarget,
    pub span: SourceSpan,
}

impl Divert {
    #[must_use]
    pub fn new(target: DivertTarget, span: SourceSpan) -> Self {
        Self { target, span }
    }
}

/// Source-level divert targets. Validation resolves unknown block references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DivertTarget {
    Block(BlockReference),
    End,
}

/// A same-file or cross-file block reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockReference {
    pub file: Option<String>,
    pub block_id: BlockId,
}

impl BlockReference {
    #[must_use]
    pub fn local(block_id: BlockId) -> Self {
        Self {
            file: None,
            block_id,
        }
    }

    #[must_use]
    pub fn external(file: impl Into<String>, block_id: BlockId) -> Self {
        Self {
            file: Some(file.into()),
            block_id,
        }
    }
}

/// A conditional branch with an optional else body.
#[derive(Clone, Debug, PartialEq)]
pub struct IfBranch {
    pub condition: ConditionExpression,
    pub then_statements: Vec<Statement>,
    pub else_statements: Vec<Statement>,
    pub span: SourceSpan,
}

impl IfBranch {
    #[must_use]
    pub fn new(
        condition: ConditionExpression,
        then_statements: Vec<Statement>,
        span: SourceSpan,
    ) -> Self {
        Self {
            condition,
            then_statements,
            else_statements: Vec::new(),
            span,
        }
    }

    #[must_use]
    pub fn with_else_statements(mut self, else_statements: Vec<Statement>) -> Self {
        self.else_statements = else_statements;
        self
    }
}

/// Restricted enum dispatch over a condition-language query.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchBranch {
    pub scrutinee: ConditionCall,
    pub arms: Vec<MatchArm>,
    pub span: SourceSpan,
}

impl MatchBranch {
    #[must_use]
    pub fn new(scrutinee: ConditionCall, arms: Vec<MatchArm>, span: SourceSpan) -> Self {
        Self {
            scrutinee,
            arms,
            span,
        }
    }
}

/// One match arm. Exhaustiveness, duplicate arms, and wildcard placement are
/// compiler validation responsibilities.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub statements: Vec<Statement>,
    pub span: SourceSpan,
}

impl MatchArm {
    #[must_use]
    pub fn new(pattern: MatchPattern, statements: Vec<Statement>, span: SourceSpan) -> Self {
        Self {
            pattern,
            statements,
            span,
        }
    }
}

/// A schema-declared enum variant or wildcard arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchPattern {
    Variant(String),
    Wildcard,
}

/// A source comment preserved when needed for tooling.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Comment {
    pub text: String,
    pub span: SourceSpan,
}

impl Comment {
    #[must_use]
    pub fn new(text: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            text: text.into(),
            span,
        }
    }
}
