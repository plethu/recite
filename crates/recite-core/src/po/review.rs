//! Review state is the gettext fuzzy flag, not a second catalogue format.
use super::{PoDocument, PoEditError, PoEntry, PoEntryId};

/// Saved review state of one PO record, shared by Writer and headless checks.
///
/// It is derived only from the record: translations, the gettext `fuzzy` flag and the `#|`
/// previous source values that a source refresh records when wording changes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum PoReviewState {
    /// A singular record has no translation, or a translated form is blank.
    Untranslated,
    /// The record is translated but flagged `fuzzy` without a recorded source change.
    NeedsReview,
    /// The record is flagged `fuzzy` and keeps the source it was translated from.
    SourceChanged,
    /// Every form is translated and the record is not `fuzzy`.
    Reviewed,
}

impl PoReviewState {
    /// Whether a person still has to act on the record.
    #[must_use]
    pub const fn needs_attention(self) -> bool {
        !matches!(self, Self::Reviewed)
    }
}

impl PoEntry {
    /// The record's saved review state.
    #[must_use]
    pub fn review_state(&self) -> PoReviewState {
        let mut forms = self.translation().into_iter().chain(
            self.plural_translations()
                .iter()
                .map(super::PoTranslation::text),
        );
        if (!self.is_plural() && self.translation().is_none())
            || forms.any(|text| text.trim().is_empty())
        {
            PoReviewState::Untranslated
        } else if !self.flags.iter().any(|flag| flag == "fuzzy") {
            PoReviewState::Reviewed
        } else if self.previous.is_empty() {
            PoReviewState::NeedsReview
        } else {
            PoReviewState::SourceChanged
        }
    }
}

impl PoDocument {
    /// Set an entry's gettext `fuzzy` flag, preserving other comments and flags.
    ///
    /// Clearing the flag revalidates the complete candidate. Invalid translations
    /// therefore remain drafts, and this document is unchanged on failure.
    pub fn set_fuzzy(&mut self, id: PoEntryId, fuzzy: bool) -> Result<(), PoEditError> {
        let entry = self.entry(id).ok_or(PoEditError::EntryNotFound(id))?;
        let existing = entry.flags.iter().any(|flag| flag == "fuzzy");
        if existing == fuzzy {
            return Ok(());
        }
        self.apply_updates([super::PoUpdate::Fuzzy { entry: id, fuzzy }])
    }
}

pub(super) fn format_fuzzy(source: &str, fuzzy: bool, obsolete: bool, line_ending: &str) -> String {
    let prefix = if obsolete { "#~ ," } else { "#," };
    let mut replacement = String::new();
    if fuzzy {
        replacement.push_str(&format!("{prefix} fuzzy{}", line_ending));
        replacement.push_str(source);
    } else {
        for line in source.split_inclusive('\n') {
            let trimmed = line.trim_start();
            let flags = if obsolete {
                trimmed
                    .strip_prefix("#~")
                    .and_then(|s| s.trim_start().strip_prefix(','))
            } else {
                trimmed
                    .strip_prefix('#')
                    .and_then(|s| s.trim_start().strip_prefix(','))
            };
            let Some(flags) = flags else {
                replacement.push_str(line);
                continue;
            };
            let flags: Vec<_> = flags.trim().split(',').map(str::trim).collect();
            if !flags.contains(&"fuzzy") {
                replacement.push_str(line);
                continue;
            }
            let retained: Vec<_> = flags.into_iter().filter(|f| *f != "fuzzy").collect();
            if !retained.is_empty() {
                replacement.push_str(&format!("{prefix} {}{}", retained.join(", "), line_ending));
            }
        }
    }
    replacement
}
