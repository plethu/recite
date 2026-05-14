//! Core Recite AST, compiled dialogue model, identifiers, values, diagnostics,
//! and schema model.

pub mod ast;
pub mod compiled;

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
pub use compiled::{
    BlockIndex, BlockLookupEntry, COMPILED_ASSET_FORMAT_VERSION_V0,
    COMPILER_COMPATIBILITY_VERSION_V0, ChoiceIndex, ChoiceLookupEntry, ChoiceRange,
    CompiledArgument, CompiledAssetEncoding, CompiledAssetHeader, CompiledAssetId, CompiledBlock,
    CompiledChoice, CompiledChoiceEcho, CompiledConditionCall, CompiledConditionExpression,
    CompiledDialogue, CompiledDivertTarget, CompiledEffect, CompiledEffectMode,
    CompiledInspectionEncoding, CompiledLine, CompiledMetadataEntry, CompiledSourceFile,
    CompiledSourceMapEntry, CompiledSpeaker, CompiledStatement, CompiledStatementKind,
    CompiledValueError, CompilerVersion, ContentFingerprint, EffectIndex, FingerprintAlgorithm,
    FingerprintDigest, LineIndex, LineLookupEntry, MetadataIndex, MetadataRange, SchemaFingerprint,
    SourceFileIndex, SourceMapId, SourceMapIndex, SpeakerIndex, StatementIndex, StatementRange,
    TableRange,
};
pub use diagnostic::{Diagnostic, DiagnosticCode, DiagnosticSeverity, RelatedSpan};
pub use error::CoreValueError;
pub use ids::{BlockId, ChoiceId, EffectId, LineId, SpeakerId};
pub use source_location::{SourcePosition, SourceSpan};
pub use value::{Metadata, MetadataEntry, ScalarValue, Value};

#[cfg(test)]
mod model_tests;
