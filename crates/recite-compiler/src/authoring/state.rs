use crate::authoring::CancellationToken;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::input::{AuthoringRequest, OpenDocument, SavedDocument};
use super::snapshot::{AnalysisDelta, AuthoringSnapshot};
use super::{AuthoringSummary, Interrupted, WorkControl};

mod preparation;
use crate::validation::ValidationParticipation;
use crate::validation::incremental::{ProjectFacts, ProjectIndex};
use recite_core::{Diagnostic, DocumentKey, schema::ProjectSchema};

/// A monotonic generation identifying one accepted authoring state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SnapshotGeneration(u64);

impl SnapshotGeneration {
    /// The initial generation before any document state has been accepted.
    #[must_use]
    pub const fn initial() -> Self {
        Self(0)
    }

    /// Creates a generation for a caller that stores one independently.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the underlying generation value.
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for SnapshotGeneration {
    fn default() -> Self {
        Self::initial()
    }
}

impl std::fmt::Display for SnapshotGeneration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Typed failures that reject a replacement without changing kernel state.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum AuthoringError {
    #[error(transparent)]
    Interrupted(#[from] Interrupted),
    #[error("expected snapshot generation {expected}, but current generation is {actual}")]
    GenerationMismatch {
        expected: SnapshotGeneration,
        actual: SnapshotGeneration,
    },
    #[error("snapshot generation {current} cannot advance further")]
    GenerationExhausted { current: SnapshotGeneration },
    #[error("saved document {key} was supplied more than once")]
    DuplicateSavedDocument { key: DocumentKey },
    #[error("open document {key} was supplied more than once")]
    DuplicateOpenDocument { key: DocumentKey },
    #[error("open document {key} reused version {version} with different text")]
    OverlayVersionConflict {
        key: DocumentKey,
        version: super::DocumentVersion,
    },
    #[error("open document {key} version {received} is not greater than active version {previous}")]
    StaleOverlayVersion {
        key: DocumentKey,
        previous: super::DocumentVersion,
        received: super::DocumentVersion,
    },
}

/// One concrete synchronous owner for effective authoring analysis state.
pub struct AuthoringKernel {
    saved: BTreeMap<DocumentKey, SavedDocument>,
    open: BTreeMap<DocumentKey, OpenDocument>,
    analyses: BTreeMap<DocumentKey, DocumentAnalysis>,
    snapshot: AuthoringSnapshot,
    project_diagnostics: Arc<BTreeMap<DocumentKey, Vec<Diagnostic>>>,
    project_index: Arc<ProjectIndex>,
    schema: Option<Arc<ProjectSchema>>,
    project_complete: bool,
}

impl Default for AuthoringKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthoringKernel {
    /// Creates an empty kernel with schema-free compiler validation.
    #[must_use]
    pub fn new() -> Self {
        let generation = SnapshotGeneration::initial();
        Self {
            saved: BTreeMap::new(),
            open: BTreeMap::new(),
            analyses: BTreeMap::new(),
            project_diagnostics: Arc::default(),
            project_index: Arc::default(),
            snapshot: AuthoringSnapshot::new(generation, Vec::new(), None, true),
            schema: None,
            project_complete: true,
        }
    }

    /// Creates an empty kernel using a caller-owned immutable project schema.
    #[must_use]
    pub fn with_schema(schema: ProjectSchema) -> Self {
        let mut kernel = Self::new();
        let schema = Arc::new(schema);
        kernel.schema = Some(Arc::clone(&schema));
        kernel.snapshot.schema = Some(schema);
        kernel
    }

    /// Returns the current deterministic snapshot.
    #[must_use]
    pub const fn snapshot(&self) -> &AuthoringSnapshot {
        &self.snapshot
    }

    /// Returns whether the current authoring state covers the complete
    /// project input set.
    #[must_use]
    pub const fn project_complete(&self) -> bool {
        self.project_complete
    }

    /// Replaces the saved and open input set transactionally.
    ///
    /// The request-owned project-completeness setting is accepted alongside
    /// the documents and controls whether project-wide validation is
    /// authoritative or indeterminate.
    pub fn apply(&mut self, request: AuthoringRequest) -> Result<AnalysisDelta, AuthoringError> {
        self.apply_request(request)
    }

    /// Replaces the input set as an incomplete project request while retaining
    /// file-local analysis. Project-wide checks are left indeterminate until
    /// all sources participate.
    pub fn apply_with_incomplete_project(
        &mut self,
        request: AuthoringRequest,
    ) -> Result<AnalysisDelta, AuthoringError> {
        self.apply_request(request.with_project_completeness(false))
    }

    /// Prepare an independent candidate, sharing unchanged document analyses.
    /// The receiver is unchanged on success, interruption, or invalid input.
    pub fn updated(
        &self,
        request: AuthoringRequest,
        control: &dyn WorkControl,
    ) -> Result<Self, AuthoringError> {
        self.prepare(request, control).map(|(kernel, _)| kernel)
    }

    /// Atomically install a completed analysis; interruption retains all state.
    pub fn apply_with_control(
        &mut self,
        request: AuthoringRequest,
        control: &dyn WorkControl,
    ) -> Result<AnalysisDelta, AuthoringError> {
        let (candidate, delta) = self.prepare(request, control)?;
        control.checkpoint()?;
        *self = candidate;
        Ok(delta)
    }

    fn apply_request(
        &mut self,
        request: AuthoringRequest,
    ) -> Result<AnalysisDelta, AuthoringError> {
        self.apply_with_control(request, &CancellationToken::new())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DocumentAnalysis {
    pub(crate) regions: Arc<[super::engine::CachedRegion]>,
    pub(crate) project_facts: Arc<ProjectFacts>,
    pub(crate) source: Arc<recite_core::SourceLineIndex>,
    pub(crate) source_fingerprint: super::SourceFingerprint,
    pub(crate) parse_diagnostics: Arc<[Diagnostic]>,
    pub(crate) local_diagnostics: Arc<[Diagnostic]>,
    pub(crate) summary: Arc<AuthoringSummary>,
    pub(crate) participation: ValidationParticipation,
    pub(crate) byte_len: usize,
    pub(crate) line_count: usize,
}

#[cfg(test)]
mod tests;
