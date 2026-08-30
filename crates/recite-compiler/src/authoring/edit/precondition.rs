use recite_core::DocumentKey;

use super::super::DocumentVersion;
use super::fingerprint::SourceFingerprint;

/// Freshness data a host must retain for one planned document.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct EditPrecondition {
    document: DocumentKey,
    expected_version: Option<DocumentVersion>,
    source_fingerprint: SourceFingerprint,
}

impl EditPrecondition {
    pub(crate) fn new(
        document: DocumentKey,
        expected_version: Option<DocumentVersion>,
        source_fingerprint: SourceFingerprint,
    ) -> Self {
        Self {
            document,
            expected_version,
            source_fingerprint,
        }
    }

    #[must_use]
    pub fn document(&self) -> &DocumentKey {
        &self.document
    }

    #[must_use]
    pub const fn expected_version(&self) -> Option<DocumentVersion> {
        self.expected_version
    }

    #[must_use]
    pub fn source_fingerprint(&self) -> &SourceFingerprint {
        &self.source_fingerprint
    }
}
