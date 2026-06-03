use crate::SourceSpan;

use super::{Choice, Divert, Effect, IfBranch, Line, MatchBranch};

/// One source-level statement in the order it appears in a block or body.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
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
