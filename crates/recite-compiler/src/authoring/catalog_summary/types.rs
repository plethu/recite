use recite_core::{PoDocument, PoDocumentFingerprint};

use super::coverage::{CatalogCoverage, CatalogEntryStatus};
use super::error::CatalogSummaryError;
use super::resolution::{CatalogEntryResolution, CatalogResolution};

/// A caller-owned identity for one dialogue PO catalogue.
///
/// The locale is supplied by the caller rather than inferred from a file name,
/// PO header, process environment, or operating-system locale.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct CatalogIdentity {
    id: String,
    locale: recite_core::LocaleId,
}

impl CatalogIdentity {
    /// Create an identity with a non-empty stable ID and explicit locale.
    pub fn new(
        id: impl Into<String>,
        locale: recite_core::LocaleId,
    ) -> Result<Self, CatalogSummaryError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(CatalogSummaryError::EmptyCatalogIdentity);
        }
        Ok(Self { id, locale })
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub const fn locale(&self) -> &recite_core::LocaleId {
        &self.locale
    }
}

/// A lossless PO document paired with its explicit authoring identity.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogInput {
    pub(super) identity: CatalogIdentity,
    pub(super) document: PoDocument,
}

impl CatalogInput {
    #[must_use]
    pub const fn new(identity: CatalogIdentity, document: PoDocument) -> Self {
        Self { identity, document }
    }

    /// Convenience constructor using the document's source name as identity.
    pub fn from_document(
        locale: recite_core::LocaleId,
        document: PoDocument,
    ) -> Result<Self, CatalogSummaryError> {
        let identity = CatalogIdentity::new(document.source_name(), locale)?;
        Ok(Self::new(identity, document))
    }

    #[must_use]
    pub const fn identity(&self) -> &CatalogIdentity {
        &self.identity
    }

    #[must_use]
    pub const fn document(&self) -> &PoDocument {
        &self.document
    }
}

/// Deterministic projection of one input PO catalogue against the expected POT.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogSummary {
    pub(super) identity: CatalogIdentity,
    pub(super) fingerprint: PoDocumentFingerprint,
    pub(super) plural_forms: Option<usize>,
    pub(super) coverage: CatalogCoverage,
    pub(super) entries: Vec<CatalogEntryStatus>,
}

impl CatalogSummary {
    #[must_use]
    pub const fn identity(&self) -> &CatalogIdentity {
        &self.identity
    }

    #[must_use]
    pub const fn locale(&self) -> &recite_core::LocaleId {
        self.identity.locale()
    }

    #[must_use]
    pub fn id(&self) -> &str {
        self.identity.id()
    }

    #[must_use]
    pub const fn fingerprint(&self) -> &PoDocumentFingerprint {
        &self.fingerprint
    }

    #[must_use]
    pub const fn content_fingerprint(&self) -> &PoDocumentFingerprint {
        self.fingerprint()
    }

    /// Number of translation arms declared by the catalogue's validated
    /// `Plural-Forms` header, when it has one.
    #[must_use]
    pub const fn plural_forms(&self) -> Option<usize> {
        self.plural_forms
    }

    #[must_use]
    pub const fn coverage(&self) -> &CatalogCoverage {
        &self.coverage
    }

    #[must_use]
    pub const fn expected_count(&self) -> usize {
        self.coverage.expected_count()
    }

    #[must_use]
    pub const fn present_count(&self) -> usize {
        self.coverage.present_count()
    }

    #[must_use]
    pub const fn translated_count(&self) -> usize {
        self.coverage.translated_count()
    }

    #[must_use]
    pub const fn missing_count(&self) -> usize {
        self.coverage.missing_count()
    }

    #[must_use]
    pub const fn fuzzy_count(&self) -> usize {
        self.coverage.fuzzy_count()
    }

    #[must_use]
    pub const fn obsolete_count(&self) -> usize {
        self.coverage.obsolete_count()
    }

    #[must_use]
    pub const fn incomplete_plural_count(&self) -> usize {
        self.coverage.incomplete_plural_count()
    }

    #[must_use]
    pub const fn context_entry_count(&self) -> usize {
        self.coverage.context_entry_count()
    }

    #[must_use]
    pub const fn context_count(&self) -> usize {
        self.coverage.context_count()
    }

    #[must_use]
    pub const fn variant_entry_count(&self) -> usize {
        self.coverage.variant_entry_count()
    }

    #[must_use]
    pub const fn variant_count(&self) -> usize {
        self.coverage.variant_count()
    }

    /// Statuses are in expected POT order, including missing entries.
    #[must_use]
    pub fn entries(&self) -> &[CatalogEntryStatus] {
        &self.entries
    }
}

/// Alias for callers that use the shorter input name.
pub type DialogueCatalogInput = CatalogInput;

/// Alias for the per-catalogue projection.
pub type DialogueCatalog = CatalogSummary;

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
