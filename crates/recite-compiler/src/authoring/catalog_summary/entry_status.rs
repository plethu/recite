use recite_core::{PoDocument, PoEntry};

use super::entry::CatalogEntryKey;

/// Translation state for one expected entry in one catalogue.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TranslationStatus {
    /// There is no matching PO record.
    Missing,
    /// A matching record exists but has no usable translation.
    Untranslated,
    /// A complete active translation is present.
    Translated,
    /// A plural record exists but one or more required arms are empty.
    IncompletePlural {
        expected_arms: Option<usize>,
        present_arms: usize,
        translated_arms: usize,
    },
}

impl TranslationStatus {
    #[must_use]
    pub const fn is_translated(&self) -> bool {
        matches!(self, Self::Translated)
    }

    #[must_use]
    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }

    #[must_use]
    pub const fn is_incomplete_plural(&self) -> bool {
        matches!(self, Self::IncompletePlural { .. })
    }

    #[must_use]
    pub const fn expected_arms(&self) -> Option<usize> {
        match self {
            Self::IncompletePlural { expected_arms, .. } => *expected_arms,
            _ => None,
        }
    }

    #[must_use]
    pub const fn present_arms(&self) -> Option<usize> {
        match self {
            Self::IncompletePlural { present_arms, .. } => Some(*present_arms),
            _ => None,
        }
    }

    #[must_use]
    pub const fn translated_arms(&self) -> Option<usize> {
        match self {
            Self::IncompletePlural {
                translated_arms, ..
            } => Some(*translated_arms),
            _ => None,
        }
    }
}

/// Coverage status for one expected entry in one PO catalogue.
///
/// Fuzzy and obsolete flags are retained as independent facts. Such records
/// remain visible here even though the runtime provider intentionally excludes
/// them from lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CatalogEntryStatus {
    key: CatalogEntryKey,
    present: bool,
    translation: TranslationStatus,
    fuzzy: bool,
    obsolete: bool,
}

impl CatalogEntryStatus {
    #[must_use]
    pub const fn key(&self) -> &CatalogEntryKey {
        &self.key
    }

    #[must_use]
    pub const fn present(&self) -> bool {
        self.present
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
    pub const fn is_missing(&self) -> bool {
        !self.translation.is_translated()
    }

    #[must_use]
    pub const fn is_fuzzy(&self) -> bool {
        self.fuzzy
    }

    #[must_use]
    pub const fn fuzzy(&self) -> bool {
        self.is_fuzzy()
    }

    #[must_use]
    pub const fn is_obsolete(&self) -> bool {
        self.obsolete
    }

    #[must_use]
    pub const fn obsolete(&self) -> bool {
        self.is_obsolete()
    }

    pub(crate) fn build(
        key: &CatalogEntryKey,
        document: &PoDocument,
        plural_forms: Option<usize>,
    ) -> Self {
        let matching = document
            .entries()
            .iter()
            .filter(|entry| key.matches(entry, key.context()))
            .collect::<Vec<_>>();
        let active = matching.iter().copied().find(|entry| {
            !entry.is_obsolete() && !entry.flags().iter().any(|flag| flag == "fuzzy")
        });
        let fuzzy = matching
            .iter()
            .any(|entry| entry.flags().iter().any(|flag| flag == "fuzzy"));
        let obsolete = matching.iter().any(|entry| entry.is_obsolete());
        let translation = active.map_or_else(
            || {
                if matching.is_empty() {
                    TranslationStatus::Missing
                } else {
                    TranslationStatus::Untranslated
                }
            },
            |entry| translation_status(entry, plural_forms),
        );
        Self {
            key: key.clone(),
            present: !matching.is_empty(),
            translation,
            fuzzy,
            obsolete,
        }
    }
}

pub(crate) fn translation_status(
    entry: &PoEntry,
    plural_forms: Option<usize>,
) -> TranslationStatus {
    if entry.is_plural() {
        let translations = entry.plural_translations();
        let translated_arms = translations
            .iter()
            .filter(|translation| !translation.text().trim().is_empty())
            .count();
        let expected_arms = plural_forms;
        if expected_arms == Some(translated_arms)
            && expected_arms == Some(translations.len())
            && expected_arms.is_some()
        {
            TranslationStatus::Translated
        } else {
            TranslationStatus::IncompletePlural {
                expected_arms,
                present_arms: translations.len(),
                translated_arms,
            }
        }
    } else if entry
        .translation()
        .is_some_and(|translation| !translation.trim().is_empty())
    {
        TranslationStatus::Translated
    } else {
        TranslationStatus::Untranslated
    }
}
