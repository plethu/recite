mod fingerprint;
mod header;
mod lookup;
mod rows;
mod table;
mod wire;

pub use fingerprint::{
    BLAKE3_DIGEST_LEN, ContentFingerprint, FingerprintAlgorithm, FingerprintDigest,
    SchemaFingerprint,
};
pub use header::{
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, CompiledAssetEncoding,
    CompiledAssetHeader, CompiledAssetId, CompiledInspectionEncoding, CompilerVersion, SourceMapId,
};
pub use lookup::{
    BlockLookupEntry, BlockLookupTable, ChoiceLookupEntry, ChoiceLookupTable, LineLookupEntry,
    LineLookupTable,
};
pub use rows::{
    CompiledArgument, CompiledBlock, CompiledChoice, CompiledChoiceEcho, CompiledConditionCall,
    CompiledConditionExpression, CompiledDialogue, CompiledDivertTarget, CompiledEffect,
    CompiledEffectMode, CompiledLine, CompiledMetadataEntry, CompiledSourceFile,
    CompiledSourceMapEntry, CompiledSpeaker, CompiledStatement, CompiledStatementKind,
};
pub use table::{
    BlockIndex, ChoiceIndex, ChoiceRange, EffectIndex, LineIndex, MetadataIndex, MetadataRange,
    SourceFileIndex, SourceMapIndex, SpeakerIndex, StatementIndex, StatementRange, TableRange,
};
pub use wire::*;

/// Error returned when constructing constrained compiled model values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledValueError {
    EmptyValue {
        kind: &'static str,
    },
    InvalidFingerprintDigestLength {
        algorithm: &'static str,
        expected: usize,
        actual: usize,
    },
    UnsortedLookupTable {
        table: &'static str,
        previous: String,
        current: String,
    },
}

impl std::fmt::Display for CompiledValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyValue { kind } => write!(formatter, "{kind} must not be empty"),
            Self::InvalidFingerprintDigestLength {
                algorithm,
                expected,
                actual,
            } => write!(
                formatter,
                "{algorithm} fingerprint digest must be {expected} bytes, got {actual}"
            ),
            Self::UnsortedLookupTable {
                table,
                previous,
                current,
            } => write!(
                formatter,
                "{table} lookup entries must be strictly sorted and unique, got `{previous}` before `{current}`"
            ),
        }
    }
}

impl std::error::Error for CompiledValueError {}
