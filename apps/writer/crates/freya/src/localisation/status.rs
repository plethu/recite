//! One review state for passage labels and queue membership.
use super::{
    catalogue::Catalogue,
    messages::{MsgId, text},
};
use recite_core::PoEntryId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TranslationStatus {
    Untranslated,
    ReviewPending,
    Reviewed,
    SourceChanged,
    NeedsReview,
}
impl TranslationStatus {
    pub fn for_entry(catalogue: &Catalogue, id: PoEntryId) -> Self {
        let Some(draft) = catalogue.draft(id) else {
            return Self::Untranslated;
        };
        if draft.text.trim().is_empty() {
            Self::Untranslated
        } else if draft.reviewed && catalogue.changed(id) {
            Self::ReviewPending
        } else if draft.reviewed {
            Self::Reviewed
        } else if catalogue
            .document
            .entry(id)
            .is_some_and(|entry| !entry.previous().is_empty())
        {
            Self::SourceChanged
        } else {
            Self::NeedsReview
        }
    }
    pub fn needs_attention(self) -> bool {
        self != Self::Reviewed
    }
    pub fn label(self) -> String {
        text(match self {
            Self::Untranslated => MsgId::WriterUntranslated,
            Self::ReviewPending => MsgId::WriterReviewPending,
            Self::Reviewed => MsgId::WriterReviewed,
            Self::SourceChanged => MsgId::WriterSourceChanged,
            Self::NeedsReview => MsgId::WriterNeedsReview,
        })
    }
}
#[cfg(test)]
mod tests;
