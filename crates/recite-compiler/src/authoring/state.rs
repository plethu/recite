use std::collections::BTreeMap;
use std::sync::Arc;

use super::AuthoringSummary;
use super::engine::{build_delta, build_documents, rebuild_analyses, validate_analyses};
use super::input::{AuthoringRequest, OpenDocument, SavedDocument};
use super::input_state::{
    changed_keys, effective_documents, unique_open, unique_saved, validate_overlay_versions,
};
use super::snapshot::{AnalysisDelta, AuthoringSnapshot};
use crate::ValidationParticipation;
use recite_core::{Diagnostic, DocumentKey, ProjectSchema, SourceFile};

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

    fn apply_request(
        &mut self,
        request: AuthoringRequest,
    ) -> Result<AnalysisDelta, AuthoringError> {
        let (expected_generation, saved_documents, open_documents, project_complete) =
            request.into_parts();
        if expected_generation != self.snapshot.generation() {
            return Err(AuthoringError::GenerationMismatch {
                expected: expected_generation,
                actual: self.snapshot.generation(),
            });
        }

        let saved = unique_saved(saved_documents)?;
        let open = unique_open(open_documents)?;
        validate_overlay_versions(&self.open, &open)?;
        if saved == self.saved && open == self.open && project_complete == self.project_complete {
            return Ok(AnalysisDelta::empty(
                self.snapshot.generation(),
                self.snapshot.generation(),
            ));
        }
        let generation = SnapshotGeneration(self.snapshot.generation().0.checked_add(1).ok_or(
            AuthoringError::GenerationExhausted {
                current: self.snapshot.generation(),
            },
        )?);

        let old_effective = effective_documents(&self.saved, &self.open);
        let new_effective = effective_documents(&saved, &open);
        let mut changed_inputs = changed_keys(&self.saved, &self.open, &saved, &open);
        if project_complete != self.project_complete {
            changed_inputs.extend(old_effective.keys().map(|key| (*key).clone()));
            changed_inputs.extend(new_effective.keys().map(|key| (*key).clone()));
        }
        let analyses = rebuild_analyses(
            std::mem::take(&mut self.analyses),
            &old_effective,
            &new_effective,
        );
        let semantic = validate_analyses(&analyses, self.schema.as_deref(), project_complete);
        let documents = build_documents(&new_effective, &analyses, &semantic, &self.snapshot);
        let (changed, removed) = build_delta(changed_inputs, &self.snapshot, &documents);
        let delta = AnalysisDelta::new(self.snapshot.generation(), generation, changed, removed);

        self.saved = saved;
        self.open = open;
        self.analyses = analyses;
        self.project_complete = project_complete;
        self.snapshot =
            AuthoringSnapshot::new(generation, documents, self.schema.clone(), project_complete);
        Ok(delta)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct DocumentAnalysis {
    pub(crate) source_file: SourceFile,
    pub(crate) source_text: Arc<str>,
    pub(crate) parse_diagnostics: Arc<[Diagnostic]>,
    pub(crate) summary: Arc<AuthoringSummary>,
    pub(crate) participation: ValidationParticipation,
    pub(crate) byte_len: usize,
    pub(crate) line_count: usize,
}

#[cfg(test)]
mod tests;
