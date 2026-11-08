use recite_core::DocumentKey;

mod equality;

/// A caller-owned saved logical document.
#[derive(Clone, Debug, Eq)]
pub struct SavedDocument {
    key: DocumentKey,
    text: std::sync::Arc<str>,
}

impl SavedDocument {
    /// Creates a saved document from a validated logical key and source text.
    #[must_use]
    pub fn new(key: DocumentKey, text: impl Into<String>) -> Self {
        Self {
            key,
            text: text.into().into(),
        }
    }

    /// Creates saved input by sharing an existing immutable text allocation.
    #[must_use]
    pub fn from_shared(key: DocumentKey, text: std::sync::Arc<str>) -> Self {
        Self { key, text }
    }

    /// Returns the document's logical key.
    #[must_use]
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }

    pub(super) fn shared_text(&self) -> &std::sync::Arc<str> {
        &self.text
    }

    /// Returns the complete saved source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// An open editor overlay over a logical document.
#[derive(Clone, Debug, Eq)]
pub struct OpenDocument {
    key: DocumentKey,
    version: DocumentVersion,
    text: std::sync::Arc<str>,
}

impl OpenDocument {
    /// Creates an overlay from a validated key, opaque version, and full text.
    #[must_use]
    pub fn new(key: DocumentKey, version: DocumentVersion, text: impl Into<String>) -> Self {
        Self {
            key,
            version,
            text: text.into().into(),
        }
    }

    /// Creates overlay input by sharing an existing immutable text allocation.
    #[must_use]
    pub fn from_shared(
        key: DocumentKey,
        version: DocumentVersion,
        text: std::sync::Arc<str>,
    ) -> Self {
        Self { key, version, text }
    }

    /// Returns the overlaid document's logical key.
    #[must_use]
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }

    /// Returns the caller-provided document version.
    #[must_use]
    pub const fn version(&self) -> DocumentVersion {
        self.version
    }

    pub(super) fn shared_text(&self) -> &std::sync::Arc<str> {
        &self.text
    }

    /// Returns the complete overlay source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// An opaque signed document version supplied by an authoring host.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DocumentVersion(i64);

impl DocumentVersion {
    /// Creates a document version without imposing a host-specific range.
    #[must_use]
    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    /// Returns the signed value for comparisons at a host boundary.
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for DocumentVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// The saved/overlay input set for one authoring refresh.
///
/// The request owns the project-completeness assertion used by authoring
/// validation; it is retained as part of the accepted kernel state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoringRequest {
    expected_generation: super::SnapshotGeneration,
    saved_documents: Vec<SavedDocument>,
    open_documents: Vec<OpenDocument>,
    saved_catalogs: Vec<SavedDocument>,
    open_catalogs: Vec<OpenDocument>,
    project_complete: bool,
}

/// An accepted request split into the owned input sets the kernel retains.
pub(crate) struct RequestParts {
    pub(crate) expected_generation: super::SnapshotGeneration,
    pub(crate) saved_documents: Vec<SavedDocument>,
    pub(crate) open_documents: Vec<OpenDocument>,
    pub(crate) saved_catalogs: Vec<SavedDocument>,
    pub(crate) open_catalogs: Vec<OpenDocument>,
    pub(crate) project_complete: bool,
}

impl Default for AuthoringRequest {
    fn default() -> Self {
        Self::new(
            super::SnapshotGeneration::initial(),
            std::iter::empty::<SavedDocument>(),
            std::iter::empty::<OpenDocument>(),
        )
    }
}

impl AuthoringRequest {
    /// Creates a full replacement request for the expected snapshot generation.
    ///
    /// New requests are marked as complete; use
    /// [`Self::with_project_completeness`] when the supplied documents are a
    /// partial project view.
    #[must_use]
    pub fn new(
        expected_generation: super::SnapshotGeneration,
        saved_documents: impl IntoIterator<Item = SavedDocument>,
        open_documents: impl IntoIterator<Item = OpenDocument>,
    ) -> Self {
        Self {
            expected_generation,
            saved_documents: saved_documents.into_iter().collect(),
            open_documents: open_documents.into_iter().collect(),
            saved_catalogs: Vec::new(),
            open_catalogs: Vec::new(),
            project_complete: true,
        }
    }

    /// Replaces the project's dialogue PO catalogues carried by this request.
    ///
    /// Catalogues are not analysed. The kernel retains their exact text so
    /// edit plans that also rewrite catalogue contexts, such as a stable-ID
    /// rename, stay conditional on the bytes they were planned against.
    /// Catalogue keys share the document key namespace and must not collide
    /// with a source document key.
    #[must_use]
    pub fn with_catalogs(
        mut self,
        saved: impl IntoIterator<Item = SavedDocument>,
        open: impl IntoIterator<Item = OpenDocument>,
    ) -> Self {
        self.saved_catalogs = saved.into_iter().collect();
        self.open_catalogs = open.into_iter().collect();
        self
    }

    /// Sets whether this request's documents cover the complete project.
    ///
    /// The setting is request-owned and participates in authoring state
    /// identity, so changing it can trigger a fresh semantic analysis even
    /// when the document bytes are unchanged.
    #[must_use]
    pub const fn with_project_completeness(mut self, project_complete: bool) -> Self {
        self.project_complete = project_complete;
        self
    }

    /// Returns whether this request contains the complete project input set.
    #[must_use]
    pub const fn project_complete(&self) -> bool {
        self.project_complete
    }

    /// Returns the generation against which this replacement is conditional.
    #[must_use]
    pub const fn expected_generation(&self) -> super::SnapshotGeneration {
        self.expected_generation
    }

    /// Returns caller-owned saved documents in supplied order.
    #[must_use]
    pub fn saved_documents(&self) -> &[SavedDocument] {
        &self.saved_documents
    }

    /// Returns caller-owned open overlays in supplied order.
    #[must_use]
    pub fn open_documents(&self) -> &[OpenDocument] {
        &self.open_documents
    }

    /// Returns caller-owned saved catalogues in supplied order.
    #[must_use]
    pub fn saved_catalogs(&self) -> &[SavedDocument] {
        &self.saved_catalogs
    }

    /// Returns caller-owned open catalogue overlays in supplied order.
    #[must_use]
    pub fn open_catalogs(&self) -> &[OpenDocument] {
        &self.open_catalogs
    }

    pub(crate) fn into_parts(self) -> RequestParts {
        RequestParts {
            expected_generation: self.expected_generation,
            saved_documents: self.saved_documents,
            open_documents: self.open_documents,
            saved_catalogs: self.saved_catalogs,
            open_catalogs: self.open_catalogs,
            project_complete: self.project_complete,
        }
    }
}
