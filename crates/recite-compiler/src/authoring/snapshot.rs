use recite_core::{Diagnostic, DocumentKey};

use super::{AuthoringSummary, DocumentVersion};
use crate::ValidationParticipation;

/// Whether a snapshot document comes from saved state or an open overlay.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DocumentLayer {
    Saved,
    Open,
}

/// Stable host-neutral metadata for one effective document.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct DocumentMetadata {
    key: DocumentKey,
    layer: DocumentLayer,
    version: Option<DocumentVersion>,
    byte_len: usize,
    line_count: usize,
    participation: ValidationParticipation,
}

impl DocumentMetadata {
    #[must_use]
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }
    #[must_use]
    pub const fn layer(&self) -> DocumentLayer {
        self.layer
    }
    #[must_use]
    pub const fn version(&self) -> Option<DocumentVersion> {
        self.version
    }
    #[must_use]
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }
    #[must_use]
    pub const fn line_count(&self) -> usize {
        self.line_count
    }
    #[must_use]
    pub const fn participation(&self) -> ValidationParticipation {
        self.participation
    }
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.participation.ast_structure().is_complete()
    }
}

/// One document and its compiler-visible analysis in an authoring snapshot.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DocumentSnapshot {
    metadata: DocumentMetadata,
    diagnostics: Vec<Diagnostic>,
    summary: AuthoringSummary,
}

impl DocumentSnapshot {
    pub(crate) fn new(
        metadata: DocumentMetadata,
        diagnostics: Vec<Diagnostic>,
        summary: AuthoringSummary,
    ) -> Self {
        Self {
            metadata,
            diagnostics,
            summary,
        }
    }

    #[must_use]
    pub const fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }
    #[must_use]
    pub fn key(&self) -> &DocumentKey {
        self.metadata.key()
    }
    #[must_use]
    pub const fn layer(&self) -> DocumentLayer {
        self.metadata.layer()
    }
    #[must_use]
    pub const fn version(&self) -> Option<DocumentVersion> {
        self.metadata.version()
    }
    #[must_use]
    pub const fn participation(&self) -> ValidationParticipation {
        self.metadata.participation()
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    #[must_use]
    pub const fn summary(&self) -> &AuthoringSummary {
        &self.summary
    }
}

/// Deterministically ordered view of all effective saved and open documents.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AuthoringSnapshot {
    generation: super::SnapshotGeneration,
    documents: Vec<DocumentSnapshot>,
}

impl AuthoringSnapshot {
    #[must_use]
    pub(crate) fn new(
        generation: super::SnapshotGeneration,
        documents: Vec<DocumentSnapshot>,
    ) -> Self {
        Self {
            generation,
            documents,
        }
    }

    #[must_use]
    pub const fn generation(&self) -> super::SnapshotGeneration {
        self.generation
    }
    #[must_use]
    pub fn documents(&self) -> &[DocumentSnapshot] {
        &self.documents
    }
    #[must_use]
    pub fn document(&self, key: &DocumentKey) -> Option<&DocumentSnapshot> {
        self.documents
            .binary_search_by(|document| document.key().cmp(key))
            .ok()
            .map(|index| &self.documents[index])
    }
}

/// Coarse metadata delta produced by one accepted snapshot replacement.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnalysisDelta {
    generation: super::SnapshotGeneration,
    changed: Vec<DocumentDelta>,
    removed: Vec<DocumentDelta>,
}

impl AnalysisDelta {
    #[must_use]
    pub(crate) fn new(
        generation: super::SnapshotGeneration,
        changed: Vec<DocumentDelta>,
        removed: Vec<DocumentDelta>,
    ) -> Self {
        Self {
            generation,
            changed,
            removed,
        }
    }

    #[must_use]
    pub const fn generation(&self) -> super::SnapshotGeneration {
        self.generation
    }
    #[must_use]
    pub fn changed(&self) -> &[DocumentDelta] {
        &self.changed
    }
    #[must_use]
    pub fn removed(&self) -> &[DocumentDelta] {
        &self.removed
    }
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changed.is_empty() && self.removed.is_empty()
    }
}

/// Previous and current metadata for one changed or removed document key.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct DocumentDelta {
    key: DocumentKey,
    previous: Option<DocumentMetadata>,
    current: Option<DocumentMetadata>,
}

impl DocumentDelta {
    #[must_use]
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }
    #[must_use]
    pub fn previous(&self) -> Option<&DocumentMetadata> {
        self.previous.as_ref()
    }
    #[must_use]
    pub fn current(&self) -> Option<&DocumentMetadata> {
        self.current.as_ref()
    }
    #[must_use]
    pub fn previous_version(&self) -> Option<DocumentVersion> {
        self.previous
            .as_ref()
            .and_then(|metadata| metadata.version())
    }
    #[must_use]
    pub fn current_version(&self) -> Option<DocumentVersion> {
        self.current
            .as_ref()
            .and_then(|metadata| metadata.version())
    }
}

pub(crate) fn metadata(
    key: DocumentKey,
    layer: DocumentLayer,
    version: Option<DocumentVersion>,
    text: &str,
    participation: ValidationParticipation,
) -> DocumentMetadata {
    DocumentMetadata {
        key,
        layer,
        version,
        byte_len: text.len(),
        line_count: text.lines().count(),
        participation,
    }
}

pub(crate) fn delta(
    key: DocumentKey,
    previous: Option<DocumentMetadata>,
    current: Option<DocumentMetadata>,
) -> DocumentDelta {
    DocumentDelta {
        key,
        previous,
        current,
    }
}
