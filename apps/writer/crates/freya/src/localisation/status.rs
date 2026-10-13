//! One review state for passage labels and queue membership.
use super::{
    catalogue::Catalogue,
    messages::{MsgId, text},
};
use recite_core::po::PoEntryId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TranslationStatus {
    Untranslated,
    ReviewPending,
    Reviewed,
    SourceChanged,
    NeedsReview,
}
impl TranslationStatus {
    pub fn entry_label(catalogue: &Catalogue, id: PoEntryId) -> String {
        let status = Self::for_entry(catalogue, id).label();
        if catalogue.changed(id) {
            format!("{status} · {}", text(MsgId::WriterUnsaved))
        } else {
            status
        }
    }
    pub fn for_entry(catalogue: &Catalogue, id: PoEntryId) -> Self {
        let Some(entry) = catalogue.document.entry(id) else {
            return Self::Untranslated;
        };
        if (!entry.is_plural() && entry.translation().is_none() && !catalogue.changed(id))
            || catalogue.forms(id).any(|text| text.trim().is_empty())
        {
            Self::Untranslated
        } else if catalogue.reviewed(id) && catalogue.changed(id) {
            Self::ReviewPending
        } else if catalogue.reviewed(id) {
            Self::Reviewed
        } else if !entry.previous().is_empty() {
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
