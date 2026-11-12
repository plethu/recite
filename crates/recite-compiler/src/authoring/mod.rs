mod build;
mod catalog_summary;
mod control;
mod edit;
mod engine;
mod input;
mod input_state;
mod query;
mod recording;
mod schema_summary;
mod snapshot;
mod state;
mod summary;

pub use build::{
    AffectedInput, AffectedInputReason, BuildAuthority, BuildAuthorityError, BuildAuthorityFence,
    BuildCancellation, BuildCandidate, BuildCheck, BuildCheckError, BuildControl, BuildCoordinator,
    BuildEngine, BuildEventKind, BuildFailure, BuildFailureReason, BuildFingerprintSet,
    BuildGeneration, BuildGenerationError, BuildInput, BuildInputAuthority, BuildInputFingerprint,
    BuildInputKind, BuildInputPayload, BuildInputPolicy, BuildLifecycle, BuildPhase,
    BuildPreparedHandle, BuildPublishPermit, BuildPublisher, BuildRequest, BuildRequestError,
    BuildRequestIdentity, BuildResult, BuildResultFailure, BuildRunError, BuildState,
    BuildStatusProjection, BuildTarget, BuildTargetError, BuildTelemetry, BuildTerminalStatus,
    BuildTransition, BuildTransitionError, FreshnessAssessment, FreshnessFailureReason,
    FreshnessFinalization, FreshnessStatus, PreparedPublishIdentity, PublishAbortReason,
    PublishFailure, PublishFailureReason, PublishNotAttemptedReason, PublishOutcome,
    PublishOutcomeError, PublishRefusal, RecoveryNeeded, RestartGuidance, StaleReason,
};
pub use catalog_summary::{
    CatalogCoverage, CatalogCoverageSummary, CatalogEntryKey, CatalogEntryResolution,
    CatalogEntryStatus, CatalogFallbackCandidate, CatalogIdentity, CatalogInput, CatalogMatch,
    CatalogRecordStatus, CatalogResolution, CatalogResolutionPolicy, CatalogSummary,
    CatalogSummaryError, CatalogVariant, TranslationStatus,
};
pub use edit::{
    AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan, EditPrecondition, SourceEdit,
    SourceFingerprint, SourceRange, StableIdOccurrence, StableIdRefusal, StableIdTarget,
    plan_create_block_stub, plan_create_block_stub_in_range, plan_insert_missing_id,
    plan_insert_missing_ids, plan_insert_missing_ids_for_document,
    plan_insert_missing_ids_in_range, plan_reassign_stable_id, plan_rename_block,
    plan_rename_stable_id,
};
pub use input::{AuthoringRequest, DocumentVersion, OpenDocument, SavedDocument};
pub use query::{
    BlockTarget, ClauseKind, CompletionCandidate, CompletionCandidateDetail,
    CompletionCandidateKind, CompletionItem, CompletionSite, CompletionSiteKind, HoverInfo,
    MetadataValueDetail, NavigationResult, QueryClass, QueryResult, QueryUnavailableReason,
    SemanticFact, SemanticSymbolKind, SymbolIdentity, SymbolKind, SymbolLocation,
    SymbolQueryOptions, SymbolRole,
};
pub use recording::{
    ImportPlan, ImportRefusal, RecordingEntry, RecordingItem, RecordingState, RowRefusal,
    handoff_rows, plan_import, recording_items, recording_report,
};
pub use schema_summary::{
    AuthoringSchemaSummary, AvailabilityReasonSummary, ConditionSummary, EffectSummary,
    FreshnessSnapshotSide, MarkupSummary, MetadataDomainSummary, MetadataKeySummary,
    PresentationProjectorSummary, ProducerActionDescriptor, ProducerActionEvidence,
    ProducerActionEvidenceError, ProducerActionOperation, ProducerActionOutputEvidence,
    ProducerActionRequest, ProducerActionRequestError, ProducerActionRequestIdentity,
    ProducerActionResult, ProducerActionResultError, ProducerActionResultOutcome,
    ProducerActionStatus, ProducerCapabilityStatus, ProducerFailureEvidence,
    ProducerFingerprintScopes, ProducerFingerprintScopesError, ProducerLaunchSnapshot,
    ProducerLaunchSnapshotError, ProducerMetadataSummary, ProducerRetryGuidance,
    ProjectionQueryFunctionSummary, RegistrySummary, SchemaAction, SchemaCapability,
    SchemaCapabilityUnavailableReason, SchemaDeclarationProvenance, SchemaDeclarationSummary,
    SchemaFingerprintSummary, SchemaFreshness, SchemaFreshnessEvidence,
    SchemaFreshnessSnapshotIdentity, SchemaFreshnessUnavailableReason, SchemaOwnership,
    SchemaSourceSummary, SchemaSummary, SchemaSummaryBuildError, SchemaSummaryEvidence,
    SchemaSummaryEvidenceBuilder, SchemaSummaryEvidenceError, SchemaTypeSummary, SpeakerSummary,
};
pub use snapshot::{
    AnalysisDelta, AuthoringSnapshot, CatalogSnapshot, DiagnosticCollection, DiagnosticIter,
    DocumentDelta, DocumentLayer, DocumentMetadata, DocumentSnapshot, EditSource,
};
pub use state::{AuthoringError, AuthoringKernel, SnapshotGeneration};
pub use summary::{
    AuthoringSummary, BlockDefinitionSummary, BlockReferenceSummary, FunctionReferenceKind,
    FunctionReferenceSummary, MetadataScalar, MetadataSummary, MetadataValue, MetadataValueKind,
    StableIdKind, StableIdSummary,
};

pub use control::{CancellationToken, Interrupted, WorkControl};

pub use query::AuthoringQuery;
