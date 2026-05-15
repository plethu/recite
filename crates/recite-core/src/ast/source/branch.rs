use crate::SourceSpan;

use super::{ConditionCall, ConditionExpression, Statement};

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
