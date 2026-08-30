mod model;

pub use model::{DocumentLayer, DocumentMetadata, DocumentSnapshot};

use recite_core::{Diagnostic, DocumentKey};

use super::DocumentVersion;
use crate::ValidationParticipation;

/// Deterministically ordered view of all effective saved and open documents.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AuthoringSnapshot {
    generation: super::SnapshotGeneration,
    documents: Vec<DocumentSnapshot>,
    diagnostics: Vec<Diagnostic>,
}

impl AuthoringSnapshot {
    #[must_use]
    pub(crate) fn new(
        generation: super::SnapshotGeneration,
        documents: Vec<DocumentSnapshot>,
    ) -> Self {
        Self {
            generation,
            diagnostics: documents
                .iter()
                .flat_map(|document| document.diagnostics.iter().cloned())
                .collect(),
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

    pub(super) fn diagnostic_values(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

/// Coarse metadata delta produced by one accepted snapshot replacement.
///
/// The generation describes whole-snapshot freshness. Changed and removed
/// entries cover input metadata only; readers should reread the snapshot when
/// the generation changes because diagnostics may also change in unchanged
/// documents.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnalysisDelta {
    previous_generation: super::SnapshotGeneration,
    generation: super::SnapshotGeneration,
    changed: Vec<DocumentDelta>,
    removed: Vec<DocumentDelta>,
}

impl AnalysisDelta {
    #[must_use]
    pub(crate) fn new(
        previous_generation: super::SnapshotGeneration,
        generation: super::SnapshotGeneration,
        changed: Vec<DocumentDelta>,
        removed: Vec<DocumentDelta>,
    ) -> Self {
        Self {
            previous_generation,
            generation,
            changed,
            removed,
        }
    }

    pub(crate) fn empty(
        previous_generation: super::SnapshotGeneration,
        generation: super::SnapshotGeneration,
    ) -> Self {
        Self::new(previous_generation, generation, Vec::new(), Vec::new())
    }

    #[must_use]
    pub const fn previous_generation(&self) -> super::SnapshotGeneration {
        self.previous_generation
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
    byte_len: usize,
    line_count: usize,
    participation: ValidationParticipation,
) -> DocumentMetadata {
    DocumentMetadata {
        key,
        layer,
        version,
        byte_len,
        line_count,
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
