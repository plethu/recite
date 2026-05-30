//! Core Recite AST, compiled dialogue model, identifiers, values, diagnostics,
//! and schema model.

pub mod ast;
pub mod compiled;
pub mod project;
pub mod schema;

mod diagnostic;
mod error;
mod ids;
mod source_location;
mod value;

pub use ast::{
    Argument, Block, BlockReference, Choice, ChoiceEcho, ChoiceTarget, Comment, ConditionCall,
    ConditionExpression, ConditionGroup, ConditionUnary, Divert, DivertTarget, END_DIVERT_TARGET,
    Effect, EffectMode, IfBranch, Line, MatchArm, MatchBranch, MatchPattern, SourceFile,
    SourceText, Statement, StatementKind,
};
pub use compiled::{
    BLAKE3_DIGEST_LEN, BlockIndex, BlockLookupEntry, BlockLookupTable,
    COMPILED_ASSET_FORMAT_VERSION_V0, COMPILER_COMPATIBILITY_VERSION_V0, ChoiceIndex,
    ChoiceLookupEntry, ChoiceLookupTable, ChoiceRange, CompiledArgument, CompiledAssetDecodeError,
    CompiledAssetEncoding, CompiledAssetHeader, CompiledAssetId, CompiledBlock, CompiledChoice,
    CompiledChoiceEcho, CompiledConditionCall, CompiledConditionExpression, CompiledDialogue,
    CompiledDivertTarget, CompiledEffect, CompiledEffectMode, CompiledInspectionEncoding,
    CompiledLine, CompiledMatchArm, CompiledMatchPattern, CompiledMetadataEntry,
    CompiledSourceFile, CompiledSourceMapEntry, CompiledSpeaker, CompiledStatement,
    CompiledStatementKind, CompiledValueError, CompilerVersion, ContentFingerprint, EffectIndex,
    FingerprintAlgorithm, FingerprintDigest, LineIndex, LineLookupEntry, LineLookupTable,
    MatchArmIndex, MatchArmRange, MetadataIndex, MetadataRange, SchemaFingerprint, SourceFileIndex,
    SourceMapId, SourceMapIndex, SpeakerIndex, StatementIndex, StatementRange, TableRange,
    V0_ARGUMENT_TAG_IDENTIFIER, V0_ARGUMENT_TAG_VALUE, V0_ASSET_ENCODING_MESSAGEPACK,
    V0_ASSET_HEADER_FIELDS, V0_BLOCK_FIELDS, V0_CHOICE_ECHO_TAG_EXPLICIT_LINE,
    V0_CHOICE_ECHO_TAG_NONE, V0_CHOICE_ECHO_TAG_SELECTED_TEXT, V0_CHOICE_FIELDS,
    V0_COMPILED_DIALOGUE_FIELDS, V0_CONDITION_TAG_AND, V0_CONDITION_TAG_CALL, V0_CONDITION_TAG_NOT,
    V0_CONDITION_TAG_OR, V0_DIVERT_TARGET_TAG_BLOCK, V0_DIVERT_TARGET_TAG_END, V0_EFFECT_FIELDS,
    V0_EFFECT_MODE_TAG_BLOCKING, V0_EFFECT_MODE_TAG_DEFERRED, V0_EFFECT_MODE_TAG_IMMEDIATE,
    V0_FINGERPRINT_FIELDS, V0_INSPECTION_ENCODING_COMPACT_JSON, V0_LINE_FIELDS,
    V0_LOOKUP_ENTRY_FIELDS, V0_MATCH_ARM_FIELDS, V0_MATCH_PATTERN_TAG_VARIANT,
    V0_MATCH_PATTERN_TAG_WILDCARD, V0_METADATA_ENTRY_FIELDS, V0_RANGE_FIELDS,
    V0_SCALAR_TAG_BOOLEAN, V0_SCALAR_TAG_FLOAT, V0_SCALAR_TAG_INTEGER, V0_SCALAR_TAG_STRING,
    V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT, V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA,
    V0_SOURCE_FILE_FIELDS, V0_SOURCE_MAP_ENTRY_FIELDS, V0_SOURCE_SPAN_FIELDS, V0_SPEAKER_FIELDS,
    V0_STATEMENT_FIELDS, V0_STATEMENT_TAG_DIVERT, V0_STATEMENT_TAG_EFFECT, V0_STATEMENT_TAG_END,
    V0_STATEMENT_TAG_IF, V0_STATEMENT_TAG_LINE, V0_STATEMENT_TAG_MATCH, V0_STATEMENT_TAG_PROMPT,
    V0_VALUE_TAG_ARRAY, V0_VALUE_TAG_SCALAR, canonical_source_fingerprint,
    decode_compiled_dialogue_messagepack,
};
pub use diagnostic::{
    Diagnostic, DiagnosticCategory, DiagnosticCode, DiagnosticSeverity, RelatedSpan,
};
pub use error::CoreValueError;
pub use ids::{BlockId, ChoiceId, EffectId, LineId, LocaleId, SpeakerId};
pub use project::{
    ProjectFreshnessInput, ProjectManifest, ProjectManifestLoadReport, ProjectManifestMetadata,
    ProjectScene, project_scene_key_span, validate_project_freshness, validate_project_manifest,
};
pub use schema::{
    ConditionDefinition, ConditionReturnType, EffectDefinition, EnumTypeDefinition,
    MarkupDefinition, MetadataDefinition, MetadataTarget, ParameterDefinition, ProjectSchema,
    RegistryDefinition, SchemaLoadReport, SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition,
    canonical_schema_fingerprint, load_schema_manifest_str,
};
pub use source_location::{SourcePosition, SourceSpan};
pub use value::{Metadata, MetadataEntry, ScalarValue, Value};
