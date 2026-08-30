use std::collections::BTreeMap;

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
    delta: AnalysisDelta,
    schema: Option<ProjectSchema>,
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
            snapshot: AuthoringSnapshot::new(generation, Vec::new()),
            delta: AnalysisDelta::new(generation, Vec::new(), Vec::new()),
            schema: None,
        }
    }

    /// Creates an empty kernel using a caller-owned immutable project schema.
    #[must_use]
    pub fn with_schema(schema: ProjectSchema) -> Self {
        Self {
            schema: Some(schema),
            ..Self::new()
        }
    }

    /// Returns the current deterministic snapshot.
    #[must_use]
    pub const fn snapshot(&self) -> &AuthoringSnapshot {
        &self.snapshot
    }

    /// Returns the most recent accepted coarse metadata delta.
    #[must_use]
    pub const fn delta(&self) -> &AnalysisDelta {
        &self.delta
    }

    /// Replaces the complete saved and open input set transactionally.
    pub fn apply(&mut self, request: AuthoringRequest) -> Result<(), AuthoringError> {
        if request.expected_generation() != self.snapshot.generation() {
            return Err(AuthoringError::GenerationMismatch {
                expected: request.expected_generation(),
                actual: self.snapshot.generation(),
            });
        }

        let saved = unique_saved(request.saved_documents())?;
        let open = unique_open(request.open_documents())?;
        validate_overlay_versions(&self.open, &open)?;
        if saved == self.saved && open == self.open {
            return Ok(());
        }
        let generation = SnapshotGeneration(self.snapshot.generation().0.checked_add(1).ok_or(
            AuthoringError::GenerationExhausted {
                current: self.snapshot.generation(),
            },
        )?);

        let old_effective = effective_documents(&self.saved, &self.open);
        let new_effective = effective_documents(&saved, &open);
        let changed_inputs = changed_keys(&self.saved, &self.open, &saved, &open);
        let analyses = rebuild_analyses(
            std::mem::take(&mut self.analyses),
            &old_effective,
            &new_effective,
        );
        let semantic = validate_analyses(&analyses, self.schema.as_ref());
        let documents = build_documents(&new_effective, &analyses, &semantic);
        let (changed, removed) = build_delta(changed_inputs, &self.snapshot, &documents);

        self.saved = saved;
        self.open = open;
        self.analyses = analyses;
        self.snapshot = AuthoringSnapshot::new(generation, documents);
        self.delta = AnalysisDelta::new(generation, changed, removed);
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct DocumentAnalysis {
    pub(crate) text: String,
    pub(crate) source_file: SourceFile,
    pub(crate) parse_diagnostics: Vec<Diagnostic>,
    pub(crate) summary: AuthoringSummary,
    pub(crate) participation: ValidationParticipation,
}

#[cfg(test)]
mod tests;
