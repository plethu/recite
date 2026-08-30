use std::sync::Arc;

use recite_core::{Diagnostic, DocumentKey};

use super::super::{AuthoringSummary, DocumentVersion};
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
    pub(crate) key: DocumentKey,
    pub(crate) layer: DocumentLayer,
    pub(crate) version: Option<DocumentVersion>,
    pub(crate) byte_len: usize,
    pub(crate) line_count: usize,
    pub(crate) participation: ValidationParticipation,
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
    pub(super) metadata: DocumentMetadata,
    pub(super) diagnostics: Arc<[Diagnostic]>,
    pub(super) summary: Arc<AuthoringSummary>,
    pub(super) source_text: Arc<str>,
}

impl DocumentSnapshot {
    pub(crate) fn from_shared(
        metadata: DocumentMetadata,
        diagnostics: Arc<[Diagnostic]>,
        summary: Arc<AuthoringSummary>,
        source_text: Arc<str>,
    ) -> Self {
        Self {
            metadata,
            diagnostics,
            summary,
            source_text,
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
    pub(crate) fn shared_diagnostics(&self) -> &Arc<[Diagnostic]> {
        &self.diagnostics
    }
    #[must_use]
    pub fn summary(&self) -> &AuthoringSummary {
        &self.summary
    }
    #[must_use]
    pub fn source_text(&self) -> &str {
        &self.source_text
    }
}
