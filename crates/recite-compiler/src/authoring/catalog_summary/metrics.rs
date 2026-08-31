use super::super::record_status::CatalogRecordStatus;
use super::entry_status::CatalogEntryStatus;

/// Aggregate expected-entry coverage for one catalogue.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogCoverage {
    expected_count: usize,
    present_count: usize,
    translated_count: usize,
    missing_count: usize,
    fuzzy_count: usize,
    obsolete_count: usize,
    incomplete_plural_count: usize,
    context_entry_count: usize,
    variant_entry_count: usize,
}

impl CatalogCoverage {
    pub(crate) fn from_entries(
        expected_count: usize,
        entries: &[CatalogEntryStatus],
        records: &[CatalogRecordStatus],
    ) -> Self {
        let present_count = entries.iter().filter(|entry| entry.present()).count();
        let translated_count = entries.iter().filter(|entry| entry.is_translated()).count();
        Self {
            expected_count,
            present_count,
            translated_count,
            // "Missing" is the authoring coverage gap: fuzzy, obsolete,
            // empty, and incomplete records are not usable translations.
            missing_count: expected_count.saturating_sub(translated_count),
            fuzzy_count: records.iter().filter(|entry| entry.is_fuzzy()).count(),
            obsolete_count: records.iter().filter(|entry| entry.is_obsolete()).count(),
            incomplete_plural_count: records
                .iter()
                .filter(|entry| entry.is_incomplete_plural())
                .count(),
            context_entry_count: records
                .iter()
                .filter(|entry| entry.context().is_some())
                .count(),
            variant_entry_count: records
                .iter()
                .filter(|entry| entry.variant().is_some())
                .count(),
        }
    }

    #[must_use]
    pub const fn expected_count(&self) -> usize {
        self.expected_count
    }

    #[must_use]
    pub const fn present_count(&self) -> usize {
        self.present_count
    }

    #[must_use]
    pub const fn translated_count(&self) -> usize {
        self.translated_count
    }

    /// Number of expected entries without a usable active translation.
    #[must_use]
    pub const fn missing_count(&self) -> usize {
        self.missing_count
    }

    #[must_use]
    pub const fn fuzzy_count(&self) -> usize {
        self.fuzzy_count
    }

    #[must_use]
    pub const fn obsolete_count(&self) -> usize {
        self.obsolete_count
    }

    #[must_use]
    pub const fn incomplete_plural_count(&self) -> usize {
        self.incomplete_plural_count
    }

    #[must_use]
    pub const fn context_entry_count(&self) -> usize {
        self.context_entry_count
    }

    #[must_use]
    pub const fn context_count(&self) -> usize {
        self.context_entry_count()
    }

    #[must_use]
    pub const fn variant_entry_count(&self) -> usize {
        self.variant_entry_count
    }

    #[must_use]
    pub const fn variant_count(&self) -> usize {
        self.variant_entry_count()
    }
}
