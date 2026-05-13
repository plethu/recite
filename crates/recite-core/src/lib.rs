//! Core Recite AST, identifiers, values, diagnostics, and schema model.

pub mod ast;

mod diagnostic;
mod error;
mod ids;
mod source_location;
mod value;

pub use ast::{
    Argument, Block, BlockReference, Choice, ChoiceEcho, ChoiceTarget, Comment, ConditionCall,
    ConditionExpression, ConditionGroup, ConditionUnary, Divert, DivertTarget, Effect, EffectMode,
    IfBranch, Line, MatchArm, MatchBranch, MatchPattern, SourceFile, SourceText, Statement,
    StatementKind,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticSeverity, RelatedSpan};
pub use error::CoreValueError;
pub use ids::{BlockId, ChoiceId, EffectId, LineId, SpeakerId};
pub use source_location::{SourcePosition, SourceSpan};
pub use value::{Metadata, MetadataEntry, ScalarValue, Value};

#[cfg(test)]
mod model_tests;
