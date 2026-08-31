use recite_core::PoDocumentFingerprint;

use super::resolution::{CatalogEntryResolution, CatalogResolution};
use super::types::CatalogSummary;

/// Coverage and fallback information for one expected dialogue POT.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogCoverageSummary {
    pub(super) expected_fingerprint: PoDocumentFingerprint,
    pub(super) expected_count: usize,
    pub(super) catalogs: Vec<CatalogSummary>,
    pub(super) resolution: CatalogResolution,
    pub(super) entries: Vec<CatalogEntryResolution>,
}

/// Dialogue-oriented name for [`CatalogCoverageSummary`].
pub type DialogueCatalogSummary = CatalogCoverageSummary;

impl CatalogCoverageSummary {
    #[must_use]
    pub const fn expected_fingerprint(&self) -> &PoDocumentFingerprint {
        &self.expected_fingerprint
    }

    #[must_use]
    pub const fn expected_content_fingerprint(&self) -> &PoDocumentFingerprint {
        self.expected_fingerprint()
    }

    #[must_use]
    pub const fn expected_count(&self) -> usize {
        self.expected_count
    }

    /// Deterministically sorted catalogue projections.
    #[must_use]
    pub fn catalogs(&self) -> &[CatalogSummary] {
        &self.catalogs
    }

    /// The explicit locale/variant candidate sequence used for every expected
    /// entry. It is empty for source-only (`None`) locale policy.
    #[must_use]
    pub const fn resolution(&self) -> &CatalogResolution {
        &self.resolution
    }

    /// Per-entry resolution, in expected POT order.
    #[must_use]
    pub fn entries(&self) -> &[CatalogEntryResolution] {
        &self.entries
    }
}
