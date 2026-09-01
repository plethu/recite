use recite_core::DocumentKey;

/// A caller-owned saved logical document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavedDocument {
    key: DocumentKey,
    text: String,
}

impl SavedDocument {
    /// Creates a saved document from a validated logical key and source text.
    #[must_use]
    pub fn new(key: DocumentKey, text: impl Into<String>) -> Self {
        Self {
            key,
            text: text.into(),
        }
    }

    /// Returns the document's logical key.
    #[must_use]
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }

    /// Returns the complete saved source text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

/// An open editor overlay over a logical document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenDocument {
    key: DocumentKey,
    version: DocumentVersion,
    text: String,
}

impl OpenDocument {
    /// Creates an overlay from a validated key, opaque version, and full text.
    #[must_use]
    pub fn new(key: DocumentKey, version: DocumentVersion, text: impl Into<String>) -> Self {
        Self {
            key,
            version,
            text: text.into(),
        }
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

/// The complete saved/overlay input set for one authoring refresh.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuthoringRequest {
    expected_generation: super::SnapshotGeneration,
    saved_documents: Vec<SavedDocument>,
    open_documents: Vec<OpenDocument>,
    project_complete: bool,
}

impl AuthoringRequest {
    /// Creates a full replacement request for the expected snapshot generation.
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
            project_complete: true,
        }
    }

    /// Marks whether the supplied documents cover the complete project.
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

    pub(crate) fn into_parts(
        self,
    ) -> (
        super::SnapshotGeneration,
        Vec<SavedDocument>,
        Vec<OpenDocument>,
        bool,
    ) {
        (
            self.expected_generation,
            self.saved_documents,
            self.open_documents,
            self.project_complete,
        )
    }
}
