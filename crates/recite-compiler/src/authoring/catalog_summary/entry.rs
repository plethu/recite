use recite_core::PoEntry;

use crate::PotEntry;

/// The expected identity of a localisable gettext entry.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct CatalogEntryKey {
    context: String,
    source_text: String,
    plural_source_text: Option<String>,
}

impl CatalogEntryKey {
    pub(crate) fn from_pot_entry(
        entry: &PotEntry,
    ) -> Result<Self, super::super::CatalogSummaryError> {
        if entry.context.trim().is_empty() {
            return Err(super::super::CatalogSummaryError::EmptyExpectedContext);
        }
        if entry.source_text.trim().is_empty() {
            return Err(super::super::CatalogSummaryError::EmptyExpectedSourceText);
        }
        if entry
            .plural_source_text
            .as_deref()
            .is_some_and(|text| text.trim().is_empty())
        {
            return Err(super::super::CatalogSummaryError::EmptyExpectedPluralSourceText);
        }
        Ok(Self {
            context: entry.context.clone(),
            source_text: entry.source_text.clone(),
            plural_source_text: entry.plural_source_text.clone(),
        })
    }

    #[must_use]
    pub fn context(&self) -> &str {
        &self.context
    }

    #[must_use]
    pub fn source_text(&self) -> &str {
        &self.source_text
    }

    #[must_use]
    pub fn plural_source_text(&self) -> Option<&str> {
        self.plural_source_text.as_deref()
    }

    #[must_use]
    pub fn variant(&self) -> Option<&str> {
        self.context
            .split_once('&')
            .and_then(|(_, variant)| (!variant.is_empty()).then_some(variant))
    }

    #[must_use]
    pub const fn is_plural(&self) -> bool {
        self.plural_source_text.is_some()
    }

    pub(crate) fn matches(&self, entry: &PoEntry, context: &str) -> bool {
        !entry.is_header()
            && entry.context() == Some(context)
            && entry.source_text() == self.source_text
            && entry.plural_source_text() == self.plural_source_text()
    }
}
