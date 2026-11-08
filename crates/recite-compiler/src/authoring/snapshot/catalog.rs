use std::sync::Arc;

use recite_core::{DocumentKey, SourceLineIndex};

use super::super::{DocumentVersion, SourceFingerprint};
use super::DocumentLayer;

/// One effective dialogue PO catalogue retained by an authoring snapshot.
///
/// Catalogues are not analysed. The snapshot keeps their exact text so edit
/// plans that rewrite catalogue contexts remain conditional on it.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct CatalogSnapshot {
    key: DocumentKey,
    layer: DocumentLayer,
    version: Option<DocumentVersion>,
    source: Arc<SourceLineIndex>,
    source_fingerprint: SourceFingerprint,
}

impl CatalogSnapshot {
    pub(crate) fn new(
        key: DocumentKey,
        layer: DocumentLayer,
        version: Option<DocumentVersion>,
        text: &Arc<str>,
    ) -> Self {
        Self {
            key,
            layer,
            version,
            source_fingerprint: SourceFingerprint::for_source(text),
            source: Arc::new(SourceLineIndex::new(Arc::clone(text))),
        }
    }

    /// Whether this snapshot already describes the supplied input, so a
    /// refresh can reuse its line index and fingerprint.
    pub(crate) fn describes(
        &self,
        layer: DocumentLayer,
        version: Option<DocumentVersion>,
        text: &str,
    ) -> bool {
        self.layer == layer && self.version == version && self.source.source() == text
    }

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
    pub fn source_index(&self) -> &SourceLineIndex {
        &self.source
    }
    #[must_use]
    pub fn source_text(&self) -> &str {
        self.source.source()
    }
    pub(crate) fn source_fingerprint(&self) -> &SourceFingerprint {
        &self.source_fingerprint
    }
}

/// The current source of one document an edit plan may change: either a
/// dialogue source document or a catalogue.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct EditSource<'a> {
    key: &'a DocumentKey,
    layer: DocumentLayer,
    version: Option<DocumentVersion>,
    source: &'a SourceLineIndex,
    source_fingerprint: &'a SourceFingerprint,
}

impl<'a> EditSource<'a> {
    #[must_use]
    pub const fn key(&self) -> &'a DocumentKey {
        self.key
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
    pub const fn source_index(&self) -> &'a SourceLineIndex {
        self.source
    }
    #[must_use]
    pub fn source_text(&self) -> &'a str {
        self.source.source()
    }
    pub(crate) const fn source_fingerprint(&self) -> &'a SourceFingerprint {
        self.source_fingerprint
    }
}

impl super::AuthoringSnapshot {
    /// Effective catalogues sorted by key.
    #[must_use]
    pub fn catalogs(&self) -> &[CatalogSnapshot] {
        &self.catalogs
    }

    #[must_use]
    pub fn catalog(&self, key: &DocumentKey) -> Option<&CatalogSnapshot> {
        self.catalogs
            .binary_search_by(|catalog| catalog.key().cmp(key))
            .ok()
            .map(|index| &self.catalogs[index])
    }

    /// The source document or catalogue an edit plan addresses by `key`.
    #[must_use]
    pub fn edit_source(&self, key: &DocumentKey) -> Option<EditSource<'_>> {
        if let Some(document) = self.document(key) {
            return Some(EditSource {
                key: document.key(),
                layer: document.layer(),
                version: document.version(),
                source: document.source_index(),
                source_fingerprint: document.source_fingerprint(),
            });
        }
        self.catalog(key).map(|catalog| EditSource {
            key: catalog.key(),
            layer: catalog.layer(),
            version: catalog.version(),
            source: catalog.source_index(),
            source_fingerprint: catalog.source_fingerprint(),
        })
    }
}
