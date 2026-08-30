//! Recite compiler, validator, POT extractor, and compiled asset writer.
//!
//! This crate turns raw Recite source into deterministic compiled assets for
//! `recite-runtime`. It also exposes validation and gettext POT extraction
//! entry points for CLI, editor, CI, and adapter tooling.
//!
//! `CompileReport` separates recoverable content diagnostics from hard failures:
//! malformed source or invalid schema use returns diagnostics with no asset,
//! while serialization or impossible internal states return `CompileError`.
//! Callers should inspect structured diagnostics instead of parsing rendered
//! messages.
//!
//! Broader authoring workflow guides live in the
//! [docs site][guides] as they are filled in. This Rustdoc focuses on the
//! library API.
//!
//! [guides]: https://github.com/plethu/recite/tree/main/docs-site/src/content/docs
//!
//! # Example: Compile An In-Memory Scene
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
//! use recite_core::{
//!     CompiledAssetId, CompilerVersion, SchemaFingerprint, SourceMapId,
//! };
//!
//! let source = concat!(
//!     ":: start default\n",
//!     "> intro_001@8843fd6f53f020a12b31\n",
//!     "  Hello.\n",
//!     "-> END\n",
//! );
//! let options = CompileOptions::new(
//!     CompilerVersion::new("0.0.1")?,
//!     CompiledAssetId::new("example-dialogue")?,
//!     SourceMapId::new("example-source-map")?,
//!     SchemaFingerprint::NoSchema,
//! );
//!
//! let report = compile_inputs(
//!     [CompileInput::new("dialogue/start.recite", source)],
//!     options,
//! )?;
//!
//! assert!(report.diagnostics.is_empty());
//! let asset = report.asset.expect("valid source emits an asset");
//! assert_eq!(asset.dialogue.lines[0].id.as_str(), "8843fd6f53f020a12b31");
//! assert!(!asset.messagepack.is_empty());
//! # Ok(())
//! # }
//! ```

mod authoring;
mod compile;
mod diagnostics;
mod pot;
mod validation;
mod wire;

#[cfg(feature = "bench-support")]
#[doc(hidden)]
pub mod bench_support;

pub use authoring::{
    AffectedInput, AffectedInputReason, AnalysisDelta, AuthoringEditError, AuthoringEditOperation,
    AuthoringEditPlan, AuthoringError, AuthoringKernel,
    AuthoringRequest, AuthoringSnapshot, AuthoringSummary, BlockDefinitionSummary,
    BlockReferenceSummary, BlockTarget, BuildAuthority, BuildCancellation, BuildCandidate, BuildCheck,
    BuildControl, BuildCoordinator, BuildEngine, BuildEventKind, BuildFailure, BuildFailureReason,
    BuildFingerprintSet, BuildGeneration, BuildGenerationError, BuildInput, BuildInputAuthority,
    BuildInputFingerprint, BuildInputKind, BuildInputPolicy, BuildLifecycle, BuildPhase,
    BuildPublisher, BuildRequest, BuildRequestError, BuildResult, BuildRunError, BuildState,
    BuildTarget, BuildTargetError, BuildTelemetry, BuildTerminalStatus, BuildTransition,
    BuildTransitionError, ClauseKind, CompletionCandidate, CompletionCandidateDetail,
    CompletionCandidateKind, CompletionItem, CompletionSite, CompletionSiteKind,
    DiagnosticCollection, DiagnosticIter, DocumentDelta, DocumentLayer, DocumentMetadata,
    DocumentSnapshot, DocumentVersion, EditPrecondition, FreshnessAssessment, FreshnessStatus,
    FunctionReferenceKind,
    FunctionReferenceSummary, HoverInfo, MetadataScalar, MetadataSummary, MetadataValue,
    MetadataValueDetail, MetadataValueKind, NavigationResult, OpenDocument, PublishFailure,
    PublishFailureReason, PublishNotAttemptedReason, PublishOutcome, PublishRefusal, QueryClass,
    QueryResult, QueryUnavailableReason, RecoveryNeeded, RestartGuidance, SavedDocument,
    SemanticFact, SemanticSymbolKind, SnapshotGeneration, SourceEdit, SourceFingerprint, SourceRange,
    StableIdKind, StableIdSummary, StaleReason, SymbolIdentity, SymbolKind, SymbolLocation,
    SymbolQueryOptions, SymbolRole, plan_create_block_stub, plan_insert_missing_id,
    plan_insert_missing_ids, plan_rename_block,
};
pub use compile::{
    CompileError, CompileInput, CompileOptions, CompileReport, CompiledAssetOutput, compile_inputs,
    compile_inputs_with_schema,
};
pub use pot::{
    PotDocument, PotEntry, PotExtractionReport, PotReference, extract_pot, extract_pot_with_schema,
};
pub use validation::{
    ValidationCompleteness, ValidationInput, ValidationParticipation, ValidationReport,
    validate_source_file, validate_source_files, validate_source_files_with_participation,
    validate_source_files_with_participation_with_schema, validate_source_files_with_schema,
};
