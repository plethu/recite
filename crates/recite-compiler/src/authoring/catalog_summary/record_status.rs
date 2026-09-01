use recite_core::PoEntry;

use super::coverage::{TranslationStatus, translation_status};

/// Lossless status inventory for every non-header PO record, including stale,
/// fuzzy, and obsolete records that are not present in the current POT.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogRecordStatus {
    entry: PoEntry,
    translation: TranslationStatus,
    fuzzy: bool,
    obsolete: bool,
}

impl CatalogRecordStatus {
    pub(crate) fn build(entry: &PoEntry, plural_forms: Option<usize>) -> Self {
        Self {
            entry: entry.clone(),
            translation: translation_status(entry, plural_forms),
            fuzzy: entry.flags().iter().any(|flag| flag == "fuzzy"),
            obsolete: entry.is_obsolete(),
        }
    }

    /// The original lossless PO record, retaining comments, unknown fields,
    /// source ranges, and all translation arms.
    #[must_use]
    pub const fn entry(&self) -> &PoEntry {
        &self.entry
    }

    #[must_use]
    pub const fn translation(&self) -> &TranslationStatus {
        &self.translation
    }

    #[must_use]
    pub const fn is_translated(&self) -> bool {
        self.translation.is_translated()
    }

    #[must_use]
    pub const fn is_incomplete_plural(&self) -> bool {
        self.translation.is_incomplete_plural()
    }

    #[must_use]
    pub const fn is_fuzzy(&self) -> bool {
        self.fuzzy
    }

    #[must_use]
    pub const fn is_obsolete(&self) -> bool {
        self.obsolete
    }

    #[must_use]
    pub fn context(&self) -> Option<&str> {
        self.entry.context()
    }

    #[must_use]
    pub fn variant(&self) -> Option<&str> {
        self.entry.variant()
    }
}
