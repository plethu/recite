mod model;

pub use model::{DocumentLayer, DocumentMetadata, DocumentSnapshot};

use recite_core::ProjectSchema;
use recite_core::{Diagnostic, DocumentKey};
use std::sync::Arc;

use super::DocumentVersion;
use crate::ValidationParticipation;

/// Deterministically ordered view of all effective saved and open documents.
#[derive(Clone, Debug, PartialEq)]
pub struct AuthoringSnapshot {
    generation: super::SnapshotGeneration,
    documents: Vec<DocumentSnapshot>,
    pub(super) schema: Option<Arc<ProjectSchema>>,
    pub(super) project_complete: bool,
}

impl Default for AuthoringSnapshot {
    fn default() -> Self {
        Self {
            generation: super::SnapshotGeneration::initial(),
            documents: Vec::new(),
            schema: None,
            project_complete: true,
        }
    }
}

impl AuthoringSnapshot {
    #[must_use]
    pub(crate) fn new(
        generation: super::SnapshotGeneration,
        documents: Vec<DocumentSnapshot>,
        schema: Option<Arc<ProjectSchema>>,
        project_complete: bool,
    ) -> Self {
        Self {
            generation,
            documents,
            schema,
            project_complete,
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

    pub fn diagnostics(&self) -> DiagnosticCollection<'_> {
        DiagnosticCollection {
            documents: self.documents.iter(),
        }
    }
}

/// Deterministic borrowed aggregation over per-document diagnostics.
pub struct DiagnosticCollection<'a> {
    documents: std::slice::Iter<'a, DocumentSnapshot>,
}

impl<'a> DiagnosticCollection<'a> {
    #[must_use]
    pub fn iter(self) -> DiagnosticIter<'a> {
        DiagnosticIter {
            documents: self.documents,
            current: None,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.documents
            .clone()
            .map(|document| document.diagnostics().len())
            .sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documents
            .clone()
            .all(|document| document.diagnostics().is_empty())
    }
}

pub struct DiagnosticIter<'a> {
    documents: std::slice::Iter<'a, DocumentSnapshot>,
    current: Option<std::slice::Iter<'a, Diagnostic>>,
}

impl<'a> Iterator for DiagnosticIter<'a> {
    type Item = &'a Diagnostic;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(current) = &mut self.current
                && let Some(diagnostic) = current.next()
            {
                return Some(diagnostic);
            }
            let document = self.documents.next()?;
            self.current = Some(document.diagnostics().iter());
        }
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
