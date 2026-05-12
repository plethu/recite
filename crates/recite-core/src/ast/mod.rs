//! Source-level AST for Recite dialogue files.
//!
//! These types model parsed source structure before semantic validation. They
//! intentionally allow states such as missing line IDs, unknown block targets,
//! duplicate match arms, or schema-unknown functions because parser output must
//! preserve source faithfully for compiler, CLI, and LSP diagnostics.

mod condition;
mod effect;
mod source;

pub use condition::{Argument, ConditionCall, ConditionExpression, ConditionGroup, ConditionUnary};
pub use effect::{Effect, EffectMode};
pub use source::{
    Block, BlockReference, Choice, ChoiceEcho, Comment, Divert, DivertTarget, IfBranch, Line,
    MatchArm, MatchBranch, MatchPattern, SourceFile, SourceText, Statement, StatementKind,
};

#[cfg(test)]
mod tests;
