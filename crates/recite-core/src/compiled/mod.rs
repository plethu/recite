mod fingerprint;
mod header;
mod rows;
mod table;

pub use fingerprint::{
    ContentFingerprint, FingerprintAlgorithm, FingerprintDigest, SchemaFingerprint,
};
pub use header::{
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, CompiledAssetEncoding,
    CompiledAssetHeader, CompiledAssetId, CompiledInspectionEncoding, CompilerVersion, SourceMapId,
};
pub use rows::{
    BlockLookupEntry, ChoiceLookupEntry, CompiledArgument, CompiledBlock, CompiledChoice,
    CompiledChoiceEcho, CompiledConditionCall, CompiledConditionExpression, CompiledDialogue,
    CompiledDivertTarget, CompiledEffect, CompiledEffectMode, CompiledLine, CompiledMetadataEntry,
    CompiledSourceFile, CompiledSourceMapEntry, CompiledSpeaker, CompiledStatement,
    CompiledStatementKind, LineLookupEntry,
};
pub use table::{
    BlockIndex, ChoiceIndex, ChoiceRange, EffectIndex, LineIndex, MetadataIndex, MetadataRange,
    SourceFileIndex, SourceMapIndex, SpeakerIndex, StatementIndex, StatementRange, TableRange,
};

/// Error returned when constructing constrained compiled model values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledValueError {
    EmptyValue { kind: &'static str },
}

impl std::fmt::Display for CompiledValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyValue { kind } => write!(formatter, "{kind} must not be empty"),
        }
    }
}

impl std::error::Error for CompiledValueError {}
